//! The simulated event source.
//!
//! # The only module that knows the traffic is not real
//!
//! Everything above this module — the domain, the use cases, the Tauri layer,
//! the webview — is written as though these events came from hardware. There is
//! no flag to check, no `is_simulated` field riding along on an event, and no
//! `Fake` in any name it exposes. Replacing this with real MIDI input means
//! writing a different [`EventSource`] and changing the line that constructs it.
//!
//! That containment is the point of the whole arrangement: a mock that leaks
//! upward becomes a rewrite the moment real input arrives.

pub mod catalogue;
pub mod traffic;

use crate::application::error::CoreError;
use crate::application::ports::{Clock, EventSink, EventSource};
use crate::domain::event::MidiEvent;
use crate::domain::ids::EventId;
use crate::domain::source::Source;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use traffic::TrafficGenerator;

/// How long the generator sleeps between ticks.
///
/// With several messages per tick this lands comfortably above the 500
/// events/second the interface is required to stay responsive under, so the
/// throughput requirement is actually exercised rather than merely claimed.
const TICK_INTERVAL: Duration = Duration::from_millis(8);

/// An [`EventSource`] that generates traffic instead of reading hardware.
pub struct SimulatedSource {
    clock: Arc<dyn Clock>,
    running: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

impl SimulatedSource {
    /// Creates a source that timestamps its events with `clock`.
    #[must_use]
    pub fn new(clock: Arc<dyn Clock>) -> Self {
        Self {
            clock,
            running: Arc::new(AtomicBool::new(false)),
            worker: None,
        }
    }
}

impl EventSource for SimulatedSource {
    /// Spawns the generation thread.
    ///
    /// # Errors
    ///
    /// Never fails in this implementation — generation needs no resources that
    /// could be unavailable. The port still returns `Result` because a real MIDI
    /// adapter can fail to open a port, and widening that signature later would
    /// touch every caller.
    fn start(&mut self, sink: EventSink) -> Result<(), CoreError> {
        if self.running.swap(true, Ordering::SeqCst) {
            return Ok(());
        }

        let running = Arc::clone(&self.running);
        let clock = Arc::clone(&self.clock);

        self.worker = Some(thread::spawn(move || {
            let mut generator = TrafficGenerator::new();
            let mut next_id = EventId::FIRST;

            while running.load(Ordering::SeqCst) {
                for generated in generator.tick() {
                    let event =
                        MidiEvent::new(next_id, clock.now(), generated.source, generated.message);
                    next_id = next_id.next();
                    sink(event);
                }
                thread::sleep(TICK_INTERVAL);
            }
        }));

        Ok(())
    }

    /// Signals the generation thread to finish and waits for it.
    ///
    /// Joining rather than detaching means a stopped source has really stopped;
    /// a detached thread could still deliver an event after the caller believed
    /// monitoring had ceased.
    fn stop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
        if let Some(worker) = self.worker.take() {
            // A worker that panicked has already stopped, which is the outcome
            // being asked for, so the join result carries no useful signal.
            drop(worker.join());
        }
    }

    fn catalogue(&self) -> Vec<Source> {
        catalogue::build()
    }
}

impl Drop for SimulatedSource {
    fn drop(&mut self) {
        self.stop();
    }
}
