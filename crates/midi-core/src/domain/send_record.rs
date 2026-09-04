//! What was sent, when, and whether it worked.

use super::ids::{SendRecordId, Timestamp};
use super::message::MidiMessage;

/// One attempted send.
///
/// # Why this exists at all
///
/// Without it, a send that failed silently and a send that succeeded but was
/// ignored by the receiving program look identical. Telling those two apart is
/// the same job the event table does for incoming traffic, and it is the reason
/// anyone opens a monitor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SendRecord {
    /// Identity, so a re-send names the entry the user pointed at even after
    /// older entries have been evicted and the list has shifted.
    pub id: SendRecordId,
    /// When the send was attempted.
    pub time: Timestamp,
    /// The target's name **as it was at the time of the send**.
    ///
    /// Copied rather than referenced, and that is the point: the record has to
    /// still read correctly after the device is unplugged, which is precisely the
    /// moment someone goes back to read it.
    pub target: String,
    /// The interpretation, for the record's description column.
    pub message: MidiMessage,
    /// The bytes exactly as transmitted.
    ///
    /// Stored rather than re-derived, so a re-send of a hand-typed message sends
    /// what was typed rather than what re-encoding would produce.
    pub bytes: Vec<u8>,
    /// Whether it worked, and why not if it did not.
    pub outcome: SendOutcome,
}

/// How a send turned out.
///
/// # Why an enum rather than `Option<String>`
///
/// The same argument the platform capabilities already make: "it worked" is a
/// fact worth naming, not the absence of a complaint. As an `Option`, success
/// would be indistinguishable from a failure whose reason nobody filled in — and
/// the interface is required to make the two visibly different, not merely
/// differently annotated.
///
/// Matched exhaustively with no catch-all arm, so a third outcome would be a
/// compile error at every site that renders one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SendOutcome {
    /// The platform accepted the bytes.
    ///
    /// This means transmitted, and nothing more. Whether a receiving program
    /// acted on them is unknowable from here, and the interface must not claim
    /// otherwise.
    Sent,
    /// The platform refused, and this is what it reported.
    Failed {
        /// What went wrong, in the platform's own words.
        detail: String,
    },
}
