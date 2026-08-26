//! The monitor: where an arriving event meets the user's current settings.
//!
//! # Why one service rather than several
//!
//! Source selection, filtering, retention, and column visibility all read or
//! write the same two pieces of state — the retained log and the settings — and
//! several of them must change together. Deselecting a source, for instance,
//! both stops future events and purges retained ones. Splitting that across
//! services would mean coordinating them from the caller, which is exactly the
//! business logic the Tauri layer is forbidden to hold.

use super::error::CoreError;
use super::settings::PersistedSettings;
use crate::domain::column::{Column, ColumnVisibility};
use crate::domain::event::MidiEvent;
use crate::domain::event_log::EventLog;
use crate::domain::filter::{DataPrefixFilter, FilterSettings};
use crate::domain::ids::{RetentionLimit, SourceGroupId, SourceId};
use crate::domain::source::{CheckState, Source, SourceCatalogue};

/// Holds everything the monitor knows and answers everything the interface asks.
#[derive(Debug)]
pub struct Monitor {
    catalogue: SourceCatalogue,
    log: EventLog,
    filter: FilterSettings,
    columns: ColumnVisibility,
}

impl Monitor {
    /// Builds a monitor over a source catalogue, restoring saved settings.
    ///
    /// On a first run — no persisted settings — every source starts selected, so
    /// the window shows traffic immediately rather than requiring the user to go
    /// find the Sources panel first.
    #[must_use]
    pub fn new(sources: Vec<Source>, saved: Option<PersistedSettings>) -> Self {
        let mut catalogue = SourceCatalogue::new(sources);
        let settings = match saved {
            Some(settings) => {
                catalogue.apply_selection(&settings.selected_sources);
                settings
            }
            None => PersistedSettings::default(),
        };

        Self {
            catalogue,
            log: EventLog::new(settings.retention),
            filter: settings.filter,
            columns: settings.columns,
        }
    }

    /// Admits an event, retains it, and reports whether it should be streamed.
    ///
    /// Returns the event when it passes the current filter and should reach the
    /// webview, or [`None`] when it was suppressed. Suppressed events are still
    /// *retained* if their source is selected — that is what lets re-ticking a
    /// filter checkbox reveal them without waiting for new traffic.
    ///
    /// Events from deselected sources are dropped entirely and never retained.
    pub fn ingest(&mut self, event: MidiEvent) -> Option<MidiEvent> {
        if !self.catalogue.is_selected(event.source) {
            return None;
        }
        let streamed = self.filter.admits(&event).then(|| event.clone());
        self.log.push(event);
        streamed
    }

    /// The retained events that pass the current filter, oldest first.
    ///
    /// Borrowed rather than cloned: a snapshot at the maximum retention limit
    /// would otherwise copy a hundred thousand events every time a checkbox is
    /// ticked. The caller maps straight to its wire types.
    pub fn visible_events(&self) -> impl Iterator<Item = &MidiEvent> + '_ {
        self.log.view(&self.filter)
    }

    /// How many events are retained, before filters are applied.
    #[must_use]
    pub fn retained_count(&self) -> usize {
        self.log.len()
    }

    /// Whether any source is selected.
    ///
    /// False means the empty list is a configuration state, not an absence of
    /// traffic, and the interface must say so rather than looking frozen.
    #[must_use]
    pub fn is_monitoring(&self) -> bool {
        self.catalogue.any_selected()
    }

    /// The source catalogue, for rendering the Sources panel.
    #[must_use]
    pub const fn catalogue(&self) -> &SourceCatalogue {
        &self.catalogue
    }

    /// The active filter settings.
    #[must_use]
    pub const fn filter(&self) -> &FilterSettings {
        &self.filter
    }

    /// The active column visibility.
    #[must_use]
    pub const fn columns(&self) -> &ColumnVisibility {
        &self.columns
    }

    /// The current retention cap.
    #[must_use]
    pub const fn retention(&self) -> RetentionLimit {
        self.log.limit()
    }

    /// A group's tri-state checkbox value.
    #[must_use]
    pub fn group_state(&self, group: SourceGroupId) -> CheckState {
        self.catalogue.group_state(group)
    }

    /// Selects or deselects one source, purging its retained events when off.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::UnknownSource`] when the id is not in the catalogue,
    /// which means the caller is holding a stale list.
    pub fn set_source_selected(&mut self, id: SourceId, selected: bool) -> Result<(), CoreError> {
        self.catalogue.set_selected(id, selected)?;
        self.purge_deselected();
        Ok(())
    }

    /// Applies one selection state to every source in a group.
    pub fn set_group_selected(&mut self, group: SourceGroupId, selected: bool) {
        self.catalogue.set_group_selected(group, selected);
        self.purge_deselected();
    }

    /// Replaces the message-kind and channel filter, keeping the prefix filter.
    ///
    /// The prefix filter is preserved because it is a separate control in the
    /// interface: ticking a message checkbox should not silently clear what the
    /// user typed into the hex field.
    pub fn set_filter(&mut self, kinds_and_channel: FilterSettings) {
        let data_filter = self.filter.data_filter.clone();
        self.filter = FilterSettings {
            data_filter,
            ..kinds_and_channel
        };
    }

    /// Replaces the hexadecimal prefix filter.
    pub fn set_data_prefix_filter(&mut self, data_filter: DataPrefixFilter) {
        self.filter.data_filter = data_filter;
    }

    /// Replaces the retention cap, discarding excess oldest events at once.
    pub fn set_retention(&mut self, limit: RetentionLimit) {
        self.log.set_limit(limit);
    }

    /// Discards every retained event.
    pub fn clear(&mut self) {
        self.log.clear();
    }

    /// Replaces which columns are shown.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::LastColumnVisible`] when `visible` is empty.
    ///
    /// Deliberately does not touch the log or the filter: column visibility is a
    /// display concern, and routing it through the same path as filtering would
    /// invite the two to be confused later.
    pub fn set_columns(&mut self, visible: &[Column]) -> Result<(), CoreError> {
        self.columns = ColumnVisibility::from_visible(visible)?;
        Ok(())
    }

    /// The current state, shaped for persistence.
    #[must_use]
    pub fn persisted_settings(&self) -> PersistedSettings {
        PersistedSettings {
            selected_sources: self.catalogue.selected_ids(),
            filter: self.filter.clone(),
            columns: self.columns.clone(),
            retention: self.log.limit(),
        }
    }

    /// Drops retained events whose source is no longer selected.
    fn purge_deselected(&mut self) {
        let deselected: Vec<SourceId> = self.catalogue.deselected_ids().collect();
        if deselected.is_empty() {
            return;
        }
        self.log
            .retain_selected(|source| !deselected.contains(&source));
    }
}
