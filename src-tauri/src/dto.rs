//! The wire types.
//!
//! # Why these are separate from the domain types
//!
//! Domain types answer to the rules of MIDI monitoring; these answer to what the
//! webview needs to render and what generation can express. Deriving
//! `specta::Type` on the domain would make the core's shape hostage to the
//! webview's serialization needs — one type with two reasons to change.
//!
//! # Why display values are rendered here rather than in TypeScript
//!
//! The Data cell, the Time string, and the raw hex are computed in Rust and
//! cross as finished strings. What belongs in the Data cell is a fact about MIDI,
//! not a presentation choice, so rendering it on the far side of a serialization
//! boundary would put a domain rule in the webview and duplicate it.

use midi_core::application::monitor::Monitor;
use midi_core::application::ports::{ByteFidelity, MidiSystemStatus};
use midi_core::domain::column::Column;
use midi_core::domain::event::MidiEvent;
use midi_core::domain::filter::{ChannelMode, PrefixMode};
use midi_core::domain::ids::SourceGroupId;
use midi_core::domain::message::{MessageCategory, MessageKind};
use midi_core::domain::source::{Availability, CheckState, SourceCatalogue};
use serde::{Deserialize, Serialize};
use specta::Type;

/// One row of the event table.
#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct EventDto {
    /// Arrival order; the list key and the stream's high-water mark.
    pub id: u32,
    /// Pre-formatted `HH:MM:SS.mmm`.
    pub time: String,
    /// The Source column's label, such as `From MidiKeys`.
    pub source: String,
    /// The Message column's name, such as `Note On`.
    pub message: String,
    /// The Chan column, or `null` for messages carrying no channel.
    ///
    /// Null rather than an absent key: the cell must render *blank*, and a
    /// missing field would let a renderer print `undefined`.
    pub channel: Option<u8>,
    /// The Data column, rendered per message type.
    pub data: String,
    /// Uppercase, separator-free raw bytes — what the prefix filter matches.
    pub raw_hex: String,
}

impl EventDto {
    /// Renders one event for the wire.
    pub fn from_event(event: &MidiEvent, catalogue: &SourceCatalogue) -> Self {
        Self {
            id: event.id.get(),
            time: event.timestamp.to_display(),
            source: catalogue
                .event_label(event.source)
                .unwrap_or_else(|| "(unknown source)".to_owned()),
            message: event.message.display_name().to_owned(),
            channel: event.channel().map(|channel| channel.get()),
            data: event.message.data_display(),
            raw_hex: event.raw_hex(),
        }
    }
}

/// A coalesced group of events, as delivered over the stream channel.
#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct EventBatchDto {
    /// The events, in arrival order.
    pub events: Vec<EventDto>,
}

/// Tri-state for a group or category checkbox.
#[derive(Debug, Clone, Copy, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum CheckStateDto {
    /// Every child selected.
    Checked,
    /// No child selected.
    Unchecked,
    /// Some but not all — the indeterminate box.
    Mixed,
}

impl From<CheckState> for CheckStateDto {
    fn from(state: CheckState) -> Self {
        match state {
            CheckState::Checked => Self::Checked,
            CheckState::Unchecked => Self::Unchecked,
            CheckState::Mixed => Self::Mixed,
        }
    }
}

/// One row in the Sources panel.
#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SourceDto {
    /// Identity — what selection keys on, since names can repeat.
    pub id: u32,
    /// The name shown in the list, exactly as the operating system supplies it.
    pub name: String,
    /// Whether events from it are admitted.
    pub selected: bool,
    /// Whether this source can currently deliver, and why not if it cannot.
    ///
    /// # Why a tagged union rather than a nullable reason string
    ///
    /// This used to be `unavailable: Option<String>`, which was enough while
    /// every unavailability meant the same thing to the control. It no longer
    /// does: a row held by another program stays **tickable**, so it starts
    /// being monitored the moment that program lets go, while a row the platform
    /// can never satisfy must not be tickable at all. A reason string cannot
    /// tell those apart, and a second boolean beside it could contradict it.
    ///
    /// The shape follows [`MidiSystemStatusDto`], which solved the adjacent
    /// problem the same way — this is the house pattern, not a new one.
    pub availability: AvailabilityDto,
}

/// Whether a source can deliver, and why not if it cannot.
///
/// Mirrors [`Availability`] one variant for one variant, and is matched
/// exhaustively with no catch-all arm so that a new state becomes a compile
/// error here and a type error in the webview rather than a silent default.
#[derive(Debug, Clone, Serialize, Type)]
#[serde(tag = "type", content = "data", rename_all = "camelCase")]
pub enum AvailabilityDto {
    /// Attached and listening.
    Open,
    /// Attached, but the port could not be opened. Still tickable.
    Unopenable {
        /// What the operating system reported.
        detail: String,
    },
    /// Remembered from a previous session and not currently attached.
    Absent,
    /// The platform in use cannot offer this at all. Not tickable.
    Unsupported {
        /// What the platform cannot do, and what the user can do instead.
        detail: String,
    },
}

impl From<&Availability> for AvailabilityDto {
    fn from(availability: &Availability) -> Self {
        match availability {
            Availability::Open => Self::Open,
            Availability::Unopenable { detail } => Self::Unopenable {
                detail: detail.clone(),
            },
            Availability::Absent => Self::Absent,
            Availability::Unsupported { detail } => Self::Unsupported {
                detail: detail.clone(),
            },
        }
    }
}

/// How faithfully this platform reports the bytes that arrived.
///
/// # Why this crosses the wire at all
///
/// The `Data` column must carry a standing statement on a platform that
/// assembles messages before the application can see them, and must not carry
/// one where it would not be true. Deciding that in the webview would put a
/// platform check on the wrong side of the boundary; deciding it in this crate
/// would put one in a layer that is required to hold no rules. So the adapter
/// says, and everything above renders what it is told.
#[derive(Debug, Clone, Serialize, Type)]
#[serde(tag = "type", content = "data", rename_all = "camelCase")]
pub enum ByteFidelityDto {
    /// The bytes shown are the bytes the cable carried.
    AsTransmitted,
    /// The platform assembles messages before delivery, and says what that means.
    Assembled {
        /// What the platform assembles, and what it still reports exactly.
        detail: String,
    },
}

impl From<&ByteFidelity> for ByteFidelityDto {
    fn from(fidelity: &ByteFidelity) -> Self {
        match fidelity {
            ByteFidelity::AsTransmitted => Self::AsTransmitted,
            ByteFidelity::Assembled { detail } => Self::Assembled {
                detail: detail.clone(),
            },
        }
    }
}

/// A group heading and its indented members.
#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SourceGroupDto {
    /// Stable identifier, or `null` for the standalone rows.
    pub id: Option<String>,
    /// The verbatim group label, or `null` for standalone rows.
    pub label: Option<String>,
    /// The group checkbox's tri-state.
    pub state: CheckStateDto,
    /// The sources beneath this heading, in display order.
    pub sources: Vec<SourceDto>,
    /// Why this group has no members, or `null` when it is an ordinary group.
    ///
    /// Carries the deferred `Spy on output to destinations` group. The group stays
    /// on screen because the reference screenshots are the design authority and a
    /// control they depict may not be removed — but it may explain what it cannot
    /// yet do, rather than sitting there looking broken.
    pub unavailable_reason: Option<String>,
}

/// Whether the platform's MIDI system could be reached.
///
/// # Why this is a tagged union and not a nullable string
///
/// `detail: string | null` would allow a reason to be present while the system is
/// fine, and would make "available" indistinguishable from "unavailable, no
/// reason given". Neither state should be spellable.
#[derive(Debug, Clone, Serialize, Type)]
#[serde(tag = "type", content = "data", rename_all = "camelCase")]
pub enum MidiSystemStatusDto {
    /// Reached. The catalogue is trustworthy, empty or not.
    Available,
    /// Not reached, and this is why.
    Unavailable {
        /// What the platform reported.
        detail: String,
    },
}

impl From<&MidiSystemStatus> for MidiSystemStatusDto {
    fn from(status: &MidiSystemStatus) -> Self {
        match status {
            MidiSystemStatus::Available => Self::Available,
            MidiSystemStatus::Unavailable { detail } => Self::Unavailable {
                detail: detail.clone(),
            },
        }
    }
}

/// The Sources panel's structure together with how it was obtained.
///
/// # Why the system status travels with the catalogue
///
/// An empty list means two completely different things depending on whether the
/// MIDI system answered. Sending them together makes it impossible for the
/// webview to render one without knowing the other.
#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CatalogueDto {
    /// The groups and their rows, in display order.
    pub groups: Vec<SourceGroupDto>,
    /// Whether the MIDI system could be reached.
    pub midi_system: MidiSystemStatusDto,
    /// How faithfully this platform reports the bytes that arrived.
    ///
    /// Travels with the catalogue rather than with each event because it is a
    /// standing fact about the machine, identical for every row — attaching it
    /// per event would repeat one sentence a hundred thousand times and invite a
    /// reader to think it varied.
    pub byte_fidelity: ByteFidelityDto,
}

/// The result of a mutation that can also change the catalogue.
///
/// # Why only some commands return this
///
/// Selecting a source can surface a port that will not open, which the snapshot
/// has no field for. A filter or column change cannot affect the catalogue, and
/// returning one from those would invite the webview to rebuild the Sources panel
/// on every checkbox tick — implying a coupling that does not exist.
#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct MutationResultDto {
    /// The visible events and counters after the change.
    pub snapshot: SnapshotDto,
    /// The Sources panel after the change.
    pub catalogue: CatalogueDto,
}

/// One filter checkbox, described by Rust so the panel cannot invent entries.
#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct MessageKindDto {
    /// Stable identifier, generated as a TypeScript string-union member.
    pub id: String,
    /// The verbatim checkbox label.
    pub label: String,
    /// The column this sits in, or `null` for the standalone entries.
    pub category: Option<String>,
    /// Whether the box is ticked.
    pub enabled: bool,
}

/// A Filter panel column heading and its entries.
#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct MessageCategoryDto {
    /// Stable identifier.
    pub id: String,
    /// The verbatim heading.
    pub label: String,
    /// The parent checkbox's tri-state.
    pub state: CheckStateDto,
    /// The entries beneath it, in display order.
    pub kinds: Vec<MessageKindDto>,
}

/// The channel radio pair's state.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type)]
#[serde(tag = "type", content = "data", rename_all = "camelCase")]
pub enum ChannelModeDto {
    /// `All Channels` selected.
    AllChannels,
    /// `One Channel` selected, with the chosen channel.
    OneChannel(u8),
}

/// Whether prefix matches are shown or hidden.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum PrefixModeDto {
    /// Show only matches.
    Include,
    /// Hide only matches.
    Exclude,
}

impl From<PrefixModeDto> for PrefixMode {
    fn from(mode: PrefixModeDto) -> Self {
        match mode {
            PrefixModeDto::Include => Self::Include,
            PrefixModeDto::Exclude => Self::Exclude,
        }
    }
}

impl From<PrefixMode> for PrefixModeDto {
    fn from(mode: PrefixMode) -> Self {
        match mode {
            PrefixMode::Include => Self::Include,
            PrefixMode::Exclude => Self::Exclude,
        }
    }
}

/// The full state of the Filter panel.
#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct FilterViewDto {
    /// The three columns and their entries.
    pub categories: Vec<MessageCategoryDto>,
    /// `System Exclusive` and `Invalid`, which sit outside the columns.
    pub standalone: Vec<MessageKindDto>,
    /// The channel radio pair.
    pub channel_mode: ChannelModeDto,
    /// Whether prefix matches are shown or hidden.
    pub prefix_mode: PrefixModeDto,
    /// The prefixes currently applied, normalised.
    pub prefixes: Vec<String>,
}

/// Which columns are shown, in display order.
#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ColumnDto {
    /// Stable identifier.
    pub id: String,
    /// The verbatim header label.
    pub label: String,
    /// Whether it is displayed.
    pub visible: bool,
}

/// The complete visible state, returned by every mutating command.
///
/// # Why every mutation returns one of these
///
/// The webview never patches its own list after a settings change — it replaces
/// it. One round trip, one source of truth, and no way for the visible list to
/// disagree with the controls that produced it.
#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotDto {
    /// Every retained event passing the current filter, oldest first.
    pub events: Vec<EventDto>,
    /// How many events are retained before filtering.
    pub retained_count: u32,
    /// The active retention cap.
    pub retention_limit: u32,
    /// False when no source is selected — an explained empty list, not a stalled one.
    pub monitoring: bool,
    /// The id of the newest event included, or `null` when empty.
    ///
    /// The webview discards stream batches at or below this mark, so a batch in
    /// flight during a settings change cannot resurrect a filtered-out event.
    pub high_water_mark: Option<u32>,
}

impl SnapshotDto {
    /// Renders the monitor's current visible state.
    pub fn from_monitor(monitor: &Monitor) -> Self {
        let events: Vec<EventDto> = monitor
            .visible_events()
            .map(|event| EventDto::from_event(event, monitor.catalogue()))
            .collect();
        let high_water_mark = events.last().map(|event| event.id);

        Self {
            events,
            retained_count: u32::try_from(monitor.retained_count()).unwrap_or(u32::MAX),
            retention_limit: u32::try_from(monitor.retention().get()).unwrap_or(u32::MAX),
            monitoring: monitor.is_monitoring(),
            high_water_mark,
        }
    }
}

/// What the deferred `Spy on output to destinations` group says for itself.
///
/// Observing another application's outgoing MIDI needs privileged system support
/// installed outside this application, which is out of scope for now. The group
/// still appears — the screenshots are the design authority — so it explains
/// itself instead of looking empty and broken.
const SPY_UNAVAILABLE: &str = "Observing output to destinations is not available in this version.";

/// Renders one source row.
fn source_dto(source: &midi_core::domain::source::Source) -> SourceDto {
    SourceDto {
        id: source.id.get(),
        name: source.name.clone(),
        selected: source.selected,
        availability: (&source.availability).into(),
    }
}

/// Builds the Sources panel's structure from the catalogue.
pub fn source_groups(monitor: &Monitor) -> Vec<SourceGroupDto> {
    let catalogue = monitor.catalogue();
    let mut groups = Vec::new();

    for group in SourceGroupId::ALL {
        let sources: Vec<SourceDto> = catalogue
            .sources()
            .iter()
            .filter(|source| source.group == Some(group))
            .map(source_dto)
            .collect();

        // The spy group is always emitted and always empty. Populating it with
        // anything at all would be the fabrication this feature exists to remove.
        let unavailable_reason = match group {
            SourceGroupId::SpyOnOutput => Some(SPY_UNAVAILABLE.to_owned()),
            SourceGroupId::MidiSources => None,
        };

        groups.push(SourceGroupDto {
            id: Some(wire_group_id(group).to_owned()),
            label: Some(group.label().to_owned()),
            state: monitor.group_state(group).into(),
            sources,
            unavailable_reason,
        });

        // The standalone row sits between the two groups in the reference
        // image, so it is emitted in that position rather than appended.
        if group == SourceGroupId::MidiSources {
            let standalone: Vec<SourceDto> = catalogue
                .sources()
                .iter()
                .filter(|source| source.group.is_none())
                .map(source_dto)
                .collect();
            if !standalone.is_empty() {
                groups.push(SourceGroupDto {
                    id: None,
                    label: None,
                    state: CheckStateDto::Unchecked,
                    sources: standalone,
                    unavailable_reason: None,
                });
            }
        }
    }

    groups
}

/// Builds the Sources panel together with the MIDI system's reachability.
pub fn catalogue(monitor: &Monitor) -> CatalogueDto {
    CatalogueDto {
        groups: source_groups(monitor),
        midi_system: monitor.status().into(),
        byte_fidelity: (&monitor.capabilities().byte_fidelity).into(),
    }
}

/// Builds the Filter panel's structure and current state.
pub fn filter_view(monitor: &Monitor) -> FilterViewDto {
    let filter = monitor.filter();

    let categories = MessageCategory::ALL
        .into_iter()
        .map(|category| MessageCategoryDto {
            id: category_id(category).to_owned(),
            label: category.label().to_owned(),
            state: filter.category_state(category).into(),
            kinds: MessageKind::ALL
                .into_iter()
                .filter(|kind| kind.category() == Some(category))
                .map(|kind| kind_dto(kind, filter.kinds.contains(&kind)))
                .collect(),
        })
        .collect();

    let standalone = MessageKind::ALL
        .into_iter()
        .filter(|kind| kind.category().is_none())
        .map(|kind| kind_dto(kind, filter.kinds.contains(&kind)))
        .collect();

    FilterViewDto {
        categories,
        standalone,
        channel_mode: match filter.channel_mode {
            ChannelMode::AllChannels => ChannelModeDto::AllChannels,
            ChannelMode::OneChannel(channel) => ChannelModeDto::OneChannel(channel.get()),
        },
        prefix_mode: filter.data_filter.mode.into(),
        prefixes: filter
            .data_filter
            .prefixes
            .iter()
            .map(|prefix| prefix.as_str().to_owned())
            .collect(),
    }
}

/// Builds the column list with current visibility.
pub fn columns(monitor: &Monitor) -> Vec<ColumnDto> {
    Column::ALL
        .into_iter()
        .map(|column| ColumnDto {
            id: column.id().to_owned(),
            label: column.label().to_owned(),
            visible: monitor.columns().is_visible(column),
        })
        .collect()
}

/// One filter entry, with its current tick state.
fn kind_dto(kind: MessageKind, enabled: bool) -> MessageKindDto {
    MessageKindDto {
        id: kind.id().to_owned(),
        label: kind.label().to_owned(),
        category: kind.category().map(|c| category_id(c).to_owned()),
        enabled,
    }
}

/// The wire identifier for a filter column.
const fn category_id(category: MessageCategory) -> &'static str {
    match category {
        MessageCategory::VoiceMessages => "voiceMessages",
        MessageCategory::SystemCommon => "systemCommon",
        MessageCategory::RealTime => "realTime",
    }
}

/// The wire identifier for a source group.
///
/// Public so command handlers can resolve an identifier back to a group without
/// a second, separately maintained mapping that could drift from this one.
#[must_use]
pub const fn wire_group_id(group: SourceGroupId) -> &'static str {
    match group {
        SourceGroupId::MidiSources => "midiSources",
        SourceGroupId::SpyOnOutput => "spyOnOutput",
    }
}
