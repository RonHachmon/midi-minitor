//! The macOS CoreMIDI adapter: real device access behind `midi-core`'s ports.
//!
//! # Why this is its own crate rather than code in `src-tauri`
//!
//! Three reasons, in order of weight.
//!
//! **Layering.** The constitution places device access in the infrastructure
//! layer and requires the Tauri layer to be a translation surface holding no
//! logic. Enumerating endpoints, opening ports, and decoding a byte stream is
//! logic. Putting it in `src-tauri` would merge two layers that the project
//! deliberately keeps apart.
//!
//! **Containment.** A macOS-only dependency lives behind one crate boundary, so
//! the rest of the workspace never sees a `cfg` for it. A future cross-platform
//! feature adds a sibling crate; it does not thread conditionals through files
//! that have nothing to do with platform detail.
//!
//! **Symmetry.** This crate occupies exactly the position the deleted simulator
//! did: one implementation of [`midi_core::application::ports::EventSource`],
//! selected at a single line in the Tauri shell. That the seam survived the
//! substitution is the part of the design that worked.
//!
//! # What does not live here
//!
//! No MIDI *meaning*. Byte-to-message interpretation belongs to
//! [`midi_core::domain::decoder`], which is pure and platform-free. This crate
//! moves bytes from CoreMIDI into that decoder and moves the results into the
//! application's sinks — nothing more. The division matters: a decoder that
//! could only be exercised through a CoreMIDI callback would be a decoder nobody
//! could reason about.

#![cfg(target_os = "macos")]

pub mod endpoints;
pub mod notifications;
pub mod source;

pub use source::CoreMidiSource;
