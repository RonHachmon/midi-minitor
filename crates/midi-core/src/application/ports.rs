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
/// Device access is platform-specific and the domain must stay free of it. This
/// port is the seam: the macOS CoreMIDI adapter is one implementation, and
/// nothing above it names a platform or knows which one is in use.
///
/// Note what the trait does *not* expose: no flag describing where the data came
/// from, and no way for a caller to distinguish one implementation from another.
/// That is deliberate and load-bearing — it is what kept the event path, the
/// filters, and the whole webview unchanged when generated traffic was replaced
/// by real hardware.
pub trait EventSource: Send {
    /// Begins producing events, delivering each to `events`.
    ///
    /// `catalogue` is called whenever the set of available sources changes.
    ///
    /// Both sinks are called from the source's own thread, so implementations of
    /// them must be cheap and must not block — the monitor's ingest path is the
    /// intended consumer.
    ///
    /// # Errors
    ///
    /// Returns a [`CoreError`] when the source cannot start at all — typically
    /// [`CoreError::MidiSystemUnavailable`] when the platform's MIDI service
    /// cannot be reached. A *single* port failing to open is not an error here:
    /// it is reported on that source's
    /// [`crate::domain::source::Availability`], because the application must keep
    /// monitoring everything else.
    fn start(&mut self, events: EventSink, catalogue: CatalogueSink) -> Result<(), CoreError>;

    /// Stops producing events. Idempotent.
    fn stop(&mut self);

    /// The sources available right now.
    ///
    /// # Why this is only a snapshot
    ///
    /// It once promised a fixed list callers could cache. It cannot any more:
    /// devices are attached and removed while the application runs, so this is
    /// the set at the moment of asking and nothing more. Every later change
    /// arrives through the [`CatalogueSink`] given to [`Self::start`].
    fn catalogue(&self) -> Vec<Source>;
}

/// Where an [`EventSource`] delivers what it observes.
///
/// A boxed closure rather than a channel so the source stays unaware of how
/// delivery is coordinated — the monitor may lock, batch, or drop, and none of
/// that is the source's business.
pub type EventSink = Box<dyn Fn(MidiEvent) + Send + Sync>;

/// Where an [`EventSource`] reports that the set of sources has changed.
///
/// # The problem this solves
///
/// **Observer.** The pressure is concrete and belongs to the operating system,
/// not to this application: only the OS knows when someone plugs in a cable, so
/// there is no moment at which asking would be correct. The application has to be
/// told.
///
/// The alternative — polling [`EventSource::catalogue`] on a timer from the layer
/// above — was rejected twice over: it would put a refresh policy in the Tauri
/// layer, which is required to hold no logic, and it would trade a correct push
/// for a latency-versus-CPU tradeoff that a user watching the Sources list would
/// see.
pub type CatalogueSink = Box<dyn Fn(Vec<Source>) + Send + Sync>;

/// Whether the platform's MIDI system could be reached at all.
///
/// # Why this is not just an empty catalogue
///
/// "Nothing is attached" and "I cannot see what is attached" look identical in a
/// list of zero devices, and they call for completely different responses — plug
/// something in, versus grant permission or investigate the system. Conflating
/// them would leave a user staring at an empty window with no idea which
/// situation they are in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MidiSystemStatus {
    /// The MIDI system was reached. The catalogue is trustworthy, empty or not.
    Available,
    /// The MIDI system could not be reached, and this is why.
    Unavailable {
        /// What the platform reported, for the interface to show.
        detail: String,
    },
}

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
