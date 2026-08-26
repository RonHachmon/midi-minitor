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
//! # Wiring, in order
//!
//! 1. Restore saved settings, or fall back to first-run defaults.
//! 2. Build the [`Monitor`] over the source catalogue the event source reports.
//! 3. Start the batching pump that feeds the webview's channel.
//! 4. Start the event source, whose sink ingests into the monitor and queues
//!    whatever survives the filter.
//!
//! Step 4 is the only place that names the simulator. Swapping in real MIDI input
//! is a change to that one line.

pub mod commands;
pub mod dto;
pub mod error;
pub mod settings;
pub mod state;
pub mod stream;

use midi_core::application::monitor::Monitor;
use midi_core::application::ports::{Clock, EventSource, SettingsRepository};
use midi_core::simulator::SimulatedSource;
use std::sync::Arc;
use tauri::Manager;

use crate::dto::EventDto;
use crate::settings::{StoreSettingsRepository, SystemClock};
use crate::state::AppState;
use crate::stream::EventPump;

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
            commands::get_filter_model,
            commands::get_columns,
            commands::snapshot,
            commands::subscribe_events,
            commands::set_source_selected,
            commands::set_group_selected,
            commands::set_filter,
            commands::set_data_prefix_filter,
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

            // The one line that names the simulator. A real MIDI adapter would
            // be substituted here and nothing below would change.
            let mut source = SimulatedSource::new(Arc::clone(&clock));
            let catalogue = source.catalogue();

            let monitor = Monitor::new(catalogue, saved);
            let pump = Arc::new(EventPump::new());
            pump.start();

            // Registered before the source starts, and registered as `AppState`
            // rather than `Arc<AppState>`: Tauri resolves managed state by exact
            // type, so wrapping it here would leave every `State<AppState>`
            // parameter unresolvable at runtime.
            app.manage(AppState::new(monitor, Arc::clone(&pump), repository));

            let sink_handle = handle.clone();
            source.start(Box::new(move |event| {
                // Runs on the generator's thread. Ingest decides whether the
                // event is retained and whether it is currently visible; only
                // visible events are queued, so suppressed traffic never
                // crosses the IPC boundary at all.
                let state = sink_handle.state::<AppState>();
                let Ok(mut monitor) = state.monitor() else {
                    return;
                };
                if let Some(visible) = monitor.ingest(event) {
                    let dto = EventDto::from_event(&visible, monitor.catalogue());
                    // The lock is released before queuing so the pump's flush
                    // thread never waits on the generator.
                    drop(monitor);
                    state.pump.push(dto);
                }
            }))?;

            // The source is kept alive for the process lifetime; dropping it
            // would stop generation on the next statement.
            app.manage(SourceHandle(source));

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Keeps the event source alive and stops it when the application exits.
///
/// Tauri's managed state is the only thing with the right lifetime here; without
/// it the source would be dropped at the end of `setup` and generation would
/// stop before the window appeared.
struct SourceHandle(SimulatedSource);

impl Drop for SourceHandle {
    fn drop(&mut self) {
        self.0.stop();
    }
}
