//! The WinMM event source: real ports, real bytes, real device changes.
//!
//! # Why one open handle per selected port
//!
//! WinMM opens a port at a time and gives each open handle its own callback and
//! its own instance pointer. That instance pointer is what carries *which source*
//! a delivery belongs to, so attribution is free and correct — unlike the macOS
//! adapter, which needs one input port per source for the same reason and says so
//! in its own module docs.
//!
//! Windows grants MIDI input ports **exclusively**. A port another program has
//! open cannot be opened here at all, so "another program is using this" is a
//! routine, recoverable state on this platform rather than the rarity it is on
//! macOS — which is why it is reported on the row and the row stays tickable.
//!
//! # Why the callback context is boxed and held here
//!
//! Windows keeps the instance pointer for the lifetime of the open handle and
//! passes it to a callback running on its own thread. Anything that could move or
//! free it while the port is open would leave the driver calling into freed
//! memory. Each open port therefore owns a pinned context, released only after
//! the port has been reset and closed.
//!
//! # Why `Availability::Absent` is never constructed here
//!
//! On macOS an endpoint can report itself offline while still being enumerated,
//! so a row can exist for a device that is not there. Windows does not do that: a
//! removed device simply stops being enumerated. There is no moment at which this
//! adapter could truthfully say "present but absent", so it never says it. The
//! variant is left alone rather than repurposed — a remembered selection for a
//! device that is gone is held by the monitor, not represented as a row here.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use midi_core::application::error::CoreError;
use midi_core::application::ports::{
    ByteFidelity, CatalogueSink, Clock, EventSink, EventSource, PlatformCapabilities,
    PublicationSupport, Transmitter,
};
use midi_core::domain::ids::{
    EventId, PublishedName, SourceGroupId, SourceId, SourceKey, TargetId, TargetKey,
};
use midi_core::domain::source::{Availability, Source};
use midi_core::domain::target::Target;
use windows::Win32::Media::Audio::{
    midiInClose, midiInOpen, midiInReset, midiInStart, midiInStop, CALLBACK_FUNCTION, HMIDIIN,
};
use windows::Win32::Media::MMSYSERR_NOERROR;

use crate::endpoints::{self, PortSnapshot};
use crate::notifications::DeviceWatcher;
use crate::receive::{midi_in_proc, Arrival, CallbackContext, PortHandle, Worker};
use crate::send::OutputHandle;
use crate::sysex::BufferPool;

/// The verbatim label of the standalone row this application cannot fill here.
///
/// Copied from `screenshots/sources.png`; the screenshots are the design
/// authority, so this string is normative rather than a description. It is the
/// same string the macOS adapter uses, and it must stay that way — the row is in
/// the same place saying the same thing on both platforms, and only its
/// availability differs.
const DESTINATION_ROW_LABEL: &str = "Act as a destination for other programs";

/// What the destination row says for itself on this platform.
///
/// # Why the row stays, and why it says this
///
/// Windows has no built-in way for an application to publish a MIDI destination
/// other programs can send to. Doing it anyway would mean installing a
/// system-wide driver with elevation, changing the machine's MIDI configuration
/// for every application on it — a far larger intervention than a monitor has any
/// business making.
///
/// So the row cannot work. It is not hidden, because the screenshots are the
/// design authority and a control they depict may not be removed; and it is not
/// left tickable, because a control that accepts a click and does nothing is a
/// lie a user only discovers after wasting time on it. It stays where it is and
/// says what it cannot do, and names the thing that does work.
const DESTINATION_UNSUPPORTED: &str = concat!(
    "Windows provides no built-in way for an application to publish a MIDI ",
    "destination that other programs can send to. To monitor what another ",
    "program sends, install a MIDI loopback utility: its ports appear under ",
    "MIDI sources and are monitored like any other port.",
);

/// What the `Data` column says for itself on this platform.
///
/// The substance is the platform fact, stated plainly: nothing is invented and
/// nothing is dropped, but for short messages the byte count shown is the message
/// Windows delivered rather than the count the cable carried.
const BYTE_FIDELITY_DETAIL: &str = concat!(
    "Windows delivers short messages already assembled, so the bytes shown are ",
    "the complete message Windows delivered rather than the exact bytes carried ",
    "on the cable — a message sent using running status arrives with its status ",
    "byte already restored. System Exclusive transfers are exact.",
);

/// What the publish control says on Windows.
///
/// Windows offers no supported way for an application to publish a MIDI source
/// that other programs can receive from — doing it anyway means shipping a
/// system-wide kernel driver, which this project does not. The wording is a
/// deliberate sibling of the `Act as a destination for other programs` message,
/// because it is the same limitation seen from the other direction, and it names
/// the way through rather than stopping at the refusal.
const PUBLICATION_DETAIL: &str = concat!(
    "Windows has no built-in way for an application to publish a MIDI source ",
    "other programs can receive from, and doing it anyway would mean installing ",
    "a system-wide driver. To reach another program, install a MIDI loopback ",
    "utility — its port appears in this list like any other destination.",
);

/// What a row says when nothing can tell it apart from another row.
///
/// Deliberately says what happened rather than what to do, because there is
/// nothing the user can do about it beyond ticking the one they want — which
/// they still can, since the row stays operable.
const UNMATCHABLE_SELECTION: &str = concat!(
    "Windows reports no identity for this port beyond its name, and another ",
    "port has the same name, so a previously saved selection could not be ",
    "matched to either.",
);

/// The display names carried by more than one port that has no other identity.
///
/// # Why a remembered selection is applied to *neither* of them
///
/// When Windows reports no device-interface string, a port's name is the only
/// identity left — and two ports can share a name. Restoring a saved selection
/// onto one of them would mean picking arbitrarily, and a monitor that starts
/// watching a device the user never chose is worse than one that watches
/// nothing: the rows would look right and be wrong, which is the failure this
/// application exists to prevent. So neither is selected and both rows say so,
/// leaving the choice where it belongs.
///
/// Ports that *do* report an interface string are never ambiguous even when they
/// share a name, which is the ordinary case and is unaffected.
fn ambiguous_names(ports: &[PortSnapshot]) -> Vec<String> {
    let nameless: Vec<&String> = ports
        .iter()
        .filter(|port| port.interface_id.is_none())
        .map(|port| &port.display_name)
        .collect();

    nameless
        .iter()
        .filter(|name| nameless.iter().filter(|other| other == name).count() > 1)
        .map(|name| (*name).clone())
        .collect()
}

/// How one port's session id is minted, given what else is attached.
///
/// # Why this is a function and not written out at each call site
///
/// `scan` and `sync_ports` both mint ids, and they **must** agree: the ids `scan`
/// puts in the catalogue are the ids `sync_ports` looks selections up by, so a
/// difference between them would silently open the wrong port or none at all.
/// One function is what keeps them from drifting.
fn mint_key(snapshot: &PortSnapshot, ambiguous: &[String]) -> MintKey {
    if ambiguous.contains(&snapshot.display_name) {
        MintKey::Positional(snapshot.device_index)
    } else {
        MintKey::Persistent(snapshot.key())
    }
}

/// Assigns and remembers session ids for ports.
///
/// # Why ids are stable within a session
///
/// A device unplugged and replugged keeps the id it had, so the webview updates a
/// row it already has rather than discarding and rebuilding it. Shared because
/// both the adapter and the device-change callback mint ids, and they must agree.
#[derive(Debug, Default)]
struct IdMinter {
    known: Mutex<HashMap<MintKey, SourceId>>,
    next: AtomicU32,
}

/// What a session id is remembered against.
///
/// # Why this is not simply [`SourceKey`]
///
/// Two ports with no device-interface string and the same name carry the *same*
/// `SourceKey`, because that key is the honest statement that nothing
/// distinguishes them. Minting session ids from it would then hand both rows one
/// id — and two sources sharing an id are not separately selectable, which is a
/// requirement in its own right. So identity-for-this-session and
/// identity-across-sessions are different questions, and this type is the first.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum MintKey {
    /// The port has an identity that outlives the session.
    Persistent(SourceKey),
    /// Nothing distinguishes this port but where it currently sits.
    ///
    /// Deliberately positional, and deliberately session-only: it is never
    /// persisted, because restoring a choice onto today's enumeration order is
    /// exactly the mistake [`SourceKey`] exists to prevent.
    Positional(u32),
}

impl IdMinter {
    /// The id for a key, minting one the first time the key is seen.
    fn id_for(&self, key: MintKey) -> SourceId {
        let Ok(mut known) = self.known.lock() else {
            // Only reachable if a panic poisoned the map. A fresh id costs a row
            // re-render; refusing would drop the device from the list entirely.
            return SourceId::new(self.next.fetch_add(1, Ordering::SeqCst) + 1);
        };
        if let Some(id) = known.get(&key) {
            return *id;
        }
        let id = SourceId::new(self.next.fetch_add(1, Ordering::SeqCst) + 1);
        known.insert(key, id);
        id
    }
}

/// The current catalogue: real input ports, plus the standalone row.
///
/// Shaped identically to the macOS adapter's `scan`, and deliberately so: the two
/// differ in exactly one `Availability` value, and reviewing them side by side is
/// how that stays true.
fn scan(minter: &IdMinter) -> Vec<Source> {
    let ports = endpoints::enumerate();
    let ambiguous = ambiguous_names(&ports);

    let mut sources: Vec<Source> = ports
        .iter()
        .map(|snapshot| {
            // Enumerated means present. Whether it will *open* is not known until
            // something tries, and a failure then is reported on the row by
            // `sync_ports` rather than guessed at here.
            let availability = if ambiguous.contains(&snapshot.display_name) {
                Availability::Unopenable {
                    detail: UNMATCHABLE_SELECTION.to_owned(),
                }
            } else {
                Availability::Open
            };
            let id = minter.id_for(mint_key(snapshot, &ambiguous));
            snapshot.to_source(id, availability)
        })
        .collect();

    // Always listed, exactly as on macOS: it is a capability the layout shows
    // unconditionally, not a device that comes and goes. The one difference
    // between the platforms is this availability.
    sources.push(Source {
        id: minter.id_for(MintKey::Persistent(SourceKey::VirtualDestination)),
        key: SourceKey::VirtualDestination,
        name: DESTINATION_ROW_LABEL.to_owned(),
        group: None,
        selected: false,
        availability: Availability::Unsupported {
            detail: DESTINATION_UNSUPPORTED.to_owned(),
        },
    });

    // The spy group is listed but never populated, on both platforms: observing
    // another application's outgoing traffic needs privileged system support that
    // is out of scope. Inventing placeholder sources to fill it would be exactly
    // the fabrication this application exists to avoid.
    let _ = SourceGroupId::SpyOnOutput;

    sources
}

/// One port this adapter currently has open.
struct OpenPort {
    handle: HMIDIIN,
    buffers: BufferPool,
    /// The callback's context, pinned for as long as the port is open.
    ///
    /// Held as a raw pointer because Windows holds one too. Reclaimed in
    /// [`WindowsMidiSource::close_port`], after the port is reset and closed and
    /// the callback can no longer run.
    context: *mut CallbackContext,
}

// SAFETY: everything here is a handle or a pinned allocation that only this
// adapter touches, from whichever single thread currently owns the adapter.
// WinMM handles are documented as usable from any thread — the API hands one to a
// callback on a system thread and expects the application to act on it elsewhere
// — and the context is written once before the port starts and read only by
// Windows until the port is closed. The compiler's caution here is about raw
// pointers in general, not about anything this code does with them.
unsafe impl Send for OpenPort {}

/// An [`EventSource`] backed by the machine's WinMM MIDI input ports.
pub struct WindowsMidiSource {
    clock: Arc<dyn Clock>,
    minter: Arc<IdMinter>,
    ports: HashMap<SourceId, OpenPort>,
    /// The hand-off to the worker, cloned into every open port's context.
    arrivals: Option<Sender<Arrival>>,
    worker: Option<Arc<Worker>>,
    worker_thread: Option<JoinHandle<()>>,
    watcher: Option<DeviceWatcher>,
    dropped: Arc<AtomicU32>,
    /// Session ids for send targets, stable for the same reason source ids are.
    target_minter: TargetIdMinter,
    /// The device currently open for sending, if any.
    ///
    /// One at a time: the port sends to one target per call, and holding every
    /// device open would take them all away from other programs for no gain.
    output: Option<OutputHandle>,
}

impl WindowsMidiSource {
    /// Creates an adapter that timestamps its events with `clock`.
    ///
    /// Nothing is opened here. Unlike the macOS adapter — whose client creation
    /// is bound to the thread it happens on — this is a plain constructor,
    /// because Windows attaches no thread meaning to any of these calls.
    #[must_use]
    pub fn new(clock: Arc<dyn Clock>) -> Self {
        Self {
            clock,
            minter: Arc::new(IdMinter::default()),
            ports: HashMap::new(),
            arrivals: None,
            worker: None,
            worker_thread: None,
            watcher: None,
            dropped: Arc::new(AtomicU32::new(0)),
            target_minter: TargetIdMinter::new(),
            output: None,
        }
    }

    /// Opens one port and starts it delivering.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::PortUnavailable`] when the port exists but cannot be
    /// opened — on this platform most often because another program holds it,
    /// which is routine rather than exceptional.
    fn open_port(&self, snapshot: &PortSnapshot, id: SourceId) -> Result<OpenPort, CoreError> {
        let (Some(arrivals), true) = (self.arrivals.as_ref(), self.worker.is_some()) else {
            return Err(CoreError::PortUnavailable {
                name: snapshot.display_name.clone(),
                detail: "the adapter is not running".to_owned(),
            });
        };

        let mut handle = HMIDIIN::default();
        // Pinned before the port is opened: Windows may call back the instant
        // `midiInStart` runs, and the pointer must already be valid.
        let context = Box::into_raw(Box::new(CallbackContext {
            source: id,
            port: PortHandle(handle),
            clock: Arc::clone(&self.clock),
            arrivals: arrivals.clone(),
            dropped: Arc::clone(&self.dropped),
        }));

        // SAFETY: `handle` is a live local the call writes once; the callback
        // pointer is a real `extern "system"` function with the signature WinMM
        // expects, and `context` is pinned and outlives the open port.
        let result = unsafe {
            midiInOpen(
                &raw mut handle,
                snapshot.device_index,
                Some(midi_in_proc as *const () as usize),
                Some(context as usize),
                CALLBACK_FUNCTION,
            )
        };

        if result != MMSYSERR_NOERROR {
            // SAFETY: the port did not open, so Windows kept no pointer to the
            // context and it is ours to reclaim.
            drop(unsafe { Box::from_raw(context) });
            return Err(CoreError::PortUnavailable {
                name: snapshot.display_name.clone(),
                detail: open_failure_detail(result),
            });
        }

        // The context was built before the handle existed, so it carries a null
        // handle until now. It is written before any buffer is queued, which is
        // the earliest the callback can fire.
        // SAFETY: the context is pinned, live, and not yet reachable by the
        // callback — no buffer has been queued and the port has not been started.
        unsafe {
            (*context).port = PortHandle(handle);
        }

        let buffers = BufferPool::attach(handle);

        // SAFETY: the port is open and owned by this adapter.
        let started = unsafe { midiInStart(handle) };
        if started != MMSYSERR_NOERROR {
            let mut port = OpenPort {
                handle,
                buffers,
                context,
            };
            Self::close_port(&mut port);
            return Err(CoreError::PortUnavailable {
                name: snapshot.display_name.clone(),
                detail: open_failure_detail(started),
            });
        }

        Ok(OpenPort {
            handle,
            buffers,
            context,
        })
    }

    /// Stops, resets, closes, and reclaims everything one open port owns.
    ///
    /// The order is the platform's, not a preference: **stop** ends delivery,
    /// **reset** makes the driver hand back every queued buffer, and only then
    /// may the buffers be unprepared and freed. Closing before resetting leaves
    /// the driver holding memory this process is about to free.
    fn close_port(port: &mut OpenPort) {
        // SAFETY: the handle is one this adapter opened and has not yet closed.
        unsafe {
            let _ = midiInStop(port.handle);
            let _ = midiInReset(port.handle);
        }
        port.buffers.release(port.handle);
        // SAFETY: the port has been reset, so no queued buffer remains and the
        // callback cannot fire again.
        unsafe {
            let _ = midiInClose(port.handle);
        }
        // SAFETY: the port is closed, so Windows will not call back into the
        // context again and it is ours to reclaim.
        drop(unsafe { Box::from_raw(port.context) });
    }
}

/// The user-facing reason a port would not open.
///
/// Windows reports exclusive-use refusal with its own code, and that is the case
/// worth naming: it is common here, it is not the user's mistake, and it clears
/// on its own when the other program lets go.
fn open_failure_detail(result: u32) -> String {
    /// Windows' code for "this device is already in use".
    const MIDIERR_ALLOCATED: u32 = 68;

    if result == MIDIERR_ALLOCATED {
        "Another program is using this port.".to_owned()
    } else {
        format!("Windows would not open this port (error {result}).")
    }
}

impl EventSource for WindowsMidiSource {
    /// Starts the worker thread and begins listening for device changes.
    ///
    /// # No thread constraint, unlike macOS
    ///
    /// The macOS adapter must be started on the main thread because CoreMIDI
    /// binds notification delivery to the run loop current when the first client
    /// is created. Nothing here does that: device notifications arrive through a
    /// callback that needs no window and no message pump, and WinMM handles are
    /// thread-agnostic. The shell still calls this early, which costs nothing.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::MidiSystemUnavailable`] when device-change
    /// notifications cannot be registered at all, which is the one failure that
    /// means this adapter cannot do its job. A *single* port failing to open is
    /// not an error here — it is reported on that source's availability, because
    /// the application must keep monitoring everything else.
    fn start(&mut self, events: EventSink, catalogue: CatalogueSink) -> Result<(), CoreError> {
        let (sender, receiver) = channel::<Arrival>();

        let worker = Arc::new(Worker {
            events,
            decoders: Mutex::new(HashMap::new()),
            next_event_id: Mutex::new(EventId::FIRST),
            dropped: Arc::clone(&self.dropped),
        });

        let runner = Arc::clone(&worker);
        // The worker owns the receiving end, so it stops when the adapter drops
        // the last sender — which is exactly what `stop` does.
        let thread = std::thread::spawn(move || runner.run(&receiver));

        // The device-change callback outlives every borrow this code could lend
        // it, so it holds only what is safe to share and re-enumerates on its
        // own. Enumeration needs nothing open, which is what makes that possible.
        let minter = Arc::clone(&self.minter);
        let watcher = DeviceWatcher::register(move || catalogue(scan(&minter)))?;

        self.arrivals = Some(sender);
        self.worker = Some(worker);
        self.worker_thread = Some(thread);
        self.watcher = Some(watcher);

        Ok(())
    }

    /// Closes everything and stops the worker. Idempotent.
    fn stop(&mut self) {
        self.watcher = None;
        // Closing the output device also resets it, releasing any note left
        // sounding by the last thing sent.
        self.output = None;

        let now = self.clock.arrival();
        for (id, mut port) in self.ports.drain() {
            Self::close_port(&mut port);
            // An interrupted transfer is reported rather than stranded.
            if let Some(worker) = self.worker.as_ref() {
                worker.abandon(id, now);
            }
        }

        // Dropping the last sender is what ends the worker's loop; joining then
        // guarantees every arrival already queued has been published before this
        // returns.
        self.arrivals = None;
        if let Some(thread) = self.worker_thread.take() {
            let _ = thread.join();
        }
        self.worker = None;
    }

    fn catalogue(&self) -> Vec<Source> {
        scan(&self.minter)
    }

    fn sync_ports(&mut self, sources: &[Source]) -> Vec<(SourceId, String)> {
        if self.worker.is_none() {
            return Vec::new();
        }

        let selected: Vec<SourceId> = sources
            .iter()
            .filter(|source| source.selected && source.key != SourceKey::VirtualDestination)
            .map(|source| source.id)
            .collect();

        let mut failures = Vec::new();

        // Minted exactly as `scan` mints, from the same enumeration, so the ids
        // here are the ids the catalogue the caller is holding was built from.
        let ports = endpoints::enumerate();
        let ambiguous = ambiguous_names(&ports);

        for snapshot in &ports {
            let id = self.minter.id_for(mint_key(snapshot, &ambiguous));
            if !selected.contains(&id) || self.ports.contains_key(&id) {
                continue;
            }
            match self.open_port(snapshot, id) {
                Ok(port) => {
                    self.ports.insert(id, port);
                }
                Err(error) => failures.push((id, reason_of(&error))),
            }
        }

        // Ports whose device has gone, or whose row was unticked: close them and
        // flush any transfer still arriving, so an interrupted dump is reported
        // rather than stranded.
        let stale: Vec<SourceId> = self
            .ports
            .keys()
            .copied()
            .filter(|id| !selected.contains(id))
            .collect();
        let now = self.clock.arrival();
        for id in stale {
            if let Some(mut port) = self.ports.remove(&id) {
                Self::close_port(&mut port);
            }
            if let Some(worker) = self.worker.as_ref() {
                worker.abandon(id, now);
            }
        }

        // The standalone row is deliberately absent from this loop. On this
        // platform it cannot be switched on at all, so there is nothing to open
        // and nothing to fail — and no event is ever attributed to it.
        failures
    }

    /// Windows assembles short messages before any application can see them.
    ///
    /// Every byte reported is a byte Windows delivered, in the order delivered,
    /// unaltered — but running status is expanded before this adapter can
    /// observe it, so for short messages the byte count is the message Windows
    /// delivered rather than the count the cable carried. System Exclusive
    /// arrives as a real byte buffer and is exact.
    fn capabilities(&self) -> PlatformCapabilities {
        PlatformCapabilities {
            publication: PublicationSupport::Unsupported {
                detail: PUBLICATION_DETAIL.to_owned(),
            },
            byte_fidelity: ByteFidelity::Assembled {
                detail: BYTE_FIDELITY_DETAIL.to_owned(),
            },
        }
    }

    fn dropped_events(&self) -> u32 {
        self.dropped.load(Ordering::Relaxed)
    }
}

/// The user-facing reason carried by a port or system error.
fn reason_of(error: &CoreError) -> String {
    match error {
        CoreError::PortUnavailable { detail, .. } | CoreError::MidiSystemUnavailable { detail } => {
            detail.clone()
        }
        other => other.to_string(),
    }
}

impl Drop for WindowsMidiSource {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Assigns and remembers session ids for send targets.
///
/// # Why targets need their own minter rather than reusing the source one
///
/// Inputs and outputs are different sets, keyed by different types. Sharing one
/// minter would mean one number space across both, which buys nothing and would
/// let an id from one list be accepted by the other's lookup.
///
/// Simpler than the source minter, and deliberately so: an output has no
/// unidentifiable case. Windows always reports a name, and the name is the key,
/// so there is no equivalent of `MintKey`'s positional fallback here.
#[derive(Debug, Default)]
struct TargetIdMinter {
    known: Mutex<HashMap<TargetKey, TargetId>>,
    next: AtomicU32,
}

impl TargetIdMinter {
    /// An empty minter.
    fn new() -> Self {
        Self::default()
    }

    /// The id for a key, minting one the first time the key is seen.
    fn id_for(&self, key: TargetKey) -> TargetId {
        let Ok(mut known) = self.known.lock() else {
            // Only reachable if a panic poisoned the map. A fresh id costs a
            // re-render of the picker; refusing would drop the device from it.
            return TargetId::new(self.next.fetch_add(1, Ordering::SeqCst) + 1);
        };
        if let Some(id) = known.get(&key) {
            return *id;
        }
        let id = TargetId::new(self.next.fetch_add(1, Ordering::SeqCst) + 1);
        known.insert(key, id);
        id
    }
}

impl Transmitter for WindowsMidiSource {
    /// Every MIDI output port Windows reports.
    ///
    /// Never includes a published source: this platform cannot publish one, and
    /// listing a target that could never be sent to would be exactly the
    /// operable-but-inert control the specification forbids.
    ///
    /// Re-enumerated on each call rather than cached, as the source catalogue is:
    /// this is the set at the moment of asking.
    fn targets(&self) -> Vec<Target> {
        endpoints::enumerate_outputs()
            .into_iter()
            .map(|snapshot| {
                let id = self
                    .target_minter
                    .id_for(TargetKey::DeviceName(snapshot.display_name.clone()));
                snapshot.to_target(id)
            })
            .collect()
    }

    /// Sends the bytes to whichever output device that target names.
    ///
    /// # Why the device is re-found on every send
    ///
    /// Windows addresses output devices by an index that shifts as devices come
    /// and go, so an index cached from an earlier scan can quietly come to mean a
    /// different device. Re-enumerating and matching on the stable key is what
    /// makes "send to the thing the user chose" true rather than probable.
    ///
    /// The open handle is kept across sends but discarded the moment the chosen
    /// device changes — including when the same name moves to a different index.
    fn transmit(&mut self, target: TargetId, bytes: &[u8]) -> Result<(), CoreError> {
        let found = endpoints::enumerate_outputs().into_iter().find(|snapshot| {
            self.target_minter
                .id_for(TargetKey::DeviceName(snapshot.display_name.clone()))
                == target
        });
        let Some(snapshot) = found else {
            return Err(CoreError::UnknownTarget {
                name: String::new(),
            });
        };

        let open_elsewhere = self
            .output
            .as_ref()
            .is_some_and(|handle| handle.device_index() != snapshot.device_index);
        if open_elsewhere {
            // Dropping closes and resets the previous device, so a note left
            // sounding on it is released rather than stranded when the user
            // switches targets.
            self.output = None;
        }

        if self.output.is_none() {
            self.output = Some(OutputHandle::open(
                snapshot.device_index,
                &snapshot.display_name,
            )?);
        }

        let Some(handle) = self.output.as_ref() else {
            return Err(CoreError::TransmitFailed {
                target: snapshot.display_name.clone(),
                detail: "the device could not be opened".to_owned(),
            });
        };
        handle.send(bytes, &snapshot.display_name)
    }

    /// Always refuses: Windows cannot publish a MIDI source.
    ///
    /// The interface is expected never to call this, because the control it sits
    /// behind is not operable on this platform — the limitation reaches the user
    /// through [`PlatformCapabilities`] before they can try. This is the backstop
    /// for a stale view, and it carries the same wording so the two cannot drift.
    fn publish(&mut self, _name: &PublishedName) -> Result<(), CoreError> {
        Err(CoreError::PublicationUnsupported {
            detail: PUBLICATION_DETAIL.to_owned(),
        })
    }

    /// Nothing to withdraw: nothing can be published on this platform.
    fn unpublish(&mut self) {}
}
