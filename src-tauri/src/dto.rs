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

use midi_core::application::capture::CaptureState;
use midi_core::application::monitor::Monitor;
use midi_core::application::ports::{ByteFidelity, MidiSystemStatus, PublicationSupport};
use midi_core::application::sender::Sender;
use midi_core::domain::column::Column;
use midi_core::domain::display::{
    group_label, ControllerFormat, DataFormat, DisplaySettings, ExpertMode, NoteFormat,
    ProgramNumbering, TimeFormat,
};
use midi_core::domain::event::MidiEvent;
use midi_core::domain::filter::{ChannelMode, DataPrefixRule, PrefixMode};
use midi_core::domain::ids::{SourceGroupId, TickRate};
use midi_core::domain::message::{MessageCategory, MessageKind};
use midi_core::domain::rendering;
use midi_core::domain::request::RequestOrigin;
use midi_core::domain::send_record::SendOutcome;
use midi_core::domain::sendable::{FieldKind, SendableKind};
use midi_core::domain::source::{Availability, CheckState, SourceCatalogue};
use midi_core::domain::target::TargetKind;
use serde::{Deserialize, Serialize};
use specta::Type;

/// One row of the event table.
#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct EventDto {
    /// Arrival order; the list key and the stream's high-water mark.
    pub id: u32,
    /// Pre-formatted per the chosen time format.
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
    ///
    /// # Why the display settings are a parameter
    ///
    /// The three cells below are rendered from them, and a row must be
    /// re-renderable at any time — a format chosen now applies to events
    /// captured before it. Passing the settings in rather than baking a
    /// rendering at capture is what lets `set_display_settings` rebuild a whole
    /// snapshot from events the monitor already holds.
    pub fn from_event(
        event: &MidiEvent,
        catalogue: &SourceCatalogue,
        settings: DisplaySettings,
        rate: TickRate,
    ) -> Self {
        Self {
            id: event.id.get(),
            time: rendering::time_display(event, settings, rate),
            source: catalogue
                .event_label(event.source)
                .unwrap_or_else(|| "(unknown source)".to_owned()),
            message: rendering::message_display(event, settings).to_owned(),
            channel: event.channel().map(|channel| channel.get()),
            data: rendering::data_display(event, settings),
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

/// Whether the monitor is taking in what arrives, or holding what it has.
///
/// # Why this crosses the wire at all
///
/// The control must show which state the monitor is in, and the core is the only
/// component that knows. A frozen list that cannot say it is frozen is
/// indistinguishable from a stalled one — which is the failure the control exists
/// to prevent.
///
/// Tagged union rather than a boolean, matching every other state the webview
/// renders: a third capture state would become a type error at each site instead
/// of silently rendering as though it were one of these two.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type)]
#[serde(tag = "type", content = "data", rename_all = "camelCase")]
pub enum CaptureStateDto {
    /// Arriving events are being retained and streamed.
    Running,
    /// Arriving events are discarded; what was retained is untouched.
    Paused,
}

impl From<CaptureStateDto> for CaptureState {
    fn from(capture: CaptureStateDto) -> Self {
        match capture {
            CaptureStateDto::Running => Self::Running,
            CaptureStateDto::Paused => Self::Paused,
        }
    }
}

impl From<CaptureState> for CaptureStateDto {
    fn from(capture: CaptureState) -> Self {
        match capture {
            CaptureState::Running => Self::Running,
            CaptureState::Paused => Self::Paused,
        }
    }
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

/// One data prefix rule as the panel lists it.
///
/// The prefix crosses as the **normalised** string the core stores, and the
/// interface sends that same string back to delete the rule. Normalising in one
/// place keeps `9a`, `9A`, and `9 A` from being three different rules depending
/// on which side of the boundary looked at them.
#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DataPrefixRuleDto {
    /// The normalised nibble prefix — also this rule's identity.
    pub prefix: String,
    /// Whether matches are the only events shown, or the only ones hidden.
    pub kind: PrefixModeDto,
}

impl From<&DataPrefixRule> for DataPrefixRuleDto {
    fn from(rule: &DataPrefixRule) -> Self {
        Self {
            prefix: rule.prefix.as_str().to_owned(),
            kind: rule.kind.into(),
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
    /// The data prefix rules in force, in the order they were added.
    pub rules: Vec<DataPrefixRuleDto>,
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
    /// Whether the monitor is taking in what arrives.
    ///
    /// # Why this is separate from `monitoring`
    ///
    /// Both can leave the list empty and motionless, and the user's next move is
    /// different for each: select a source, versus press Resume. Folding them
    /// into one flag would make the interface explain one of the two situations
    /// wrongly, every time.
    pub capture_state: CaptureStateDto,
}

impl SnapshotDto {
    /// Renders the monitor's current visible state.
    pub fn from_monitor(monitor: &Monitor) -> Self {
        let events: Vec<EventDto> = monitor
            .visible_events()
            .map(|event| {
                EventDto::from_event(
                    event,
                    monitor.catalogue(),
                    monitor.display(),
                    monitor.tick_rate(),
                )
            })
            .collect();
        let high_water_mark = events.last().map(|event| event.id);

        Self {
            events,
            retained_count: u32::try_from(monitor.retained_count()).unwrap_or(u32::MAX),
            retention_limit: u32::try_from(monitor.retention().get()).unwrap_or(u32::MAX),
            monitoring: monitor.is_monitoring(),
            high_water_mark,
            capture_state: monitor.capture_state().into(),
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
        rules: filter
            .data_filter
            .rules
            .iter()
            .map(DataPrefixRuleDto::from)
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

/// One place traffic can be sent to.
#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TargetDto {
    /// Session identity, and what the picker sends back.
    pub id: u32,
    /// The name to show, as the system or the user supplied it.
    pub name: String,
    /// Whether this is a device or this application's own published source.
    pub kind: TargetKindDto,
}

/// What sort of place a target is.
#[derive(Debug, Clone, Copy, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum TargetKindDto {
    /// A MIDI destination the operating system reports.
    Destination,
    /// This application's own published source.
    ///
    /// The interface must be able to tell this apart, because only this kind
    /// carries the name the user chose — traffic sent to a destination carries
    /// whatever origin the system reports for this application.
    PublishedSource,
}

impl From<TargetKind> for TargetKindDto {
    fn from(kind: TargetKind) -> Self {
        match kind {
            TargetKind::Destination => Self::Destination,
            TargetKind::PublishedSource => Self::PublishedSource,
        }
    }
}

/// The target list alone, pushed when devices come and go.
#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TargetsDto {
    /// Every target available right now.
    pub targets: Vec<TargetDto>,
    /// The chosen target's id, or `null` when nothing is chosen or the choice is
    /// not currently present.
    pub chosen: Option<u32>,
}

/// Whether this platform can publish a MIDI source.
#[derive(Debug, Clone, Serialize, Type)]
#[serde(tag = "type", content = "data", rename_all = "camelCase")]
pub enum PublicationSupportDto {
    /// It can, and the control is operable.
    Supported,
    /// It cannot. The control must not be operable, and must say this.
    Unsupported {
        /// What the platform cannot do, and what to do instead.
        detail: String,
    },
}

impl From<&PublicationSupport> for PublicationSupportDto {
    fn from(support: &PublicationSupport) -> Self {
        match support {
            PublicationSupport::Supported => Self::Supported,
            PublicationSupport::Unsupported { detail } => Self::Unsupported {
                detail: detail.clone(),
            },
        }
    }
}

/// The disguise, as the screen shows it.
#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PublicationDto {
    /// The name other programs see, and remember their settings against.
    pub name: String,
    /// Whether the source is published right now.
    pub published: bool,
    /// Whether this platform can publish at all.
    pub support: PublicationSupportDto,
    /// Why the last attempt to publish failed, if it did.
    ///
    /// Carries the startup restore's failure, which has no command to be
    /// returned from and would otherwise reach the user as an unexplained
    /// unpublished source.
    pub error: Option<String>,
}

/// One composable message type, for the picker.
#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SendableKindDto {
    /// Stable identifier.
    pub id: String,
    /// The name shown in the picker.
    pub label: String,
}

/// One editable value on the message being composed.
///
/// Carries its own label and bounds so the screen renders what it is handed and
/// holds no table of which message carries which values.
#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct FieldDto {
    /// Stable identifier, and what the set command sends back.
    pub id: String,
    /// What the value means, in the user's words.
    pub label: String,
    /// Lowest accepted value, or `null` for a byte entry.
    pub min: Option<u16>,
    /// Highest accepted value, or `null` for a byte entry.
    pub max: Option<u16>,
    /// The current value, in the notation the field accepts.
    pub value: String,
}

/// The message being composed, and what it will send.
#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CompositionDto {
    /// Which message type is composed, or `null` for hand-typed bytes.
    pub kind: Option<String>,
    /// Every composable message type, for the picker.
    pub kinds: Vec<SendableKindDto>,
    /// The values this message carries. Empty for hand-typed bytes.
    pub fields: Vec<FieldDto>,
    /// Exactly what will be transmitted, as spaced uppercase hexadecimal.
    ///
    /// Rendered here rather than in TypeScript for the reason every other display
    /// value is: the bytes are a fact the core owns, and computing them on the far
    /// side of the boundary would make the preview a second opinion rather than
    /// the thing itself.
    pub preview: String,
    /// Set when the composition cannot currently be encoded.
    ///
    /// Never populated by any composition this application can build; present so
    /// the screen has somewhere to put the truth rather than showing an empty
    /// preview that looks like an empty message.
    pub preview_error: Option<String>,
}

/// One request in the library.
#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RequestDto {
    /// The name, which is also the request's identity.
    pub name: String,
    /// What it does, in plain language.
    pub description: String,
    /// The bytes it sends at its stored values.
    pub preview: String,
    /// Whether the user may rename or delete it.
    ///
    /// A rendered fact rather than a rule the screen derives, so there is one
    /// definition of what a built-in is.
    pub built_in: bool,
}

/// One attempted send.
#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SendRecordDto {
    /// Identity, and what a re-send names.
    pub id: u32,
    /// Pre-formatted per the chosen time format.
    pub time: String,
    /// The target's name as it was at the time of the send.
    pub target: String,
    /// What was sent, named the way the event table names it.
    pub message: String,
    /// The bytes, as spaced uppercase hexadecimal.
    pub data: String,
    /// Why it failed, or `null` when it was transmitted.
    ///
    /// Transmitted means the platform accepted the bytes and nothing more —
    /// whether a receiving program acted on them is unknowable from here, and the
    /// interface must not imply otherwise.
    pub failure: Option<String>,
}

/// Everything the send screen shows.
///
/// Returned by every mutating send command, for the reason [`SnapshotDto`] is:
/// the webview replaces its model rather than patching it, so the two sides
/// cannot come to disagree about what is composed.
#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SendViewDto {
    /// Where traffic can go, and which is chosen.
    pub targets: TargetsDto,
    /// The disguise, and whether this platform can offer it.
    pub publication: PublicationDto,
    /// The message being built.
    pub composition: CompositionDto,
    /// The built-in library and the user's own requests.
    pub requests: Vec<RequestDto>,
    /// This session's sends, newest last.
    pub records: Vec<SendRecordDto>,
}

/// Formats bytes as spaced uppercase hexadecimal.
///
/// Spaced, unlike the raw hex on [`EventDto`], which is separator-free because a
/// prefix rule matches against it as a plain string. This one is only ever read
/// by a person, and `90 3C 64` is what a person checks against a specification.
#[must_use]
pub fn spaced_hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| format!("{byte:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// The target list and the current choice.
#[must_use]
pub fn targets(sender: &Sender) -> TargetsDto {
    TargetsDto {
        targets: sender
            .targets()
            .iter()
            .map(|target| TargetDto {
                id: target.id.get(),
                name: target.name.clone(),
                kind: target.kind.into(),
            })
            .collect(),
        chosen: sender.chosen_target().map(|target| target.id.get()),
    }
}

/// The message being composed, with its fields and its byte preview.
#[must_use]
pub fn composition(sender: &Sender) -> CompositionDto {
    let composition = sender.composition();
    let (preview, preview_error) = match composition.bytes() {
        Ok(bytes) => (spaced_hex(&bytes), None),
        Err(error) => (String::new(), Some(error.to_string())),
    };

    CompositionDto {
        kind: SendableKind::of(composition.message()).map(|kind| kind.id().to_owned()),
        kinds: SendableKind::ALL
            .into_iter()
            .map(|kind| SendableKindDto {
                id: kind.id().to_owned(),
                label: kind.label().to_owned(),
            })
            .collect(),
        fields: composition
            .fields()
            .into_iter()
            .map(|spec| {
                let (min, max) = match spec.kind {
                    FieldKind::Number { min, max } => (Some(min), Some(max)),
                    FieldKind::Bytes => (None, None),
                };
                FieldDto {
                    id: spec.id.id().to_owned(),
                    label: spec.label.to_owned(),
                    min,
                    max,
                    value: composition.field_value(spec.id).unwrap_or_default(),
                }
            })
            .collect(),
        preview,
        preview_error,
    }
}

/// This session's sends.
#[must_use]
pub fn send_records(sender: &Sender) -> Vec<SendRecordDto> {
    sender
        .records()
        .map(|record| SendRecordDto {
            id: record.id.get(),
            time: record.time.to_display(),
            target: record.target.clone(),
            message: record.message.display_name().to_owned(),
            data: spaced_hex(&record.bytes),
            failure: match &record.outcome {
                SendOutcome::Sent => None,
                SendOutcome::Failed { detail } => Some(detail.clone()),
            },
        })
        .collect()
}

/// The whole send screen model.
#[must_use]
pub fn send_view(sender: &Sender) -> SendViewDto {
    SendViewDto {
        targets: targets(sender),
        publication: PublicationDto {
            name: sender.publication().name.as_str().to_owned(),
            published: sender.publication().published,
            support: (&sender.capabilities().publication).into(),
            error: sender.publication_error().map(str::to_owned),
        },
        composition: composition(sender),
        requests: requests(sender),
        records: send_records(sender),
    }
}

/// The library, built-ins first and then the user's own.
///
/// Each entry's preview is the bytes it would send at the values it was stored
/// with, so the list can be read without loading anything into the composer.
#[must_use]
pub fn requests(sender: &Sender) -> Vec<RequestDto> {
    sender
        .library()
        .all()
        .map(|request| RequestDto {
            name: request.name.as_str().to_owned(),
            description: request.description.clone(),
            preview: request
                .composition
                .bytes()
                .map(|bytes| spaced_hex(&bytes))
                .unwrap_or_default(),
            built_in: request.origin == RequestOrigin::BuiltIn,
        })
        .collect()
}

/// The six display choices, as the webview holds them.
///
/// # Why every field is a generated union rather than a string
///
/// A bare `String` would let the webview compose a value the core has no variant
/// for, and the mistake would surface as a silent fallback at render time.
/// Generated unions make it a TypeScript compile error instead — which, with no
/// test suite, is the only place it can be caught.
///
/// `expert` crosses as a boolean because it is a checkbox on the wire; it
/// becomes [`ExpertMode`] at this boundary, which is what a translation layer is
/// for.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DisplaySettingsDto {
    /// What the Time column shows.
    pub time: TimeFormatDto,
    /// How note numbers are written.
    pub note: NoteFormatDto,
    /// How controller numbers are written.
    pub controller: ControllerFormatDto,
    /// The base for every remaining value.
    pub data: DataFormatDto,
    /// Whether programs are counted from one or from zero.
    pub program: ProgramNumberingDto,
    /// Whether the monitor's conveniences are suppressed.
    pub expert: bool,
}

/// Wire form of [`TimeFormat`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum TimeFormatDto {
    /// `Clock time`.
    ClockTime,
    /// `Host time (integer)`.
    HostInteger,
    /// `Host time (seconds)`.
    HostSeconds,
    /// `Host time (nanoseconds)`.
    HostNanoseconds,
}

/// Wire form of [`NoteFormat`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum NoteFormatDto {
    /// `Note (Middle C = C3)`.
    NameMiddleC3,
    /// `Note (Middle C = C4)`.
    NameMiddleC4,
    /// `Decimal number`.
    Decimal,
    /// `Hexadecimal number`.
    Hexadecimal,
}

/// Wire form of [`ControllerFormat`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum ControllerFormatDto {
    /// `Standard name`.
    StandardName,
    /// `Decimal number`.
    Decimal,
    /// `Hexadecimal number`.
    Hexadecimal,
}

/// Wire form of [`DataFormat`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum DataFormatDto {
    /// `Decimal number`.
    Decimal,
    /// `Hexadecimal number`.
    Hexadecimal,
}

/// Wire form of [`ProgramNumbering`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum ProgramNumberingDto {
    /// `1 – 128 (Standard)`.
    FromOne,
    /// `0 – 127 (Less common)`.
    FromZero,
}

impl From<DisplaySettings> for DisplaySettingsDto {
    fn from(settings: DisplaySettings) -> Self {
        Self {
            time: settings.time.into(),
            note: settings.note.into(),
            controller: settings.controller.into(),
            data: settings.data.into(),
            program: settings.program.into(),
            expert: settings.expert.is_on(),
        }
    }
}

impl From<DisplaySettingsDto> for DisplaySettings {
    fn from(dto: DisplaySettingsDto) -> Self {
        Self {
            time: dto.time.into(),
            note: dto.note.into(),
            controller: dto.controller.into(),
            data: dto.data.into(),
            program: dto.program.into(),
            expert: ExpertMode::from(dto.expert),
        }
    }
}

impl From<TimeFormat> for TimeFormatDto {
    fn from(value: TimeFormat) -> Self {
        match value {
            TimeFormat::ClockTime => Self::ClockTime,
            TimeFormat::HostInteger => Self::HostInteger,
            TimeFormat::HostSeconds => Self::HostSeconds,
            TimeFormat::HostNanoseconds => Self::HostNanoseconds,
        }
    }
}

impl From<TimeFormatDto> for TimeFormat {
    fn from(value: TimeFormatDto) -> Self {
        match value {
            TimeFormatDto::ClockTime => Self::ClockTime,
            TimeFormatDto::HostInteger => Self::HostInteger,
            TimeFormatDto::HostSeconds => Self::HostSeconds,
            TimeFormatDto::HostNanoseconds => Self::HostNanoseconds,
        }
    }
}

impl From<NoteFormat> for NoteFormatDto {
    fn from(value: NoteFormat) -> Self {
        match value {
            NoteFormat::NameMiddleC3 => Self::NameMiddleC3,
            NoteFormat::NameMiddleC4 => Self::NameMiddleC4,
            NoteFormat::Decimal => Self::Decimal,
            NoteFormat::Hexadecimal => Self::Hexadecimal,
        }
    }
}

impl From<NoteFormatDto> for NoteFormat {
    fn from(value: NoteFormatDto) -> Self {
        match value {
            NoteFormatDto::NameMiddleC3 => Self::NameMiddleC3,
            NoteFormatDto::NameMiddleC4 => Self::NameMiddleC4,
            NoteFormatDto::Decimal => Self::Decimal,
            NoteFormatDto::Hexadecimal => Self::Hexadecimal,
        }
    }
}

impl From<ControllerFormat> for ControllerFormatDto {
    fn from(value: ControllerFormat) -> Self {
        match value {
            ControllerFormat::StandardName => Self::StandardName,
            ControllerFormat::Decimal => Self::Decimal,
            ControllerFormat::Hexadecimal => Self::Hexadecimal,
        }
    }
}

impl From<ControllerFormatDto> for ControllerFormat {
    fn from(value: ControllerFormatDto) -> Self {
        match value {
            ControllerFormatDto::StandardName => Self::StandardName,
            ControllerFormatDto::Decimal => Self::Decimal,
            ControllerFormatDto::Hexadecimal => Self::Hexadecimal,
        }
    }
}

impl From<DataFormat> for DataFormatDto {
    fn from(value: DataFormat) -> Self {
        match value {
            DataFormat::Decimal => Self::Decimal,
            DataFormat::Hexadecimal => Self::Hexadecimal,
        }
    }
}

impl From<DataFormatDto> for DataFormat {
    fn from(value: DataFormatDto) -> Self {
        match value {
            DataFormatDto::Decimal => Self::Decimal,
            DataFormatDto::Hexadecimal => Self::Hexadecimal,
        }
    }
}

impl From<ProgramNumbering> for ProgramNumberingDto {
    fn from(value: ProgramNumbering) -> Self {
        match value {
            ProgramNumbering::FromOne => Self::FromOne,
            ProgramNumbering::FromZero => Self::FromZero,
        }
    }
}

impl From<ProgramNumberingDto> for ProgramNumbering {
    fn from(value: ProgramNumberingDto) -> Self {
        match value {
            ProgramNumberingDto::FromOne => Self::FromOne,
            ProgramNumberingDto::FromZero => Self::FromZero,
        }
    }
}

/// One selectable option within a radio group.
#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DisplayOptionDto {
    /// The wire value this option selects, matching the generated union member.
    pub id: String,
    /// The verbatim label from `screenshots/setting.jpg`.
    pub label: String,
    /// Whether this is the option currently in effect.
    pub selected: bool,
}

/// One labelled radio group on the `Display` tab.
#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DisplayGroupDto {
    /// Which setting this group sets.
    pub id: String,
    /// The verbatim group label, such as `Time format`.
    pub label: String,
    /// A second line beneath the label, or `null`.
    ///
    /// `(Decimal)` for `Program number`, which the reference image sets on two
    /// lines. Null rather than an empty string, so the renderer cannot print a
    /// stray blank line for the other four.
    pub second_line: Option<String>,
    /// The options, in the reference image's order.
    pub options: Vec<DisplayOptionDto>,
}

/// One tab of the preferences surface.
///
/// # Why the unimplemented tabs are described here rather than in the webview
///
/// The core decides which tabs are usable, so the interface cannot render an
/// operable control for one that does nothing. Filling `Sources` in later is
/// then a flag and a list of groups, not a restructuring of the screen.
#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PreferencesTabDto {
    /// Stable identifier.
    pub id: String,
    /// The verbatim tab label: `Display`, `Sources`, or `Other`.
    pub label: String,
    /// Whether this tab has settings to show.
    pub available: bool,
    /// What an unavailable tab says for itself, or `null` when available.
    pub unavailable_note: Option<String>,
}

/// The whole `Display` tab, as the webview renders it.
#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DisplayViewDto {
    /// The three tabs, in the reference image's order.
    pub tabs: Vec<PreferencesTabDto>,
    /// The five radio groups, in the reference image's order.
    pub groups: Vec<DisplayGroupDto>,
    /// The checkbox's verbatim label.
    pub expert_label: String,
    /// Whether the checkbox is ticked.
    pub expert_enabled: bool,
    /// The three explanatory lines beneath it, verbatim and in order.
    pub expert_notes: Vec<String>,
    /// The same state as values, for the webview to echo back on a change.
    pub settings: DisplaySettingsDto,
}

/// What a display change returns: the tab, and the table it re-rendered.
///
/// # Why both travel together
///
/// One change, one payload. Making the webview fetch the snapshot separately
/// would leave an interval in which the settings had changed and the visible
/// rows had not. Follows [`CatalogueChangeDto`], which pairs its two halves for
/// the same reason.
#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DisplayChangeDto {
    /// The tab after the change.
    pub view: DisplayViewDto,
    /// Retained events, re-rendered under the new settings.
    pub snapshot: SnapshotDto,
}

/// What the `Sources` tab of the preferences surface says for itself.
///
/// Not the monitor's existing Sources panel, which is a different surface and is
/// untouched by this feature — hence the pointer to where that panel actually
/// lives, so the message is useful rather than merely apologetic.
const SOURCES_TAB_UNAVAILABLE: &str = "Source preferences are not available in this version. Choose which sources to monitor from the Sources section of the Monitor screen.";

/// What the `Other` tab says for itself.
const OTHER_TAB_UNAVAILABLE: &str = "Other preferences are not available in this version.";

/// Builds the `Display` tab for the webview.
///
/// # Why the labels come from here
///
/// `get_filter_model` set the precedent and stated the reason: the panel's
/// entries and labels come from Rust so the interface cannot invent an option or
/// reword a control. `screenshots/setting.jpg` is the design authority, and
/// strings such as `Note (Middle C = C3)` and `1 – 128 (Standard)` — the latter
/// carrying an en dash rather than a hyphen — are its normative content, not
/// styling.
#[must_use]
pub fn display_view(settings: DisplaySettings) -> DisplayViewDto {
    DisplayViewDto {
        tabs: vec![
            PreferencesTabDto {
                id: "display".to_owned(),
                label: "Display".to_owned(),
                available: true,
                unavailable_note: None,
            },
            PreferencesTabDto {
                id: "sources".to_owned(),
                label: "Sources".to_owned(),
                available: false,
                unavailable_note: Some(SOURCES_TAB_UNAVAILABLE.to_owned()),
            },
            PreferencesTabDto {
                id: "other".to_owned(),
                label: "Other".to_owned(),
                available: false,
                unavailable_note: Some(OTHER_TAB_UNAVAILABLE.to_owned()),
            },
        ],
        groups: vec![
            DisplayGroupDto {
                id: "time".to_owned(),
                label: group_label::TIME.to_owned(),
                second_line: None,
                options: TimeFormat::ALL
                    .into_iter()
                    .map(|option| DisplayOptionDto {
                        id: wire_id(TimeFormatDto::from(option)),
                        label: option.label().to_owned(),
                        selected: option == settings.time,
                    })
                    .collect(),
            },
            DisplayGroupDto {
                id: "note".to_owned(),
                label: group_label::NOTE.to_owned(),
                second_line: None,
                options: NoteFormat::ALL
                    .into_iter()
                    .map(|option| DisplayOptionDto {
                        id: wire_id(NoteFormatDto::from(option)),
                        label: option.label().to_owned(),
                        selected: option == settings.note,
                    })
                    .collect(),
            },
            DisplayGroupDto {
                id: "controller".to_owned(),
                label: group_label::CONTROLLER.to_owned(),
                second_line: None,
                options: ControllerFormat::ALL
                    .into_iter()
                    .map(|option| DisplayOptionDto {
                        id: wire_id(ControllerFormatDto::from(option)),
                        label: option.label().to_owned(),
                        selected: option == settings.controller,
                    })
                    .collect(),
            },
            DisplayGroupDto {
                id: "data".to_owned(),
                label: group_label::DATA.to_owned(),
                second_line: None,
                options: DataFormat::ALL
                    .into_iter()
                    .map(|option| DisplayOptionDto {
                        id: wire_id(DataFormatDto::from(option)),
                        label: option.label().to_owned(),
                        selected: option == settings.data,
                    })
                    .collect(),
            },
            DisplayGroupDto {
                id: "program".to_owned(),
                label: group_label::PROGRAM.to_owned(),
                second_line: Some(group_label::PROGRAM_SECOND_LINE.to_owned()),
                options: ProgramNumbering::ALL
                    .into_iter()
                    .map(|option| DisplayOptionDto {
                        id: wire_id(ProgramNumberingDto::from(option)),
                        label: option.label().to_owned(),
                        selected: option == settings.program,
                    })
                    .collect(),
            },
        ],
        expert_label: ExpertMode::LABEL.to_owned(),
        expert_enabled: settings.expert.is_on(),
        expert_notes: settings
            .expert
            .notes()
            .iter()
            .map(|note| (*note).to_owned())
            .collect(),
        settings: settings.into(),
    }
}

/// The wire spelling of one settings union member.
///
/// # Why this goes through serialization rather than a hand-written table
///
/// The option ids the webview sends back must match the union members `specta`
/// generates, and both come from the same `#[serde(rename_all)]` attribute. A
/// second hand-maintained spelling beside it would be the parallel definition
/// Principle IV forbids, and it would drift silently, because nothing compiles
/// the two against each other.
fn wire_id<T: Serialize>(value: T) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(ToOwned::to_owned))
        .unwrap_or_default()
}
