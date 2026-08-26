//! The interfaces this crate needs from the outside world.
//!
//! **Ports & Adapters.** The problem each port below solves is named with it,
//! because an interface introduced without a stated pressure is the ceremony the
//! project's principles reject. Each of these exists because something concrete
//! is expected to be substituted behind it.

use crate::application::error::CoreError;
use crate::domain::event::MidiEvent;
use crate::domain::ids::Timestamp;
use crate::domain::source::Source;

/// Somewhere MIDI events come from.
///
/// # The problem this solves
///
/// This application ships with a simulator, and real MIDI input is expected to
/// replace it. Without this port that swap would reach into the domain and the
/// use cases; with it, the swap is a different implementation of one trait and
/// nothing above the adapter layer changes or even notices.
///
/// Note what the trait does *not* expose: no flag saying whether the data is
/// simulated, no "fake" in any name. Nothing above the implementation is able to
/// tell the difference, which is the whole point.
pub trait EventSource: Send {
    /// Begins producing events, delivering each to `sink`.
    ///
    /// The sink is called from the source's own thread, so implementations of it
    /// must be cheap and must not block — the monitor's ingest path is the
    /// intended consumer.
    ///
    /// # Errors
    ///
    /// Returns a [`CoreError`] when the source cannot start. The simulator
    /// cannot fail here, but a real MIDI adapter can — the port has to admit
    /// that possibility now, or every caller would need changing later.
    fn start(&mut self, sink: EventSink) -> Result<(), CoreError>;

    /// Stops producing events. Idempotent.
    fn stop(&mut self);

    /// The sources this implementation can produce events for.
    ///
    /// Fixed for the lifetime of the process: device hot-plug is out of scope,
    /// so callers may cache the result.
    fn catalogue(&self) -> Vec<Source>;
}

/// Where an [`EventSource`] delivers what it observes.
///
/// A boxed closure rather than a channel so the source stays unaware of how
/// delivery is coordinated — the monitor may lock, batch, or drop, and none of
/// that is the source's business.
pub type EventSink = Box<dyn Fn(MidiEvent) + Send + Sync>;

/// Where settings survive between sessions.
///
/// # The problem this solves
///
/// Persistence lives in the Tauri layer, because that is where the store plugin
/// and the OS config directory are. This crate must not depend on either. The
/// port lets the core ask for its settings to be saved without knowing that a
/// plugin, a JSON file, or an OS path is involved.
///
/// **Repository** is the pattern; the substitution it protects is the storage
/// mechanism, not the data.
pub trait SettingsRepository: Send + Sync {
    /// Persists the current settings.
    ///
    /// # Errors
    ///
    /// Returns a [`CoreError`] when the settings cannot be written. Callers
    /// should surface the failure but keep running: an unsaved preference is an
    /// annoyance, not a reason to stop monitoring.
    fn save(&self, settings: &super::settings::PersistedSettings) -> Result<(), CoreError>;

    /// Loads previously persisted settings, or [`None`] on a first run.
    ///
    /// # Errors
    ///
    /// Returns a [`CoreError`] only when the store exists but cannot be read.
    /// A missing store is [`None`] rather than an error, because a first launch
    /// is normal.
    fn load(&self) -> Result<Option<super::settings::PersistedSettings>, CoreError>;
}

/// Where timestamps come from.
///
/// # The problem this solves
///
/// The domain formats times to the millisecond and must not call the system
/// clock itself — a domain that reaches for wall-clock time cannot be driven
/// deterministically from anywhere else. This port keeps that dependency at the
/// edge where it belongs.
pub trait Clock: Send + Sync {
    /// The current time, as milliseconds since local midnight.
    fn now(&self) -> Timestamp;
}
