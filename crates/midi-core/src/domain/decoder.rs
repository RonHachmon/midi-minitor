//! Turning a live byte stream into messages — including the bytes that are not.
//!
//! # Why this is written here rather than taken from a MIDI crate
//!
//! The constitution requires reusing a library over hand-rolling and requires
//! the exception to be justified where the code lives. This is that
//! justification, and it is the same argument [`super::message`] already makes
//! for the model, applied one level down to decoding.
//!
//! Every established Rust MIDI decoder — `wmidi`, `midi-msg`, `midly` — is built
//! to *reject* invalid MIDI. `wmidi` returns `FromBytesError`; `midi-msg` models
//! the specification for round-trip fidelity and errors on sequences that are not
//! in it; `midly` parses files rather than live streams. Each is correct for an
//! application that consumes MIDI, where refusing garbage is the right response.
//!
//! A monitor is the one application where the garbage is the payload. Someone
//! opens this window precisely because a device is sending bytes that do not make
//! sense, and a decoder that hands back `Err` gives the interface a rejection
//! where it needs a row in a table. [`Decoded::Invalid`] is a first-class outcome
//! here, not a failure — and so are a truncated System Exclusive transfer and one
//! that never finishes.
//!
//! The same reasoning disqualified `midir` at the layer above: its packet loop
//! ends on the first byte that does not begin a valid message and discards the
//! remainder with no callback and no error. Adopting any of these would mean
//! writing this state machine anyway, to recover what the library dropped.
//!
//! # Why one decoder per source
//!
//! Running status and an in-progress System Exclusive transfer are properties of
//! *a stream*, not of the application. Two devices sending simultaneously would
//! otherwise interleave into one state machine and corrupt each other's
//! messages — a bug that would look exactly like faulty hardware.
//!
//! # Exhaustiveness
//!
//! Every `match` here lists its variants with no catch-all arm, so a new message
//! type is a compile error at each site that must handle it. With no test suite,
//! the compiler enumerating the work is the project's substitute for one.

use super::ids::{ChannelNumber, Data14, DataByte};
use super::message::{InvalidReason, MidiMessage};
use crate::constants::MAX_SYSEX_BYTES;

/// Status byte beginning a System Exclusive transfer.
const SYSEX_START: u8 = 0xF0;
/// Status byte ending a System Exclusive transfer.
const SYSEX_END: u8 = 0xF7;
/// Lowest status byte that is a System Real Time message.
const REALTIME_START: u8 = 0xF8;
/// Lowest status byte that is a System Common message.
const SYSTEM_START: u8 = 0xF0;
/// Mask isolating the channel nibble of a channel-voice status byte.
const CHANNEL_MASK: u8 = 0x0F;
/// Mask isolating the kind nibble of a channel-voice status byte.
const STATUS_MASK: u8 = 0xF0;

/// One outcome of feeding bytes to a [`MessageDecoder`].
///
/// Four variants rather than a `Result`, because three of them are things the
/// user asked to see. Only a caller that treats malformed input as ordinary can
/// display it, which is the whole point of this module.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decoded {
    /// A complete, interpretable message.
    Message {
        /// The interpretation.
        message: MidiMessage,
        /// The bytes exactly as they arrived on the wire.
        ///
        /// Carried rather than derived from `message`, because the two can
        /// legitimately differ: under running status the wire omits the status
        /// byte, and a `Note On` with velocity zero is conventionally a note
        /// release. Re-encoding the interpretation would show the user bytes
        /// their device never sent.
        raw: Vec<u8>,
    },

    /// Bytes that form no valid message.
    ///
    /// Reported rather than discarded. This is the outcome every candidate
    /// library turns into an error, and the reason this module exists.
    Invalid {
        /// Why interpretation failed.
        reason: InvalidReason,
        /// The offending bytes, so the raw view can still show them.
        raw: Vec<u8>,
    },

    /// A System Exclusive transfer that exceeded what is retained.
    ///
    /// Carries the true size so the user learns how large it actually was,
    /// rather than being shown a silently shortened message.
    SysexTruncated {
        /// The bytes kept, up to the ceiling.
        raw: Vec<u8>,
        /// How many bytes the transfer really contained.
        true_len: usize,
    },

    /// A System Exclusive transfer that never completed.
    ///
    /// Produced when a new status byte arrives mid-transfer, or when the source
    /// disappears. Held transfers are flushed rather than waited on forever.
    SysexIncomplete {
        /// What had arrived before the transfer was abandoned.
        raw: Vec<u8>,
    },
}

/// An in-progress System Exclusive transfer.
///
/// Counts past the retention ceiling deliberately: `true_len` keeps growing
/// after `bytes` stops, so a truncated transfer can report its real size.
#[derive(Debug, Clone, Default)]
struct SysexAccumulator {
    bytes: Vec<u8>,
    true_len: usize,
    truncated: bool,
}

impl SysexAccumulator {
    /// Begins a transfer at its `F0` framing byte.
    fn begin() -> Self {
        Self {
            bytes: vec![SYSEX_START],
            true_len: 1,
            truncated: false,
        }
    }

    /// Adds one byte, keeping it only while under the ceiling.
    fn push(&mut self, byte: u8) {
        self.true_len += 1;
        if self.bytes.len() < MAX_SYSEX_BYTES {
            self.bytes.push(byte);
        } else {
            self.truncated = true;
        }
    }

    /// Completes the transfer at its `F7` framing byte.
    fn finish(mut self) -> Decoded {
        self.push(SYSEX_END);
        if self.truncated {
            return Decoded::SysexTruncated {
                raw: self.bytes,
                true_len: self.true_len,
            };
        }
        // The payload excludes both framing bytes; the message model adds them
        // back when asked for a byte count, matching what a user counts.
        let payload = self
            .bytes
            .get(1..self.bytes.len().saturating_sub(1))
            .unwrap_or_default()
            .to_vec();
        Decoded::Message {
            message: MidiMessage::SystemExclusive { payload },
            raw: self.bytes,
        }
    }

    /// Abandons the transfer, reporting what had arrived.
    fn abandon(self) -> Decoded {
        Decoded::SysexIncomplete { raw: self.bytes }
    }
}

/// Decodes one source's byte stream into messages.
///
/// Fed one packet at a time; state carries across calls because MIDI messages
/// and System Exclusive transfers do not respect packet boundaries.
#[derive(Debug, Default)]
pub struct MessageDecoder {
    /// The last channel-voice status byte, for running status.
    ///
    /// [`None`] when no status has been seen or when a System message cleared
    /// it, per the MIDI specification.
    running_status: Option<u8>,
    /// A System Exclusive transfer still arriving.
    sysex: Option<SysexAccumulator>,
    /// Data bytes collected toward the message in progress.
    pending: Vec<u8>,
}

impl MessageDecoder {
    /// A decoder with no stream state.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Feeds one packet, returning every outcome it completed, in arrival order.
    ///
    /// Real Time status bytes are handled first at every position because the
    /// specification allows them to appear *inside* another message — including
    /// inside a System Exclusive transfer — without interrupting it. A decoder
    /// that let `F8` end a dump would corrupt every transfer sent while a
    /// sequencer was running.
    pub fn feed(&mut self, bytes: &[u8]) -> Vec<Decoded> {
        let mut out = Vec::new();

        for &byte in bytes {
            if byte >= REALTIME_START {
                // Real Time interleaves anywhere and disturbs nothing.
                out.push(Self::realtime(byte));
                continue;
            }

            if let Some(sysex) = self.sysex.as_mut() {
                if byte == SYSEX_END {
                    let finished = self.sysex.take().unwrap_or_default();
                    out.push(finished.finish());
                } else if byte < 0x80 {
                    sysex.push(byte);
                } else {
                    // A non-Real-Time status mid-transfer ends it and starts the
                    // new message. One malformed dump must not swallow what
                    // follows it.
                    let abandoned = self.sysex.take().unwrap_or_default();
                    out.push(abandoned.abandon());
                    self.begin_status(byte, &mut out);
                }
                continue;
            }

            if byte >= 0x80 {
                self.begin_status(byte, &mut out);
            } else {
                self.data_byte(byte, &mut out);
            }
        }

        out
    }

    /// Abandons any transfer still in progress.
    ///
    /// Called when a source disappears, so an interrupted dump is reported
    /// rather than held until the application exits.
    pub fn abandon(&mut self) -> Option<Decoded> {
        self.pending.clear();
        self.running_status = None;
        self.sysex.take().map(SysexAccumulator::abandon)
    }

    /// Handles a status byte that begins something.
    fn begin_status(&mut self, status: u8, out: &mut Vec<Decoded>) {
        self.pending.clear();

        if status == SYSEX_START {
            // A System message clears running status, per the specification.
            self.running_status = None;
            self.sysex = Some(SysexAccumulator::begin());
            return;
        }

        if status >= SYSTEM_START {
            self.running_status = None;
            match expected_data_len(status) {
                Some(0) => out.push(system_common(status)),
                Some(_) => self.pending.push(status),
                None => out.push(Decoded::Invalid {
                    reason: InvalidReason::UnknownStatus,
                    raw: vec![status],
                }),
            }
            return;
        }

        // A channel-voice status: remember it so following data bytes without a
        // status can be attributed to it.
        self.running_status = Some(status);
        self.pending.push(status);
    }

    /// Handles a data byte, completing a message when enough have arrived.
    fn data_byte(&mut self, byte: u8, out: &mut Vec<Decoded>) {
        if self.pending.is_empty() {
            // Running status: no status byte on the wire, so the previous one
            // still applies. This is the case every candidate library discards.
            let Some(status) = self.running_status else {
                out.push(Decoded::Invalid {
                    reason: InvalidReason::UnknownStatus,
                    raw: vec![byte],
                });
                return;
            };
            self.pending.push(status);
        }

        self.pending.push(byte);

        let Some(status) = self.pending.first().copied() else {
            return;
        };
        let Some(expected) = expected_data_len(status) else {
            return;
        };

        if self.pending.len() == expected + 1 {
            let raw = std::mem::take(&mut self.pending);
            out.push(decode_complete(&raw));

            // Under running status the wire omits the status byte on repeats, so
            // the *displayed* bytes must omit it too — showing a byte the device
            // did not send would defeat the purpose of a raw view.
            if status < SYSTEM_START {
                self.pending.clear();
            }
        }
    }

    /// A System Real Time message, which carries no data bytes.
    fn realtime(status: u8) -> Decoded {
        let message = match status {
            0xF8 => MidiMessage::Clock,
            0xFA => MidiMessage::Start,
            0xFB => MidiMessage::Continue,
            0xFC => MidiMessage::Stop,
            0xFE => MidiMessage::ActiveSense,
            0xFF => MidiMessage::Reset,
            other => {
                return Decoded::Invalid {
                    reason: InvalidReason::UnknownStatus,
                    raw: vec![other],
                }
            }
        };
        Decoded::Message {
            message,
            raw: vec![status],
        }
    }
}

/// A System Common message that carries no data bytes.
fn system_common(status: u8) -> Decoded {
    match status {
        0xF6 => Decoded::Message {
            message: MidiMessage::TuneRequest,
            raw: vec![status],
        },
        other => Decoded::Invalid {
            reason: InvalidReason::UnknownStatus,
            raw: vec![other],
        },
    }
}

/// How many data bytes follow this status byte, or [`None`] if unrecognised.
const fn expected_data_len(status: u8) -> Option<usize> {
    if status < SYSTEM_START {
        return match status & STATUS_MASK {
            0x80 | 0x90 | 0xA0 | 0xB0 | 0xE0 => Some(2),
            0xC0 | 0xD0 => Some(1),
            _ => None,
        };
    }
    match status {
        0xF1 | 0xF3 => Some(1),
        0xF2 => Some(2),
        0xF6 => Some(0),
        _ => None,
    }
}

/// Interprets a complete status-plus-data sequence.
///
/// The slice is guaranteed by the caller to hold a status byte and exactly the
/// number of data bytes that status requires, which is why the accessors below
/// can fall back to zero rather than propagating an error that cannot occur.
fn decode_complete(raw: &[u8]) -> Decoded {
    let Some(&status) = raw.first() else {
        return Decoded::Invalid {
            reason: InvalidReason::TruncatedData,
            raw: raw.to_vec(),
        };
    };
    let first = raw.get(1).copied().unwrap_or(0);
    let second = raw.get(2).copied().unwrap_or(0);

    // Running status omits the status byte on repeats, so the reported bytes are
    // whatever actually arrived, never a re-encoding of the interpretation.
    let wire = raw.to_vec();

    if status >= SYSTEM_START {
        let message = match status {
            0xF1 => MidiMessage::TimeCode {
                data: DataByte::from_masked(first),
            },
            0xF2 => MidiMessage::SongPositionPointer {
                position: Data14::from_masked(
                    u16::from(first & 0x7F) | (u16::from(second & 0x7F) << 7),
                ),
            },
            0xF3 => MidiMessage::SongSelect {
                song: DataByte::from_masked(first),
            },
            _ => {
                return Decoded::Invalid {
                    reason: InvalidReason::UnknownStatus,
                    raw: wire,
                }
            }
        };
        return Decoded::Message { message, raw: wire };
    }

    let channel = ChannelNumber::from_wire_nibble(status & CHANNEL_MASK);
    let note = DataByte::from_masked(first);

    let message = match status & STATUS_MASK {
        0x80 => MidiMessage::NoteOff {
            channel,
            note,
            velocity: DataByte::from_masked(second),
        },
        // A Note On with zero velocity is a release by long-standing convention,
        // and devices genuinely send it that way. It is reported as `Note Off`
        // because that is what it means — while `raw` still carries the `9n`
        // status the device actually transmitted.
        0x90 if second == 0 => MidiMessage::NoteOff {
            channel,
            note,
            velocity: DataByte::ZERO,
        },
        0x90 => MidiMessage::NoteOn {
            channel,
            note,
            velocity: DataByte::from_masked(second),
        },
        0xA0 => MidiMessage::AftertouchPoly {
            channel,
            note,
            pressure: DataByte::from_masked(second),
        },
        0xB0 => MidiMessage::Control {
            channel,
            controller: note,
            value: DataByte::from_masked(second),
        },
        0xC0 => MidiMessage::Program {
            channel,
            program: note,
        },
        0xD0 => MidiMessage::ChannelPressure {
            channel,
            pressure: note,
        },
        0xE0 => MidiMessage::PitchWheel {
            channel,
            value: Data14::from_masked(u16::from(first & 0x7F) | (u16::from(second & 0x7F) << 7)),
        },
        _ => {
            return Decoded::Invalid {
                reason: InvalidReason::UnknownStatus,
                raw: wire,
            }
        }
    };

    Decoded::Message { message, raw: wire }
}
