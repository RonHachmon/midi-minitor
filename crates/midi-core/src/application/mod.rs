//! Use cases, and the ports they need from the outside world.
//!
//! The single service here is [`monitor::Monitor`]: the one place where an
//! arriving event and the user's current settings meet. Everything the interface
//! can ask for is a method on it, which is what keeps decision-making out of the
//! Tauri layer and out of the webview.

pub mod error;
pub mod monitor;
pub mod ports;
pub mod settings;
