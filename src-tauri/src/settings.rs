//! Persistence, and the system clock.
//!
//! # Why the store plugin rather than writing a file
//!
//! **Repository pattern.** The problem it solves here is keeping
//! `tauri-plugin-store` — and `serde_json::Value`, and the OS config directory —
//! out of `midi-core`, which must stay free of Tauri. The core asks for its
//! settings to be saved; it never learns where they go.
//!
//! The plugin is used rather than hand-rolled file I/O because it already solves
//! path resolution to the platform's config directory, atomic writes, and
//! debounced auto-save. Rewriting those is the kind of hand-rolling the
//! project's principles exclude.

use midi_core::application::error::CoreError;
use midi_core::application::ports::{Clock, SettingsRepository};
use midi_core::application::settings::PersistedSettings;
use midi_core::constants::MILLIS_PER_DAY;
use midi_core::domain::ids::Timestamp;
use tauri::{AppHandle, Wry};
use tauri_plugin_store::StoreExt;

/// The store file, resolved by the plugin into the OS config directory.
const STORE_FILE: &str = "settings.json";

/// The single key inside it.
///
/// One key holding the whole settings document, rather than a key per setting:
/// the settings are written and read as a unit, and splitting them would invite
/// a half-restored state after a partial write.
const SETTINGS_KEY: &str = "settings";

/// Settings storage backed by the Tauri store plugin.
pub struct StoreSettingsRepository {
    app: AppHandle<Wry>,
}

impl StoreSettingsRepository {
    /// Creates a repository bound to the running application.
    #[must_use]
    pub const fn new(app: AppHandle<Wry>) -> Self {
        Self { app }
    }
}

impl SettingsRepository for StoreSettingsRepository {
    fn save(&self, settings: &PersistedSettings) -> Result<(), CoreError> {
        let store = self
            .app
            .store(STORE_FILE)
            .map_err(|error| storage_failure(&error))?;
        let value = serde_json::to_value(settings).map_err(|error| storage_failure(&error))?;
        store.set(SETTINGS_KEY, value);
        store.save().map_err(|error| storage_failure(&error))?;
        Ok(())
    }

    fn load(&self) -> Result<Option<PersistedSettings>, CoreError> {
        let store = self
            .app
            .store(STORE_FILE)
            .map_err(|error| storage_failure(&error))?;
        let Some(value) = store.get(SETTINGS_KEY) else {
            return Ok(None);
        };

        // A settings document this build cannot read is treated as absent rather
        // than fatal: an older or newer file on disk must never stop the
        // application from starting, and defaults are always a valid state.
        Ok(serde_json::from_value(value).ok())
    }
}

/// Wraps a storage failure as the core's error type.
///
/// The core has no vocabulary for store plugins, so the detail is flattened to
/// a message. It reaches the user as `settingsUnavailable`, which is actionable
/// on its own — the change applied, it simply will not be remembered.
fn storage_failure(error: &dyn std::fmt::Display) -> CoreError {
    CoreError::SettingsStorage {
        detail: error.to_string(),
    }
}

/// The system clock, reporting local wall-clock time.
pub struct SystemClock;

impl Clock for SystemClock {
    /// Milliseconds since local midnight.
    ///
    /// Local rather than UTC because the Time column is a wall clock the user
    /// compares against their own: an offset display would make every timestamp
    /// quietly wrong.
    fn now(&self) -> Timestamp {
        let now = chrono::Local::now();
        let midnight = now
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .map_or(0, |naive| naive.and_utc().timestamp_millis());
        let elapsed = now.naive_local().and_utc().timestamp_millis() - midnight;
        Timestamp::from_millis_since_midnight(u64::try_from(elapsed).unwrap_or(0) % MILLIS_PER_DAY)
    }
}
