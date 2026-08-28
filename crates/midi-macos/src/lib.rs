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
//! the rest of the workspace never sees a `cfg` for it. This prediction has since
//! been tested: `midi-windows` was added as a sibling crate, and the only
//! conditional in the whole application is the one that picks between them, in
//! `src-tauri/src/platform.rs`. What the seam did *not* absorb is recorded there
//! and in that crate — the port was incomplete, and a second implementation is
//! what exposed it.
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
pub mod source;

pub use source::CoreMidiSource;
