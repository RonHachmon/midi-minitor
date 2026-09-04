//! Turning a message back into the bytes that carry it.
//!
//! # Why this exists, and why here
//!
//! This is the mirror of [`super::decoder`], and it is **the byte-encoding
//! boundary** [`crate::constants::CHANNEL_RANGE`] has referred to since the first
//! feature: the one place a 1-based display channel becomes a 0-based wire
//! nibble. Nothing else in the application performs that conversion, which is
//! what makes it impossible for display or filtering code to forget the offset.
//!
//! It lives in the domain rather than in the webview or in a platform adapter for
//! one reason above all: the send screen must show the bytes it is about to
//! transmit, and the promise that the preview matches the transmission is only
//! keepable if the two are *the same value*. Two implementations that must agree
//! is a promise with a hole in it. So the preview is this function's output, and
//! so is what reaches the adapter.
//!
//! # Why the `Err` is unreachable and stays anyway
//!
//! [`MidiMessage::Invalid`] is the only input that fails. No caller in this
//! application can produce one: [`super::composition::Composition`] builds guided
//! messages from [`super::sendable::SendableKind`], which has no invalid variant,
//! and validates hand-typed bytes through the decoder before accepting them.
//!
//! The arm exists because [`MidiMessage`] models what a **monitor** observed, and
//! a monitor is required to display malformed data — that is the point of one.
//! Narrowing the message type to what a sender can emit would damage the older
//! feature to serve the newer one. Returning `Result` is the honest way to say
//! "this type has one shape I cannot encode" without reaching for the `unwrap`
//! the constitution forbids anyway.
//!
//! # Exhaustiveness is deliberate
//!
//! The `match` below lists every variant with no catch-all arm. Adding a
//! nineteenth message type is therefore a compile error here as well as in the
//! decoder — the compiler enumerates the work, which is this project's stated
//! substitute for a test suite.

use super::ids::ChannelNumber;
use super::message::MidiMessage;
use crate::application::error::CoreError;

/// Status byte for Note Off, before the channel nibble is applied.
const STATUS_NOTE_OFF: u8 = 0x80;
/// Status byte for Note On, before the channel nibble is applied.
const STATUS_NOTE_ON: u8 = 0x90;
/// Status byte for polyphonic aftertouch, before the channel nibble is applied.
const STATUS_AFTERTOUCH_POLY: u8 = 0xA0;
/// Status byte for Control Change, before the channel nibble is applied.
const STATUS_CONTROL: u8 = 0xB0;
/// Status byte for Program Change, before the channel nibble is applied.
const STATUS_PROGRAM: u8 = 0xC0;
/// Status byte for Channel Pressure, before the channel nibble is applied.
const STATUS_CHANNEL_PRESSURE: u8 = 0xD0;
/// Status byte for Pitch Wheel, before the channel nibble is applied.
const STATUS_PITCH_WHEEL: u8 = 0xE0;

/// Status byte for a MIDI Time Code quarter frame.
const STATUS_TIME_CODE: u8 = 0xF1;
/// Status byte for Song Position Pointer.
const STATUS_SONG_POSITION: u8 = 0xF2;
/// Status byte for Song Select.
const STATUS_SONG_SELECT: u8 = 0xF3;
/// Status byte for Tune Request.
const STATUS_TUNE_REQUEST: u8 = 0xF6;
/// Status byte for the timing Clock pulse.
const STATUS_CLOCK: u8 = 0xF8;
/// Status byte for Start.
const STATUS_START: u8 = 0xFA;
/// Status byte for Continue.
const STATUS_CONTINUE: u8 = 0xFB;
/// Status byte for Stop.
const STATUS_STOP: u8 = 0xFC;
/// Status byte for Active Sensing.
const STATUS_ACTIVE_SENSE: u8 = 0xFE;
/// Status byte for System Reset.
const STATUS_RESET: u8 = 0xFF;

/// Status byte that opens a System Exclusive transfer.
const SYSEX_START: u8 = 0xF0;
/// Status byte that closes a System Exclusive transfer.
const SYSEX_END: u8 = 0xF7;

/// Encodes a message into the bytes that carry it on the wire.
///
/// # Errors
///
/// Returns [`CoreError::UnsendableMessage`] for [`MidiMessage::Invalid`], and for
/// nothing else. See the module documentation for why that case cannot be reached
/// through this application's composition API and is modelled anyway.
pub fn encode(message: &MidiMessage) -> Result<Vec<u8>, CoreError> {
    let bytes = match message {
        MidiMessage::NoteOff {
            channel,
            note,
            velocity,
        } => vec![
            status(STATUS_NOTE_OFF, *channel),
            note.get(),
            velocity.get(),
        ],
        MidiMessage::NoteOn {
            channel,
            note,
            velocity,
        } => vec![status(STATUS_NOTE_ON, *channel), note.get(), velocity.get()],
        MidiMessage::AftertouchPoly {
            channel,
            note,
            pressure,
        } => vec![
            status(STATUS_AFTERTOUCH_POLY, *channel),
            note.get(),
            pressure.get(),
        ],
        MidiMessage::Control {
            channel,
            controller,
            value,
        } => vec![
            status(STATUS_CONTROL, *channel),
            controller.get(),
            value.get(),
        ],
        MidiMessage::Program { channel, program } => {
            vec![status(STATUS_PROGRAM, *channel), program.get()]
        }
        MidiMessage::ChannelPressure { channel, pressure } => {
            vec![status(STATUS_CHANNEL_PRESSURE, *channel), pressure.get()]
        }
        MidiMessage::PitchWheel { channel, value } => {
            let (least, most) = value.to_wire_pair();
            vec![status(STATUS_PITCH_WHEEL, *channel), least, most]
        }
        MidiMessage::TimeCode { data } => vec![STATUS_TIME_CODE, data.get()],
        MidiMessage::SongPositionPointer { position } => {
            let (least, most) = position.to_wire_pair();
            vec![STATUS_SONG_POSITION, least, most]
        }
        MidiMessage::SongSelect { song } => vec![STATUS_SONG_SELECT, song.get()],
        MidiMessage::TuneRequest => vec![STATUS_TUNE_REQUEST],
        MidiMessage::Clock => vec![STATUS_CLOCK],
        MidiMessage::Start => vec![STATUS_START],
        MidiMessage::Stop => vec![STATUS_STOP],
        MidiMessage::Continue => vec![STATUS_CONTINUE],
        MidiMessage::ActiveSense => vec![STATUS_ACTIVE_SENSE],
        MidiMessage::Reset => vec![STATUS_RESET],
        // The payload is stored without its framing bytes, because that is what
        // the Data column shows and what a user edits. Framing is restored here,
        // at the boundary that owns the wire format.
        MidiMessage::SystemExclusive { payload } => {
            let mut bytes = Vec::with_capacity(payload.len() + 2);
            bytes.push(SYSEX_START);
            bytes.extend_from_slice(payload);
            bytes.push(SYSEX_END);
            bytes
        }
        MidiMessage::Invalid { reason, .. } => {
            return Err(CoreError::UnsendableMessage {
                reason: reason.label().to_owned(),
            })
        }
    };
    Ok(bytes)
}

/// Combines a status byte with a channel, converting 1-based to 0-based.
///
/// The whole reason this module is called the encoding boundary: this line is the
/// only place in the application where the display convention becomes the wire
/// convention.
const fn status(base: u8, channel: ChannelNumber) -> u8 {
    base | channel.to_wire_nibble()
}
