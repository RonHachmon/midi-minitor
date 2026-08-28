//! Platform-free helpers the adapters share.
//!
//! # Why an adapter helper lives in the core crate
//!
//! [`debounce::Debouncer`] began life in the macOS adapter, where it was the
//! only adapter and the question never arose. A second adapter needs it
//! *verbatim* — the pressure it answers is identical on both platforms — and the
//! three alternatives were each worse:
//!
//! - A fourth crate for one small type is the ceremony Principle V rejects.
//! - Duplicating it means two copies of a timing policy that must agree, with
//!   nothing forcing them to.
//! - Leaving it in `midi-macos` and depending on that crate from `midi-windows`
//!   would make every Windows build depend on the macOS adapter — worse than the
//!   placement problem it solves.
//!
//! Layering is unharmed: nothing here knows a platform, this crate gains no
//! dependency, and the adapters reach inward as they already do. What must
//! *never* appear in this module is a helper that names an operating system —
//! that is the line between a shared utility and a leaked platform detail.

pub mod debounce;
