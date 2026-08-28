//! The interfaces this crate needs from the outside world.
//!
//! **Ports & Adapters.** The problem each port below solves is named with it,
//! because an interface introduced without a stated pressure is the ceremony the
//! project's principles reject. Each of these exists because something concrete
//! is expected to be substituted behind it.

use crate::application::error::CoreError;
use crate::domain::event::MidiEvent;
use crate::domain::ids::{SourceId, Timestamp};
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

    /// Opens and closes ports so the listening set matches the selections in
    /// `sources`. Returns the sources that could **not** be opened, each paired
    /// with the reason.
    ///
    /// Idempotent: calling it twice with the same `sources` changes nothing and
    /// returns the same failures.
    ///
    /// # Why failures are returned rather than raised
    ///
    /// A device another program is holding is a fact the user needs to see in
    /// the Sources panel, on the row it concerns. Raising it would imply the
    /// whole operation failed when every other device connected successfully.
    ///
    /// # Why this belongs in the port at all
    ///
    /// It was an inherent method on the one adapter that existed, and the shell
    /// called it through that concrete type — so the shell already depended on
    /// it and the port simply failed to say so. A second implementation cannot
    /// be substituted while a dependency is invisible to the compiler. The
    /// alternative, a `cfg`-selected type alias, would leave the contract
    /// *structural*: two methods that merely happen to share a name, checked
    /// only on the platform being compiled. With no test suite, an unchecked
    /// cross-platform contract is an untested one.
    fn sync_ports(&mut self, sources: &[Source]) -> Vec<(SourceId, String)>;

    /// What this platform's MIDI access can and cannot do.
    ///
    /// # The problem this solves
    ///
    /// Each platform must state its own limitations on the control they affect,
    /// and neither the shell nor the webview may decide that for itself — a
    /// platform conditional above this line is exactly what the layering
    /// forbids. The adapter is the only component that knows which platform it
    /// is. This is the method through which it says so.
    ///
    /// Read once at construction and held by the monitor. Never persisted: it
    /// describes the machine the application is running on right now.
    fn capabilities(&self) -> PlatformCapabilities;

    /// How many events were lost because the adapter could not keep pace or
    /// could not reach its own shared state.
    ///
    /// Surfaced rather than kept internal: a monitor that silently loses traffic
    /// is worse than one that admits it fell behind. Every adapter genuinely has
    /// this number, so moving it into the port preserved an existing capability
    /// rather than inventing a speculative one.
    fn dropped_events(&self) -> u32;
}

/// What the platform's MIDI access can and cannot do.
///
/// # Why this exists rather than a platform check where it is needed
///
/// A limitation must be stated on the control it affects, and the only component
/// that knows the platform is the adapter. Carrying the answer in a value lets
/// every layer above render what it is told, with no conditional of its own.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlatformCapabilities {
    /// How faithfully this platform reports the bytes that arrived.
    pub byte_fidelity: ByteFidelity,
}

/// How faithfully a platform reports the bytes that crossed the cable.
///
/// # Why an enum rather than an optional message
///
/// As `Option<String>`, faithfulness would be the *absence* of a message —
/// indistinguishable from a platform that simply forgot to describe itself.
/// "This platform is faithful" is a fact worth naming. Matched exhaustively with
/// no catch-all arm, so a third level of fidelity becomes a compile error at
/// every site that renders one rather than a silent default.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ByteFidelity {
    /// The bytes delivered are the bytes the cable carried.
    AsTransmitted,
    /// The platform assembles messages before any application can see them.
    ///
    /// Nothing is invented and nothing is dropped — but the byte *count* shown
    /// for a message the platform assembled is not necessarily the count that
    /// travelled. The adapter explains the specifics in its own words, following
    /// the pattern [`MidiSystemStatus::Unavailable`] and
    /// [`crate::domain::source::Availability::Unopenable`] already set: the
    /// platform speaks for itself and the domain holds no table of per-platform
    /// prose.
    Assembled {
        /// What the platform assembles, and what it still reports exactly.
        detail: String,
    },
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
