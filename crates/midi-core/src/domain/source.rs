//! Monitored sources and the tri-state grouping shown in the Sources panel.

use super::ids::{SourceGroupId, SourceId, SourceKey};
use serde::{Deserialize, Serialize};

/// A named origin of events, as listed in `screenshots/sources.png`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Source {
    /// Session identity. Two sources may share a display name, so this is what
    /// selection actually keys on.
    pub id: SourceId,
    /// Identity that outlives the session — what a saved selection is stored
    /// against, so a device found again after a replug is recognised as the same
    /// device rather than as a new one.
    pub key: SourceKey,
    /// The name shown in the list and in the Source column.
    ///
    /// Supplied by the operating system and used verbatim. Cleaning it up would
    /// make this list disagree with every other MIDI utility on the machine,
    /// which is the opposite of helping someone identify a device.
    pub name: String,
    /// The group this source is indented under, or [`None`] for a standalone row.
    pub group: Option<SourceGroupId>,
    /// Whether events from this source enter the monitor.
    pub selected: bool,
    /// Whether it is currently able to deliver anything.
    pub availability: Availability,
}

/// Whether a listed source can currently deliver events, and if not, why.
///
/// # The problem this solves
///
/// With a fixed set of simulated sources every row was always live, so the
/// question never arose. Real hardware makes three states routinely visible, and
/// they call for different responses from the user: nothing is wrong, something
/// else has the device, or the device is not here. Collapsing them into a
/// boolean would leave the interface unable to say which.
///
/// Exhaustive matching everywhere, with no catch-all arm: a fourth state must
/// force a compile error at every site that renders one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Availability {
    /// Attached and listening.
    Open,
    /// Attached, but the port could not be opened.
    Unopenable {
        /// What the operating system reported, for the interface to show.
        detail: String,
    },
    /// Remembered from a previous session and not currently attached.
    ///
    /// Listed rather than hidden so the user can see that a device they chose is
    /// missing, instead of silently wondering where its traffic went.
    Absent,
}

impl Availability {
    /// The reason this source cannot deliver, or [`None`] when it can.
    ///
    /// Returns the message the interface should show rather than a flag, because
    /// an unavailable row without an explanation is a state the Sources panel
    /// must never have to render.
    #[must_use]
    pub fn reason(&self) -> Option<String> {
        match self {
            Self::Open => None,
            Self::Unopenable { detail } => Some(detail.clone()),
            Self::Absent => Some("Not connected".to_owned()),
        }
    }
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

/// The sources this application is currently able to monitor.
///
/// # Why this changes at runtime
///
/// It did not always. When the sources were simulated this catalogue was built
/// once at startup and never altered, and callers were told they could hold a
/// [`SourceId`] indefinitely. Real devices are plugged in and unplugged while the
/// application runs, so that guarantee could not survive: [`Self::replace`] swaps
/// the live set whenever the operating system reports a change.
///
/// **What callers must therefore assume**: a [`SourceId`] can stop resolving at
/// any moment. Every lookup here returns an [`Option`] or a `Result` for exactly
/// that reason, and a stale id must be treated as "refresh your list", never as
/// an error worth interrupting the user over.
#[derive(Debug, Clone, Default)]
pub struct SourceCatalogue {
    sources: Vec<Source>,
}

impl SourceCatalogue {
    /// Builds a catalogue from the sources an [`crate::application::ports::EventSource`] reports.
    #[must_use]
    pub const fn new(sources: Vec<Source>) -> Self {
        Self { sources }
    }

    /// Swaps in a newly discovered set of sources, carrying selections across.
    ///
    /// # Why selection is preserved by key rather than by position
    ///
    /// The incoming list is freshly enumerated, so its [`SourceId`]s are new and
    /// its ordering may differ. Matching on [`SourceKey`] is what makes an
    /// unplug-and-replug return a device *still ticked* instead of silently
    /// reset — the behaviour a user notices immediately if it is wrong.
    ///
    /// Sources absent from `incoming` simply disappear; restoring the selections
    /// of devices that have gone away is the caller's job, since only the caller
    /// knows what was remembered from previous sessions.
    pub fn replace(&mut self, incoming: Vec<Source>) {
        let previous: Vec<(SourceKey, bool)> = self
            .sources
            .iter()
            .map(|source| (source.key, source.selected))
            .collect();

        self.sources = incoming
            .into_iter()
            .map(|mut source| {
                if let Some((_, was_selected)) = previous.iter().find(|(key, _)| *key == source.key)
                {
                    source.selected = *was_selected;
                }
                source
            })
            .collect();
    }

    /// Every source, in the order the event source reported them.
    #[must_use]
    pub fn sources(&self) -> &[Source] {
        &self.sources
    }

    /// The display name for a source, if the id still resolves.
    #[must_use]
    pub fn name_of(&self, id: SourceId) -> Option<&str> {
        self.sources
            .iter()
            .find(|source| source.id == id)
            .map(|source| source.name.as_str())
    }

    /// The persistent identity behind a session id, if it still resolves.
    #[must_use]
    pub fn key_of(&self, id: SourceId) -> Option<SourceKey> {
        self.sources
            .iter()
            .find(|source| source.id == id)
            .map(|source| source.key)
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
    /// False means the empty list is a configuration state, not an absence of
    /// traffic, and the interface must say so rather than looking frozen.
    #[must_use]
    pub fn any_selected(&self) -> bool {
        self.sources.iter().any(|source| source.selected)
    }

    /// Selects or deselects one source, purging its retained events when off.
    ///
    /// # Errors
    ///
    /// Returns [`crate::application::error::CoreError::UnknownSource`] when the
    /// id is not in the catalogue, which means the caller is working from a list
    /// made stale by a device disappearing.
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

    /// Records that a port could not be opened, so the row can explain itself.
    pub fn set_unopenable(&mut self, id: SourceId, detail: String) {
        if let Some(source) = self.sources.iter_mut().find(|source| source.id == id) {
            source.availability = Availability::Unopenable { detail };
        }
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
    /// Keys absent from `selected` are left deselected, and keys naming devices
    /// that are not attached are ignored here — the caller remembers those
    /// separately so they can be restored if the device comes back.
    pub fn apply_selection(&mut self, selected: &[SourceKey]) {
        for source in &mut self.sources {
            source.selected = selected.contains(&source.key);
        }
    }

    /// The identities of every currently selected source, for persistence.
    ///
    /// Returns keys rather than ids because these outlive the session; see
    /// [`SourceKey`].
    #[must_use]
    pub fn selected_keys(&self) -> Vec<SourceKey> {
        self.sources
            .iter()
            .filter(|source| source.selected)
            .map(|source| source.key)
            .collect()
    }

    /// Every key currently in the catalogue, selected or not.
    ///
    /// Lets the caller tell "this device is here and the user turned it off"
    /// apart from "this device is not here", which decides whether a remembered
    /// selection should be kept or dropped.
    #[must_use]
    pub fn present_keys(&self) -> Vec<SourceKey> {
        self.sources.iter().map(|source| source.key).collect()
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
