//! The conventional names of the Control Change controllers.
//!
//! # Why this table is written here rather than taken from a crate
//!
//! The constitution requires reusing a library over hand-rolling and requires
//! the exception to be justified where the code lives. This is that
//! justification.
//!
//! No crate in this workspace's dependency tree publishes these as display
//! strings, and none should be added for them. `midi-macos` and `midi-windows`
//! each record why every general MIDI crate was rejected — `midir`, `wmidi`,
//! `midi-msg`, and `midly` all refuse or discard the malformed input a monitor
//! exists to show — and none of them is in `Cargo.lock` today.
//!
//! Adopting one would not answer the question anyway. `wmidi` models controllers
//! as associated constants whose names are Rust identifiers, not text;
//! `midi-msg` models them as enum variants for round-tripping. Either way a
//! number-to-string table gets written by hand, so the crate would add a
//! dependency tree without removing the work. Controller names are also the
//! domain's own vocabulary, which is Principle II's other stated exception.
//!
//! # Why every number still renders
//!
//! Most of the 128 controller numbers have no assigned name, and a cell that
//! went blank for them would lose information the user came here for. [`name`]
//! returns [`None`] for those and the renderer shows the number, so the name is
//! always an addition to the number and never a replacement that hides it.

/// The conventional name for each controller number, or an empty string where
/// the specification assigns none.
///
/// Indexed by controller number, so position is meaning: entry `n` names
/// controller `n`. The undefined entries are held as empty strings rather than
/// as a sparse map because a fixed 128-entry array makes "is every number
/// covered?" a property of the type rather than a question about the data.
const NAMES: [&str; 128] = [
    "Bank Select (MSB)",
    "Modulation (MSB)",
    "Breath Controller (MSB)",
    "",
    "Foot Controller (MSB)",
    "Portamento Time (MSB)",
    "Data Entry (MSB)",
    "Volume (MSB)",
    "Balance (MSB)",
    "",
    "Pan (MSB)",
    "Expression (MSB)",
    "Effect Control 1 (MSB)",
    "Effect Control 2 (MSB)",
    "",
    "",
    "General Purpose 1 (MSB)",
    "General Purpose 2 (MSB)",
    "General Purpose 3 (MSB)",
    "General Purpose 4 (MSB)",
    "",
    "",
    "",
    "",
    "",
    "",
    "",
    "",
    "",
    "",
    "",
    "",
    "Bank Select (LSB)",
    "Modulation (LSB)",
    "Breath Controller (LSB)",
    "",
    "Foot Controller (LSB)",
    "Portamento Time (LSB)",
    "Data Entry (LSB)",
    "Volume (LSB)",
    "Balance (LSB)",
    "",
    "Pan (LSB)",
    "Expression (LSB)",
    "Effect Control 1 (LSB)",
    "Effect Control 2 (LSB)",
    "",
    "",
    "General Purpose 1 (LSB)",
    "General Purpose 2 (LSB)",
    "General Purpose 3 (LSB)",
    "General Purpose 4 (LSB)",
    "",
    "",
    "",
    "",
    "",
    "",
    "",
    "",
    "",
    "",
    "",
    "",
    "Sustain",
    "Portamento",
    "Sostenuto",
    "Soft Pedal",
    "Legato Footswitch",
    "Hold 2",
    "Sound Controller 1",
    "Sound Controller 2",
    "Sound Controller 3",
    "Sound Controller 4",
    "Sound Controller 5",
    "Sound Controller 6",
    "Sound Controller 7",
    "Sound Controller 8",
    "Sound Controller 9",
    "Sound Controller 10",
    "General Purpose 5",
    "General Purpose 6",
    "General Purpose 7",
    "General Purpose 8",
    "Portamento Control",
    "",
    "",
    "",
    "High Resolution Velocity Prefix",
    "",
    "",
    "Effects 1 Depth",
    "Effects 2 Depth",
    "Effects 3 Depth",
    "Effects 4 Depth",
    "Effects 5 Depth",
    "Data Increment",
    "Data Decrement",
    "NRPN (LSB)",
    "NRPN (MSB)",
    "RPN (LSB)",
    "RPN (MSB)",
    "",
    "",
    "",
    "",
    "",
    "",
    "",
    "",
    "",
    "",
    "",
    "",
    "",
    "",
    "",
    "",
    "",
    "",
    "All Sound Off",
    "Reset All Controllers",
    "Local Control",
    "All Notes Off",
    "Omni Mode Off",
    "Omni Mode On",
    "Mono Mode On",
    "Poly Mode On",
];

/// The conventional name for a controller number, or [`None`] where none is
/// assigned.
///
/// The caller must render the number when this is [`None`]; see the module
/// documentation for why a blank cell is not an acceptable outcome.
#[must_use]
pub fn name(controller: u8) -> Option<&'static str> {
    NAMES
        .get(usize::from(controller))
        .copied()
        .filter(|entry| !entry.is_empty())
}
