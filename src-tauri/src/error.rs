//! The error type that crosses the IPC boundary.
//!
//! # Why this is a separate type from `CoreError`
//!
//! The core's error enum is free to change shape as the domain does. This one is
//! part of a published contract that generated TypeScript depends on, so it
//! changes only deliberately. Keeping them apart means a domain refactor cannot
//! silently alter what the webview must handle.
//!
//! Errors cross as a discriminated union rather than a string. A string tells the
//! interface that something failed; a variant tells it *which control* to
//! explain the failure at, and carries the offending value to quote back.

use crate::dto::PrefixModeDto;
use midi_core::application::error::CoreError;
use serde::Serialize;
use specta::Type;
use thiserror::Error;

/// Something the application refused to do, in the form the webview receives.
#[derive(Debug, Clone, Serialize, Type, Error)]
#[serde(tag = "type", content = "data", rename_all = "camelCase")]
pub enum IpcError {
    /// A channel number outside 1–16 was submitted.
    ///
    /// The One Channel field should reject the entry and keep the previous
    /// channel; the previous filter remains in effect.
    #[error("channel out of range")]
    ChannelOutOfRange {
        /// The rejected value, for quoting back at the field.
        value: u8,
    },

    /// A retention limit outside the supported range was submitted.
    #[error("retention limit out of range")]
    RetentionOutOfRange {
        /// The rejected count.
        value: u32,
    },

    /// A hexadecimal prefix entry contained an invalid character.
    ///
    /// The command fails **without mutating state**, so the previously applied
    /// prefix filter keeps running and the event list does not blank out.
    #[error("malformed hex prefix")]
    MalformedHexPrefix {
        /// The offending entry.
        entry: String,
    },

    /// A hexadecimal prefix entry held no hex digits.
    #[error("empty hex prefix")]
    EmptyHexPrefix,

    /// A rule for this prefix is already in the list, under either kind.
    ///
    /// The list is unchanged. The interface should say which rule it clashed
    /// with, so the user can delete that one or enter a different prefix without
    /// going to read the list themselves.
    // `rename_all` on the enum renames the *variants*; a struct variant's own
    // fields need their own attribute. Every other variant here happens to carry
    // single-word fields, so this is the first place it shows.
    #[error("duplicate prefix rule")]
    #[serde(rename_all = "camelCase")]
    DuplicatePrefixRule {
        /// The normalised prefix that was refused.
        prefix: String,
        /// The kind the listed rule carries.
        existing_kind: PrefixModeDto,
    },

    /// This rule overlaps a listed rule of the opposite kind.
    ///
    /// The list is unchanged. Both rules must be named when this is explained:
    /// the one it conflicts with is not the one the user just typed, so a message
    /// quoting only the entry would leave them to go and work out which listed
    /// rule was meant.
    #[error("contradictory prefix rule")]
    #[serde(rename_all = "camelCase")]
    ContradictoryPrefixRule {
        /// The normalised prefix that was refused.
        prefix: String,
        /// The kind that was refused.
        kind: PrefixModeDto,
        /// The listed prefix it overlaps.
        existing_prefix: String,
        /// That rule's kind.
        existing_kind: PrefixModeDto,
    },

    /// A prefix arrived that names no rule in the list.
    ///
    /// Reachable when the interface is working from a rule list the core has
    /// since changed. The caller should refresh rather than retry.
    #[error("unknown prefix rule")]
    UnknownPrefixRule {
        /// The unrecognised prefix.
        prefix: String,
    },

    /// Hiding this column would leave no columns visible.
    #[error("last visible column")]
    LastColumnVisible,

    /// A source id arrived that the catalogue does not contain.
    #[error("unknown source")]
    UnknownSource {
        /// The unrecognised id.
        id: u32,
    },

    /// A source-group identifier arrived that does not name a known group.
    ///
    /// Only reachable from a webview built against an older contract. The
    /// caller should refresh its catalogue rather than retry.
    #[error("unknown source group")]
    UnknownGroup {
        /// The unrecognised identifier, for the message.
        id: String,
    },

    /// Settings could not be read or written.
    ///
    /// Reported but not fatal: an unsaved preference should not stop the monitor.
    #[error("settings unavailable")]
    SettingsUnavailable {
        /// What the storage layer reported.
        detail: String,
    },

    /// The monitor's lock was poisoned by a panic on another thread.
    ///
    /// Unrecoverable in practice; surfaced rather than swallowed so a user sees
    /// a stated failure instead of a window that has quietly stopped updating.
    #[error("monitor unavailable")]
    MonitorUnavailable,
}

impl From<CoreError> for IpcError {
    /// Translates a core failure into its contract counterpart.
    ///
    /// Exhaustive with no catch-all arm: a new `CoreError` variant must be given
    /// a deliberate wire representation rather than being folded into a generic
    /// bucket the webview cannot act on.
    fn from(error: CoreError) -> Self {
        match error {
            CoreError::ChannelOutOfRange { value } => Self::ChannelOutOfRange { value },
            CoreError::RetentionOutOfRange { value } => Self::RetentionOutOfRange {
                // Widths differ across the boundary: JavaScript numbers cannot
                // carry a full `usize`, and a retention limit never approaches
                // this ceiling anyway.
                value: u32::try_from(value).unwrap_or(u32::MAX),
            },
            CoreError::MalformedHexPrefix { entry } => Self::MalformedHexPrefix { entry },
            CoreError::EmptyHexPrefix => Self::EmptyHexPrefix,
            CoreError::DuplicatePrefixRule {
                prefix,
                existing_kind,
            } => Self::DuplicatePrefixRule {
                prefix,
                existing_kind: existing_kind.into(),
            },
            CoreError::ContradictoryPrefixRule {
                prefix,
                kind,
                existing_prefix,
                existing_kind,
            } => Self::ContradictoryPrefixRule {
                prefix,
                kind: kind.into(),
                existing_prefix,
                existing_kind: existing_kind.into(),
            },
            CoreError::UnknownPrefixRule { prefix } => Self::UnknownPrefixRule { prefix },
            CoreError::LastColumnVisible => Self::LastColumnVisible,
            CoreError::SettingsStorage { detail } => Self::SettingsUnavailable { detail },
            CoreError::UnknownSource { id } => Self::UnknownSource { id: id.get() },

            // Hardware trouble is deliberately **not** given its own wire error.
            //
            // A device that will not open, or a MIDI system that cannot be
            // reached, is a state the user must see and work around — not a
            // failed command. As an error it would arrive as a rejected call with
            // nowhere sensible to render it, and would imply the whole operation
            // failed when every other device connected fine. These travel as data
            // instead, on `SourceDto.unavailable` and `MidiSystemStatusDto`.
            //
            // Reaching here means one leaked out of a path that should have
            // recorded it against a row, so it is reported rather than hidden.
            CoreError::PortUnavailable { name, detail } => Self::SettingsUnavailable {
                detail: format!("{name}: {detail}"),
            },
            CoreError::MidiSystemUnavailable { detail } => Self::SettingsUnavailable { detail },
        }
    }
}

/// Every command's return type.
pub type IpcResult<T> = Result<T, IpcError>;
