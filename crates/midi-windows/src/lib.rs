//! The Windows WinMM adapter: real device access behind `midi-core`'s ports.
//!
//! # Why this is its own crate rather than code in `src-tauri`
//!
//! The same three reasons `midi-macos` gives, and this crate is the proof the
//! third one was real: **layering** (device access is infrastructure, and the
//! Tauri shell holds no logic), **containment** (a Windows-only dependency lives
//! behind one crate boundary), and **symmetry** — this crate occupies exactly the
//! position `midi-macos` occupies, one implementation of
//! [`midi_core::application::ports::EventSource`], selected at a single line in
//! [`platform`](../../../src-tauri/src/platform.rs). Nothing above the port knows
//! which of the two it is talking to.
//!
//! This crate mirrors `midi-macos` module for module so that a reader who knows
//! one knows the other. Two modules have no CoreMIDI counterpart — [`receive`]
//! and [`sysex`] — and both exist because of platform constraints named below.
//!
//! # Why classic WinMM rather than the newer Windows MIDI stack
//!
//! Because WinMM is the only Windows path that hands over **malformed** bytes,
//! and a monitor that cannot show malformed input is not a monitor.
//!
//! - **Windows MIDI Services** (the MIDI 2.0 stack) translates everything to
//!   Universal MIDI Packets inside the service. Recovering the original byte
//!   stream would mean translating back — a reconstruction, which is the
//!   fabrication `midi-macos` already rejected when it declined CoreMIDI's MIDI
//!   2.0 receive path. It is also absent from part of the supported Windows floor.
//! - **WinRT `Windows.Devices.Midi`** delivers already-parsed message objects and
//!   surfaces no equivalent of `MIM_ERROR`, so the `Invalid` category would go
//!   dark.
//! - **`midir`** discards the data on `MM_MIM_LONGERROR`, ignores `MM_MIM_ERROR`,
//!   and drops any short message whose status bit is clear.
//!
//! WinMM's cost is stated honestly rather than hidden: Windows expands running
//! status before any application sees it, so for short messages the byte *count*
//! shown is the message Windows delivered rather than the count the cable
//! carried. Nothing is invented and nothing is dropped — only the wire's
//! compression is undone before we can see it. That fact reaches the interface
//! through [`midi_core::application::ports::ByteFidelity`], and the `Data` column
//! states it. System Exclusive arrives as a real byte buffer and is exact.
//!
//! See `specs/003-windows-support/research.md`, decisions D1 and D6.
//!
//! # Why the receive callback hands off instead of decoding in place
//!
//! This is the one structural difference from `midi-macos`, and it is imposed
//! rather than chosen. `MidiInProc` may not call any multimedia function —
//! doing so can deadlock — and returning a System Exclusive buffer to the driver
//! requires `midiInAddBuffer`, which is one. So the callback timestamps the
//! arrival and pushes the bytes onto a channel, and a worker thread owned by the
//! adapter does the decoding, the sink calls, and the buffer re-queueing.
//!
//! The macOS adapter decodes inside its CoreMIDI callback because CoreMIDI
//! imposes no such rule. Neither shape is better; each is what its platform
//! permits. See research D5, and [`receive`] for the mechanics.
//!
//! # What does not live here
//!
//! No MIDI *meaning*. Byte-to-message interpretation belongs to
//! [`midi_core::domain::decoder`], which is pure and platform-free, and this
//! crate feeds it the bytes Windows delivered without inspecting them. The
//! decoder's running-status branch is therefore unreachable on this platform. It
//! is deliberately left in place: it is correct, it is exercised on macOS, and
//! deleting a correct implementation to match one platform's limitation would be
//! the wrong repair.

#![cfg(target_os = "windows")]

pub mod endpoints;
pub mod notifications;
pub mod receive;
pub mod source;
pub mod sysex;

pub use source::WindowsMidiSource;
