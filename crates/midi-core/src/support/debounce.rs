//! Collapsing a burst of device-change notifications into a single rescan.
//!
//! # Why a debounce sits between the system and the application
//!
//! One physical connection produces a *burst* of notifications, not a single
//! one: the system reports each object appearing and then reports the setup as
//! changed. Re-enumerating on each would rebuild the Sources panel several times
//! for one plug event, and the user would watch it flicker.
//!
//! Collapsing the burst into one update costs a short delay that is invisible
//! next to the time it takes a person to push a connector in, and well inside the
//! two-second responsiveness the specification asks for.
//!
//! Both platform adapters need this behaviour identically, which is why it lives
//! here rather than in either of them — see the [module docs](super) for why the
//! alternatives were worse.
//!
//! # The ordering constraint each platform imposes on its own callback
//!
//! This type imposes none. The *platforms* do, differently, and each adapter
//! documents its own at the point it registers:
//!
//! - **macOS** fixes the thread notifications are delivered on at the moment the
//!   first MIDI client is created, and binds delivery to the run loop current at
//!   that moment — so that client must be created on the main thread before any
//!   other MIDI work. The failure mode is silent: hot-plug simply never fires.
//! - **Windows** imposes no equivalent constraint. `CM_Register_Notification`
//!   needs no window handle and binds to no run loop.
//!
//! What both share is the reason [`Debouncer::schedule`] does its waiting on a
//! thread of its own: the notification thread belongs to the system, and blocking
//! it would stall every other notification the process receives.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// How long to wait for a notification burst to settle.
///
/// Long enough to collapse the several notifications one physical connection
/// produces; far shorter than the two seconds the specification allows for the
/// Sources list to catch up.
const DEBOUNCE_MILLIS: u64 = 150;

/// Collapses a burst of setup-change notifications into single rescans.
///
/// # How the collapsing works
///
/// Each notification bumps a generation counter and schedules a wake-up. When the
/// wake-up finds the counter unchanged, the burst has settled and one rescan
/// runs. When it finds the counter moved on, a later wake-up is already coming
/// and this one does nothing — so a long burst produces exactly one rescan at its
/// end, not one per notification.
pub struct Debouncer {
    generation: Arc<AtomicU64>,
}

impl Debouncer {
    /// Creates a debouncer that has seen nothing yet.
    #[must_use]
    pub fn new() -> Self {
        Self {
            generation: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Records a notification and runs `rescan` once the burst settles.
    ///
    /// `rescan` runs on a short-lived thread rather than on the notification
    /// thread, because that thread belongs to the system and blocking it would
    /// stall every other notification the process receives.
    pub fn schedule<F>(&self, rescan: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let generation = Arc::clone(&self.generation);
        let scheduled = generation.fetch_add(1, Ordering::SeqCst) + 1;

        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(DEBOUNCE_MILLIS));
            // A later notification has already claimed the rescan; this wake-up
            // has nothing left to do.
            if generation.load(Ordering::SeqCst) == scheduled {
                rescan();
            }
        });
    }
}

impl Default for Debouncer {
    fn default() -> Self {
        Self::new()
    }
}
