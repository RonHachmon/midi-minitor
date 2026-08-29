//! Whether the monitor is taking in what arrives, or holding what it has.
//!
//! # Why this is capture rather than display
//!
//! Freezing the *display* and freezing the *record* look identical for about a
//! second, and then diverge. [`crate::domain::event_log::EventLog`] evicts
//! oldest-first at the retention cap, so a monitor that keeps retaining while
//! the list is frozen will — on a stream fast enough to be worth pausing at all
//! — quietly evict the very rows the user paused to read. The failure would be
//! invisible against a slow source and certain against a fast one.
//!
//! So pausing stops retention. What was already received is untouched, and
//! traffic that passes during a pause is not recorded. That trade is deliberate:
//! the user asked to keep what they had, and the alternative — buffering — must
//! either be bounded, which loses events anyway, or unbounded, which is a leak
//! nobody can see.

/// Whether arriving events are being taken in.
///
/// # Why an enum rather than a `bool`
///
/// The same reason [`crate::domain::filter::PrefixMode`],
/// [`crate::domain::source::CheckState`], and
/// [`crate::application::ports::ByteFidelity`] are enums: `set_capture_state(Paused)`
/// says what it means where `set_paused(true)` does not, and the value crosses
/// the IPC boundary as a tagged union the webview matches exhaustively rather
/// than as a bare `true` a renderer can misread.
///
/// # Why this is never persisted
///
/// It is absent from [`crate::application::settings::PersistedSettings`] on
/// purpose. Restoring a paused monitor would open a window that shows nothing,
/// updates never, and explains itself only if the user notices one control — a
/// state indistinguishable from a broken launch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureState {
    /// Arriving events are retained, and the visible ones are streamed.
    Running,
    /// Arriving events are discarded. Everything already retained is untouched,
    /// and every filter still operates over it.
    Paused,
}

impl Default for CaptureState {
    /// A monitor that has just started is monitoring.
    ///
    /// Launching into a paused state would present an empty, motionless window
    /// as the application's normal appearance.
    fn default() -> Self {
        Self::Running
    }
}
