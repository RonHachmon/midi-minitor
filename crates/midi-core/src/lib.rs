//! Monitoring core for the MIDI Monitor.
//!
//! # Why this crate exists separately from `src-tauri`
//!
//! Every rule about *what the user sees* lives here: which sources are being
//! monitored, which messages pass the filter, how many events are retained. The
//! Tauri app shell above translates those rules to and from the webview and
//! contains none of them.
//!
//! That separation is enforced rather than merely intended. This crate does not
//! depend on `tauri`, so reaching for an `AppHandle` from a domain type is a
//! compile error instead of something a reviewer has to notice. The project has
//! no test suite by design, so pushing invariants into the compiler is the only
//! safety net available — the layering rule is worth a whole crate for that reason.
//!
//! # This crate produces no events of its own
//!
//! It once carried a simulator that generated traffic. That module is gone, and
//! its absence is a requirement rather than a tidy-up: every row the application
//! displays must be a message that genuinely arrived from the MIDI system. There
//! is deliberately **no** code path here that can construct an event from
//! anything but received bytes — no debug flag, no fallback for when no device is
//! attached, nothing. An empty window on a machine with no hardware is the
//! correct output.
//!
//! # Layering
//!
//! Dependencies point inward only:
//!
//! ```text
//! midi-macos (adapter) ──▶ application ──▶ domain
//!                                ▲
//!                                └── implemented by src-tauri (SettingsRepository)
//! ```
//!
//! - [`domain`] — MIDI concepts and the rules over them, including
//!   [`domain::decoder`], which turns received bytes into messages. Depends on
//!   nothing but the standard library.
//! - [`application`] — use cases ([`application::monitor::Monitor`]) and the
//!   ports they need from the outside world. [`application::ports::EventSource`]
//!   is the seam the platform adapter plugs into; this crate never names a
//!   platform itself.

pub mod application;
pub mod constants;
pub mod domain;
