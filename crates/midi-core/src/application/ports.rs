//! The interfaces this crate needs from the outside world.
//!
//! **Ports & Adapters.** The problem each port below solves is named with it,
//! because an interface introduced without a stated pressure is the ceremony the
//! project's principles reject. Each of these exists because something concrete
//! is expected to be substituted behind it.

use crate::application::error::CoreError;
use crate::domain::event::MidiEvent;
use crate::domain::ids::{Arrival, PublishedName, SourceId, TargetId, TickRate, Timestamp};
use crate::domain::source::Source;
use crate::domain::target::Target;

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
    /// Whether this platform can publish a MIDI source other programs receive from.
    pub publication: PublicationSupport,
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
/// # Why `arrival` exists beside `now`
///
/// The Time column can show either a wall clock or the host clock, and the two
/// readings must describe the same instant — taking them through two calls would
/// let a scheduler interleave between them, so the column would disagree with
/// itself the moment the user switched format. [`Clock::arrival`] takes both at
/// once and is what every *received* event is stamped with.
///
/// [`Clock::now`] survives for the three sites that stamp **send records**,
/// which have only a wall-clock column. Making them build an [`Arrival`] and
/// discard half of it would be ceremony.
///
/// # Why the rate is reported here rather than converted here
///
/// This port reports readings; [`crate::domain::rendering`] converts and formats
/// them. A clock that formatted would put a display rule in the application
/// layer, and a display rule is a domain fact.
pub trait Clock: Send + Sync {
    /// The current time, as milliseconds since local midnight.
    fn now(&self) -> Timestamp;

    /// Both readings of the current moment, taken together.
    fn arrival(&self) -> Arrival;

    /// How many host-clock ticks pass in a second.
    ///
    /// Fixed for the life of the process on every platform this runs on, so
    /// callers read it once and hold it rather than asking per event.
    fn tick_rate(&self) -> TickRate;
}

/// Somewhere MIDI messages go.
///
/// # The problem this solves
///
/// The counterpart of [`EventSource`], and a separate port rather than a wider
/// one. `EventSource` is about *arrival*: sinks, a catalogue push, and a
/// deliberate inability to tell implementations apart. Transmission has a
/// different shape entirely — caller-driven, synchronous, and answering per call
/// — so folding the two together would give one trait two reasons to change.
///
/// It clears the bar an abstraction has to clear here: two real implementations
/// on the day it is written, macOS and Windows, reaching the machine through
/// entirely different platform calls. That is the same pressure that justified
/// [`EventSource`], so it gets the same answer.
///
/// # What this port deliberately does not expose
///
/// - **No scheduling.** No timestamps, no send-at, no queue. The user presses
///   send and the message goes; a scheduling parameter nobody has asked for is
///   the speculative abstraction the project's principles reject.
/// - **No fan-out.** One target per call.
/// - **No delivery confirmation.** `Ok` means the platform accepted the bytes and
///   nothing more. Whether a receiving program acted on them is unknowable from
///   here, and the interface must never claim otherwise.
pub trait Transmitter: Send {
    /// The places this machine can currently be sent to.
    ///
    /// # Why this is only a snapshot
    ///
    /// The same reason [`EventSource::catalogue`] is: devices are attached and
    /// removed while the application runs. The moment to re-read it is the
    /// device-change notification delivered to the [`CatalogueSink`] given to
    /// [`EventSource::start`] — one physical event changes both lists, and a
    /// second subscription to the same notification would only make each adapter
    /// guess which list a given change affected.
    ///
    /// Implementations MUST list every destination the operating system reports,
    /// under the name it reports, and MUST list the published source if and only
    /// if it is currently published.
    fn targets(&self) -> Vec<Target>;

    /// Sends `bytes` to `target`, exactly as given.
    ///
    /// Implementations MUST transmit the bytes unaltered — same values, same
    /// order, nothing inserted, nothing assembled, nothing normalised. No status
    /// byte may be re-derived and no running status applied. This promise is what
    /// lets the interface show a user what is about to be sent.
    ///
    /// A message MUST be transmitted whole or not at all; an implementation that
    /// cannot guarantee that for a given size MUST fail rather than send a
    /// prefix.
    ///
    /// Reaching a destination and distributing from the published source are
    /// different platform mechanisms. Choosing between them is the adapter's job,
    /// because the user chose a target, not a mechanism.
    ///
    /// # Errors
    ///
    /// - [`CoreError::UnknownTarget`] when the id is not in the current snapshot,
    ///   typically because the device has been unplugged.
    /// - [`CoreError::TransmitFailed`] when the platform refused. The adapter
    ///   MUST remain usable afterwards: a later send, to this target or another,
    ///   has to work without anything being restarted.
    fn transmit(&mut self, target: TargetId, bytes: &[u8]) -> Result<(), CoreError>;

    /// Publishes a MIDI source under `name`, which other programs list among
    /// their MIDI inputs and can receive from.
    ///
    /// Idempotent: publishing while already published under the same name changes
    /// nothing. A change of name is unpublish-then-publish — the receiving
    /// program sees one device leave and another arrive, which is exactly what
    /// the user is warned about before it happens.
    ///
    /// Publishing MUST NOT disturb reception. Input ports stay open and no event
    /// is lost; on a platform where both directions share one client, that client
    /// MUST NOT be restarted to satisfy this call.
    ///
    /// # Errors
    ///
    /// - [`CoreError::PublicationUnsupported`] where the platform cannot publish
    ///   at all. Callers are expected never to reach it, because the control is
    ///   not operable there; it is a backstop against a stale view.
    /// - [`CoreError::PublicationFailed`] where the platform can publish but this
    ///   attempt did not succeed.
    fn publish(&mut self, name: &PublishedName) -> Result<(), CoreError>;

    /// Withdraws the published source.
    ///
    /// Idempotent, and infallible by design: there is no way for withdrawing an
    /// endpoint to fail that a caller could act on, so returning a `Result` would
    /// only invite a caller to invent a response to it.
    fn unpublish(&mut self);
}

/// The machine's MIDI access, in both directions.
///
/// # Why the two ports are bundled into one object
///
/// This is forced by the platform rather than chosen for tidiness. On macOS a
/// single client is the parent of every port and every virtual endpoint the
/// application owns, and this application's client is created at one exact
/// moment for one exact reason — inside [`EventSource::start`], on the main
/// thread, because the operating system binds hot-plug notification delivery to
/// whichever run loop is current at that instant and fails **silently** when it
/// is wrong.
///
/// A separate transmitting object would therefore need either a second client —
/// an avoidable risk against a constraint this codebase already documents as
/// fragile — or the first one handed to it through the application shell, which
/// would put a platform type in the layer that is forbidden to name a platform.
///
/// Windows has no such constraint; its input and output handles are independent.
/// Honouring the stricter of the two costs that platform nothing, which is the
/// same argument the start-order note already makes.
pub trait MidiAccess: EventSource + Transmitter {}

impl<T: EventSource + Transmitter> MidiAccess for T {}

/// Whether this platform can publish a MIDI source at all.
///
/// # Why an enum rather than an optional message
///
/// The same argument [`ByteFidelity`] makes: as an `Option<String>`, support
/// would be the *absence* of a complaint, indistinguishable from a platform that
/// simply forgot to describe itself. "This platform can publish" is a fact worth
/// naming. Matched exhaustively with no catch-all arm, so a third level of
/// support becomes a compile error at every site that renders one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PublicationSupport {
    /// A source can be published, and other programs will see it.
    Supported,
    /// This platform offers no way to publish one.
    ///
    /// The adapter explains the specifics in its own words, following the pattern
    /// [`MidiSystemStatus::Unavailable`] and
    /// [`crate::domain::source::Availability::Unsupported`] already set: the
    /// platform speaks for itself and the domain holds no table of per-platform
    /// prose. The detail MUST name what the user can do instead, because a
    /// limitation stated without a way forward leaves them stuck.
    Unsupported {
        /// What cannot be done here, and what to do instead.
        detail: String,
    },
}
