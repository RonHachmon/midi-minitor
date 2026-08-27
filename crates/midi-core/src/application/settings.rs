//! The slice of state that outlives a session.
//!
//! # Why retained events are not here
//!
//! Settings describe how the user has configured the monitor; retained events
//! are observations of a moment. Restoring yesterday's traffic into today's
//! window would present stale data as live, so the event log is deliberately
//! session-only and this type carries configuration alone.

use crate::domain::column::ColumnVisibility;
use crate::domain::filter::FilterSettings;
use crate::domain::ids::{RetentionLimit, SourceKey};
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
        }
    }
}
