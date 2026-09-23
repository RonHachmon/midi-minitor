//! The slice of state that outlives a session.
//!
//! # Why retained events are not here
//!
//! Settings describe how the user has configured the monitor; retained events
//! are observations of a moment. Restoring yesterday's traffic into today's
//! window would present stale data as live, so the event log is deliberately
//! session-only and this type carries configuration alone.

use crate::domain::column::ColumnVisibility;
use crate::domain::composition::PersistedComposition;
use crate::domain::display::DisplaySettings;
use crate::domain::filter::FilterSettings;
use crate::domain::ids::{RequestName, RetentionLimit, SourceKey, TargetKey};
use crate::domain::publication::Publication;
use serde::{Deserialize, Serialize};

/// Everything restored on the next launch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistedSettings {
    /// Which sources were being monitored, by an identity that outlives the
    /// session.
    ///
    /// Keyed on [`SourceKey`] rather than a session id, because a session id is
    /// minted in discovery order and would restore yesterday's choices onto
    /// today's arbitrary numbering. Includes devices that were **not attached**
    /// when the settings were written, so unplugging a device and quitting does
    /// not silently forget that the user had selected it.
    pub selected_sources: Vec<SourceKey>,
    /// Message kinds, channel mode, and the hexadecimal prefix filter.
    pub filter: FilterSettings,
    /// Which columns were shown.
    pub columns: ColumnVisibility,
    /// The retention cap.
    pub retention: RetentionLimit,
    /// How the event table writes what it holds.
    ///
    /// `#[serde(default)]` for the reason `send` records: this feature *adds* a
    /// field and changes none, so a document written by any earlier version has
    /// no `display` key, supplies the default, and keeps every other setting
    /// intact. Writing a migration here would be dead code.
    ///
    /// The per-field tolerance inside [`DisplaySettings`] is the other half of
    /// the story, and it is the half that matters more. `#[serde(default)]`
    /// answers a *missing* key; it does nothing for one that is present and
    /// unparseable. Because `StoreSettingsRepository::load` treats an unreadable
    /// document as absent, a single unrecognised value here — a setting written
    /// by a later version, a hand-edited file — would otherwise discard the
    /// user's filters, columns, retention, source selections, and saved requests
    /// along with it.
    #[serde(default)]
    pub display: DisplaySettings,
    /// The send screen's state.
    ///
    /// `#[serde(default)]` is the whole of this feature's compatibility story,
    /// and the contrast with the previous feature is worth stating. That one
    /// *changed the shape* of an existing field, and because
    /// `StoreSettingsRepository::load` treats an unreadable document as absent, a
    /// naive change there would have silently discarded source selections,
    /// columns, and retention along with the filter — so it needed a real
    /// migration. This feature adds a field and changes none. A document written
    /// by any earlier version has no `send` key, supplies the default, and keeps
    /// every other setting intact. Writing a migration here would be dead code.
    ///
    /// It is one nested field rather than several at the top level for the same
    /// reason: a defect confined to this block cannot take the user's filters or
    /// selections down with it.
    #[serde(default)]
    pub send: SendSettings,
}

impl Default for PersistedSettings {
    /// The first-launch state: every source monitored, every filter checkbox
    /// ticked, every column shown, and the reference window's retention default.
    fn default() -> Self {
        Self {
            selected_sources: Vec::new(),
            filter: FilterSettings::default(),
            columns: ColumnVisibility::default(),
            retention: RetentionLimit::default(),
            display: DisplaySettings::default(),
            send: SendSettings::default(),
        }
    }
}

/// What the send screen restores on the next launch.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SendSettings {
    /// Where traffic was last sent, by an identity that outlives the session.
    ///
    /// Keyed on [`TargetKey`] for the reason `selected_sources` is: a session id
    /// is minted while scanning and means nothing on the next launch. A key that
    /// matches nothing in today's scan is **kept, not cleared** — the device may
    /// come back, and forgetting the choice the moment a cable is unplugged would
    /// be worse than leaving nothing selected for one session.
    pub target: Option<TargetKey>,
    /// The published source's name, and whether it was published.
    pub publication: Publication,
    /// The user's own requests, in the order they created them.
    pub saved_requests: Vec<SavedRequest>,
}

/// One request the user saved, in the form that survives a restart.
///
/// # Why the composition is stored in its own captured form
///
/// A saved request must transmit **identical** bytes after a restart. For a
/// hand-typed request that means storing the bytes as typed: re-deriving them
/// from a decoded message would turn a `Note On` with velocity zero into a
/// `Note Off`, which is the one thing the composition type exists to prevent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedRequest {
    /// The name, which is also the request's identity.
    pub name: RequestName,
    /// What the request does, in the user's own words.
    pub description: String,
    /// The message, and where its authority lies.
    pub composition: PersistedComposition,
}

impl PersistedSettings {
    /// Replaces the send half of these settings.
    ///
    /// # Why this exists rather than the two halves being assembled inline
    ///
    /// The monitor and the sender each own one half of this document and neither
    /// can see the other's. [`crate::application::monitor::Monitor::persisted_settings`]
    /// therefore fills its own half and leaves this one at its default — which
    /// means saving that value directly would silently erase the user's chosen
    /// target, their published name, and every request they had saved.
    ///
    /// A named method makes the missing step greppable and gives the mistake a
    /// place to be described, which a bare struct-update expression at one call
    /// site would not.
    #[must_use]
    pub fn with_send(mut self, send: SendSettings) -> Self {
        self.send = send;
        self
    }
}
