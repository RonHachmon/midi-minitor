//! The message being built on the send screen.
//!
//! # Why there are two variants rather than one struct
//!
//! A composition holds both an interpretation and a byte string, and **which of
//! the two is authoritative differs** depending on where it came from. A single
//! struct carrying both would leave that unstated, and the difference is not
//! cosmetic.
//!
//! Type `90 3C 00` by hand — a Note On with velocity zero. The decoder correctly
//! reports it as a note *release*, because that is the convention. Re-encoding
//! that interpretation would produce `80 3C 00`, a status byte the user never
//! typed. So for a hand-typed composition the **bytes** are the authority and the
//! message is only for display.
//!
//! For a guided composition the reverse holds: the user is editing named values,
//! the message is what they are editing, and the bytes are derived from it on
//! every change.
//!
//! This is the same asymmetry [`super::event::MidiEvent`] already documents for
//! *received* data — interpretation is lossy, so the bytes that arrived are kept
//! and are the authority for the raw view. Here the direction is reversed and the
//! reasoning is identical.
//!
//! # Why hand-typed bytes are validated by the decoder
//!
//! Deciding whether a byte string is a valid MIDI message is a job this crate
//! already does, in [`super::decoder`], written for this project and already
//! handling running status and unterminated System Exclusive. A second validator
//! would disagree with it at the edges — which for a *monitor* would mean the
//! send screen refusing to transmit something the event table displays happily.
//!
//! Note the deliberate asymmetry in what the two do with the answer: the monitor
//! **shows** invalid data it receives, because that is the point of a monitor,
//! and the send screen **refuses** to transmit it, because emitting malformed
//! bytes on purpose is not the point of a sender.
//!
//! # The two catch-all arms in this module, and why they are the exception
//!
//! Everywhere else in this crate a `match` lists its variants with no wildcard,
//! so that adding a message type is a compile error at every site that must be
//! updated. [`read_field`] and [`write_number`] break that rule, and it is worth
//! being plain about the cost rather than leaving it to be discovered.
//!
//! They match on a **tuple** of message and field — a cartesian product of
//! nineteen by twelve, of which twenty-odd pairings are meaningful and the rest
//! are nonsense like "the velocity of a Song Select". Enumerating the nonsense
//! would be two hundred arms that say nothing, and the noise would hide the
//! twenty that matter.
//!
//! What stands in for the compiler here is [`super::sendable::SendableKind::fields`]:
//! it is the single definition of which values a message carries, and
//! [`Composition::set_field`] refuses any field not in that list *before* either
//! function is reached. So a new message type does still force a decision — in
//! `fields`, where the compiler's exhaustiveness check does apply — and the
//! consequence of forgetting to extend these two is a field that reads and writes
//! as absent, not a silently wrong byte.

use super::decoder::{Decoded, MessageDecoder};
use super::encoder::encode;
use super::ids::{ChannelNumber, Data14, DataByte};
use super::message::MidiMessage;
use super::sendable::{FieldId, FieldKind, FieldSpec, SendableKind};
use crate::application::error::CoreError;
use crate::constants::{CHANNEL_RANGE, MAX_DATA_BYTE};
use serde::{Deserialize, Serialize};

/// The message currently being built, and where its authority lies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Composition {
    /// Built from named controls. The message is the authority; bytes are derived.
    Guided {
        /// What the user is editing.
        message: MidiMessage,
    },
    /// Typed as bytes and validated by the decoder. The bytes are the authority.
    Raw {
        /// Exactly what the user typed, normalised only in spacing and case.
        bytes: Vec<u8>,
        /// The decoder's reading of those bytes, for display alone.
        message: MidiMessage,
    },
}

impl Composition {
    /// Starts a composition from a message type, at its defaults.
    #[must_use]
    pub fn from_kind(kind: SendableKind) -> Self {
        Self::Guided {
            message: kind.default_message(),
        }
    }

    /// Wraps an already-built message as a guided composition.
    ///
    /// Used when a request is loaded from the library, whose stored form is a
    /// message rather than a kind.
    #[must_use]
    pub const fn from_message(message: MidiMessage) -> Self {
        Self::Guided { message }
    }

    /// Parses hand-typed bytes, accepting only one complete, valid message.
    ///
    /// # Errors
    ///
    /// [`CoreError::MalformedSendBytes`] when the entry contains a character that
    /// is not a hexadecimal digit, an odd number of digits, no bytes at all, more
    /// than one message, or anything the decoder does not read as a complete
    /// message. `detail` carries the decoder's own reason wherever there is one,
    /// so the user is told what is actually wrong rather than merely that
    /// something is.
    pub fn parse_raw(entry: &str) -> Result<Self, CoreError> {
        Self::from_raw_bytes(&parse_hex_bytes(entry)?)
    }

    /// Validates already-parsed bytes as one complete message.
    ///
    /// Split from [`Self::parse_raw`] so a saved request can be rebuilt from the
    /// bytes it was stored with, without the round trip through hexadecimal text
    /// that would only give the same answer.
    ///
    /// # Errors
    ///
    /// [`CoreError::MalformedSendBytes`], as [`Self::parse_raw`].
    pub fn from_raw_bytes(bytes: &[u8]) -> Result<Self, CoreError> {
        let mut decoder = MessageDecoder::new();
        let mut decoded = decoder.feed(bytes);
        // A transfer still open at the end of the entry is incomplete, and the
        // decoder only says so when asked to give up on it.
        if let Some(abandoned) = decoder.abandon() {
            decoded.push(abandoned);
        }

        let mut outcomes = decoded.into_iter();
        let Some(first) = outcomes.next() else {
            return Err(CoreError::MalformedSendBytes {
                detail: "those bytes do not form a message".to_owned(),
            });
        };
        if outcomes.next().is_some() {
            return Err(CoreError::MalformedSendBytes {
                detail: "enter one message at a time".to_owned(),
            });
        }

        match first {
            Decoded::Message { message, raw } => Ok(Self::Raw {
                bytes: raw,
                message,
            }),
            Decoded::Invalid { reason, .. } => Err(CoreError::MalformedSendBytes {
                detail: reason.label().to_ascii_lowercase(),
            }),
            Decoded::SysexTruncated { true_len, .. } => Err(CoreError::MalformedSendBytes {
                detail: format!("that System Exclusive message is too long ({true_len} bytes)"),
            }),
            Decoded::SysexIncomplete { .. } => Err(CoreError::MalformedSendBytes {
                detail: "that System Exclusive message is not terminated".to_owned(),
            }),
        }
    }

    /// The message, for display and for the record of what was sent.
    #[must_use]
    pub const fn message(&self) -> &MidiMessage {
        match self {
            Self::Guided { message } | Self::Raw { message, .. } => message,
        }
    }

    /// The bytes this composition would transmit.
    ///
    /// Derived for a guided composition and stored for a hand-typed one — see the
    /// module documentation for why that distinction has to exist.
    ///
    /// # Errors
    ///
    /// Propagates [`CoreError::UnsendableMessage`] from the encoder. Unreachable
    /// in practice: neither constructor here can produce an invalid message.
    pub fn bytes(&self) -> Result<Vec<u8>, CoreError> {
        match self {
            Self::Guided { message } => encode(message),
            Self::Raw { bytes, .. } => Ok(bytes.clone()),
        }
    }

    /// The values this composition exposes for editing.
    ///
    /// Empty for a hand-typed composition: its authority is the byte string, so
    /// there is nothing to edit field by field and no error to invent for trying.
    #[must_use]
    pub fn fields(&self) -> Vec<FieldSpec> {
        match self {
            Self::Guided { message } => {
                SendableKind::of(message).map_or_else(Vec::new, SendableKind::fields)
            }
            Self::Raw { .. } => Vec::new(),
        }
    }

    /// The current value of one field, rendered the way the interface shows it.
    #[must_use]
    pub fn field_value(&self, field: FieldId) -> Option<String> {
        let Self::Guided { message } = self else {
            return None;
        };
        read_field(message, field)
    }

    /// Sets one value, keeping every other value as it was.
    ///
    /// # Errors
    ///
    /// - [`CoreError::UnknownField`] when the composition does not carry that
    ///   field — a stale view, or any field at all on a hand-typed composition.
    /// - [`CoreError::ValueOutOfRange`] when a number falls outside what the
    ///   message permits.
    /// - [`CoreError::MalformedSendBytes`] when a System Exclusive payload is not
    ///   readable as hexadecimal bytes.
    ///
    /// The composition is left untouched whenever this returns `Err`.
    pub fn set_field(&mut self, field: FieldId, entry: &str) -> Result<(), CoreError> {
        let Self::Guided { message } = self else {
            return Err(CoreError::UnknownField {
                field: field.id().to_owned(),
            });
        };

        let spec = SendableKind::of(message)
            .map(SendableKind::fields)
            .unwrap_or_default()
            .into_iter()
            .find(|candidate| candidate.id == field)
            .ok_or_else(|| CoreError::UnknownField {
                field: field.id().to_owned(),
            })?;

        match spec.kind {
            FieldKind::Number { min, max } => {
                let value = parse_bounded(entry, &spec, min, max)?;
                write_number(message, &spec, value)?;
            }
            FieldKind::Bytes => {
                let payload = parse_hex_bytes(entry)?;
                if let MidiMessage::SystemExclusive { payload: slot } = message {
                    *slot = payload;
                }
            }
        }
        Ok(())
    }
}

/// Reads a hexadecimal entry into bytes, ignoring separating whitespace.
///
/// Shared by the raw-entry path and the System Exclusive payload field, which
/// accept the same notation for the same reason: the user is reading bytes off a
/// specification sheet or out of the monitor's own Data column.
fn parse_hex_bytes(entry: &str) -> Result<Vec<u8>, CoreError> {
    let mut digits = String::with_capacity(entry.len());
    for character in entry.chars() {
        if character.is_whitespace() {
            continue;
        }
        if !character.is_ascii_hexdigit() {
            return Err(CoreError::MalformedSendBytes {
                detail: format!("{character} is not a hexadecimal digit"),
            });
        }
        digits.push(character);
    }

    if digits.is_empty() {
        return Err(CoreError::MalformedSendBytes {
            detail: "enter at least one byte".to_owned(),
        });
    }
    if digits.len() % 2 != 0 {
        return Err(CoreError::MalformedSendBytes {
            detail: "each byte needs two hexadecimal digits".to_owned(),
        });
    }

    let mut bytes = Vec::with_capacity(digits.len() / 2);
    let characters: Vec<char> = digits.chars().collect();
    for pair in characters.chunks(2) {
        let text: String = pair.iter().collect();
        // Both characters were checked to be hex digits above and there are
        // exactly two of them, so this cannot fail — but the constitution forbids
        // proving that with `unwrap`, so the impossible case is still named.
        let byte = u8::from_str_radix(&text, 16).map_err(|_| CoreError::MalformedSendBytes {
            detail: format!("{text} is not a byte"),
        })?;
        bytes.push(byte);
    }
    Ok(bytes)
}

/// Parses a numeric entry and checks it against the field's own bounds.
fn parse_bounded(entry: &str, spec: &FieldSpec, min: u16, max: u16) -> Result<u16, CoreError> {
    let out_of_range = || CoreError::ValueOutOfRange {
        field: spec.label.to_owned(),
        min,
        max,
    };
    let value: u16 = entry.trim().parse().map_err(|_| out_of_range())?;
    if value < min || value > max {
        return Err(out_of_range());
    }
    Ok(value)
}

/// Renders one field's current value.
///
/// Exhaustive over every message that carries fields; the field-less messages
/// return [`None`] because there is nothing to show.
fn read_field(message: &MidiMessage, field: FieldId) -> Option<String> {
    let value = match (message, field) {
        (
            MidiMessage::NoteOn { channel, .. }
            | MidiMessage::NoteOff { channel, .. }
            | MidiMessage::AftertouchPoly { channel, .. }
            | MidiMessage::Control { channel, .. }
            | MidiMessage::Program { channel, .. }
            | MidiMessage::ChannelPressure { channel, .. }
            | MidiMessage::PitchWheel { channel, .. },
            FieldId::Channel,
        ) => u16::from(channel.get()),
        (
            MidiMessage::NoteOn { note, .. }
            | MidiMessage::NoteOff { note, .. }
            | MidiMessage::AftertouchPoly { note, .. },
            FieldId::Note,
        ) => u16::from(note.get()),
        (
            MidiMessage::NoteOn { velocity, .. } | MidiMessage::NoteOff { velocity, .. },
            FieldId::Velocity,
        ) => u16::from(velocity.get()),
        (MidiMessage::Control { controller, .. }, FieldId::Controller) => {
            u16::from(controller.get())
        }
        (MidiMessage::Control { value, .. }, FieldId::ControlValue) => u16::from(value.get()),
        (MidiMessage::Program { program, .. }, FieldId::Program) => u16::from(program.get()),
        (
            MidiMessage::AftertouchPoly { pressure, .. }
            | MidiMessage::ChannelPressure { pressure, .. },
            FieldId::Pressure,
        ) => u16::from(pressure.get()),
        (MidiMessage::PitchWheel { value, .. }, FieldId::Bend) => value.get(),
        (MidiMessage::TimeCode { data }, FieldId::TimeCodeData) => u16::from(data.get()),
        (MidiMessage::SongPositionPointer { position }, FieldId::SongPosition) => position.get(),
        (MidiMessage::SongSelect { song }, FieldId::Song) => u16::from(song.get()),
        (MidiMessage::SystemExclusive { payload }, FieldId::Payload) => {
            return Some(to_hex(payload))
        }
        // Every remaining pairing asks a message for a value it does not carry.
        // See the note on catch-all arms in this module's documentation for why
        // the two functions here are the exception to the project's no-wildcard
        // rule, and what stands in for the compiler's check.
        _ => return None,
    };
    Some(value.to_string())
}

/// Writes one numeric field, having already checked it against the field's range.
///
/// # Why this returns `Result` for a value already known to be in range
///
/// Narrowing sixteen bits to eight, and building a channel, are both fallible
/// operations in general. `parse_bounded` has already ruled out every input that
/// could fail — but proving that with an `as` cast or an `unwrap` is exactly what
/// the project forbids, because both survive a later change to the bounds while
/// silently losing their justification. Propagating the impossible error costs one
/// `?` and keeps the narrowing honest, which is the same trade the encoder makes
/// for [`MidiMessage::Invalid`].
fn write_number(message: &mut MidiMessage, spec: &FieldSpec, value: u16) -> Result<(), CoreError> {
    match (message, spec.id) {
        (
            MidiMessage::NoteOn { channel, .. }
            | MidiMessage::NoteOff { channel, .. }
            | MidiMessage::AftertouchPoly { channel, .. }
            | MidiMessage::Control { channel, .. }
            | MidiMessage::Program { channel, .. }
            | MidiMessage::ChannelPressure { channel, .. }
            | MidiMessage::PitchWheel { channel, .. },
            FieldId::Channel,
        ) => *channel = to_channel(value, spec)?,
        (
            MidiMessage::NoteOn { note, .. }
            | MidiMessage::NoteOff { note, .. }
            | MidiMessage::AftertouchPoly { note, .. },
            FieldId::Note,
        ) => *note = to_seven_bit(value, spec)?,
        (
            MidiMessage::NoteOn { velocity, .. } | MidiMessage::NoteOff { velocity, .. },
            FieldId::Velocity,
        ) => *velocity = to_seven_bit(value, spec)?,
        (MidiMessage::Control { controller, .. }, FieldId::Controller) => {
            *controller = to_seven_bit(value, spec)?;
        }
        (MidiMessage::Control { value: slot, .. }, FieldId::ControlValue) => {
            *slot = to_seven_bit(value, spec)?;
        }
        (MidiMessage::Program { program, .. }, FieldId::Program) => {
            *program = to_seven_bit(value, spec)?;
        }
        (
            MidiMessage::AftertouchPoly { pressure, .. }
            | MidiMessage::ChannelPressure { pressure, .. },
            FieldId::Pressure,
        ) => *pressure = to_seven_bit(value, spec)?,
        (MidiMessage::PitchWheel { value: slot, .. }, FieldId::Bend) => {
            *slot = Data14::from_masked(value);
        }
        (MidiMessage::TimeCode { data }, FieldId::TimeCodeData) => {
            *data = to_seven_bit(value, spec)?;
        }
        (MidiMessage::SongPositionPointer { position }, FieldId::SongPosition) => {
            *position = Data14::from_masked(value);
        }
        (MidiMessage::SongSelect { song }, FieldId::Song) => *song = to_seven_bit(value, spec)?,
        // Every remaining pairing was already refused by the field lookup in
        // `set_field`, which only offers the fields this message actually carries.
        _ => {}
    }
    Ok(())
}

/// Narrows a range-checked value into a seven-bit data byte.
///
/// The `Err` cannot be produced from within this module: every caller has been
/// through [`parse_bounded`] against a seven-bit field. It is propagated rather
/// than assumed away because an `as` cast or an `unwrap` would survive a later
/// change to the bounds and silently lose the reasoning that made it safe.
fn to_seven_bit(value: u16, spec: &FieldSpec) -> Result<DataByte, CoreError> {
    u8::try_from(value)
        .map(DataByte::from_masked)
        .map_err(|_| CoreError::ValueOutOfRange {
            field: spec.label.to_owned(),
            min: 0,
            max: u16::from(MAX_DATA_BYTE),
        })
}

/// Narrows a range-checked value into a channel.
///
/// Routed through [`ChannelNumber::new`], the validating constructor, rather than
/// through the wire-nibble one: the user is entering a 1-based channel here, and
/// the offset belongs to the encoder, not to this function.
fn to_channel(value: u16, spec: &FieldSpec) -> Result<ChannelNumber, CoreError> {
    u8::try_from(value)
        .map_err(|_| CoreError::ValueOutOfRange {
            field: spec.label.to_owned(),
            min: u16::from(*CHANNEL_RANGE.start()),
            max: u16::from(*CHANNEL_RANGE.end()),
        })
        .and_then(ChannelNumber::new)
}

/// Formats bytes as spaced uppercase hexadecimal, the notation the entry accepts.
fn to_hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| format!("{byte:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// A composition in the form that survives a restart.
///
/// # Why this is not just a serialized [`Composition`]
///
/// Deriving `Serialize` on [`Composition`] would mean deriving it on
/// [`MidiMessage`] and on every value type inside it, which would tie the shape
/// of the domain's central type to what a settings file happens to look like.
/// The crate's own manifest already refuses that trade for the webview's sake;
/// the same reasoning applies to disk.
///
/// This form records the *minimum* needed to rebuild the composition exactly: for
/// a guided one, which message and what its values were; for a hand-typed one,
/// the bytes as typed. It is also the more durable of the two — a stored field
/// name that no longer exists is one refused request, whereas a stored enum shape
/// that no longer parses would take the whole document with it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PersistedComposition {
    /// A composition built from named controls.
    Guided {
        /// Which message, by the identifier [`SendableKind::id`] produces.
        kind: String,
        /// Each field's identifier and the value it held.
        fields: Vec<PersistedField>,
    },
    /// A composition typed as bytes.
    Raw {
        /// Exactly the bytes that were typed, so a restart transmits them
        /// unchanged rather than re-encoding an interpretation of them.
        bytes: Vec<u8>,
    },
}

/// One stored field value.
///
/// A named pair rather than a tuple, so the settings file reads as something a
/// person can inspect and correct by hand.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistedField {
    /// The field's identifier, as [`FieldId::id`] produces it.
    pub id: String,
    /// The value, in the notation the field accepts.
    pub value: String,
}

impl PersistedComposition {
    /// Captures a composition for storage.
    #[must_use]
    pub fn capture(composition: &Composition) -> Self {
        match composition {
            Composition::Guided { message } => {
                let kind = SendableKind::of(message);
                Self::Guided {
                    kind: kind.map_or_else(String::new, |kind| kind.id().to_owned()),
                    fields: composition
                        .fields()
                        .into_iter()
                        .filter_map(|spec| {
                            composition
                                .field_value(spec.id)
                                .map(|value| PersistedField {
                                    id: spec.id.id().to_owned(),
                                    value,
                                })
                        })
                        .collect(),
                }
            }
            Composition::Raw { bytes, .. } => Self::Raw {
                bytes: bytes.clone(),
            },
        }
    }

    /// Rebuilds the composition this was captured from.
    ///
    /// # Errors
    ///
    /// - [`CoreError::UnknownSendableKind`] when the stored message type is not
    ///   one this build knows.
    /// - [`CoreError::UnknownField`], [`CoreError::ValueOutOfRange`], or
    ///   [`CoreError::MalformedSendBytes`] when a stored value is not one this
    ///   build accepts.
    ///
    /// Every one of these means the document was written by a different version
    /// or edited by hand. The caller drops the one request rather than refusing
    /// the whole document.
    pub fn rebuild(&self) -> Result<Composition, CoreError> {
        match self {
            Self::Guided { kind, fields } => {
                let kind = SendableKind::from_id(kind)
                    .ok_or_else(|| CoreError::UnknownSendableKind { id: kind.clone() })?;
                let mut composition = Composition::from_kind(kind);
                for field in fields {
                    let id =
                        FieldId::from_id(&field.id).ok_or_else(|| CoreError::UnknownField {
                            field: field.id.clone(),
                        })?;
                    composition.set_field(id, &field.value)?;
                }
                Ok(composition)
            }
            Self::Raw { bytes } => Composition::from_raw_bytes(bytes),
        }
    }
}
