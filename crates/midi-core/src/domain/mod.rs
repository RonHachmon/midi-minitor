//! MIDI concepts and the rules that operate on them.
//!
//! This layer depends on nothing but the standard library and `serde`. It knows
//! nothing about Tauri, about the webview, or about where events come from —
//! which is what lets the same rules serve simulated traffic today and real MIDI
//! input later without being touched.

pub mod column;
pub mod event;
pub mod event_log;
pub mod filter;
pub mod ids;
pub mod message;
pub mod source;
