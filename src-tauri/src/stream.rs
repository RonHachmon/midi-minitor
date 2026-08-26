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

use crate::dto::EventDto;
use std::sync::atomic::{AtomicBool, Ordering};
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
}

impl EventPump {
    /// Creates an idle pump with no subscriber.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            pending: Mutex::new(Vec::new()),
            channel: Mutex::new(None),
            running: AtomicBool::new(false),
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
    /// Called from the source's thread, so it does no more than append. A lock
    /// that cannot be taken drops the event rather than blocking the generator —
    /// a dropped event in a monitor is a far better outcome than a stalled one.
    pub fn push(&self, event: EventDto) {
        if let Ok(mut pending) = self.pending.lock() {
            pending.push(event);
        }
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
