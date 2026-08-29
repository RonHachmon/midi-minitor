//! Where sent traffic goes.

use super::ids::{TargetId, TargetKey};

/// A place this application can send to.
///
/// The direct counterpart of [`super::source::Source`], and split the same way
/// for the same reason: [`TargetId`] is session identity, because two
/// destinations may legitimately share a display name, and [`TargetKey`] is the
/// identity that outlives the session, so a chosen target is found again after a
/// replug or a restart.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    /// Session identity. What a choice is keyed on while the application runs.
    pub id: TargetId,
    /// Identity that outlives the session. What a saved choice is stored against.
    pub key: TargetKey,
    /// The name shown in the picker and recorded against a send.
    ///
    /// Supplied by the operating system for a destination, and by the user for
    /// the published source. Used verbatim in both cases: cleaning up a device
    /// name would make this list disagree with every other MIDI utility on the
    /// machine.
    pub name: String,
    /// Whether this is a device the machine reports, or this application's own
    /// published source.
    pub kind: TargetKind,
}

/// What sort of place a target is.
///
/// # Why this exists rather than being inferred from the key
///
/// Two reasons, and both are load-bearing. The adapter reaches the two through
/// entirely different platform calls — an output port for a destination, and
/// distribution from a virtual source for the published one — so it must be able
/// to tell them apart without unwrapping an identity type. And the screen must
/// say which is in force, because only one of them carries the name the user
/// chose; traffic sent to a destination carries whatever origin the operating
/// system reports for this application, and implying otherwise would be a lie.
///
/// Matched exhaustively with no catch-all arm, so a third kind of target would be
/// a compile error at every site that renders or transmits one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetKind {
    /// A MIDI destination the operating system reports.
    Destination,
    /// This application's own published source, under the user's chosen name.
    PublishedSource,
}

impl Target {
    /// Builds a target.
    #[must_use]
    pub const fn new(id: TargetId, key: TargetKey, name: String, kind: TargetKind) -> Self {
        Self {
            id,
            key,
            name,
            kind,
        }
    }
}
