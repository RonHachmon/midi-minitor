//! What can be composed, and which values each composable message carries.
//!
//! # Why this is a third message-shaped type, and not duplication
//!
//! The crate now holds three enums that all look like "a kind of MIDI message".
//! They answer three different questions and change for three different reasons,
//! which is exactly the test single responsibility applies:
//!
//! | Type | Question | Changes when |
//! |---|---|---|
//! | [`MidiMessage`] | What did we observe, with what payload? | The set of observable messages changes |
//! | [`super::message::MessageKind`] | Which filter checkbox does this fall under? | `screenshots/filters.png` changes |
//! | [`SendableKind`] | What can I compose, with which values? | The set of *sendable* messages changes |
//!
//! [`super::message::MessageKind`] is the wrong granularity here, and its own
//! documentation says why: it collapses Note On and Note Off behind one checkbox
//! and Start, Stop, and Continue behind another, because that is what the
//! reference image shows. A composer needs them apart — you send a Note On, not a
//! "Note On/Off".
//!
//! The alternative that looks cleanest — a payload-carrying `SendableMessage`, so
//! that [`MidiMessage::Invalid`] becomes structurally unrepresentable — was
//! rejected because it would duplicate all eighteen payload-carrying variants,
//! and two payload enums for one concept drift silently. A payload-free
//! discriminant cannot drift the same way: it is a list of names, every `match`
//! over it is exhaustive with no catch-all, and adding a message type breaks the
//! build in both files at once.
//!
//! # Why the field table lives here
//!
//! "Which values does a Control Change carry?" is the MIDI specification, not a
//! layout preference. Keeping it here means the send screen renders the fields it
//! is handed and holds no table of its own — the same arrangement that lets the
//! Filter panel render its checkboxes without knowing what a message is.

use super::ids::{ChannelNumber, Data14, DataByte};
use super::message::MidiMessage;
use crate::constants::{CHANNEL_RANGE, IDENTITY_REQUEST, MAX_DATA_14, MAX_DATA_BYTE};

/// Default note for the note-bearing messages: middle C.
const DEFAULT_NOTE: u8 = 60;
/// Default velocity: firm, but short of maximum, so a synth responds audibly.
const DEFAULT_VELOCITY: u8 = 100;
/// Default pressure and controller value: the midpoint of the seven-bit range.
const DEFAULT_MIDPOINT: u8 = 64;
/// Default controller: channel volume, the controller most receivers implement.
const DEFAULT_CONTROLLER: u8 = 7;
/// Centre of the pitch wheel's fourteen-bit range — no bend applied.
const PITCH_WHEEL_CENTRE: u16 = 8_192;

/// One editable value on a composed message.
///
/// Carries its own label and its own accepted range, so the interface never has
/// to know which message it is rendering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldSpec {
    /// Which value this is, for the command that sets it.
    pub id: FieldId,
    /// What the value means, in the user's words rather than the wire's.
    pub label: &'static str,
    /// What kind of entry it accepts, and within what bounds.
    pub kind: FieldKind,
}

/// What a field accepts.
///
/// # Why an enum rather than a min and a max on every field
///
/// A System Exclusive payload is a byte string with no numeric bounds, so
/// `min` and `max` would be meaningless on it and some sentinel would have to
/// stand in for "not applicable". A variant makes that state unrepresentable
/// instead, and tells the interface which control to render without it having to
/// guess from the field's name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldKind {
    /// A bounded number — a seven-bit value, a fourteen-bit value, or a channel.
    Number {
        /// Lowest accepted value, inclusive.
        min: u16,
        /// Highest accepted value, inclusive.
        max: u16,
    },
    /// A System Exclusive payload, entered as hexadecimal bytes.
    ///
    /// Excludes the framing bytes, which the encoder restores: the user edits the
    /// content, not the envelope.
    Bytes,
}

/// Which value on a composed message a field refers to.
///
/// Distinct identifiers wherever the *meaning* differs, even where the wire
/// representation does not: a controller number and a note number are both seven
/// bits, and confusing them is exactly the mistake this type exists to prevent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FieldId {
    /// The channel a channel-bearing message is addressed to.
    Channel,
    /// The note number.
    Note,
    /// The note's velocity.
    Velocity,
    /// The controller number of a Control Change.
    Controller,
    /// The value carried by a Control Change.
    ControlValue,
    /// The program number of a Program Change.
    Program,
    /// Pressure, whether per-note or whole-channel.
    Pressure,
    /// The pitch wheel's fourteen-bit position.
    Bend,
    /// The quarter-frame byte of a Time Code message.
    TimeCodeData,
    /// The fourteen-bit position of a Song Position Pointer.
    SongPosition,
    /// The song number of a Song Select.
    Song,
    /// A System Exclusive payload.
    Payload,
}

impl FieldId {
    /// Every field identifier, for resolving one that arrived from the interface.
    const ALL: [Self; 12] = [
        Self::Channel,
        Self::Note,
        Self::Velocity,
        Self::Controller,
        Self::ControlValue,
        Self::Program,
        Self::Pressure,
        Self::Bend,
        Self::TimeCodeData,
        Self::SongPosition,
        Self::Song,
        Self::Payload,
    ];

    /// A stable identifier for crossing the IPC boundary.
    ///
    /// Generated into TypeScript, so a typo in a field identifier becomes a
    /// compile error in the webview rather than a control that silently does
    /// nothing.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Channel => "channel",
            Self::Note => "note",
            Self::Velocity => "velocity",
            Self::Controller => "controller",
            Self::ControlValue => "value",
            Self::Program => "program",
            Self::Pressure => "pressure",
            Self::Bend => "bend",
            Self::TimeCodeData => "timeCodeData",
            Self::SongPosition => "songPosition",
            Self::Song => "song",
            Self::Payload => "payload",
        }
    }

    /// Resolves an identifier that arrived from the interface.
    #[must_use]
    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|candidate| candidate.id() == id)
    }
}

/// A message that can be composed and sent.
///
/// One variant per composable message — every [`MidiMessage`] except
/// [`MidiMessage::Invalid`], which a monitor observes and a sender never emits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SendableKind {
    /// Note On.
    NoteOn,
    /// Note Off.
    NoteOff,
    /// Polyphonic aftertouch.
    AftertouchPoly,
    /// Control Change.
    Control,
    /// Program Change.
    Program,
    /// Channel Pressure.
    ChannelPressure,
    /// Pitch Wheel.
    PitchWheel,
    /// MIDI Time Code quarter frame.
    TimeCode,
    /// Song Position Pointer.
    SongPositionPointer,
    /// Song Select.
    SongSelect,
    /// Tune Request.
    TuneRequest,
    /// Timing Clock.
    Clock,
    /// Start.
    Start,
    /// Stop.
    Stop,
    /// Continue.
    Continue,
    /// Active Sensing.
    ActiveSense,
    /// System Reset.
    Reset,
    /// System Exclusive.
    SystemExclusive,
}

impl SendableKind {
    /// Every composable message, in the order the picker lists them.
    ///
    /// Channel-bearing messages first, then system common, then real time, then
    /// System Exclusive — the same grouping order the Filter panel uses, so a
    /// user who knows one screen can predict the other.
    pub const ALL: [Self; 18] = [
        Self::NoteOn,
        Self::NoteOff,
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
        Self::Start,
        Self::Stop,
        Self::Continue,
        Self::ActiveSense,
        Self::Reset,
        Self::SystemExclusive,
    ];

    /// The name shown in the message-type picker.
    ///
    /// Where the reference image already names a message, that spelling is used
    /// verbatim — `Aftertouch (Poly)`, `Active Sense`, `Reset` — so the two
    /// screens never call one thing by two names.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::NoteOn => "Note On",
            Self::NoteOff => "Note Off",
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
            Self::Start => "Start",
            Self::Stop => "Stop",
            Self::Continue => "Continue",
            Self::ActiveSense => "Active Sense",
            Self::Reset => "Reset",
            Self::SystemExclusive => "System Exclusive",
        }
    }

    /// A stable identifier for crossing the IPC boundary.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::NoteOn => "noteOn",
            Self::NoteOff => "noteOff",
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
            Self::Start => "start",
            Self::Stop => "stop",
            Self::Continue => "continue",
            Self::ActiveSense => "activeSense",
            Self::Reset => "reset",
            Self::SystemExclusive => "systemExclusive",
        }
    }

    /// Resolves an identifier that arrived from the interface.
    #[must_use]
    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|candidate| candidate.id() == id)
    }

    /// Which kind an already-composed message is, so the picker can report its
    /// current selection.
    ///
    /// [`None`] for [`MidiMessage::Invalid`], which is not composable.
    #[must_use]
    pub const fn of(message: &MidiMessage) -> Option<Self> {
        let kind = match message {
            MidiMessage::NoteOn { .. } => Self::NoteOn,
            MidiMessage::NoteOff { .. } => Self::NoteOff,
            MidiMessage::AftertouchPoly { .. } => Self::AftertouchPoly,
            MidiMessage::Control { .. } => Self::Control,
            MidiMessage::Program { .. } => Self::Program,
            MidiMessage::ChannelPressure { .. } => Self::ChannelPressure,
            MidiMessage::PitchWheel { .. } => Self::PitchWheel,
            MidiMessage::TimeCode { .. } => Self::TimeCode,
            MidiMessage::SongPositionPointer { .. } => Self::SongPositionPointer,
            MidiMessage::SongSelect { .. } => Self::SongSelect,
            MidiMessage::TuneRequest => Self::TuneRequest,
            MidiMessage::Clock => Self::Clock,
            MidiMessage::Start => Self::Start,
            MidiMessage::Stop => Self::Stop,
            MidiMessage::Continue => Self::Continue,
            MidiMessage::ActiveSense => Self::ActiveSense,
            MidiMessage::Reset => Self::Reset,
            MidiMessage::SystemExclusive { .. } => Self::SystemExclusive,
            MidiMessage::Invalid { .. } => return None,
        };
        Some(kind)
    }

    /// The values this message carries, in the order they are shown.
    ///
    /// # Why this allocates rather than returning a `&'static [FieldSpec]`
    ///
    /// The bounds come from [`CHANNEL_RANGE`], [`MAX_DATA_BYTE`], and
    /// [`MAX_DATA_14`] — the constants that already define them — and a `static`
    /// cannot read those without re-typing their values here as literals. One
    /// small allocation on a user interaction is a better trade than a second
    /// copy of every range that could drift from the first.
    #[must_use]
    pub fn fields(self) -> Vec<FieldSpec> {
        match self {
            Self::NoteOn | Self::NoteOff => vec![
                channel_field(),
                seven_bit(FieldId::Note, "Note"),
                seven_bit(FieldId::Velocity, "Velocity"),
            ],
            Self::AftertouchPoly => vec![
                channel_field(),
                seven_bit(FieldId::Note, "Note"),
                seven_bit(FieldId::Pressure, "Pressure"),
            ],
            Self::Control => vec![
                channel_field(),
                seven_bit(FieldId::Controller, "Controller"),
                seven_bit(FieldId::ControlValue, "Value"),
            ],
            Self::Program => vec![channel_field(), seven_bit(FieldId::Program, "Program")],
            Self::ChannelPressure => {
                vec![channel_field(), seven_bit(FieldId::Pressure, "Pressure")]
            }
            Self::PitchWheel => vec![channel_field(), fourteen_bit(FieldId::Bend, "Bend")],
            Self::TimeCode => vec![seven_bit(FieldId::TimeCodeData, "Data")],
            Self::SongPositionPointer => vec![fourteen_bit(FieldId::SongPosition, "Position")],
            Self::SongSelect => vec![seven_bit(FieldId::Song, "Song")],
            Self::SystemExclusive => vec![FieldSpec {
                id: FieldId::Payload,
                label: "Payload",
                kind: FieldKind::Bytes,
            }],
            // These carry nothing at all: the status byte is the whole message.
            Self::TuneRequest
            | Self::Clock
            | Self::Start
            | Self::Stop
            | Self::Continue
            | Self::ActiveSense
            | Self::Reset => Vec::new(),
        }
    }

    /// The message this kind starts as, before the user adjusts anything.
    ///
    /// Defaults are chosen to *do something audible* on a receiving instrument
    /// wherever that is possible — middle C at a firm velocity, channel volume
    /// for a Control Change — because the first thing anyone does with this
    /// screen is find out whether the other end is listening at all.
    #[must_use]
    pub fn default_message(self) -> MidiMessage {
        // Channel 1, built from the wire nibble because that path cannot fail and
        // so needs no error branch that could never be taken.
        let channel = ChannelNumber::from_wire_nibble(0);
        match self {
            Self::NoteOn => MidiMessage::NoteOn {
                channel,
                note: DataByte::from_masked(DEFAULT_NOTE),
                velocity: DataByte::from_masked(DEFAULT_VELOCITY),
            },
            Self::NoteOff => MidiMessage::NoteOff {
                channel,
                note: DataByte::from_masked(DEFAULT_NOTE),
                velocity: DataByte::ZERO,
            },
            Self::AftertouchPoly => MidiMessage::AftertouchPoly {
                channel,
                note: DataByte::from_masked(DEFAULT_NOTE),
                pressure: DataByte::from_masked(DEFAULT_MIDPOINT),
            },
            Self::Control => MidiMessage::Control {
                channel,
                controller: DataByte::from_masked(DEFAULT_CONTROLLER),
                value: DataByte::from_masked(DEFAULT_VELOCITY),
            },
            Self::Program => MidiMessage::Program {
                channel,
                program: DataByte::ZERO,
            },
            Self::ChannelPressure => MidiMessage::ChannelPressure {
                channel,
                pressure: DataByte::from_masked(DEFAULT_MIDPOINT),
            },
            Self::PitchWheel => MidiMessage::PitchWheel {
                channel,
                value: Data14::from_masked(PITCH_WHEEL_CENTRE),
            },
            Self::TimeCode => MidiMessage::TimeCode {
                data: DataByte::ZERO,
            },
            Self::SongPositionPointer => MidiMessage::SongPositionPointer {
                position: Data14::from_masked(0),
            },
            Self::SongSelect => MidiMessage::SongSelect {
                song: DataByte::ZERO,
            },
            Self::TuneRequest => MidiMessage::TuneRequest,
            Self::Clock => MidiMessage::Clock,
            Self::Start => MidiMessage::Start,
            Self::Stop => MidiMessage::Stop,
            Self::Continue => MidiMessage::Continue,
            Self::ActiveSense => MidiMessage::ActiveSense,
            Self::Reset => MidiMessage::Reset,
            // The identity request without its framing, which the encoder
            // restores. It is the one System Exclusive worth defaulting to,
            // because it is the one that reliably provokes a reply.
            Self::SystemExclusive => MidiMessage::SystemExclusive {
                payload: IDENTITY_REQUEST[1..IDENTITY_REQUEST.len() - 1].to_vec(),
            },
        }
    }
}

/// The channel field, bounded by the range the rest of the application uses.
fn channel_field() -> FieldSpec {
    FieldSpec {
        id: FieldId::Channel,
        label: "Channel",
        kind: FieldKind::Number {
            min: u16::from(*CHANNEL_RANGE.start()),
            max: u16::from(*CHANNEL_RANGE.end()),
        },
    }
}

/// A seven-bit field: any of the ordinary data values.
fn seven_bit(id: FieldId, label: &'static str) -> FieldSpec {
    FieldSpec {
        id,
        label,
        kind: FieldKind::Number {
            min: 0,
            max: u16::from(MAX_DATA_BYTE),
        },
    }
}

/// A fourteen-bit field: pitch wheel position, or song position.
fn fourteen_bit(id: FieldId, label: &'static str) -> FieldSpec {
    FieldSpec {
        id,
        label,
        kind: FieldKind::Number {
            min: 0,
            max: MAX_DATA_14,
        },
    }
}
