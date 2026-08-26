//! Shared application state.
//!
//! # Why the monitor sits behind a mutex here rather than owning a thread
//!
//! Three parties touch it: the simulator's generation thread ingesting events,
//! the command handlers responding to the user, and the pump reading what to
//! send. A mutex is the smallest thing that serialises them. An actor with a
//! message queue would be the ceremony the project's principles reject — there
//! is one shared structure and no ordering requirement beyond mutual exclusion.

use crate::error::{IpcError, IpcResult};
use crate::stream::EventPump;
use midi_core::application::monitor::Monitor;
use midi_core::application::ports::SettingsRepository;
use std::sync::{Arc, Mutex, MutexGuard};

/// Everything the command handlers need.
pub struct AppState {
    monitor: Mutex<Monitor>,
    /// The batching pump that feeds the webview's channel.
    pub pump: Arc<EventPump>,
    settings: Arc<dyn SettingsRepository>,
}

impl AppState {
    /// Assembles the shared state.
    #[must_use]
    pub fn new(
        monitor: Monitor,
        pump: Arc<EventPump>,
        settings: Arc<dyn SettingsRepository>,
    ) -> Self {
        Self {
            monitor: Mutex::new(monitor),
            pump,
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
}
