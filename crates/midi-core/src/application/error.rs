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

use crate::domain::filter::PrefixMode;
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

    /// A prefix rule was added whose prefix is already in the list.
    ///
    /// Produced under **either** kind, because the prefix is a rule's identity:
    /// two rules carrying the same prefix would make deletion ambiguous, and one
    /// of each kind would contradict outright. The caller should report the
    /// refusal and leave the list untouched; the remedy is to delete the listed
    /// rule or to enter a different prefix.
    #[error("a rule for '{prefix}' is already in the list")]
    DuplicatePrefixRule {
        /// The normalised prefix, for quoting back at the entry field.
        prefix: String,
        /// The kind the listed rule carries, so the message can name it.
        existing_kind: PrefixMode,
    },

    /// A prefix rule was added that contradicts one already in the list.
    ///
    /// Produced when the two prefixes overlap — one begins the other — and their
    /// kinds differ, which is the only way an event could match both a show-only
    /// rule and a hide rule. Refusing the pair here is what lets
    /// [`crate::domain::filter::DataPrefixFilter::admits`] need no precedence
    /// rule at all.
    ///
    /// Distinct from [`Self::DuplicatePrefixRule`] because the remedy differs:
    /// there is no duplicate to delete, one of the two prefixes has to be
    /// narrowed or one of the kinds changed. The caller should name both rules,
    /// since the conflicting one is not the one the user just typed.
    #[error(
        "'{prefix}' overlaps the existing rule for '{existing_prefix}', which has the opposite effect"
    )]
    ContradictoryPrefixRule {
        /// The normalised prefix being added.
        prefix: String,
        /// The kind being added.
        kind: PrefixMode,
        /// The listed prefix it overlaps.
        existing_prefix: String,
        /// That rule's kind.
        existing_kind: PrefixMode,
    },

    /// A prefix arrived that names no rule in the list.
    ///
    /// Signals an interface holding a rule list the core has since changed — the
    /// prefix-rule counterpart of [`Self::UnknownSource`]. The caller should
    /// refresh its view of the list rather than retry.
    #[error("no rule for '{prefix}' is in the list")]
    UnknownPrefixRule {
        /// The unrecognised prefix.
        prefix: String,
    },

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
    /// With real hardware this is ordinary rather than exceptional: a device can
    /// be unplugged between the webview rendering a row and the user clicking
    /// it. The caller should refresh its catalogue rather than retry.
    #[error("no source with id {}", id.get())]
    UnknownSource {
        /// The unrecognised id.
        id: SourceId,
    },

    /// A port exists but could not be opened for listening.
    ///
    /// Produced when another application holds the device exclusively, or when
    /// the operating system refuses the connection. The caller must **keep
    /// monitoring every port that did open** and surface this against the one
    /// that did not — one unavailable device is not a reason to stop watching
    /// the others.
    #[error("could not open '{name}': {detail}")]
    PortUnavailable {
        /// The port's display name, so the message can identify it to the user.
        name: String,
        /// What the operating system reported.
        detail: String,
    },

    /// The MIDI system itself could not be reached.
    ///
    /// Produced when the platform's MIDI service is unavailable or access was
    /// refused. The caller must present this as **distinct from having found no
    /// devices**: "there is nothing attached" and "I cannot see what is
    /// attached" call for entirely different responses from the user, and an
    /// empty list would conflate them.
    #[error("the MIDI system is unavailable: {detail}")]
    MidiSystemUnavailable {
        /// What the platform reported, for the message.
        detail: String,
    },
}
