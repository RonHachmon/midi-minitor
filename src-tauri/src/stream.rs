//! The event stream from Rust to the webview.
//!
//! # Why a channel rather than the event system
//!
//! Tauri's documentation is explicit that its `emit`/`listen` event system "is
//! not designed for low latency or high throughput situations" and that "events
//! have no strong type support, event payloads are always JSON strings". Both
//! halves disqualify it here: this stream must sustain 500 events per second,
//! and the project's IPC contract must be typed end to end. Channels are the
//! mechanism Tauri documents for streaming, and they carry a real payload type.
//!
//! # Why batches rather than individual events
//!
//! At 500 events per second, one message per event means 500 serialization
//! round-trips and 500 React state updates every second. Coalescing into one
//! batch per frame reduces that to roughly sixty messages carrying eight events
//! each — less work on both sides, and aligned with when the browser actually
//! repaints.

use crate::dto::{CatalogueDto, EventDto, TargetsDto};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tauri::ipc::Channel;

/// How long events accumulate before a batch is sent.
///
/// One display frame at 60 Hz. Shorter buys no visible smoothness because the
/// webview cannot paint faster; longer starts to feel like lag.
const BATCH_INTERVAL: Duration = Duration::from_millis(16);

/// Accumulates admitted events and flushes them to the webview in batches.
pub struct EventPump {
    pending: Mutex<Vec<EventDto>>,
    channel: Mutex<Option<Channel<crate::dto::EventBatchDto>>>,
    running: AtomicBool,
    /// Events lost because the queue could not be reached.
    ///
    /// # Why this is counted rather than merely tolerated
    ///
    /// Dropping an event under contention is the right call — stalling the MIDI
    /// callback thread would be worse. But a monitor that loses traffic without
    /// saying so misreports what the device sent, which is the one failure this
    /// application cannot afford. Counting makes the loss reportable.
    dropped: AtomicU32,
}

impl EventPump {
    /// Creates an idle pump with no subscriber.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            pending: Mutex::new(Vec::new()),
            channel: Mutex::new(None),
            running: AtomicBool::new(false),
            dropped: AtomicU32::new(0),
        }
    }

    /// Registers the webview's channel, replacing any previous subscriber.
    ///
    /// Replacing rather than rejecting matters during development: a hot reload
    /// mounts a fresh webview, and the stale channel must not keep the old one
    /// alive.
    pub fn subscribe(&self, channel: Channel<crate::dto::EventBatchDto>) {
        if let Ok(mut slot) = self.channel.lock() {
            *slot = Some(channel);
        }
        if let Ok(mut pending) = self.pending.lock() {
            pending.clear();
        }
    }

    /// Queues one event for the next batch.
    ///
    /// Called from a MIDI callback thread, so it does no more than append. A lock
    /// that cannot be taken drops the event rather than blocking that thread —
    /// stalling MIDI delivery would back up every other device in the process.
    ///
    /// The drop is **counted**, not swallowed: see [`Self::dropped`].
    pub fn push(&self, event: EventDto) {
        if let Ok(mut pending) = self.pending.lock() {
            pending.push(event);
        } else {
            self.dropped.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// How many events were lost because the queue could not be reached.
    #[must_use]
    pub fn dropped(&self) -> u32 {
        self.dropped.load(Ordering::Relaxed)
    }

    /// Discards anything queued but not yet sent.
    ///
    /// Used when a settings change replaces the visible list, so events admitted
    /// under the previous settings do not arrive immediately afterwards.
    pub fn discard_pending(&self) {
        if let Ok(mut pending) = self.pending.lock() {
            pending.clear();
        }
    }

    /// Starts the flush loop on its own thread. Idempotent.
    pub fn start(self: &Arc<Self>) {
        if self.running.swap(true, Ordering::SeqCst) {
            return;
        }
        let pump = Arc::clone(self);
        thread::spawn(move || {
            while pump.running.load(Ordering::SeqCst) {
                pump.flush();
                thread::sleep(BATCH_INTERVAL);
            }
        });
    }

    /// Stops the flush loop.
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }

    /// Sends everything queued, if there is a subscriber and anything to send.
    fn flush(&self) {
        let batch = match self.pending.lock() {
            Ok(mut pending) if !pending.is_empty() => std::mem::take(&mut *pending),
            _ => return,
        };

        let Ok(slot) = self.channel.lock() else {
            return;
        };
        let Some(channel) = slot.as_ref() else {
            return;
        };

        // A send failure means the webview is gone — during a reload, or at
        // shutdown. Dropping the batch is the correct response; there is nobody
        // left to report it to.
        drop(channel.send(crate::dto::EventBatchDto { events: batch }));
    }
}

impl Default for EventPump {
    fn default() -> Self {
        Self::new()
    }
}

/// Pushes a latest-wins payload to the webview when something changes.
///
/// # Why this is not folded into [`EventPump`]
///
/// The two carry unrelated payloads at unrelated rates. Events arrive hundreds
/// per second and are worth batching on a frame timer; the changes this pump
/// carries happen when someone physically touches a cable. Sharing one channel
/// would force a sum type onto the hot path and make the batching interval the
/// floor for how quickly the Sources list could react.
///
/// No batching here for the same reason: there is nothing to coalesce. The burst
/// one plug event produces is already collapsed by the adapter, before it reaches
/// this layer.
///
/// # Why this is generic when it once was not
///
/// It carried only the source catalogue. The send screen needs exactly the same
/// behaviour for the target list — latest wins, no batching, on the same
/// human-scale event — so a second hand-written copy would be twenty duplicated
/// lines whose only difference is a type. `EventPump` stays separate because its
/// substance *is* the batching, and none of that is shared.
pub struct PushPump<T> {
    channel: Mutex<Option<Channel<T>>>,
}

impl<T: Clone + serde::Serialize + Send + 'static> PushPump<T> {
    /// Creates a pump with no subscriber.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            channel: Mutex::new(None),
        }
    }

    /// Registers the webview's channel, replacing any previous subscriber.
    ///
    /// Replacing rather than rejecting matters during development: a hot reload
    /// mounts a fresh webview, and the stale channel must not keep the old one
    /// alive.
    pub fn subscribe(&self, channel: Channel<T>) {
        if let Ok(mut slot) = self.channel.lock() {
            *slot = Some(channel);
        }
    }

    /// Sends the latest payload to the webview, if anyone is listening.
    pub fn send(&self, payload: T) {
        let Ok(slot) = self.channel.lock() else {
            return;
        };
        let Some(channel) = slot.as_ref() else {
            return;
        };
        // A send failure means the webview is gone — during a reload, or at
        // shutdown. There is nobody left to report it to.
        drop(channel.send(payload));
    }
}

impl<T: Clone + serde::Serialize + Send + 'static> Default for PushPump<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// The pump that pushes the Sources catalogue as devices come and go.
pub type CataloguePump = PushPump<CatalogueDto>;

/// The pump that pushes the send screen's target list.
///
/// Driven by the same device-change notification the catalogue is: one cable
/// moved changes both lists, and both are read from the adapter in the same
/// callback.
pub type TargetPump = PushPump<TargetsDto>;
