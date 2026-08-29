//! MIDI concepts and the rules that operate on them.
//!
//! This layer depends on nothing but the standard library and `serde`. It knows
//! nothing about Tauri, about the webview, or about which platform's MIDI system
//! delivered the bytes — which is what let the same rules carry over unchanged
//! when real device input replaced generated traffic.

pub mod column;
pub mod composition;
pub mod decoder;
pub mod encoder;
pub mod event;
pub mod event_log;
pub mod filter;
pub mod ids;
pub mod message;
pub mod publication;
pub mod request;
pub mod send_record;
pub mod sendable;
pub mod source;
pub mod target;
