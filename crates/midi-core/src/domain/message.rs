//! The MIDI message model: what a message is, what it is called, and how it renders.
//!
//! # Why this model is hand-written rather than taken from a MIDI crate
//!
//! The constitution requires reusing a library over hand-rolling, and requires
//! any exception to be justified where the code lives. This is that
//! justification.
//!
//! Crates such as `midi-msg`, `wmidi`, and `midir` model *the MIDI protocol*:
//! `midir` is device access this application deliberately does not perform, and
//! the others optimise for parsing and encoding fidelity against the
//! specification. What this application needs is different in kind — it needs
//! the **Filter panel's vocabulary**. [`MessageKind`] has exactly one variant
//! per checkbox in `screenshots/filters.png`, which is what makes every filter
//! control mechanically mappable to a message rather than mapped by a hand-kept
//! table. It also needs an [`MidiMessage::Invalid`] case, which no correct MIDI
//! library would willingly produce, and per-variant rendering rules for the Data
//! column that a protocol crate has no reason to carry.
//!
//! Adapting a protocol crate would mean writing this same mapping anyway, on top
//! of a dependency. The domain enum *is* the domain here.
//!
//! [`super::decoder`] makes the same argument one level down, for turning a live
//! byte stream into these variants: every candidate decoder returns `Err` for the
//! malformed input this application is required to display as a row.
//!
//! # Exhaustiveness is deliberate
//!
//! Every `match` in this module lists its variants with no catch-all arm. Adding
//! a message type later is therefore a compile error at each site that must be
//! updated — the compiler enumerates the work, which is the project's stated
//! substitute for a test suite.

use super::ids::{ChannelNumber, Data14, DataByte};
use serde::{Deserialize, Serialize};

/// The parent groupings of the Filter panel's three columns.
///
/// `System Exclusive` and `Invalid` sit outside all three in the reference
/// image, which is why membership is [`Option`] rather than a fourth variant:
/// those two have no parent checkbox to drive them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MessageCategory {
    /// First column: channel-bearing performance messages.
    VoiceMessages,
    /// Second column: system messages that are not real-time.
    SystemCommon,
    /// Third column: the high-rate clock and transport messages.
    RealTime,
}

impl MessageCategory {
    /// Every category, in the left-to-right order of `screenshots/filters.png`.
    pub const ALL: [Self; 3] = [Self::VoiceMessages, Self::SystemCommon, Self::RealTime];

    /// The category heading, copied verbatim from the reference image.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::VoiceMessages => "Voice Messages",
            Self::SystemCommon => "System Common",
            Self::RealTime => "Real Time",
        }
    }
}

/// One filter checkbox — the unit the user actually toggles.
///
/// # Why this is separate from [`MidiMessage`]
///
/// They do not correspond one to one, and the screenshot is the reason. Note On
/// and Note Off are distinct messages that display distinct names in the Message
/// column, but the Filter panel offers a single `Note On/Off` checkbox; Start,
/// Stop, and Continue are three messages behind one `Start/Stop/Continue`
/// checkbox. Collapsing them here keeps that grouping in the domain, where the
/// screenshot's authority belongs, instead of letting the interface invent it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MessageKind {
    /// `Note On/Off` — covers both note messages.
    NoteOnOff,
    /// `Aftertouch (Poly)` — per-note pressure.
    AftertouchPoly,
    /// `Control` — control change.
    Control,
    /// `Program` — program change.
    Program,
    /// `Channel Pressure` — whole-channel pressure.
    ChannelPressure,
    /// `Pitch Wheel` — 14-bit bend.
    PitchWheel,
    /// `Time Code` — MIDI time code quarter frame.
    TimeCode,
    /// `Song Position Pointer`.
    SongPositionPointer,
    /// `Song Select`.
    SongSelect,
    /// `Tune Request`.
    TuneRequest,
    /// `Clock` — the 24-per-quarter-note timing pulse.
    Clock,
    /// `Start/Stop/Continue` — covers all three transport messages.
    StartStopContinue,
    /// `Active Sense` — the keep-alive heartbeat.
    ActiveSense,
    /// `Reset`.
    Reset,
    /// `System Exclusive` — standalone in the reference image.
    SystemExclusive,
    /// `Invalid` — malformed or unrecognised data; standalone.
    Invalid,
}

impl MessageKind {
    /// Every kind, in the order the reference image lists them.
    ///
    /// Column by column: the six Voice Messages entries, then the four System
    /// Common, then the four Real Time, then the two standalone checkboxes.
    pub const ALL: [Self; 16] = [
        Self::NoteOnOff,
        Self::AftertouchPoly,
        Self::Control,
        Self::Program,
        Self::ChannelPressure,
        Self::PitchWheel,
        Self::TimeCode,
        Self::SongPositionPointer,
        Self::SongSelect,
        Self::TuneRequest,
        Self::Clock,
        Self::StartStopContinue,
        Self::ActiveSense,
        Self::Reset,
        Self::SystemExclusive,
        Self::Invalid,
    ];

    /// The checkbox label, copied verbatim from `screenshots/filters.png`.
    ///
    /// These strings are normative under the project's design-authority rule.
    /// Rewording, sentence-casing, or "fixing" the punctuation of any of them is
    /// a fidelity violation, not an improvement.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::NoteOnOff => "Note On/Off",
            Self::AftertouchPoly => "Aftertouch (Poly)",
            Self::Control => "Control",
            Self::Program => "Program",
            Self::ChannelPressure => "Channel Pressure",
            Self::PitchWheel => "Pitch Wheel",
            Self::TimeCode => "Time Code",
            Self::SongPositionPointer => "Song Position Pointer",
            Self::SongSelect => "Song Select",
            Self::TuneRequest => "Tune Request",
            Self::Clock => "Clock",
            Self::StartStopContinue => "Start/Stop/Continue",
            Self::ActiveSense => "Active Sense",
            Self::Reset => "Reset",
            Self::SystemExclusive => "System Exclusive",
            Self::Invalid => "Invalid",
        }
    }

    /// The column this checkbox sits in, or [`None`] for the standalone entries.
    #[must_use]
    pub const fn category(self) -> Option<MessageCategory> {
        match self {
            Self::NoteOnOff
            | Self::AftertouchPoly
            | Self::Control
            | Self::Program
            | Self::ChannelPressure
            | Self::PitchWheel => Some(MessageCategory::VoiceMessages),
            Self::TimeCode | Self::SongPositionPointer | Self::SongSelect | Self::TuneRequest => {
                Some(MessageCategory::SystemCommon)
            }
            Self::Clock | Self::StartStopContinue | Self::ActiveSense | Self::Reset => {
                Some(MessageCategory::RealTime)
            }
            Self::SystemExclusive | Self::Invalid => None,
        }
    }

    /// A stable identifier for crossing the IPC boundary.
    ///
    /// Generated into TypeScript as a string-union member, so a typo in a filter
    /// identifier becomes a compile error in the webview rather than a filter
    /// that silently never matches.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::NoteOnOff => "noteOnOff",
            Self::AftertouchPoly => "aftertouchPoly",
            Self::Control => "control",
            Self::Program => "program",
            Self::ChannelPressure => "channelPressure",
            Self::PitchWheel => "pitchWheel",
            Self::TimeCode => "timeCode",
            Self::SongPositionPointer => "songPositionPointer",
            Self::SongSelect => "songSelect",
            Self::TuneRequest => "tuneRequest",
            Self::Clock => "clock",
            Self::StartStopContinue => "startStopContinue",
            Self::ActiveSense => "activeSense",
            Self::Reset => "reset",
            Self::SystemExclusive => "systemExclusive",
            Self::Invalid => "invalid",
        }
    }
}

/// Why a message was classified as invalid.
///
/// Produced by [`super::decoder`] when arriving bytes form no valid message.
/// Carrying the reason rather than just the bytes is what lets the Data column
/// say *how* the data was malformed, which is usually the fact the person
/// debugging the device actually needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InvalidReason {
    /// A status byte that no MIDI status corresponds to.
    UnknownStatus,
    /// A recognised status whose data bytes were cut short.
    TruncatedData,
    /// The operating system itself reported these bytes as not forming a
    /// message.
    ///
    /// # Why the system's verdict is its own reason
    ///
    /// Some platforms hand over malformed input already labelled as malformed,
    /// rather than as a byte stream this application judges for itself. Passing
    /// such bytes back through the decoder would let it disagree with the system
    /// — reporting a plausible-looking message where the system saw an error —
    /// which would be this application inventing an interpretation. The bytes
    /// shown are the ones the system supplied, and the reason records who said
    /// so.
    ReportedInvalid,
    /// The system reported invalid input but not how much of it was meaningful.
    ///
    /// Only the bytes whose extent could be established are shown. This exists
    /// so the row can say the remainder was not determinable instead of padding
    /// the display with bytes that may be nothing at all — which would be
    /// exactly the fabrication a monitor exists to prevent.
    ExtentUnknown,
}

impl InvalidReason {
    /// A short explanation for the Data column.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::UnknownStatus => "unknown status",
            Self::TruncatedData => "truncated",
            Self::ReportedInvalid => "reported invalid by the system",
            Self::ExtentUnknown => "reported invalid by the system, length not determinable",
        }
    }
}

/// The note names used when rendering a note number, sharps only.
///
/// The reference image shows `C2`; enharmonic spelling is not something MIDI
/// carries, so a single naming convention is the only honest choice.
const NOTE_NAMES: [&str; 12] = [
    "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
];

/// One MIDI message, modelled as the monitor needs to see it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MidiMessage {
    /// A note beginning, with its velocity.
    NoteOn {
        /// Channel the note was played on.
        channel: ChannelNumber,
        /// Note number, rendered as a name such as `C2`.
        note: DataByte,
        /// Attack velocity.
        velocity: DataByte,
    },
    /// A note ending.
    NoteOff {
        /// Channel the note was released on.
        channel: ChannelNumber,
        /// Note number.
        note: DataByte,
        /// Release velocity.
        velocity: DataByte,
    },
    /// Per-note pressure applied after the note began.
    AftertouchPoly {
        /// Channel carrying the pressure.
        channel: ChannelNumber,
        /// Note the pressure applies to.
        note: DataByte,
        /// Pressure amount.
        pressure: DataByte,
    },
    /// A control change.
    Control {
        /// Channel the controller belongs to.
        channel: ChannelNumber,
        /// Controller number.
        controller: DataByte,
        /// Controller value.
        value: DataByte,
    },
    /// A program change.
    Program {
        /// Channel whose program changed.
        channel: ChannelNumber,
        /// Program number.
        program: DataByte,
    },
    /// Pressure applied across a whole channel.
    ChannelPressure {
        /// Channel carrying the pressure.
        channel: ChannelNumber,
        /// Pressure amount.
        pressure: DataByte,
    },
    /// A pitch bend.
    PitchWheel {
        /// Channel being bent.
        channel: ChannelNumber,
        /// Bend amount, centred at 8192.
        value: Data14,
    },
    /// A MIDI time code quarter frame.
    TimeCode {
        /// The packed nibble payload.
        data: DataByte,
    },
    /// A song position pointer.
    SongPositionPointer {
        /// Position in sixteenth notes since the start.
        position: Data14,
    },
    /// A song selection.
    SongSelect {
        /// Selected song number.
        song: DataByte,
    },
    /// A tune request.
    TuneRequest,
    /// The timing clock pulse, sent 24 times per quarter note.
    Clock,
    /// Transport start.
    Start,
    /// Transport stop.
    Stop,
    /// Transport continue.
    Continue,
    /// The active sensing heartbeat.
    ActiveSense,
    /// A system reset.
    Reset,
    /// A system exclusive message.
    SystemExclusive {
        /// Payload between the `F0` and `F7` framing bytes.
        payload: Vec<u8>,
    },
    /// Data that could not be interpreted as a valid message.
    Invalid {
        /// Why interpretation failed.
        reason: InvalidReason,
        /// The bytes as received, so the raw view can still show them.
        bytes: Vec<u8>,
    },
}

impl MidiMessage {
    /// Which filter checkbox governs this message.
    ///
    /// Exhaustive with no catch-all arm: a new message variant must be assigned
    /// a kind, and the compiler will insist.
    #[must_use]
    pub const fn kind(&self) -> MessageKind {
        match self {
            Self::NoteOn { .. } | Self::NoteOff { .. } => MessageKind::NoteOnOff,
            Self::AftertouchPoly { .. } => MessageKind::AftertouchPoly,
            Self::Control { .. } => MessageKind::Control,
            Self::Program { .. } => MessageKind::Program,
            Self::ChannelPressure { .. } => MessageKind::ChannelPressure,
            Self::PitchWheel { .. } => MessageKind::PitchWheel,
            Self::TimeCode { .. } => MessageKind::TimeCode,
            Self::SongPositionPointer { .. } => MessageKind::SongPositionPointer,
            Self::SongSelect { .. } => MessageKind::SongSelect,
            Self::TuneRequest => MessageKind::TuneRequest,
            Self::Clock => MessageKind::Clock,
            Self::Start | Self::Stop | Self::Continue => MessageKind::StartStopContinue,
            Self::ActiveSense => MessageKind::ActiveSense,
            Self::Reset => MessageKind::Reset,
            Self::SystemExclusive { .. } => MessageKind::SystemExclusive,
            Self::Invalid { .. } => MessageKind::Invalid,
        }
    }

    /// The channel this message carries, if it carries one.
    ///
    /// # Why this returns an [`Option`]
    ///
    /// Two requirements depend on the distinction and both are easy to forget:
    /// the Chan cell must be *blank* rather than zero for system messages, and
    /// selecting One Channel must exclude channel-less messages entirely.
    /// Returning `Option` makes both unavoidable at every call site instead of
    /// relying on a sentinel value nobody remembers to check.
    #[must_use]
    pub const fn channel(&self) -> Option<ChannelNumber> {
        match self {
            Self::NoteOn { channel, .. }
            | Self::NoteOff { channel, .. }
            | Self::AftertouchPoly { channel, .. }
            | Self::Control { channel, .. }
            | Self::Program { channel, .. }
            | Self::ChannelPressure { channel, .. }
            | Self::PitchWheel { channel, .. } => Some(*channel),
            Self::TimeCode { .. }
            | Self::SongPositionPointer { .. }
            | Self::SongSelect { .. }
            | Self::TuneRequest
            | Self::Clock
            | Self::Start
            | Self::Stop
            | Self::Continue
            | Self::ActiveSense
            | Self::Reset
            | Self::SystemExclusive { .. }
            | Self::Invalid { .. } => None,
        }
    }

    /// The name shown in the Message column.
    ///
    /// Per-variant rather than per-kind, because the reference window shows
    /// `Note On` and `Note Off` as separate rows even though one checkbox
    /// controls both.
    #[must_use]
    pub const fn display_name(&self) -> &'static str {
        match self {
            Self::NoteOn { .. } => "Note On",
            Self::NoteOff { .. } => "Note Off",
            Self::AftertouchPoly { .. } => "Aftertouch (Poly)",
            Self::Control { .. } => "Control",
            Self::Program { .. } => "Program",
            Self::ChannelPressure { .. } => "Channel Pressure",
            Self::PitchWheel { .. } => "Pitch Wheel",
            Self::TimeCode { .. } => "Time Code",
            Self::SongPositionPointer { .. } => "Song Position Pointer",
            Self::SongSelect { .. } => "Song Select",
            Self::TuneRequest => "Tune Request",
            Self::Clock => "Clock",
            Self::Start => "Start",
            Self::Stop => "Stop",
            Self::Continue => "Continue",
            Self::ActiveSense => "Active Sense",
            Self::Reset => "Reset",
            Self::SystemExclusive { .. } => "System Exclusive",
            Self::Invalid { .. } => "Invalid",
        }
    }

    /// The Data column's contents for this message.
    ///
    /// # Why rendering lives in the domain
    ///
    /// What belongs in the Data cell is a fact about MIDI, not a presentation
    /// choice: a note shows its name and velocity, Channel Pressure shows a bare
    /// amount, System Exclusive shows a byte count. Rendering it here keeps one
    /// implementation; rendering it in the webview would put a MIDI rule on the
    /// far side of a serialization boundary and duplicate it.
    #[must_use]
    pub fn data_display(&self) -> String {
        match self {
            Self::NoteOn { note, velocity, .. } | Self::NoteOff { note, velocity, .. } => {
                format!("{} {}", note_name(*note), velocity.get())
            }
            Self::AftertouchPoly { note, pressure, .. } => {
                format!("{} {}", note_name(*note), pressure.get())
            }
            Self::Control {
                controller, value, ..
            } => format!("{} {}", controller.get(), value.get()),
            Self::Program { program, .. } => program.get().to_string(),
            Self::ChannelPressure { pressure, .. } => pressure.get().to_string(),
            Self::PitchWheel { value, .. } => value.get().to_string(),
            Self::TimeCode { data } => data.get().to_string(),
            Self::SongPositionPointer { position } => position.get().to_string(),
            Self::SongSelect { song } => song.get().to_string(),
            Self::TuneRequest
            | Self::Clock
            | Self::Start
            | Self::Stop
            | Self::Continue
            | Self::ActiveSense
            | Self::Reset => String::new(),
            Self::SystemExclusive { payload } => {
                // Framing bytes count toward what the user thinks of as the message.
                let total = payload.len() + 2;
                format!("{total} bytes")
            }
            Self::Invalid { reason, bytes } => {
                format!("{} ({} bytes)", reason.label(), bytes.len())
            }
        }
    }
}

/// Renders a note number as a name and octave, such as `C2`.
///
/// Octave numbering places middle C (note 60) at `C4`, which is the scientific
/// convention and the one that makes note 36 read as `C2` — matching the
/// reference window.
fn note_name(note: DataByte) -> String {
    let value = note.get();
    let name = NOTE_NAMES[usize::from(value % 12)];
    let octave = i16::from(value / 12) - 1;
    format!("{name}{octave}")
}
