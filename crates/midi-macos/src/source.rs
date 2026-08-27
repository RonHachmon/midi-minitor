//! The CoreMIDI event source: real ports, real bytes, real device changes.
//!
//! # Why one input port per source rather than one shared port
//!
//! CoreMIDI offers two receive paths. The MIDI 2.0 path carries a per-source
//! context, which would make attribution free, but delivers Universal MIDI
//! Packets — 32-bit words. Reaching the original byte stream from those means
//! translating back, and a monitor whose displayed bytes are a reconstruction
//! cannot honestly claim to show what arrived.
//!
//! The MIDI 1.0 path delivers the packet bytes themselves, which is what this
//! application must display, but its port carries no context. So each source gets
//! its own port and its own closure, and the closure captures which source it
//! belongs to. Real machines have single-digit port counts, so a handful of ports
//! is a small price for bytes that are actually the device's.
//!
//! # Why the notification callback cannot touch the adapter
//!
//! It is handed to the operating system and outlives every borrow this code could
//! give it. So it holds only what is safe to share — the id minter and the
//! catalogue sink — and re-enumerates on its own. Enumeration needs no client, so
//! this costs nothing. Deciding which *ports* to open and close in response stays
//! with the owner of the adapter, through [`CoreMidiSource::sync_ports`].
//!
//! # Why the decoders are held centrally
//!
//! Running status and an in-progress System Exclusive transfer are per-stream
//! state that must survive between packets. Holding them keyed by source also
//! means a disappearing device can have its half-finished transfer flushed rather
//! than stranded.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

use coremidi::{
    Client, InputPort, Notification, PacketList, Source as CoreSource, VirtualDestination,
};
use midi_core::application::error::CoreError;
use midi_core::application::ports::{CatalogueSink, Clock, EventSink, EventSource};
use midi_core::domain::decoder::{Decoded, MessageDecoder};
use midi_core::domain::event::MidiEvent;
use midi_core::domain::ids::{EventId, SourceGroupId, SourceId, SourceKey};
use midi_core::domain::message::{InvalidReason, MidiMessage};
use midi_core::domain::source::{Availability, Source};

use crate::endpoints;
use crate::notifications::Debouncer;

/// The name this application publishes itself under to other programs.
///
/// Appears in every other application's destination list, so it names the
/// product rather than describing the mechanism.
const VIRTUAL_DESTINATION_NAME: &str = "MIDI Monitor";

/// The name given to the CoreMIDI client.
const CLIENT_NAME: &str = "MIDI Monitor";

/// The verbatim label of the standalone row this application's own endpoint fills.
///
/// Copied from `screenshots/sources.png`; the screenshots are the design
/// authority, so this string is normative rather than a description.
const DESTINATION_ROW_LABEL: &str = "Act as a destination for other programs";

/// Assigns and remembers session ids for endpoints.
///
/// # Why ids are stable within a session
///
/// A device unplugged and replugged keeps the id it had, so the webview updates a
/// row it already has rather than discarding and rebuilding it. Shared because
/// both the adapter and the notification callback mint ids, and they must agree.
#[derive(Debug, Default)]
struct IdMinter {
    known: Mutex<HashMap<SourceKey, SourceId>>,
    next: AtomicU32,
}

impl IdMinter {
    /// The id for a key, minting one the first time the key is seen.
    fn id_for(&self, key: SourceKey) -> SourceId {
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

/// The current catalogue: real input ports, plus this application's own row.
fn scan(minter: &IdMinter) -> Vec<Source> {
    let mut sources: Vec<Source> = endpoints::enumerate()
        .into_iter()
        .map(|(snapshot, _)| {
            let id = minter.id_for(SourceKey::Endpoint(snapshot.unique_id));
            snapshot.to_source(id)
        })
        .collect();

    // Always listed, whether or not the endpoint currently exists: it is a
    // capability the user switches on, not a device that comes and goes, and the
    // reference layout shows it unconditionally.
    sources.push(Source {
        id: minter.id_for(SourceKey::VirtualDestination),
        key: SourceKey::VirtualDestination,
        name: DESTINATION_ROW_LABEL.to_owned(),
        group: None,
        selected: false,
        availability: Availability::Open,
    });

    // The spy group is listed but never populated: observing another
    // application's outgoing traffic needs privileged system support that is out
    // of scope here. The group's own row explains that; inventing placeholder
    // sources to fill it would be exactly the fabrication this feature removed.
    let _ = SourceGroupId::SpyOnOutput;

    sources
}

/// Everything shared with the CoreMIDI receive callbacks.
///
/// Callbacks fire on the system's own threads, so every field they touch lives
/// behind a lock or an atomic.
struct Shared {
    events: EventSink,
    clock: Arc<dyn Clock>,
    decoders: Mutex<HashMap<SourceId, MessageDecoder>>,
    next_event_id: Mutex<EventId>,
    /// Counts events lost because shared state could not be reached, so the
    /// application can say it fell behind rather than quietly losing traffic.
    dropped: AtomicU32,
}

impl Shared {
    /// Decodes one packet list and delivers whatever it completed.
    ///
    /// Runs on a CoreMIDI callback thread and does the least possible work:
    /// decode, wrap, hand off. Anything slower here would back up the system's
    /// delivery of every other device's packets.
    fn receive(&self, source: SourceId, packets: &PacketList) {
        let Ok(mut decoders) = self.decoders.lock() else {
            self.dropped.fetch_add(1, Ordering::Relaxed);
            return;
        };
        let decoder = decoders.entry(source).or_default();

        let mut outcomes = Vec::new();
        for packet in packets.iter() {
            outcomes.extend(decoder.feed(packet.data()));
        }
        drop(decoders);

        for outcome in outcomes {
            self.deliver(source, outcome);
        }
    }

    /// Turns one decoded outcome into an event and hands it to the sink.
    ///
    /// Every outcome becomes a row, including the three that are not valid
    /// messages. That is the point of the decoder reporting them as values.
    fn deliver(&self, source: SourceId, outcome: Decoded) {
        let (message, raw) = match outcome {
            Decoded::Message { message, raw } => (message, raw),
            Decoded::Invalid { reason, raw } => (
                MidiMessage::Invalid {
                    reason,
                    bytes: raw.clone(),
                },
                raw,
            ),
            Decoded::SysexTruncated { raw, true_len: _ } | Decoded::SysexIncomplete { raw } => (
                MidiMessage::Invalid {
                    reason: InvalidReason::TruncatedData,
                    bytes: raw.clone(),
                },
                raw,
            ),
        };

        let Ok(mut next) = self.next_event_id.lock() else {
            self.dropped.fetch_add(1, Ordering::Relaxed);
            return;
        };
        let id = *next;
        *next = next.next();
        drop(next);

        (self.events)(MidiEvent::new(id, self.clock.now(), source, message, raw));
    }

    /// Flushes a departing source's half-finished transfer, if it had one.
    fn abandon(&self, source: SourceId) {
        let Ok(mut decoders) = self.decoders.lock() else {
            return;
        };
        let outcome = decoders.get_mut(&source).and_then(MessageDecoder::abandon);
        decoders.remove(&source);
        drop(decoders);

        if let Some(outcome) = outcome {
            self.deliver(source, outcome);
        }
    }
}

/// An [`EventSource`] backed by the machine's CoreMIDI endpoints.
pub struct CoreMidiSource {
    clock: Arc<dyn Clock>,
    client: Option<Client>,
    shared: Option<Arc<Shared>>,
    ports: HashMap<SourceId, InputPort>,
    virtual_destination: Option<VirtualDestination>,
    minter: Arc<IdMinter>,
}

impl CoreMidiSource {
    /// Creates an adapter that timestamps its events with `clock`.
    ///
    /// The CoreMIDI client is deliberately **not** created here. It is created in
    /// [`EventSource::start`], because the operating system binds notification
    /// delivery to whichever thread and run loop are current at that moment — so
    /// when the client is created is a correctness question, not a lifecycle
    /// convenience. See the note on [`EventSource::start`].
    #[must_use]
    pub fn new(clock: Arc<dyn Clock>) -> Self {
        Self {
            clock,
            client: None,
            shared: None,
            ports: HashMap::new(),
            virtual_destination: None,
            minter: Arc::new(IdMinter::default()),
        }
    }

    /// Opens and closes ports so the listening set matches `sources`.
    ///
    /// Called by the owner of this adapter when the catalogue changes, and once
    /// at startup. Returns the sources that could not be opened, paired with the
    /// reason, so the caller can mark those rows rather than failing outright:
    /// one device held by another application must not stop the others being
    /// watched.
    pub fn sync_ports(&mut self, sources: &[Source]) -> Vec<(SourceId, String)> {
        let (Some(client), Some(shared)) = (self.client.as_ref(), self.shared.as_ref()) else {
            return Vec::new();
        };

        let selected: Vec<SourceId> = sources
            .iter()
            .filter(|source| source.selected && source.key != SourceKey::VirtualDestination)
            .map(|source| source.id)
            .collect();

        let mut failures = Vec::new();

        for (snapshot, endpoint) in endpoints::enumerate() {
            let id = self.minter.id_for(SourceKey::Endpoint(snapshot.unique_id));
            if !selected.contains(&id) || self.ports.contains_key(&id) {
                continue;
            }
            match Self::open(client, shared, id, &snapshot.display_name, &endpoint) {
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
        for id in stale {
            self.ports.remove(&id);
            shared.abandon(id);
        }

        // The standalone row is a published endpoint rather than a port to open,
        // so it follows its own checkbox separately.
        let wants_destination = sources
            .iter()
            .any(|source| source.key == SourceKey::VirtualDestination && source.selected);
        if wants_destination {
            if let Err(error) = self.open_virtual_destination() {
                if let Some(source) = sources
                    .iter()
                    .find(|source| source.key == SourceKey::VirtualDestination)
                {
                    failures.push((source.id, reason_of(&error)));
                }
            }
        } else {
            self.virtual_destination = None;
        }

        failures
    }

    /// Opens a listening port for one endpoint.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::PortUnavailable`] when the endpoint exists but cannot
    /// be opened — typically because another application holds it exclusively.
    fn open(
        client: &Client,
        shared: &Arc<Shared>,
        id: SourceId,
        name: &str,
        endpoint: &CoreSource,
    ) -> Result<InputPort, CoreError> {
        let callback_shared = Arc::clone(shared);
        let port = client
            .input_port(name, move |packets: &PacketList| {
                callback_shared.receive(id, packets);
            })
            .map_err(|status| CoreError::PortUnavailable {
                name: name.to_owned(),
                detail: format!("the MIDI system refused the port (status {status})"),
            })?;

        port.connect_source(endpoint)
            .map_err(|status| CoreError::PortUnavailable {
                name: name.to_owned(),
                detail: format!("the device could not be connected (status {status})"),
            })?;

        Ok(port)
    }

    /// Publishes this application as a destination other programs can send to.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::PortUnavailable`] when the endpoint cannot be
    /// created. The caller marks its row; everything else keeps working.
    fn open_virtual_destination(&mut self) -> Result<(), CoreError> {
        if self.virtual_destination.is_some() {
            return Ok(());
        }
        let (Some(client), Some(shared)) = (self.client.as_ref(), self.shared.as_ref()) else {
            return Ok(());
        };

        let id = self.minter.id_for(SourceKey::VirtualDestination);
        let callback_shared = Arc::clone(shared);
        let destination = client
            .virtual_destination(VIRTUAL_DESTINATION_NAME, move |packets: &PacketList| {
                callback_shared.receive(id, packets);
            })
            .map_err(|status| CoreError::PortUnavailable {
                name: DESTINATION_ROW_LABEL.to_owned(),
                detail: format!("the destination could not be published (status {status})"),
            })?;

        self.virtual_destination = Some(destination);
        Ok(())
    }

    /// How many events were lost because shared state could not be reached.
    ///
    /// Surfaced rather than kept internal: a monitor that silently loses traffic
    /// is worse than one that admits it fell behind.
    #[must_use]
    pub fn dropped_events(&self) -> u32 {
        self.shared
            .as_ref()
            .map_or(0, |shared| shared.dropped.load(Ordering::Relaxed))
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

impl EventSource for CoreMidiSource {
    /// Creates the client and begins listening.
    ///
    /// # The ordering that must not change
    ///
    /// This is where the process's first CoreMIDI client is created, and macOS
    /// binds notification delivery to the thread and run loop current at that
    /// moment. It must therefore run on the main thread, before any other MIDI
    /// work. Moving MIDI work above this call breaks hot-plug **silently** —
    /// notifications simply never arrive, with no error to notice.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::MidiSystemUnavailable`] when the MIDI system cannot
    /// be reached at all, which the interface must present as distinct from
    /// having found no devices.
    fn start(&mut self, events: EventSink, catalogue: CatalogueSink) -> Result<(), CoreError> {
        let shared = Arc::new(Shared {
            events,
            clock: Arc::clone(&self.clock),
            decoders: Mutex::new(HashMap::new()),
            next_event_id: Mutex::new(EventId::FIRST),
            dropped: AtomicU32::new(0),
        });

        // The callback outlives every borrow this code could lend it, so it holds
        // only shareable state and re-enumerates on its own. Enumeration needs no
        // client, which is what makes that possible.
        let sink = Arc::new(catalogue);
        let minter = Arc::clone(&self.minter);
        let debouncer = Debouncer::new();

        // The parameter type is spelled out because inference otherwise binds the
        // closure to one specific lifetime, and the callback must accept a
        // notification borrowed for any.
        let client = Client::new_with_notifications(CLIENT_NAME, move |_: &Notification| {
            let sink = Arc::clone(&sink);
            let minter = Arc::clone(&minter);
            // Every notification kind is treated the same way: re-read the truth
            // from the system. Reacting to specific kinds would mean trusting the
            // notification to describe the change completely, and a rescan is
            // cheap enough that trusting it buys nothing.
            debouncer.schedule(move || sink(scan(&minter)));
        })
        .map_err(|status| CoreError::MidiSystemUnavailable {
            detail: format!("the MIDI system could not be opened (status {status})"),
        })?;

        self.client = Some(client);
        self.shared = Some(shared);

        Ok(())
    }

    fn stop(&mut self) {
        self.ports.clear();
        self.virtual_destination = None;
        self.client = None;
        self.shared = None;
    }

    fn catalogue(&self) -> Vec<Source> {
        scan(&self.minter)
    }
}

impl Drop for CoreMidiSource {
    fn drop(&mut self) {
        self.stop();
    }
}
