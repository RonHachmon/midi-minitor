//! One observed MIDI event.

use super::ids::{ChannelNumber, EventId, SourceId, Timestamp};
use super::message::MidiMessage;

/// A single message as observed from a source, at a moment in time.
///
/// # Why the derived values are not stored
///
/// The Chan cell, the Data cell, and the raw hex string are all functions of
/// [`Self::message`], so each is computed on demand rather than cached
/// alongside it. Caching them would create several representations of one fact
/// that could disagree after a change — and with no test suite, a disagreement
/// between a stored display string and the message it came from would surface
/// as a user seeing the wrong value rather than as a failing assertion.
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
    /// The message itself — the authority for every derived column.
    pub message: MidiMessage,
}

impl MidiEvent {
    /// Assembles an event.
    #[must_use]
    pub const fn new(
        id: EventId,
        timestamp: Timestamp,
        source: SourceId,
        message: MidiMessage,
    ) -> Self {
        Self {
            id,
            timestamp,
            source,
            message,
        }
    }

    /// The channel for the Chan column, or [`None`] when the message carries none.
    #[must_use]
    pub const fn channel(&self) -> Option<ChannelNumber> {
        self.message.channel()
    }

    /// The uppercase, separator-free hex string the prefix filter matches against.
    #[must_use]
    pub fn raw_hex(&self) -> String {
        self.message.raw_hex()
    }
}
