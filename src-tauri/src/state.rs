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
use crate::stream::{CataloguePump, EventPump, TargetPump};
use midi_core::application::monitor::Monitor;
use midi_core::application::ports::{Clock, MidiAccess, SettingsRepository};
use midi_core::application::sender::{Outgoing, Sender};
use midi_core::domain::ids::SendRecordId;
use midi_core::domain::send_record::SendOutcome;
use midi_core::domain::source::Source;
use midi_core::domain::target::Target;
use std::sync::{Arc, Mutex, MutexGuard};

/// Everything the command handlers need.
pub struct AppState {
    monitor: Mutex<Monitor>,
    sender: Mutex<Sender>,
    /// The batching pump that feeds the webview's event channel.
    pub pump: Arc<EventPump>,
    /// The pump that pushes catalogue changes as devices come and go.
    pub catalogue_pump: Arc<CataloguePump>,
    /// The pump that pushes the send screen's target list.
    pub target_pump: Arc<TargetPump>,
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
    /// [`crate::platform::midi_access`] hands back a `Box` so its two arms can
    /// share one return type, and the composition root drives the adapter
    /// directly — starting it, reading its catalogue and its capabilities —
    /// before anything else can hold it. A `Mutex<Box<dyn _>>` cannot be coerced
    /// to a `Mutex<dyn _>` afterwards, and the alternatives all cost more than
    /// the one pointer hop this keeps: locking the adapter for the whole startup
    /// sequence would invent an unreachable poisoned-lock branch, and a blanket
    /// `MidiAccess for Box<dyn MidiAccess>` would add an impl to the core crate
    /// to save an indirection nobody can measure.
    source: Arc<Mutex<Box<dyn MidiAccess>>>,
    settings: Arc<dyn SettingsRepository>,
    /// Where send records take their timestamps from.
    clock: Arc<dyn Clock>,
}

/// The three channels that push to the webview.
///
/// # Why they travel together
///
/// Not to shorten a parameter list, though it does that. They are one thing: the
/// whole of what this process pushes to the interface without being asked. Adding
/// a fourth should be a change in one place, and a caller that has two of the
/// three should not compile.
pub struct Pumps {
    /// Events, batched on a frame timer.
    pub events: Arc<EventPump>,
    /// The Sources catalogue, latest wins.
    pub catalogue: Arc<CataloguePump>,
    /// The send screen's target list, latest wins.
    pub targets: Arc<TargetPump>,
}

impl AppState {
    /// Assembles the shared state.
    #[must_use]
    pub fn new(
        monitor: Monitor,
        sender: Sender,
        pumps: Pumps,
        source: Arc<Mutex<Box<dyn MidiAccess>>>,
        settings: Arc<dyn SettingsRepository>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            monitor: Mutex::new(monitor),
            sender: Mutex::new(sender),
            pump: pumps.events,
            catalogue_pump: pumps.catalogue,
            target_pump: pumps.targets,
            source,
            settings,
            clock,
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

    /// Writes the current settings to storage — both halves together.
    ///
    /// # Why this takes both services
    ///
    /// The document has a monitor half and a send half, and neither service can
    /// see the other's. Taking one and reading the other from a lock held inside
    /// would mean two lock orders in one process, which is how deadlocks are
    /// written. Taking both as arguments makes the caller acquire them, in the
    /// order this type documents: **monitor first, then sender**, everywhere.
    ///
    /// It is also the reason
    /// [`midi_core::application::settings::PersistedSettings::with_send`] exists
    /// rather than a struct-update expression: the missing half is impossible to
    /// forget rather than merely easy to remember.
    ///
    /// # Errors
    ///
    /// Returns [`IpcError::SettingsUnavailable`] when the store cannot be
    /// written. Callers should surface it without aborting the operation that
    /// triggered it — the change is already applied in memory, and failing to
    /// remember a preference is not a reason to reject it.
    pub fn persist(&self, monitor: &Monitor, sender: &Sender) -> IpcResult<()> {
        let settings = monitor
            .persisted_settings()
            .with_send(sender.persisted_settings());
        self.settings
            .save(&settings)
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

impl AppState {
    /// Locks the sender.
    ///
    /// # Errors
    ///
    /// Returns [`IpcError::MonitorUnavailable`] when the lock was poisoned by a
    /// panic on another thread. The same variant the monitor uses, because it is
    /// the same failure from the user's side — a window that has quietly stopped
    /// responding — and inventing a second name for it would only make the two
    /// look like different problems.
    pub fn sender(&self) -> IpcResult<MutexGuard<'_, Sender>> {
        self.sender.lock().map_err(|_| IpcError::MonitorUnavailable)
    }

    /// Sends what the sender says to send, and records what happened.
    ///
    /// # Why the three steps live here rather than in the core
    ///
    /// This is the same shape [`Self::sync_ports`] uses, and for the same reason:
    /// the core decides and the adapter acts, so something has to hold them
    /// together. That something is the composition root, and it decides nothing —
    /// `outgoing` chooses what and where, the port performs it, and `record`
    /// stores the result.
    ///
    /// # Errors
    ///
    /// Propagates whatever the sender or the adapter reported. A transmission
    /// failure is **both** returned and recorded: returned so the message lands
    /// where the user is looking, and recorded so it is still there when they
    /// look back.
    pub fn transmit(&self, sender: &mut Sender) -> IpcResult<()> {
        let outgoing = sender.outgoing()?;
        let outcome = self.transmit_bytes(&outgoing);
        self.record(sender, &outgoing, outcome)
    }

    /// Re-sends the exact bytes of an earlier send.
    ///
    /// The bytes come from the record rather than from re-encoding, so a re-send
    /// of a hand-typed message sends what was typed.
    ///
    /// # Errors
    ///
    /// As [`Self::transmit`], plus [`IpcError::UnknownSendRecord`] when the entry
    /// has been evicted past the retention ceiling.
    pub fn resend(&self, sender: &mut Sender, id: SendRecordId) -> IpcResult<()> {
        let recorded = sender.recorded(id)?;
        let target = sender.outgoing()?;
        let outgoing = Outgoing {
            target: target.target,
            target_name: target.target_name,
            message: recorded.message.clone(),
            bytes: recorded.bytes.clone(),
        };
        let outcome = self.transmit_bytes(&outgoing);
        self.record(sender, &outgoing, outcome)
    }

    /// Hands the bytes to the adapter and turns the answer into an outcome.
    fn transmit_bytes(&self, outgoing: &Outgoing) -> Result<(), String> {
        let Ok(mut source) = self.source.lock() else {
            return Err("the MIDI system is unavailable".to_owned());
        };
        source
            .transmit(outgoing.target, &outgoing.bytes)
            .map_err(|error| error.to_string())
    }

    /// Stores the outcome and returns it as a command result.
    fn record(
        &self,
        sender: &mut Sender,
        outgoing: &Outgoing,
        outcome: Result<(), String>,
    ) -> IpcResult<()> {
        let at = self.clock.now();
        match outcome {
            Ok(()) => {
                sender.record(outgoing, SendOutcome::Sent, at);
                Ok(())
            }
            Err(detail) => {
                sender.record(
                    outgoing,
                    SendOutcome::Failed {
                        detail: detail.clone(),
                    },
                    at,
                );
                Err(IpcError::TransmitFailed {
                    target: outgoing.target_name.clone(),
                    detail,
                })
            }
        }
    }

    /// Re-reads the target list from the adapter and pushes it to the webview.
    ///
    /// Called from the same catalogue callback that replaces the source list: one
    /// cable moved changes both, and re-reading a snapshot is not a decision.
    pub fn refresh_targets(&self) {
        let Ok(source) = self.source.lock() else {
            return;
        };
        let targets = source.targets();
        drop(source);

        let Ok(mut sender) = self.sender.lock() else {
            return;
        };
        sender.replace_targets(targets);
        self.target_pump.send(crate::dto::targets(&sender));
    }

    /// Publishes or withdraws the source to match what the sender holds.
    ///
    /// # Errors
    ///
    /// Propagates the adapter's refusal. The sender's state is left as it was, so
    /// the screen never reports a source as published that is not.
    pub fn apply_publication(&self, sender: &Sender) -> IpcResult<()> {
        let Ok(mut source) = self.source.lock() else {
            return Err(IpcError::MonitorUnavailable);
        };
        let publication = sender.publication();
        if publication.published {
            source.publish(&publication.name)?;
        } else {
            source.unpublish();
        }
        Ok(())
    }
}

impl AppState {
    /// The targets the adapter reports right now.
    ///
    /// # The lock order this exists to keep
    ///
    /// The source lock is taken and **released** before the caller does anything
    /// with the answer. That matters: a send holds the sender and then takes the
    /// source, so anything that took the source and then the sender would close a
    /// cycle between the two. Reading the list out and letting go is what keeps
    /// this one-directional.
    #[must_use]
    pub fn current_targets(&self) -> Vec<Target> {
        let Ok(source) = self.source.lock() else {
            return Vec::new();
        };
        source.targets()
    }
}
