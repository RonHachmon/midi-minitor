//! One observed MIDI event.

use super::ids::{ChannelNumber, EventId, SourceId, Timestamp};
use super::message::MidiMessage;

/// A single message as observed from a source, at a moment in time.
///
/// # Why the raw bytes are stored but the display values are not
///
/// The Chan cell and the Data cell are functions of [`Self::message`], so each is
/// computed on demand. Caching them would create several representations of one
/// fact that could disagree after a change.
///
/// The raw bytes are different, and the distinction matters. They are **not**
/// derivable from the message, because interpretation is lossy in two ordinary
/// cases: under running status the wire carries no status byte, and a `Note On`
/// with velocity zero means a release, so re-encoding it would produce an `8n`
/// status the device never sent. A monitor that showed reconstructed bytes would
/// be lying in exactly the situations someone opened it to investigate — so the
/// bytes that arrived are kept, and they are the authority for the raw view and
/// for the hex prefix filter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MidiEvent {
    /// Arrival order and identity.
    ///
    /// Monotonic across the session: it keys the rendered list and lets the
    /// webview discard batches that were in flight when a settings change
    /// replaced the visible events.
    pub id: EventId,
    /// When the event arrived, to the millisecond the Time column displays.
    pub timestamp: Timestamp,
    /// Which source produced it.
    pub source: SourceId,
    /// The interpretation — the authority for the Message, Chan, and Data columns.
    pub message: MidiMessage,
    /// The bytes exactly as they arrived, unaltered in value or order.
    pub raw: Vec<u8>,
}

impl MidiEvent {
    /// Assembles an event from an interpretation and the bytes it came from.
    #[must_use]
    pub const fn new(
        id: EventId,
        timestamp: Timestamp,
        source: SourceId,
        message: MidiMessage,
        raw: Vec<u8>,
    ) -> Self {
        Self {
            id,
            timestamp,
            source,
            message,
            raw,
        }
    }

    /// The channel for the Chan column, or [`None`] when the message carries none.
    #[must_use]
    pub const fn channel(&self) -> Option<ChannelNumber> {
        self.message.channel()
    }

    /// The uppercase, separator-free hex string the prefix filter matches against.
    ///
    /// Formatted from the received bytes rather than from the message, so a user
    /// reading this string is reading what the device actually transmitted and can
    /// type any leading portion of it into the prefix filter with confidence.
    #[must_use]
    pub fn raw_hex(&self) -> String {
        let mut hex = String::with_capacity(self.raw.len() * 2);
        for byte in &self.raw {
            // Two uppercase hex digits per byte, zero padded, so nibble positions
            // line up and prefix matching stays a plain string test.
            hex.push_str(&format!("{byte:02X}"));
        }
        hex
    }
}
