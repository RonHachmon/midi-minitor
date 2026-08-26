//! Monitored sources and the tri-state grouping shown in the Sources panel.

use super::ids::{SourceGroupId, SourceId};
use serde::{Deserialize, Serialize};

/// A named origin of events, as listed in `screenshots/sources.png`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Source {
    /// Identity. Two sources may share a display name, so this is what
    /// selection actually keys on.
    pub id: SourceId,
    /// The name shown in the list and in the Source column.
    pub name: String,
    /// The group this source is indented under, or [`None`] for a standalone row.
    pub group: Option<SourceGroupId>,
    /// Whether events from this source enter the monitor.
    pub selected: bool,
}

/// Whether a group's checkbox is on, off, or partially on.
///
/// # Why this is derived and never stored
///
/// A stored parent state can disagree with its children — the classic
/// "parent says checked, two children are not" bug. Computing it from the
/// children each time makes that state unrepresentable rather than merely
/// unlikely.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckState {
    /// Every child is selected.
    Checked,
    /// No child is selected.
    Unchecked,
    /// Some but not all children are selected — the indeterminate box.
    Mixed,
}

impl CheckState {
    /// Derives a group's state from its children's selections.
    ///
    /// An empty group reports [`Self::Unchecked`]: there is nothing selected in
    /// it, which is the honest answer and keeps the caller from having to treat
    /// emptiness specially.
    pub fn from_children(mut children: impl Iterator<Item = bool>) -> Self {
        let Some(first) = children.next() else {
            return Self::Unchecked;
        };
        for child in children {
            if child != first {
                return Self::Mixed;
            }
        }
        if first {
            Self::Checked
        } else {
            Self::Unchecked
        }
    }
}

/// The fixed set of sources this application monitors.
///
/// The catalogue is established once at startup. Simulated device hot-plug is
/// out of scope, so there is no add or remove — which is why callers can hold a
/// [`SourceId`] without worrying that it will stop resolving.
#[derive(Debug, Clone, Default)]
pub struct SourceCatalogue {
    sources: Vec<Source>,
}

impl SourceCatalogue {
    /// Builds a catalogue from the sources a [`crate::application::ports::EventSource`] reports.
    #[must_use]
    pub const fn new(sources: Vec<Source>) -> Self {
        Self { sources }
    }

    /// Every source, in the order the reference screenshot lists them.
    #[must_use]
    pub fn sources(&self) -> &[Source] {
        &self.sources
    }

    /// The display name for a source, if the id is known.
    #[must_use]
    pub fn name_of(&self, id: SourceId) -> Option<&str> {
        self.sources
            .iter()
            .find(|source| source.id == id)
            .map(|source| source.name.as_str())
    }

    /// The label the Source column shows for events from this source.
    ///
    /// # Why this differs from the name in the Sources list
    ///
    /// The reference window lists a source as `MidiKeys` but labels its events
    /// `From MidiKeys`, because the column is describing a direction of travel
    /// rather than naming a device. Traffic observed on the way *out* to a
    /// destination reads `To` instead. The rule belongs here beside the source
    /// model rather than in the interface, which would otherwise have to know
    /// which group means which direction.
    #[must_use]
    pub fn event_label(&self, id: SourceId) -> Option<String> {
        let source = self.sources.iter().find(|source| source.id == id)?;
        let direction = match source.group {
            Some(SourceGroupId::SpyOnOutput) => "To",
            Some(SourceGroupId::MidiSources) | None => "From",
        };
        Some(format!("{direction} {}", source.name))
    }

    /// Whether events from this source are currently admitted.
    ///
    /// An unknown id reports `false` rather than erroring: a stale reference
    /// should suppress traffic, never admit it.
    #[must_use]
    pub fn is_selected(&self, id: SourceId) -> bool {
        self.sources
            .iter()
            .find(|source| source.id == id)
            .is_some_and(|source| source.selected)
    }

    /// Whether any source at all is selected.
    ///
    /// Drives the "nothing is being monitored" state, which must be
    /// distinguishable from "no traffic right now".
    #[must_use]
    pub fn any_selected(&self) -> bool {
        self.sources.iter().any(|source| source.selected)
    }

    /// Selects or deselects one source.
    ///
    /// # Errors
    ///
    /// Returns [`crate::application::error::CoreError::UnknownSource`] when the
    /// id is not in the catalogue, which means the caller is working from a
    /// stale list and should refresh it.
    pub fn set_selected(
        &mut self,
        id: SourceId,
        selected: bool,
    ) -> Result<(), crate::application::error::CoreError> {
        let Some(source) = self.sources.iter_mut().find(|source| source.id == id) else {
            return Err(crate::application::error::CoreError::UnknownSource { id });
        };
        source.selected = selected;
        Ok(())
    }

    /// Applies one selection state to every member of a group.
    pub fn set_group_selected(&mut self, group: SourceGroupId, selected: bool) {
        for source in self
            .sources
            .iter_mut()
            .filter(|source| source.group == Some(group))
        {
            source.selected = selected;
        }
    }

    /// The tri-state for a group's own checkbox.
    #[must_use]
    pub fn group_state(&self, group: SourceGroupId) -> CheckState {
        CheckState::from_children(
            self.sources
                .iter()
                .filter(|source| source.group == Some(group))
                .map(|source| source.selected),
        )
    }

    /// The ids of every currently deselected source.
    ///
    /// Used to purge retained events when a source is switched off, so the
    /// visible list always matches the current selection.
    pub fn deselected_ids(&self) -> impl Iterator<Item = SourceId> + '_ {
        self.sources
            .iter()
            .filter(|source| !source.selected)
            .map(|source| source.id)
    }

    /// Restores selections saved from a previous session.
    ///
    /// Ids absent from `selected` are left deselected, and ids that no longer
    /// exist are ignored — a settings file from an older catalogue should not
    /// prevent startup.
    pub fn apply_selection(&mut self, selected: &[SourceId]) {
        for source in &mut self.sources {
            source.selected = selected.contains(&source.id);
        }
    }

    /// The ids of every currently selected source, for persistence.
    #[must_use]
    pub fn selected_ids(&self) -> Vec<SourceId> {
        self.sources
            .iter()
            .filter(|source| source.selected)
            .map(|source| source.id)
            .collect()
    }
}

/// A snapshot of one group and its members, shaped for display.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceGroupView {
    /// The group's verbatim label, or [`None`] for standalone rows.
    pub group: Option<SourceGroupId>,
    /// The ids of the sources in this group, in display order.
    pub source_ids: Vec<SourceId>,
}
