//! Which platform adapter this build talks to.
//!
//! # Why this file exists
//!
//! It is the **only** place in the application that names an operating system.
//! Everything above it — the shell's wiring, the command handlers, the wire
//! types, the whole webview — holds one type, [`EventSource`], and cannot tell
//! which implementation is behind it. Every platform difference a user can
//! observe reaches them as *data* the adapter reported, never as a conditional
//! someone wrote further up.
//!
//! That rule is worth stating because it is easy to erode one small check at a
//! time. If a `cfg(target_os)` is ever needed in another file in this crate, the
//! answer is almost always that the port is missing a method — which is exactly
//! what happened to `sync_ports`, and what
//! [`EventSource::capabilities`](midi_core::application::ports::EventSource::capabilities)
//! now exists to prevent for platform limitations.
//!
//! # Why a function rather than a `cfg`-selected type alias
//!
//! An alias would give the shell one name for less code, but it would leave the
//! contract between the two adapters *structural*: two inherent methods that
//! merely happen to share a signature, checked only on the platform being
//! compiled. A macOS developer never compiles the Windows adapter, so drift
//! would stay invisible until a Windows build ran. Returning a trait object
//! makes the compiler enforce, on both platforms, precisely what the shell
//! depends on.

use midi_core::application::ports::{Clock, EventSource};
use std::sync::Arc;

/// Builds the MIDI adapter this platform provides, timestamping with `clock`.
///
/// Boxed rather than returned concretely so the two arms below have one return
/// type. The cost is one virtual call per event batch, which is nothing next to
/// the work of decoding one.
#[cfg(target_os = "macos")]
#[must_use]
pub fn event_source(clock: Arc<dyn Clock>) -> Box<dyn EventSource> {
    Box::new(midi_macos::CoreMidiSource::new(clock))
}

/// Builds the MIDI adapter this platform provides, timestamping with `clock`.
///
/// Boxed rather than returned concretely so the two arms above have one return
/// type. The cost is one virtual call per event batch, which is nothing next to
/// the work of decoding one.
#[cfg(target_os = "windows")]
#[must_use]
pub fn event_source(clock: Arc<dyn Clock>) -> Box<dyn EventSource> {
    Box::new(midi_windows::WindowsMidiSource::new(clock))
}
