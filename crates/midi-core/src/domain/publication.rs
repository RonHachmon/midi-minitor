//! The disguise: the MIDI source this application offers to the rest of the machine.

use super::ids::PublishedName;
use serde::{Deserialize, Serialize};

/// The published source's name, and whether it is currently published.
///
/// # Why both halves persist
///
/// A user who left the disguise on expects to find their device present in the
/// receiving program on the next launch, without opening this screen at all. And
/// the name is the identity that receiving program remembers its own settings
/// against, so losing it across a restart would silently orphan their mapping.
///
/// # Why `published` is a request rather than a guarantee
///
/// The platform has the final say. A saved `true` is what the application asks
/// for at startup; if the platform refuses — or if the settings were written on a
/// platform that can publish and opened on one that cannot — the state reported
/// afterwards is the truth, and the saved name is kept rather than discarded so
/// the same profile still works when it is carried back.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Publication {
    /// The name other programs see, and remember their settings against.
    pub name: PublishedName,
    /// Whether the source is published right now.
    pub published: bool,
}

impl Default for Publication {
    /// The product name, not published.
    ///
    /// Off by default because publishing puts an endpoint on the user's machine
    /// that other software can see, and that is a thing to opt into rather than
    /// to discover.
    fn default() -> Self {
        Self {
            name: PublishedName::default(),
            published: false,
        }
    }
}
