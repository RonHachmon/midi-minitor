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
    catalogue, columns, filter_view, CaptureStateDto, CatalogueDto, ChannelModeDto, ColumnDto,
    EventBatchDto, FilterViewDto, MutationResultDto, PrefixModeDto, SnapshotDto,
};
use crate::dto::{display_view, DisplayChangeDto, DisplaySettingsDto, DisplayViewDto};
use crate::dto::{send_view, targets, SendViewDto, TargetsDto};
use crate::error::{IpcError, IpcResult};
use crate::state::AppState;
use midi_core::domain::column::Column;
use midi_core::domain::filter::{ChannelMode, DataPrefixFilter, DataPrefixRule, FilterSettings};
use midi_core::domain::ids::{
    ChannelNumber, HexPrefix, RequestName, RetentionLimit, SourceGroupId, SourceId, TargetId,
};
use midi_core::domain::ids::{PublishedName, SendRecordId};
use midi_core::domain::message::MessageKind;
use midi_core::domain::sendable::{FieldId, SendableKind};
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
    let sender = state.sender()?;
    state.persist(&monitor, &sender)?;
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
    let sender = state.sender()?;
    state.persist(&monitor, &sender)?;
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
    let sender = state.sender()?;
    state.persist(&monitor, &sender)?;
    Ok(SnapshotDto::from_monitor(&monitor))
}

/// Adds one data prefix rule.
///
/// # Errors
///
/// A malformed entry, an empty one, or one that clashes with a listed rule fails
/// **before** any state is changed, so the rules in force keep running and the
/// event list does not blank out because of a typo or a refused addition.
#[tauri::command]
#[specta::specta]
pub fn add_data_prefix_rule(
    state: State<'_, AppState>,
    prefix: String,
    kind: PrefixModeDto,
) -> IpcResult<SnapshotDto> {
    let prefix = HexPrefix::parse(&prefix)?;

    let mut monitor = state.monitor()?;
    monitor.add_prefix_rule(DataPrefixRule::new(prefix, kind.into()))?;
    state.pump.discard_pending();
    let sender = state.sender()?;
    state.persist(&monitor, &sender)?;
    Ok(SnapshotDto::from_monitor(&monitor))
}

/// Removes the data prefix rule carrying this prefix.
///
/// # Errors
///
/// Returns [`IpcError::UnknownPrefixRule`] when no rule carries it, which means
/// the interface is working from a rule list the core has since changed.
///
/// The prefix is parsed rather than compared raw so that the same normalisation
/// applies on the way out as on the way in — an interface echoing back what it
/// was given always matches, and a hand-built call in a different notation still
/// finds the right rule.
#[tauri::command]
#[specta::specta]
pub fn remove_data_prefix_rule(
    state: State<'_, AppState>,
    prefix: String,
) -> IpcResult<SnapshotDto> {
    let prefix = HexPrefix::parse(&prefix)?;

    let mut monitor = state.monitor()?;
    monitor.remove_prefix_rule(&prefix)?;
    state.pump.discard_pending();
    let sender = state.sender()?;
    state.persist(&monitor, &sender)?;
    Ok(SnapshotDto::from_monitor(&monitor))
}

/// Replaces the retention cap.
#[tauri::command]
#[specta::specta]
pub fn set_retention_limit(state: State<'_, AppState>, limit: u32) -> IpcResult<SnapshotDto> {
    let limit = RetentionLimit::new(limit as usize)?;
    let mut monitor = state.monitor()?;
    monitor.set_retention(limit);
    let sender = state.sender()?;
    state.persist(&monitor, &sender)?;
    Ok(SnapshotDto::from_monitor(&monitor))
}

/// Starts or stops taking in what arrives.
///
/// # Why the pending batch is discarded
///
/// The pump flushes on a frame timer, so up to one interval's worth of events
/// can already be queued when the user pauses. Delivering them afterwards would
/// append rows to a list the user has just frozen. They are dropped rather than
/// lost: every one of them was retained before the pause and is therefore
/// present in the snapshot this returns.
///
/// # Why this does not persist
///
/// The capture state is session-only by design — see
/// [`midi_core::application::capture::CaptureState`]. Writing it would mean a
/// relaunch could open a window that shows nothing and never updates.
#[tauri::command]
#[specta::specta]
pub fn set_capture_state(
    state: State<'_, AppState>,
    capture: CaptureStateDto,
) -> IpcResult<SnapshotDto> {
    let mut monitor = state.monitor()?;
    monitor.set_capture_state(capture.into());
    state.pump.discard_pending();
    Ok(SnapshotDto::from_monitor(&monitor))
}

/// Discards every retained event.
///
/// Behaves identically whether the monitor is running or paused: it empties the
/// log and changes nothing else, so pausing and clearing stay independent.
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
    let sender = state.sender()?;
    state.persist(&monitor, &sender)?;
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

/// The whole send screen model.
///
/// Read once when the screen mounts. Every mutating send command returns the same
/// shape, so the screen replaces its model rather than patching it.
#[tauri::command]
#[specta::specta]
pub fn get_send_view(state: State<'_, AppState>) -> IpcResult<SendViewDto> {
    let sender = state.sender()?;
    Ok(send_view(&sender))
}

/// Subscribes to target-list changes and returns the list as it stands.
///
/// Returning the current list matters: without it there is a moment where the
/// screen is subscribed and empty, and the user sees no targets on a machine that
/// has them.
#[tauri::command]
#[specta::specta]
pub fn subscribe_send_targets(
    state: State<'_, AppState>,
    channel: Channel<TargetsDto>,
) -> IpcResult<TargetsDto> {
    state.target_pump.subscribe(channel);
    let sender = state.sender()?;
    Ok(targets(&sender))
}

/// Chooses where traffic goes.
///
/// Persisted by the identity that outlives the session, so the choice comes back
/// after a replug and on the next launch.
#[tauri::command]
#[specta::specta]
pub fn set_send_target(state: State<'_, AppState>, target_id: u32) -> IpcResult<SendViewDto> {
    let monitor = state.monitor()?;
    let mut sender = state.sender()?;
    sender.set_target(TargetId::new(target_id))?;
    state.persist(&monitor, &sender)?;
    Ok(send_view(&sender))
}

/// Loads a request into the composition, ready to send or adjust.
#[tauri::command]
#[specta::specta]
pub fn select_request(state: State<'_, AppState>, name: String) -> IpcResult<SendViewDto> {
    let mut sender = state.sender()?;
    sender.select_request(&RequestName::parse(&name)?)?;
    Ok(send_view(&sender))
}

/// Sets one value on the message being composed.
///
/// The returned preview is the re-encoded byte string, which is what makes "what
/// you see is what is sent" one value rather than two that have to agree.
#[tauri::command]
#[specta::specta]
pub fn set_composition_field(
    state: State<'_, AppState>,
    field_id: String,
    value: String,
) -> IpcResult<SendViewDto> {
    let field = FieldId::from_id(&field_id).ok_or(IpcError::UnknownField { field: field_id })?;
    let mut sender = state.sender()?;
    sender.set_composition_field(field, &value)?;
    Ok(send_view(&sender))
}

/// Transmits the current composition to the chosen target.
///
/// # Why the view is returned on failure too
///
/// A send that failed is a send that happened, and it is in the record. Returning
/// the error alone would leave the screen showing a record that does not yet
/// contain the very thing the user is being told about.
#[tauri::command]
#[specta::specta]
pub fn send(state: State<'_, AppState>) -> IpcResult<SendViewDto> {
    let mut sender = state.sender()?;
    let outcome = state.transmit(&mut sender);
    let view = send_view(&sender);
    drop(sender);
    match outcome {
        Ok(()) => Ok(view),
        Err(error) => Err(error),
    }
}

/// Replaces the composition with a different message type, at its defaults.
///
/// The returned fields are exactly the values that message carries — no more and
/// no fewer — because the field table lives in the core rather than in the screen.
#[tauri::command]
#[specta::specta]
pub fn set_composition_kind(state: State<'_, AppState>, kind_id: String) -> IpcResult<SendViewDto> {
    let kind =
        SendableKind::from_id(&kind_id).ok_or(IpcError::UnknownSendableKind { id: kind_id })?;
    let mut sender = state.sender()?;
    sender.set_composition_kind(kind);
    Ok(send_view(&sender))
}

/// Replaces the composition with hand-typed bytes.
///
/// Accepted only when the entry is exactly one valid MIDI message, judged by the
/// same decoder the event table's rows come from. A refusal leaves the previous
/// composition in force.
#[tauri::command]
#[specta::specta]
pub fn compose_raw(state: State<'_, AppState>, entry: String) -> IpcResult<SendViewDto> {
    let mut sender = state.sender()?;
    sender.compose_raw(&entry)?;
    Ok(send_view(&sender))
}

/// Sets the name the published source carries.
///
/// Republishes under the new name when the source is up. The receiving program
/// sees one device leave and another arrive — which is what the user is warned
/// about before this is called.
#[tauri::command]
#[specta::specta]
pub fn set_publication_name(state: State<'_, AppState>, name: String) -> IpcResult<SendViewDto> {
    let monitor = state.monitor()?;
    let mut sender = state.sender()?;
    sender.set_publication_name(PublishedName::parse(&name)?);
    if sender.publication().published {
        state.apply_publication(&sender)?;
    }
    sender.clear_publication_error();
    state.persist(&monitor, &sender)?;
    state.target_pump.send(targets(&sender));
    Ok(send_view(&sender))
}

/// Starts or stops publishing the source.
///
/// Stopping removes it from other programs' lists and from the target list. If it
/// was the chosen target, the choice is cleared rather than left pointing at
/// something that no longer exists.
#[tauri::command]
#[specta::specta]
pub fn set_publication_enabled(
    state: State<'_, AppState>,
    published: bool,
) -> IpcResult<SendViewDto> {
    let monitor = state.monitor()?;
    let mut sender = state.sender()?;
    sender.set_published(published)?;
    state.apply_publication(&sender)?;
    sender.clear_publication_error();
    state.persist(&monitor, &sender)?;
    drop(monitor);

    // The published source enters and leaves the target list, so the list has
    // changed even though no cable moved.
    sender.replace_targets(state.current_targets());
    state.target_pump.send(targets(&sender));
    Ok(send_view(&sender))
}

/// Re-sends the exact bytes of an earlier send.
///
/// The bytes come from the record rather than from re-encoding, so a re-send of a
/// hand-typed message sends what was typed.
#[tauri::command]
#[specta::specta]
pub fn resend(state: State<'_, AppState>, record_id: u32) -> IpcResult<SendViewDto> {
    let mut sender = state.sender()?;
    let outcome = state.resend(&mut sender, SendRecordId::new(record_id));
    let view = send_view(&sender);
    drop(sender);
    outcome.map(|()| view)
}

/// Saves the current composition under a name of the user's choosing.
#[tauri::command]
#[specta::specta]
pub fn save_request(state: State<'_, AppState>, name: String) -> IpcResult<SendViewDto> {
    let monitor = state.monitor()?;
    let mut sender = state.sender()?;
    sender.save_request(RequestName::parse(&name)?)?;
    state.persist(&monitor, &sender)?;
    Ok(send_view(&sender))
}

/// Renames one of the user's own requests.
#[tauri::command]
#[specta::specta]
pub fn rename_request(
    state: State<'_, AppState>,
    from: String,
    to: String,
) -> IpcResult<SendViewDto> {
    let from = RequestName::parse(&from)?;
    let to = RequestName::parse(&to)?;
    let monitor = state.monitor()?;
    let mut sender = state.sender()?;
    sender.library_mut().rename(&from, to)?;
    state.persist(&monitor, &sender)?;
    Ok(send_view(&sender))
}

/// Deletes one of the user's own requests.
#[tauri::command]
#[specta::specta]
pub fn delete_request(state: State<'_, AppState>, name: String) -> IpcResult<SendViewDto> {
    let name = RequestName::parse(&name)?;
    let monitor = state.monitor()?;
    let mut sender = state.sender()?;
    sender.library_mut().delete(&name)?;
    state.persist(&monitor, &sender)?;
    Ok(send_view(&sender))
}

/// The `Display` tab of the preferences surface.
///
/// Read once when the screen mounts. Mirrors [`get_filter_model`] deliberately,
/// including its reason for existing: the tab's entries and labels come from
/// Rust so the interface cannot invent an option or reword a control. Those
/// strings are the normative content of `screenshots/setting.jpg`.
#[tauri::command]
#[specta::specta]
pub fn get_display_model(state: State<'_, AppState>) -> IpcResult<DisplayViewDto> {
    let monitor = state.monitor()?;
    Ok(display_view(monitor.display()))
}

/// Replaces every display setting, and re-renders what the monitor already holds.
///
/// # Why the snapshot comes back with it
///
/// A format change must apply to events captured before it, not only to those
/// arriving after. Returning a snapshot built from `Monitor::visible_events` is
/// what does that, and it costs a return value rather than a mechanism — the
/// same shape `set_retention_limit` and `clear_events` already use.
///
/// Sending both halves in one payload leaves no interval in which the settings
/// have changed and the visible rows have not.
///
/// # Why this does not discard the pending batch
///
/// [`set_capture_state`] and [`clear_events`] call `pump.discard_pending()`
/// because they freeze or empty the list, and delivering queued rows into a list
/// the user has just frozen would be wrong. A display change does neither: those
/// events are still arriving normally and must still be shown. The webview
/// discards only batches at or below the returned snapshot's high-water mark, so
/// one in flight cannot duplicate a row the snapshot already carries.
///
/// # Why the whole settings object rather than one field per command
///
/// The tab is one form and no field has a validation error to report — every
/// value is a generated union, so [`DisplaySettingsDto`] cannot carry an invalid
/// state. Six commands would be six round trips to the same persistence write
/// with nothing to distinguish them.
#[tauri::command]
#[specta::specta]
pub fn set_display_settings(
    state: State<'_, AppState>,
    settings: DisplaySettingsDto,
) -> IpcResult<DisplayChangeDto> {
    let mut monitor = state.monitor()?;
    monitor.set_display(settings.into());
    let sender = state.sender()?;
    // Through `state.persist` rather than writing the monitor's half directly:
    // that is what folds in the sender's half, and skipping it would erase the
    // user's chosen target, published name, and every saved request.
    state.persist(&monitor, &sender)?;

    Ok(DisplayChangeDto {
        view: display_view(monitor.display()),
        snapshot: SnapshotDto::from_monitor(&monitor),
    })
}
