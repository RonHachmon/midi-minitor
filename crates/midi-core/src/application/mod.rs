//! Use cases, and the ports they need from the outside world.
//!
//! The single service here is [`monitor::Monitor`]: the one place where an
//! arriving event and the user's current settings meet. Everything the interface
//! can ask for is a method on it, which is what keeps decision-making out of the
//! Tauri layer and out of the webview.
//!
//! [`sender::Sender`] is its counterpart for the other direction: the one place
//! where what the user is composing, where it goes, and what went meet. It is one
//! service for the same reason [`monitor::Monitor`] is — its state is entangled —
//! and it holds no port, for the same reason too.
//!
//! [`capture::CaptureState`] sits beside it as the one piece of monitor state
//! that is neither a MIDI concept nor a saved preference: whether what arrives is
//! being taken in at all.

pub mod capture;
pub mod error;
pub mod monitor;
pub mod ports;
pub mod sender;
pub mod settings;
