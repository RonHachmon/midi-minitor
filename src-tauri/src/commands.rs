//! The command handlers — the application's only entry points from the webview.
//!
//! # What these are allowed to do
//!
//! **Command pattern**, solving the problem of exposing one named, typed
//! operation per user intent. Each handler validates its input, delegates to the
//! monitor, persists, and maps errors. None of them decides whether an event is
//! visible, which source is selected, or what a filter means: those are business
//! rules and they live in the core. A handler that starts branching on message
//! kinds has taken on work that belongs one layer down.
//!
//! Every mutating handler returns a fresh snapshot, so the webview replaces its
//! list rather than patching it.

use crate::dto::{
    catalogue, columns, filter_view, CatalogueDto, ChannelModeDto, ColumnDto, EventBatchDto,
    FilterViewDto, MutationResultDto, PrefixModeDto, SnapshotDto,
};
use crate::error::{IpcError, IpcResult};
use crate::state::AppState;
use midi_core::domain::column::Column;
use midi_core::domain::filter::{ChannelMode, DataPrefixFilter, FilterSettings};
use midi_core::domain::ids::{ChannelNumber, HexPrefix, RetentionLimit, SourceGroupId, SourceId};
use midi_core::domain::message::MessageKind;
use tauri::ipc::Channel;
use tauri::State;

/// The Sources panel's structure, current selections, and MIDI system status.
#[tauri::command]
#[specta::specta]
pub fn get_catalogue(state: State<'_, AppState>) -> IpcResult<CatalogueDto> {
    let monitor = state.monitor()?;
    Ok(catalogue(&monitor))
}

/// Registers the webview's channel for catalogue changes.
///
/// Returns the current catalogue in the same round trip, so there is never a
/// moment where the webview is subscribed but has nothing rendered — the same
/// pattern [`subscribe_events`] uses.
#[tauri::command]
#[specta::specta]
pub fn subscribe_catalogue(
    state: State<'_, AppState>,
    channel: Channel<CatalogueDto>,
) -> IpcResult<CatalogueDto> {
    state.catalogue_pump.subscribe(channel);
    let monitor = state.monitor()?;
    Ok(catalogue(&monitor))
}

/// The Filter panel's structure and current state.
///
/// The panel's entries and labels come from Rust so the interface cannot invent
/// a category or reword a checkbox — the screenshot is the design authority, and
/// these strings are its normative content.
#[tauri::command]
#[specta::specta]
pub fn get_filter_model(state: State<'_, AppState>) -> IpcResult<FilterViewDto> {
    let monitor = state.monitor()?;
    Ok(filter_view(&monitor))
}

/// Which columns exist and which are shown.
#[tauri::command]
#[specta::specta]
pub fn get_columns(state: State<'_, AppState>) -> IpcResult<Vec<ColumnDto>> {
    let monitor = state.monitor()?;
    Ok(columns(&monitor))
}

/// The current visible state, without changing anything.
#[tauri::command]
#[specta::specta]
pub fn snapshot(state: State<'_, AppState>) -> IpcResult<SnapshotDto> {
    let monitor = state.monitor()?;
    Ok(SnapshotDto::from_monitor(&monitor))
}

/// Registers the webview's channel for the event stream.
#[tauri::command]
#[specta::specta]
pub fn subscribe_events(
    state: State<'_, AppState>,
    channel: Channel<EventBatchDto>,
) -> IpcResult<SnapshotDto> {
    state.pump.subscribe(channel);
    let monitor = state.monitor()?;
    Ok(SnapshotDto::from_monitor(&monitor))
}

/// Selects or deselects one source, opening or closing its port to match.
///
/// Returns the catalogue as well as the snapshot: selecting a device can reveal
/// that its port will not open, and the snapshot has nowhere to report that.
#[tauri::command]
#[specta::specta]
pub fn set_source_selected(
    state: State<'_, AppState>,
    source_id: u32,
    selected: bool,
) -> IpcResult<MutationResultDto> {
    let mut monitor = state.monitor()?;
    monitor.set_source_selected(SourceId::new(source_id), selected)?;
    state.sync_ports(&mut monitor);
    state.pump.discard_pending();
    state.persist(&monitor)?;
    Ok(MutationResultDto {
        snapshot: SnapshotDto::from_monitor(&monitor),
        catalogue: catalogue(&monitor),
    })
}

/// Applies one selection state to every source in a group.
#[tauri::command]
#[specta::specta]
pub fn set_group_selected(
    state: State<'_, AppState>,
    group_id: String,
    selected: bool,
) -> IpcResult<MutationResultDto> {
    let group = parse_group(&group_id)?;
    let mut monitor = state.monitor()?;
    monitor.set_group_selected(group, selected);
    state.sync_ports(&mut monitor);
    state.pump.discard_pending();
    state.persist(&monitor)?;
    Ok(MutationResultDto {
        snapshot: SnapshotDto::from_monitor(&monitor),
        catalogue: catalogue(&monitor),
    })
}

/// Replaces the message-kind and channel filter.
#[tauri::command]
#[specta::specta]
pub fn set_filter(
    state: State<'_, AppState>,
    kinds: Vec<String>,
    channel_mode: ChannelModeDto,
) -> IpcResult<SnapshotDto> {
    let channel_mode = match channel_mode {
        ChannelModeDto::AllChannels => ChannelMode::AllChannels,
        ChannelModeDto::OneChannel(channel) => {
            ChannelMode::OneChannel(ChannelNumber::new(channel)?)
        }
    };

    // Unrecognised identifiers are ignored rather than rejected: they can only
    // come from a webview built against an older contract, and silently
    // narrowing the filter is safer than refusing to apply any of it.
    let kinds = MessageKind::ALL
        .into_iter()
        .filter(|kind| kinds.iter().any(|id| id == kind.id()))
        .collect();

    let mut monitor = state.monitor()?;
    monitor.set_filter(FilterSettings {
        kinds,
        channel_mode,
        data_filter: DataPrefixFilter::default(),
    });
    state.pump.discard_pending();
    state.persist(&monitor)?;
    Ok(SnapshotDto::from_monitor(&monitor))
}

/// Replaces the hexadecimal prefix filter.
///
/// # Errors
///
/// A malformed entry fails **before** any state is changed, so the previously
/// applied filter stays in effect and the event list does not blank out because
/// of a typo.
#[tauri::command]
#[specta::specta]
pub fn set_data_prefix_filter(
    state: State<'_, AppState>,
    mode: PrefixModeDto,
    prefixes: Vec<String>,
) -> IpcResult<SnapshotDto> {
    let parsed = prefixes
        .iter()
        .map(|entry| HexPrefix::parse(entry))
        .collect::<Result<Vec<_>, _>>()?;

    let mut monitor = state.monitor()?;
    monitor.set_data_prefix_filter(DataPrefixFilter {
        mode: mode.into(),
        prefixes: parsed,
    });
    state.pump.discard_pending();
    state.persist(&monitor)?;
    Ok(SnapshotDto::from_monitor(&monitor))
}

/// Replaces the retention cap.
#[tauri::command]
#[specta::specta]
pub fn set_retention_limit(state: State<'_, AppState>, limit: u32) -> IpcResult<SnapshotDto> {
    let limit = RetentionLimit::new(limit as usize)?;
    let mut monitor = state.monitor()?;
    monitor.set_retention(limit);
    state.persist(&monitor)?;
    Ok(SnapshotDto::from_monitor(&monitor))
}

/// Discards every retained event.
#[tauri::command]
#[specta::specta]
pub fn clear_events(state: State<'_, AppState>) -> IpcResult<SnapshotDto> {
    let mut monitor = state.monitor()?;
    monitor.clear();
    state.pump.discard_pending();
    Ok(SnapshotDto::from_monitor(&monitor))
}

/// Replaces which columns are shown.
///
/// Returns the column list rather than a snapshot: returning events here would
/// imply visibility affects which of them are listed, and it must not.
#[tauri::command]
#[specta::specta]
pub fn set_column_visibility(
    state: State<'_, AppState>,
    visible: Vec<String>,
) -> IpcResult<Vec<ColumnDto>> {
    let visible: Vec<Column> = Column::ALL
        .into_iter()
        .filter(|column| visible.iter().any(|id| id == column.id()))
        .collect();

    let mut monitor = state.monitor()?;
    monitor.set_columns(&visible)?;
    state.persist(&monitor)?;
    Ok(columns(&monitor))
}

/// Resolves a wire group identifier.
fn parse_group(id: &str) -> IpcResult<SourceGroupId> {
    match SourceGroupId::ALL
        .into_iter()
        .find(|group| crate::dto::wire_group_id(*group) == id)
    {
        Some(group) => Ok(group),
        None => Err(IpcError::UnknownGroup { id: id.to_owned() }),
    }
}
