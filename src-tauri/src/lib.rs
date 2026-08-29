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
use std::sync::{Arc, Mutex};
use tauri::Manager;

use crate::dto::EventDto;
use crate::settings::{StoreSettingsRepository, SystemClock};
use crate::state::AppState;
use crate::stream::{CataloguePump, EventPump};

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
            let mut source = platform::event_source(Arc::clone(&clock));

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
                        let dto = EventDto::from_event(&visible, monitor.catalogue());
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
            let monitor = Monitor::new(catalogue, saved, status, capabilities);

            let pump = Arc::new(EventPump::new());
            pump.start();
            let catalogue_pump = Arc::new(CataloguePump::new());
            let source = Arc::new(Mutex::new(source));

            // Registered as `AppState` rather than `Arc<AppState>`: Tauri resolves
            // managed state by exact type, so wrapping it here would leave every
            // `State<AppState>` parameter unresolvable at runtime.
            app.manage(AppState::new(
                monitor,
                Arc::clone(&pump),
                Arc::clone(&catalogue_pump),
                Arc::clone(&source),
                repository,
            ));

            // Open ports for whatever the restored settings had selected. Done
            // after `manage` because it needs the state the callbacks resolve.
            let state = handle.state::<AppState>();
            if let Ok(mut monitor) = state.monitor() {
                state.sync_ports(&mut monitor);
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
