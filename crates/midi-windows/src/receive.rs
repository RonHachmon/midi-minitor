//! The WinMM input callback and the worker thread it hands bytes to.
//!
//! # Why the callback does almost nothing
//!
//! Windows documents one hard constraint on a MIDI input callback: it may not
//! call any multimedia function, because doing so can deadlock. Returning a
//! System Exclusive buffer to the driver requires `midiInAddBuffer`, which is one
//! of those functions. That single fact forces the split — the callback cannot
//! finish the work, so it must hand it on.
//!
//! Having been forced to hand off, decoding on the worker as well is free, and it
//! keeps the callback short, which the platform's own guidance asks for
//! independently.
//!
//! This is the one structural difference from the macOS adapter, which decodes
//! inside its CoreMIDI callback because CoreMIDI imposes no such rule. It is
//! platform-imposed, not a preference.
//!
//! # What the split must not cost
//!
//! **Time.** The arrival timestamp is taken *in the callback*, never on the
//! worker, so however long a batch waits in the queue, the time shown is the time
//! the message arrived.
//!
//! **Order.** Every port feeds the same channel and the channel is first-in
//! first-out, so a burst across several devices reaches the decoder in the order
//! Windows delivered it.
//!
//! # Why some bytes bypass the decoder
//!
//! Windows does not merely hand over malformed input, it *labels* it — that is
//! the whole reason this adapter uses WinMM. Bytes the system has already called
//! invalid are reported as invalid, carrying exactly what the system supplied.
//! Feeding them back through the decoder would let this application disagree with
//! the system and render a plausible message where the system saw an error, which
//! would be an interpretation nobody can support.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};

use midi_core::application::ports::{Clock, EventSink};
use midi_core::domain::decoder::{expected_data_len, Decoded, MessageDecoder};
use midi_core::domain::event::MidiEvent;
// `Arrival` is aliased because this module already has an enum of that name for
// a *delivery*. The domain type is a pair of clock readings for one instant;
// keeping both spellings visible is clearer than renaming either.
use midi_core::domain::ids::{Arrival as ArrivalTime, EventId, SourceId};
use midi_core::domain::message::{InvalidReason, MidiMessage};
use windows::Win32::Media::Audio::{HMIDIIN, MIDIHDR};
use windows::Win32::Media::{MM_MIM_DATA, MM_MIM_ERROR, MM_MIM_LONGDATA, MM_MIM_LONGERROR};

use crate::sysex::BufferPool;

/// How many bytes of a packed short message Windows can hand over at once.
///
/// A status byte and at most two data bytes, which is every short message MIDI
/// defines. The fourth byte of the doubleword is never part of a message.
const PACKED_MESSAGE_BYTES: usize = 3;

/// One delivery from Windows, timestamped at the moment it arrived.
pub enum Arrival {
    /// Bytes Windows delivered as a well-formed message.
    Decodable {
        /// Which port it came from.
        source: SourceId,
        /// When it arrived, taken in the callback.
        at: ArrivalTime,
        /// The bytes exactly as Windows supplied them.
        bytes: Vec<u8>,
    },
    /// Bytes Windows itself reported as not forming a message.
    Rejected {
        /// Which port it came from.
        source: SourceId,
        /// When it arrived, taken in the callback.
        at: ArrivalTime,
        /// Whatever bytes Windows made available.
        bytes: Vec<u8>,
        /// Whether the extent of the invalid data could be established.
        reason: InvalidReason,
    },
    /// A System Exclusive buffer that must go back to the driver.
    ///
    /// Carried through the same channel as the bytes it delivered so that the
    /// buffer is never re-queued before its contents have been read.
    ReturnBuffer {
        /// The port that owns the buffer.
        port: PortHandle,
        /// The header to hand back.
        header: HeaderPtr,
    },
}

/// A port handle, marked as safe to move between this adapter's own threads.
///
/// # Why this wrapper exists
///
/// The handle is a raw pointer, so the compiler will not move it across threads
/// on its own account. Windows documents these handles as usable from any thread
/// — the callback receives one and the worker must act on it — so the constraint
/// is the compiler's conservatism about pointers, not the platform's.
#[derive(Clone, Copy)]
pub struct PortHandle(pub HMIDIIN);

// SAFETY: WinMM handles are process-wide and thread-agnostic; the API's own
// design hands one to a callback on a system thread and expects the application
// to use it elsewhere. Nothing here dereferences it.
unsafe impl Send for PortHandle {}

/// A queued System Exclusive header on its way back to the driver.
#[derive(Clone, Copy)]
pub struct HeaderPtr(pub *mut MIDIHDR);

// SAFETY: the pointer is only ever read on the worker thread, and only to hand
// it straight back to the driver. The driver has released it by the time it
// travels, because it only travels after being handed to the callback.
unsafe impl Send for HeaderPtr {}

/// Everything the input callback is allowed to touch.
///
/// Deliberately tiny: a source id, a clock, a channel, and a counter. Anything
/// more would be work happening on a thread that must not do work.
pub struct CallbackContext {
    /// Which source this port's events are attributed to.
    pub source: SourceId,
    /// The port itself, so a returned buffer knows where to go back to.
    pub port: PortHandle,
    /// Where arrival times come from.
    pub clock: Arc<dyn Clock>,
    /// The hand-off to the worker.
    pub arrivals: Sender<Arrival>,
    /// Counts deliveries lost because the worker could no longer be reached.
    pub dropped: Arc<AtomicU32>,
}

/// The callback Windows invokes for every delivery on one port.
///
/// # Safety
///
/// Windows calls this on one of its own threads with `instance` set to the
/// pointer given to `midiInOpen`, which this adapter guarantees is a live
/// [`CallbackContext`] for as long as the port is open. `param1` and `param2`
/// mean different things per message, as handled below.
pub unsafe extern "system" fn midi_in_proc(
    _port: HMIDIIN,
    message: u32,
    instance: usize,
    param1: usize,
    param2: usize,
) {
    let _ = param2;
    if instance == 0 {
        return;
    }
    // SAFETY: `instance` is the context pointer handed to `midiInOpen`, kept
    // alive by the adapter until after the port is closed and reset, which is
    // the last moment this callback can run.
    let context = unsafe { &*(instance as *const CallbackContext) };

    // The timestamp is taken here and nowhere else. Anything measured on the
    // worker would include however long this delivery waited in the queue.
    let at = context.clock.arrival();

    let arrival = match message {
        MM_MIM_DATA => Arrival::Decodable {
            source: context.source,
            at,
            bytes: unpack_short(param1),
        },
        MM_MIM_ERROR => {
            let (bytes, reason) = unpack_invalid(param1);
            Arrival::Rejected {
                source: context.source,
                at,
                bytes,
                reason,
            }
        }
        MM_MIM_LONGDATA | MM_MIM_LONGERROR => {
            let header = param1 as *mut MIDIHDR;
            if header.is_null() {
                return;
            }
            // SAFETY: for these two messages Windows sets `param1` to a header
            // this adapter prepared and queued, and has finished writing to it
            // before calling. Only the two length fields and the data pointer are
            // read, and only here where the driver has handed it back.
            let bytes = unsafe {
                let recorded = (*header).dwBytesRecorded as usize;
                let data = (*header).lpData.0;
                if data.is_null() || recorded == 0 {
                    Vec::new()
                } else {
                    std::slice::from_raw_parts(data, recorded).to_vec()
                }
            };

            let carried = if message == MM_MIM_LONGDATA {
                Arrival::Decodable {
                    source: context.source,
                    at,
                    bytes,
                }
            } else {
                Arrival::Rejected {
                    source: context.source,
                    at,
                    bytes,
                    reason: InvalidReason::ReportedInvalid,
                }
            };

            // The bytes travel first, then the instruction to give the buffer
            // back. Reversing these would let the driver refill a buffer whose
            // contents had not been read yet.
            if context.arrivals.send(carried).is_err() {
                context.dropped.fetch_add(1, Ordering::Relaxed);
                return;
            }
            Arrival::ReturnBuffer {
                port: context.port,
                header: HeaderPtr(header),
            }
        }
        // Every other message — the port opening, closing, or reporting that a
        // buffer is partly filled — carries no MIDI the user asked to see.
        _ => return,
    };

    if context.arrivals.send(arrival).is_err() {
        // The worker has gone, which happens only while the adapter is stopping.
        // Counting it is what lets the application admit a loss rather than hide
        // one.
        context.dropped.fetch_add(1, Ordering::Relaxed);
    }
}

/// Splits a packed short message into exactly the bytes it contains.
///
/// # Why the doubleword is trimmed rather than passed on whole
///
/// Windows packs a short message into four bytes whatever its real length, so a
/// two-byte Program Change arrives with a third byte that is not part of it.
/// Passing that on would put a byte on screen that nothing transmitted. How many
/// bytes are meaningful follows from the status byte, which is MIDI knowledge and
/// is asked of the domain rather than restated here.
///
/// # Why an unrecognised status yields the status byte alone
///
/// The domain reports no length for a System Real Time status, because it
/// handles those separately — they carry no data bytes at all — and it reports
/// none for a status it does not recognise either. Both cases mean the same
/// thing here: *nothing is known about any byte after the first*. The only
/// honest answer is to pass on the one byte that is certainly a message and
/// discard the padding.
///
/// Defaulting to the full three bytes instead is not a harmless over-report. The
/// padding is zeroes, zero is a valid data byte, and the decoder carries running
/// status — so a Clock arriving after a Control Change would complete a second,
/// entirely fictional Control Change from two padding bytes. A monitor that
/// invents a message out of padding is the exact failure this application exists
/// to prevent, and it was found by driving real bytes through a loopback port
/// rather than by reading this function.
fn unpack_short(packed: usize) -> Vec<u8> {
    let bytes = (packed as u32).to_le_bytes();
    let status = bytes[0];

    let len = expected_data_len(status).map_or(1, |data| data + 1);
    bytes[..len.min(PACKED_MESSAGE_BYTES)].to_vec()
}

/// Splits a packed *invalid* message into what can honestly be shown of it.
///
/// Windows packs the offending bytes the same way but does not say how many are
/// meaningful. Where the first byte is a status this application recognises, its
/// length is implied and those bytes are reported. Where it is not, only the byte
/// Windows placed first is reported and the row says the rest could not be
/// determined — because the alternative is displaying up to three bytes that may
/// be nothing at all.
fn unpack_invalid(packed: usize) -> (Vec<u8>, InvalidReason) {
    let bytes = (packed as u32).to_le_bytes();
    let status = bytes[0];

    expected_data_len(status).map_or_else(
        || (vec![status], InvalidReason::ExtentUnknown),
        |data| {
            let len = (data + 1).min(PACKED_MESSAGE_BYTES);
            (bytes[..len].to_vec(), InvalidReason::ReportedInvalid)
        },
    )
}

/// Everything the worker thread needs to turn arrivals into events.
pub struct Worker {
    /// Where completed events go.
    pub events: EventSink,
    /// Per-source decoding state, so a transfer spanning deliveries is joined.
    pub decoders: Mutex<std::collections::HashMap<SourceId, MessageDecoder>>,
    /// The next event's identifier.
    pub next_event_id: Mutex<EventId>,
    /// Counts events lost because shared state could not be reached.
    pub dropped: Arc<AtomicU32>,
}

impl Worker {
    /// Drains arrivals until every sender has gone, in the order they arrived.
    pub fn run(&self, arrivals: &Receiver<Arrival>) {
        for arrival in arrivals {
            match arrival {
                Arrival::Decodable { source, at, bytes } => self.decode(source, at, &bytes),
                Arrival::Rejected {
                    source,
                    at,
                    bytes,
                    reason,
                } => self.publish(
                    source,
                    at,
                    MidiMessage::Invalid {
                        reason,
                        bytes: bytes.clone(),
                    },
                    bytes,
                ),
                // SAFETY: this instruction is only ever queued by the callback,
                // immediately after the driver handed that exact header back for
                // that exact port, and it is queued behind the bytes it carried —
                // so the contents have been read and the driver is not holding it.
                Arrival::ReturnBuffer { port, header } => unsafe {
                    BufferPool::requeue(port.0, header.0);
                },
            }
        }
    }

    /// Runs bytes through the domain decoder and publishes what it completed.
    fn decode(&self, source: SourceId, at: ArrivalTime, bytes: &[u8]) {
        let Ok(mut decoders) = self.decoders.lock() else {
            self.dropped.fetch_add(1, Ordering::Relaxed);
            return;
        };
        let outcomes = decoders.entry(source).or_default().feed(bytes);
        drop(decoders);

        for outcome in outcomes {
            self.deliver(source, at, outcome);
        }
    }

    /// Flushes a departing source's half-finished transfer, if it had one.
    ///
    /// Called when a port closes, so an interrupted dump is *reported* rather
    /// than held forever waiting for an end that will not come.
    pub fn abandon(&self, source: SourceId, at: ArrivalTime) {
        let Ok(mut decoders) = self.decoders.lock() else {
            return;
        };
        let outcome = decoders.get_mut(&source).and_then(MessageDecoder::abandon);
        decoders.remove(&source);
        drop(decoders);

        if let Some(outcome) = outcome {
            self.deliver(source, at, outcome);
        }
    }

    /// Turns one decoded outcome into an event.
    ///
    /// Every outcome becomes a row, including the three that are not valid
    /// messages. That is the point of the decoder reporting them as values.
    fn deliver(&self, source: SourceId, at: ArrivalTime, outcome: Decoded) {
        let (message, raw) = match outcome {
            Decoded::Message { message, raw } => (message, raw),
            Decoded::Invalid { reason, raw } => (
                MidiMessage::Invalid {
                    reason,
                    bytes: raw.clone(),
                },
                raw,
            ),
            Decoded::SysexTruncated { raw, true_len: _ } | Decoded::SysexIncomplete { raw } => (
                MidiMessage::Invalid {
                    reason: InvalidReason::TruncatedData,
                    bytes: raw.clone(),
                },
                raw,
            ),
        };
        self.publish(source, at, message, raw);
    }

    /// Numbers an event and hands it to the sink.
    fn publish(&self, source: SourceId, at: ArrivalTime, message: MidiMessage, raw: Vec<u8>) {
        let Ok(mut next) = self.next_event_id.lock() else {
            self.dropped.fetch_add(1, Ordering::Relaxed);
            return;
        };
        let id = *next;
        *next = next.next();
        drop(next);

        (self.events)(MidiEvent::new(id, at, source, message, raw));
    }
}
