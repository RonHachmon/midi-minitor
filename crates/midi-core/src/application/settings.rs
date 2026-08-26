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
use crate::domain::ids::{RetentionLimit, SourceId};
use serde::{Deserialize, Serialize};

/// Everything restored on the next launch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistedSettings {
    /// Which sources were being monitored.
    pub selected_sources: Vec<SourceId>,
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
