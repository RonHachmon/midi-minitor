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
use midi_core::domain::ids::{Arrival, HostTicks, HostTime, TickRate, Timestamp};
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
        //
        // This rule earns its keep at exactly one known boundary: selections used
        // to be stored as bare session ids and are now stored by device identity.
        // Those old ids named simulated sources that no longer exist, so there is
        // nothing worth migrating — falling back to a first run is both the
        // simplest and the most honest outcome.
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

    /// Both readings of this instant.
    ///
    /// Taken in one call so the Time column cannot disagree with itself when the
    /// user switches between `Clock time` and a `Host time` format.
    ///
    /// The host half is always [`HostTime::Stamped`] here. A source that stamps
    /// its own packets — CoreMIDI does — supplies the zero-means-now case
    /// itself; this clock is only ever asked what time it is now, and "now" is
    /// never zero.
    fn arrival(&self) -> Arrival {
        Arrival::new(self.now(), HostTime::Stamped(host_ticks()))
    }

    /// The host clock's rate, read from the platform.
    ///
    /// Falls back to [`FALLBACK_TICK_RATE`] rather than failing the launch: a
    /// platform that cannot report its timebase costs the user the three
    /// `Host time` formats, and taking the whole monitor down over a column
    /// format would be wildly out of proportion.
    fn tick_rate(&self) -> TickRate {
        TickRate::new(platform_tick_rate()).unwrap_or(TickRate::NANOSECOND)
    }
}

/// Reads the host clock.
///
/// # Why each platform uses the counter it does
///
/// macOS: `mach_absolute_time` is the clock CoreMIDI stamps its packets with, so
/// a reading taken here and a timestamp taken from a packet are in the same
/// domain and can be compared.
///
/// Windows: the performance counter, read here in the same call the arrival is
/// recorded. WinMM's own `dwParam2` is deliberately unused — it counts
/// milliseconds since `midiInStart`, which is coarser and based at a different
/// instant.
#[cfg(target_os = "macos")]
fn host_ticks() -> HostTicks {
    // SAFETY: `mach_absolute_time` takes no arguments, touches no memory this
    // caller owns, and has no failure mode. It is `unsafe` only because it is
    // FFI. Declared by `libc` rather than by hand — see this crate's manifest.
    HostTicks::new(unsafe { libc::mach_absolute_time() })
}

#[cfg(target_os = "windows")]
fn host_ticks() -> HostTicks {
    use windows::Win32::System::Performance::QueryPerformanceCounter;

    let mut counter: i64 = 0;
    // SAFETY: the pointer is to a live local that outlives the call, which is
    // the whole of this function's contract with the OS.
    let read = unsafe { QueryPerformanceCounter(&mut counter) };
    match read {
        Ok(()) => HostTicks::new(counter.unsigned_abs()),
        // A counter that will not read leaves the reading at zero rather than
        // taking down the MIDI callback thread. The row still appears; only its
        // host time is missing, and only if the user asked to see it.
        Err(_) => HostTicks::new(0),
    }
}

/// Reads the host clock's rate in ticks per second, or zero when the platform
/// will not say.
#[cfg(target_os = "macos")]
fn platform_tick_rate() -> u64 {
    let mut info = libc::mach_timebase_info { numer: 0, denom: 0 };
    // SAFETY: the pointer is to a live local that outlives the call.
    let read = unsafe { libc::mach_timebase_info(&mut info) };
    if read != 0 || info.numer == 0 {
        return 0;
    }
    // `numer/denom` converts ticks to nanoseconds, so ticks per second is that
    // ratio inverted and scaled by a billion.
    TickRate::NANOSECOND.get() * u64::from(info.denom) / u64::from(info.numer)
}

#[cfg(target_os = "windows")]
fn platform_tick_rate() -> u64 {
    use windows::Win32::System::Performance::QueryPerformanceFrequency;

    let mut frequency: i64 = 0;
    // SAFETY: the pointer is to a live local that outlives the call.
    let read = unsafe { QueryPerformanceFrequency(&mut frequency) };
    match read {
        Ok(()) => frequency.unsigned_abs(),
        Err(_) => 0,
    }
}
