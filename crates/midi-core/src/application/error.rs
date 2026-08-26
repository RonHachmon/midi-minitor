//! The core's error type.
//!
//! # Why errors are values here
//!
//! Every fallible operation in this crate returns `Result` with a variant that
//! names what went wrong and carries what the caller needs to respond. Panicking
//! is not an option the constitution leaves open, and for good reason: each of
//! these failures is a user typing something into a field, which is ordinary
//! behaviour rather than a bug. A malformed hex prefix must leave the previous
//! filter running, and it can only do that if it arrives as a value.

use crate::domain::ids::SourceId;
use thiserror::Error;

/// Something the core refused to do, and why.
///
/// Each variant documents the condition that produces it and what the caller
/// should do about it, because a caller that cannot act on an error has been
/// handed a string with extra steps.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CoreError {
    /// A channel number outside 1–16 was supplied.
    ///
    /// Produced when the One Channel field is typed into directly. The caller
    /// should reject the entry and keep the previously selected channel, so the
    /// filter never applies a channel that cannot exist.
    #[error("channel {value} is outside the valid range 1-16")]
    ChannelOutOfRange {
        /// The rejected value, so the interface can explain the refusal at the
        /// control that caused it rather than showing a generic failure.
        value: u8,
    },

    /// A retention limit outside the supported range was supplied.
    ///
    /// Produced by zero, or by a value above the stated ceiling. The caller
    /// should keep the previous limit rather than accept a number the
    /// application will not honour.
    #[error("retention limit {value} is outside the supported range 1-100000")]
    RetentionOutOfRange {
        /// The rejected count.
        value: usize,
    },

    /// A hex prefix entry contained a character that is not a hex digit.
    ///
    /// The caller must report the entry as invalid and **leave the last valid
    /// filter in effect** — blanking the event list because of a typo would
    /// look like a crash.
    #[error("'{entry}' is not a valid hexadecimal prefix")]
    MalformedHexPrefix {
        /// The offending entry, echoed back so the message can quote it.
        entry: String,
    },

    /// A hex prefix entry held no hex digits at all.
    ///
    /// Distinct from [`Self::MalformedHexPrefix`] because the remedy differs:
    /// there is nothing to correct, the user simply has not finished typing.
    #[error("a hexadecimal prefix cannot be empty")]
    EmptyHexPrefix,

    /// Hiding this column would leave the table with no columns at all.
    ///
    /// The caller should refuse the interaction. Enforced in the core rather
    /// than in the interface so the rule cannot be bypassed by a future caller.
    #[error("at least one column must remain visible")]
    LastColumnVisible,

    /// Settings could not be read from or written to storage.
    ///
    /// The caller should surface this but keep running: the change it
    /// accompanies has already been applied in memory, and failing to remember
    /// a preference is not a reason to reject it.
    #[error("settings storage failed: {detail}")]
    SettingsStorage {
        /// What the storage layer reported, for the message.
        detail: String,
    },

    /// A source id arrived that the catalogue does not contain.
    ///
    /// Signals a stale interface referring to a source that no longer exists.
    /// The caller should refresh its catalogue rather than retry.
    #[error("no source with id {}", id.get())]
    UnknownSource {
        /// The unrecognised id.
        id: SourceId,
    },
}
