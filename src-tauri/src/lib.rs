//! The Tauri application shell.
//!
//! # What lives here and what does not
//!
//! This crate owns the boundary: the command handlers, the wire types, the event
//! channel, and the settings adapter. It owns no rules. Which events are visible,
//! which sources are selected, and what a filter means are all decisions made in
//! `midi-core`, which does not depend on Tauri and therefore cannot be dragged
//! into presentation concerns.
//!
//! # Wiring, in order — and the order matters
//!
//! 1. Restore saved settings, or fall back to first-run defaults.
//! 2. **Start the MIDI adapter**, on the main thread and before any other MIDI
//!    work. The reason is macOS's and not every platform's, which is why it is
//!    stated as a platform note rather than a rule: CoreMIDI binds notification
//!    delivery to the thread and run loop current when the process's first client
//!    is created, so creating it anywhere else breaks device hot-plug *silently*
//!    — no error, notifications simply never arrive. The Windows adapter has no
//!    such constraint, and honouring the stricter of the two costs nothing.
//! 3. Build the [`Monitor`] over the catalogue the adapter reports, and over the
//!    capabilities it declares.
//! 4. Start the batching pump that feeds the webview's channel.
//! 5. Open ports for whatever the restored settings had selected.
//!
//! # What the port seam did and did not absorb
//!
//! An earlier version of this file claimed that swapping in real MIDI input would
//! be "a change to that one line". That was half right, and the half it got wrong
//! is worth recording. The **event path** did survive untouched: the wire types,
//! the filters, the log, retention, and every command below are the same code that
//! served generated traffic.
//!
//! What the seam could not absorb was an assumption underneath it — that the set
//! of sources never changes. Real devices are plugged and unplugged, so the port
//! grew a catalogue channel and the domain's catalogue became mutable. Honest
//! layering bought a great deal here; it did not buy a one-line swap.

pub mod commands;
pub mod dto;
pub mod error;
pub mod platform;
pub mod settings;
pub mod state;
pub mod stream;

use midi_core::application::monitor::Monitor;
use midi_core::application::ports::{Clock, MidiSystemStatus, SettingsRepository};
use midi_core::application::sender::Sender;
use std::sync::{Arc, Mutex};
use tauri::Manager;

use crate::dto::EventDto;
use crate::settings::{StoreSettingsRepository, SystemClock};
use crate::state::{AppState, Pumps};
use crate::stream::{CataloguePump, EventPump, TargetPump};

/// Builds the typed command surface and the TypeScript it generates.
///
/// # Why the bindings are generated rather than written
///
/// A hand-maintained TypeScript mirror of these signatures drifts the moment a
/// field changes, and with no test suite nothing would catch the drift until a
/// user hit it. Generating them makes a contract change a compile error in the
/// webview instead.
fn specta_builder() -> tauri_specta::Builder<tauri::Wry> {
    tauri_specta::Builder::<tauri::Wry>::new()
        .commands(tauri_specta::collect_commands![
            commands::get_catalogue,
            commands::subscribe_catalogue,
            commands::get_filter_model,
            commands::get_columns,
            commands::snapshot,
            commands::subscribe_events,
            commands::set_source_selected,
            commands::set_group_selected,
            commands::set_filter,
            commands::add_data_prefix_rule,
            commands::remove_data_prefix_rule,
            commands::set_capture_state,
            commands::set_retention_limit,
            commands::clear_events,
            commands::set_column_visibility,
            commands::get_display_model,
            commands::set_display_settings,
            commands::get_send_view,
            commands::subscribe_send_targets,
            commands::set_send_target,
            commands::select_request,
            commands::set_composition_kind,
            commands::set_composition_field,
            commands::compose_raw,
            commands::send,
            commands::resend,
            commands::set_publication_name,
            commands::set_publication_enabled,
            commands::save_request,
            commands::rename_request,
            commands::delete_request,
        ])
        .error_handling(tauri_specta::ErrorHandlingMode::Result)
}

/// Starts the application.
///
/// # Panics
///
/// Panics only if the application cannot be constructed at all — a missing
/// configuration or an unusable webview. There is nothing to fall back to at
/// that point, and this is the one place the project's no-panic rule yields:
/// failing loudly at startup beats a window that never appears.
#[allow(clippy::expect_used)]
pub fn run() {
    let builder = specta_builder();

    #[cfg(debug_assertions)]
    builder
        .export(
            specta_typescript::Typescript::default(),
            "../src/bindings.ts",
        )
        .expect("failed to export TypeScript bindings");

    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::default().build())
        .invoke_handler(builder.invoke_handler())
        .setup(move |app| {
            let handle = app.handle().clone();

            let repository: Arc<dyn SettingsRepository> =
                Arc::new(StoreSettingsRepository::new(handle.clone()));
            let saved = repository.load().ok().flatten();

            let clock: Arc<dyn Clock> = Arc::new(SystemClock);

            // The one call that reaches the platform adapter — and the only file
            // on the far side of it that knows which platform this is.
            let mut source = platform::midi_access(Arc::clone(&clock));

            let event_handle = handle.clone();
            let catalogue_handle = handle.clone();

            // MUST come before any other MIDI call — see the module docs. On
            // macOS this is where the first CoreMIDI client is created, and the
            // system decides here and only here which run loop will carry
            // hot-plug notifications. On Windows nothing depends on the calling
            // thread, so honouring the constraint costs that platform nothing.
            let status = match source.start(
                Box::new(move |event| {
                    // Runs on a MIDI callback thread. Ingest decides whether the
                    // event is retained and whether it is currently visible; only
                    // visible events are queued, so suppressed traffic never
                    // crosses the IPC boundary at all.
                    let state = event_handle.state::<AppState>();
                    let Ok(mut monitor) = state.monitor() else {
                        return;
                    };
                    if let Some(visible) = monitor.ingest(event) {
                        let dto = EventDto::from_event(
                            &visible,
                            monitor.catalogue(),
                            monitor.display(),
                            monitor.tick_rate(),
                        );
                        // The lock is released before queuing so the pump's flush
                        // thread never waits on a MIDI callback.
                        drop(monitor);
                        state.pump.push(dto);
                    }
                }),
                Box::new(move |sources| {
                    // A device was attached or removed. Replace the catalogue,
                    // make the open ports match, and tell the webview. Retained
                    // events are deliberately left alone: what was observed does
                    // not stop being true because a cable moved.
                    let state = catalogue_handle.state::<AppState>();
                    let Ok(mut monitor) = state.monitor() else {
                        return;
                    };
                    monitor.replace_catalogue(sources);
                    state.sync_ports(&mut monitor);
                    state.publish_catalogue(&monitor);
                    drop(monitor);
                    // One cable moved changes both lists. Re-reading the targets
                    // here is what keeps the send screen honest without a second
                    // subscription to the same notification.
                    state.refresh_targets();
                }),
            ) {
                Ok(()) => MidiSystemStatus::Available,
                // Not fatal. The window opens, says it cannot reach the MIDI
                // system, and every other control keeps working — which is a far
                // more useful failure than refusing to launch.
                Err(error) => MidiSystemStatus::Unavailable {
                    detail: error.to_string(),
                },
            };

            let catalogue = source.catalogue();
            // Read once, here: capabilities describe the machine this is running
            // on and cannot change while it runs.
            let capabilities = source.capabilities();
            // The send side is built from the same three ingredients as the
            // monitor: what the machine reports now, what was saved last time, and
            // what this platform can do. Reading the targets here rather than
            // later means the send screen is correct the first time it is opened,
            // with no separate warm-up path to get wrong.
            //
            // Built before the monitor only because the monitor takes ownership of
            // the saved settings and this needs to borrow them first.
            let sender = Sender::new(source.targets(), saved.as_ref(), capabilities.clone());
            // The host-clock rate is read once, here, and handed to the monitor
            // for the life of the process. It is fixed on both platforms, so
            // reading it per event or per render would be waste, and a renderer
            // that asked the platform for it would be a domain rule doing I/O.
            let tick_rate = clock.tick_rate();
            let monitor = Monitor::new(catalogue, saved, status, capabilities, tick_rate);

            let pump = Arc::new(EventPump::new());
            pump.start();
            let catalogue_pump = Arc::new(CataloguePump::new());
            let target_pump = Arc::new(TargetPump::new());
            let source = Arc::new(Mutex::new(source));

            // Registered as `AppState` rather than `Arc<AppState>`: Tauri resolves
            // managed state by exact type, so wrapping it here would leave every
            // `State<AppState>` parameter unresolvable at runtime.
            app.manage(AppState::new(
                monitor,
                sender,
                Pumps {
                    events: Arc::clone(&pump),
                    catalogue: Arc::clone(&catalogue_pump),
                    targets: Arc::clone(&target_pump),
                },
                Arc::clone(&source),
                repository,
                Arc::clone(&clock),
            ));

            // Open ports for whatever the restored settings had selected. Done
            // after `manage` because it needs the state the callbacks resolve.
            let state = handle.state::<AppState>();
            if let Ok(mut monitor) = state.monitor() {
                state.sync_ports(&mut monitor);
            }

            // Republish the source if the user left the disguise on. A saved
            // `published: true` is a *request*, not a guarantee — the platform has
            // the final say, and a refusal must not stop the window opening. So
            // the failure is recorded against the sender's own state and the
            // screen reports it, rather than being raised here where there is
            // nobody to tell.
            if let Ok(mut sender) = state.sender() {
                if sender.publication().published {
                    if let Err(error) = state.apply_publication(&sender) {
                        sender.report_publication_failure(&error.to_string());
                    }
                }
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
