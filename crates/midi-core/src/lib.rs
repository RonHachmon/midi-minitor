//! Monitoring core for the mock MIDI Monitor.
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
//! # Layering
//!
//! Dependencies point inward only:
//!
//! ```text
//! simulator ──▶ application ──▶ domain
//!                    ▲
//!                    └── implemented by src-tauri (SettingsRepository)
//! ```
//!
//! - [`domain`] — MIDI concepts and the rules over them. Depends on nothing but
//!   the standard library.
//! - [`application`] — use cases ([`application::monitor::Monitor`]) and the
//!   ports they need from the outside world.
//! - [`simulator`] — the only module that knows the traffic is not real. Nothing
//!   above it can tell the difference, which is what makes swapping in real MIDI
//!   input a matter of providing a different [`application::ports::EventSource`].

pub mod application;
pub mod constants;
pub mod domain;
pub mod simulator;
