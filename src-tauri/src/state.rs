//! Shared application state.
//!
//! # Why the monitor sits behind a mutex here rather than owning a thread
//!
//! Several parties touch it: the MIDI callback threads ingesting events, the
//! command handlers responding to the user, the pump reading what to send, and
//! the hot-plug path replacing the catalogue. A mutex is the smallest thing that
//! serialises them. An actor with a message queue would be the ceremony the
//! project's principles reject — there is one shared structure and no ordering
//! requirement beyond mutual exclusion.

use crate::error::{IpcError, IpcResult};
use crate::stream::{CataloguePump, EventPump};
use midi_core::application::monitor::Monitor;
use midi_core::application::ports::{EventSource, SettingsRepository};
use midi_core::domain::source::Source;
use std::sync::{Arc, Mutex, MutexGuard};

/// Everything the command handlers need.
pub struct AppState {
    monitor: Mutex<Monitor>,
    /// The batching pump that feeds the webview's event channel.
    pub pump: Arc<EventPump>,
    /// The pump that pushes catalogue changes as devices come and go.
    pub catalogue_pump: Arc<CataloguePump>,
    /// The MIDI adapter, so selections can open and close real ports.
    ///
    /// Held here rather than as separate managed state because opening a port is
    /// always a consequence of a selection change, and the two must not be able
    /// to drift out of step.
    ///
    /// # Why the concrete adapter is not named
    ///
    /// This file once said `CoreMidiSource`, and called an *inherent*
    /// `sync_ports` on it — so the shell was coupled to one platform by a method
    /// the port did not describe, and the module docs' claim that the platform
    /// appeared on a single line was not true. Holding the trait object makes the
    /// claim true and makes the compiler check the contract both adapters must
    /// meet. Which one this is remains
    /// [`platform`](crate::platform)'s business alone.
    ///
    /// # Why the box, rather than `Arc<Mutex<dyn EventSource>>`
    ///
    /// [`crate::platform::event_source`] hands back a `Box` so its two arms can
    /// share one return type, and the composition root drives the adapter
    /// directly — starting it, reading its catalogue and its capabilities —
    /// before anything else can hold it. A `Mutex<Box<dyn _>>` cannot be coerced
    /// to a `Mutex<dyn _>` afterwards, and the alternatives all cost more than
    /// the one pointer hop this keeps: locking the adapter for the whole startup
    /// sequence would invent an unreachable poisoned-lock branch, and a blanket
    /// `EventSource for Box<dyn EventSource>` would add an impl to the core crate
    /// to save an indirection nobody can measure.
    source: Arc<Mutex<Box<dyn EventSource>>>,
    settings: Arc<dyn SettingsRepository>,
}

impl AppState {
    /// Assembles the shared state.
    #[must_use]
    pub fn new(
        monitor: Monitor,
        pump: Arc<EventPump>,
        catalogue_pump: Arc<CataloguePump>,
        source: Arc<Mutex<Box<dyn EventSource>>>,
        settings: Arc<dyn SettingsRepository>,
    ) -> Self {
        Self {
            monitor: Mutex::new(monitor),
            pump,
            catalogue_pump,
            source,
            settings,
        }
    }

    /// Locks the monitor.
    ///
    /// # Errors
    ///
    /// Returns [`IpcError::MonitorUnavailable`] when the lock was poisoned by a
    /// panic on another thread. Surfaced rather than swallowed: a window that
    /// has quietly stopped updating is worse than one that says why.
    pub fn monitor(&self) -> IpcResult<MutexGuard<'_, Monitor>> {
        self.monitor
            .lock()
            .map_err(|_| IpcError::MonitorUnavailable)
    }

    /// Writes the current settings to storage.
    ///
    /// # Errors
    ///
    /// Returns [`IpcError::SettingsUnavailable`] when the store cannot be
    /// written. Callers should surface it without aborting the operation that
    /// triggered it — the change is already applied in memory, and failing to
    /// remember a preference is not a reason to reject it.
    pub fn persist(&self, monitor: &Monitor) -> IpcResult<()> {
        self.settings
            .save(&monitor.persisted_settings())
            .map_err(|error| IpcError::SettingsUnavailable {
                detail: error.to_string(),
            })
    }

    /// Makes the open ports match the current selection.
    ///
    /// # Why a port that will not open is recorded rather than returned
    ///
    /// A device held by another application is a fact the user needs to see in
    /// the Sources panel, next to the row it concerns. Returning it as a command
    /// error would surface it as a rejected call with nowhere sensible to render
    /// it — and would imply the whole operation failed, when every other device
    /// was connected successfully.
    pub fn sync_ports(&self, monitor: &mut Monitor) {
        let sources: Vec<Source> = monitor.catalogue().sources().to_vec();
        let Ok(mut source) = self.source.lock() else {
            return;
        };
        let failures = source.sync_ports(&sources);
        drop(source);

        for (id, detail) in failures {
            monitor.catalogue_mut().set_unopenable(id, detail);
        }
    }

    /// Pushes the current catalogue to the webview.
    pub fn publish_catalogue(&self, monitor: &Monitor) {
        self.catalogue_pump.send(crate::dto::catalogue(monitor));
    }
}
