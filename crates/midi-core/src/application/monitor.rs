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
use super::ports::MidiSystemStatus;
use super::settings::PersistedSettings;
use crate::domain::column::{Column, ColumnVisibility};
use crate::domain::event::MidiEvent;
use crate::domain::event_log::EventLog;
use crate::domain::filter::{DataPrefixFilter, FilterSettings};
use crate::domain::ids::{RetentionLimit, SourceGroupId, SourceId, SourceKey};
use crate::domain::source::{CheckState, Source, SourceCatalogue};

/// Holds everything the monitor knows and answers everything the interface asks.
#[derive(Debug)]
pub struct Monitor {
    catalogue: SourceCatalogue,
    log: EventLog,
    filter: FilterSettings,
    columns: ColumnVisibility,
    /// Selections for devices that are not currently attached.
    ///
    /// # Why these are held separately
    ///
    /// The catalogue only knows about devices that are here. If persistence read
    /// straight from it, quitting while a device was unplugged would write a
    /// settings file with that device missing — silently forgetting a choice the
    /// user made. Keeping the remembered set alongside the live one lets a
    /// device come back selected however long it has been away.
    remembered: Vec<SourceKey>,
    /// Whether the platform's MIDI system could be reached.
    status: MidiSystemStatus,
}

impl Monitor {
    /// Builds a monitor over a source catalogue, restoring saved settings.
    ///
    /// # First launch versus every launch after
    ///
    /// With **no persisted settings at all**, every discovered source starts
    /// selected, so the window shows traffic immediately rather than sending the
    /// user to find the Sources panel first.
    ///
    /// With settings present, a source the file does not mention starts
    /// **unselected**. The asymmetry is deliberate: once a user has narrowed the
    /// list, attaching a device that emits Clock at speed must not silently flood
    /// it. A simulator with a fixed cast of sources never had to make this
    /// distinction; real hardware does.
    #[must_use]
    pub fn new(
        sources: Vec<Source>,
        saved: Option<PersistedSettings>,
        status: MidiSystemStatus,
    ) -> Self {
        let mut catalogue = SourceCatalogue::new(sources);
        let (settings, remembered) = match saved {
            Some(settings) => {
                catalogue.apply_selection(&settings.selected_sources);
                let remembered = settings.selected_sources.clone();
                (settings, remembered)
            }
            None => {
                let all = catalogue.present_keys();
                catalogue.apply_selection(&all);
                (PersistedSettings::default(), all)
            }
        };

        Self {
            catalogue,
            log: EventLog::new(settings.retention),
            filter: settings.filter,
            columns: settings.columns,
            remembered,
            status,
        }
    }

    /// Applies a newly discovered set of sources after a hot-plug change.
    ///
    /// Selections carry across by key, and a device that has just reappeared is
    /// restored from the remembered set — so unplugging and replugging returns it
    /// still ticked, with no interaction required.
    ///
    /// **Retained events are deliberately untouched.** Rows produced by a device
    /// before it was removed stay listed and stay correctly attributed; the
    /// history of what was observed does not depend on what is currently plugged
    /// in.
    pub fn replace_catalogue(&mut self, sources: Vec<Source>) {
        self.catalogue.replace(sources);
        let remembered = self.remembered.clone();
        for source in self.catalogue.present_keys() {
            if remembered.contains(&source) {
                self.select_by_key(source);
            }
        }
    }

    /// Records the MIDI system's reachability.
    pub fn set_status(&mut self, status: MidiSystemStatus) {
        self.status = status;
    }

    /// Whether the MIDI system could be reached, and why not if it could not.
    #[must_use]
    pub const fn status(&self) -> &MidiSystemStatus {
        &self.status
    }

    /// Ticks the source carrying this key, if it is currently present.
    fn select_by_key(&mut self, key: SourceKey) {
        let ids: Vec<SourceId> = self
            .catalogue
            .sources()
            .iter()
            .filter(|source| source.key == key)
            .map(|source| source.id)
            .collect();
        for id in ids {
            // The id came from the catalogue a statement ago, so it resolves;
            // the result carries no information worth branching on.
            drop(self.catalogue.set_selected(id, true));
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

    /// The source catalogue, for recording what the hardware reported.
    ///
    /// Exposed mutably only so the adapter can mark a port it could not open.
    /// Selection still goes through [`Self::set_source_selected`], which keeps
    /// the remembered set and the retained log in step — a caller that reached
    /// around it would break both.
    pub const fn catalogue_mut(&mut self) -> &mut SourceCatalogue {
        &mut self.catalogue
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
        self.remember_current();
        self.purge_deselected();
        Ok(())
    }

    /// Applies one selection state to every source in a group.
    pub fn set_group_selected(&mut self, group: SourceGroupId, selected: bool) {
        self.catalogue.set_group_selected(group, selected);
        self.remember_current();
        self.purge_deselected();
    }

    /// Folds the live selection into the remembered set.
    ///
    /// Only keys for *present* devices are touched: a device the user just
    /// unticked is dropped from the remembered set, while one that is simply not
    /// attached is left alone. Without that distinction, unplugging a device
    /// would look identical to deselecting it.
    fn remember_current(&mut self) {
        let present = self.catalogue.present_keys();
        let selected = self.catalogue.selected_keys();
        self.remembered.retain(|key| !present.contains(key));
        for key in selected {
            if !self.remembered.contains(&key) {
                self.remembered.push(key);
            }
        }
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
    ///
    /// Selections are the **union** of what is selected now and what was
    /// remembered for devices that are not attached. Saving only the live
    /// catalogue would erase the choice for every device currently unplugged —
    /// so quitting with a device disconnected would lose it, which is precisely
    /// the moment a user is least likely to notice.
    #[must_use]
    pub fn persisted_settings(&self) -> PersistedSettings {
        let mut selected_sources = self.remembered.clone();
        for key in self.catalogue.selected_keys() {
            if !selected_sources.contains(&key) {
                selected_sources.push(key);
            }
        }

        PersistedSettings {
            selected_sources,
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
