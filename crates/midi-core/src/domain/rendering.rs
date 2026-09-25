//! Turning an observed event into the strings the table shows.
//!
//! # Why this is a module of its own
//!
//! [`super::message`] models what a message *is*; this models how it is
//! *written*. Those are two reasons to change — one moves when MIDI gains a
//! message type, the other when the user picks a different format — and
//! Principle I requires them to be separate. `data_display` and `note_name`
//! lived on [`super::message::MidiMessage`] until this feature gave them a
//! parameter, at which point the double responsibility stopped being tolerable.
//!
//! # Why this is domain code and not webview code
//!
//! The argument the old `data_display` made, unchanged: what belongs in the Data
//! cell is a fact about MIDI, not a presentation choice. A note shows its name
//! and velocity, Channel Pressure shows a bare amount, System Exclusive shows a
//! byte count. A *setting* that chooses between two such facts is still one.
//! Rendering here keeps a single implementation; rendering in the webview would
//! put a MIDI rule on the far side of a serialization boundary and duplicate the
//! 128-entry controller table there.
//!
//! # Exhaustiveness
//!
//! Every `match` over a settings enum lists its variants with no catch-all arm,
//! so a seventh format is a compile error at each site that must handle it. With
//! no test suite, the compiler enumerating the work is this project's substitute
//! for one.

use super::controller;
use super::display::{ControllerFormat, DisplaySettings, ExpertMode, NoteFormat, TimeFormat};
use super::event::MidiEvent;
use super::ids::{HostTime, TickRate};
use super::message::MidiMessage;

/// Semitones per octave — the divisor turning a note number into an octave.
const SEMITONES_PER_OCTAVE: u8 = 12;

/// Octave offset placing middle C (note 60) at `C3`.
const OCTAVE_OFFSET_MIDDLE_C3: i16 = 2;

/// Octave offset placing middle C (note 60) at `C4`.
const OCTAVE_OFFSET_MIDDLE_C4: i16 = 1;

/// Nanoseconds in one second.
const NANOS_PER_SECOND: u128 = 1_000_000_000;

/// How many fractional digits `Host time (seconds)` shows.
///
/// Nine, so the seconds form is the nanosecond form with the point moved: the
/// two are then visibly the same reading, which is the consistency the three
/// host formats promise.
const HOST_SECONDS_PRECISION: usize = 9;

/// The pitch class names, sharp-spelled.
///
/// Enharmonic spelling is not something MIDI carries, so a single naming
/// convention is the only honest choice. The reference images show sharps.
const NOTE_NAMES: [&str; 12] = [
    "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
];

/// Status nibble of a `Note On`, used to recover what the wire actually carried.
const STATUS_NOTE_ON: u8 = 0x90;

/// Mask isolating the kind nibble of a channel-voice status byte.
const STATUS_MASK: u8 = 0xF0;

/// The Time column's contents.
///
/// # Why the tick rate is a parameter rather than a field of the event
///
/// It is fixed for the life of the process, so carrying it on every event would
/// be one constant copied a thousand times.
/// [`crate::application::monitor::Monitor`] holds it and hands it here.
#[must_use]
pub fn time_display(event: &MidiEvent, settings: DisplaySettings, rate: TickRate) -> String {
    let ticks = host_ticks(event, settings.expert);
    match settings.time {
        TimeFormat::ClockTime => event.arrival.wall.to_display(),
        TimeFormat::HostInteger => ticks.to_string(),
        TimeFormat::HostSeconds => {
            let nanos = rate.to_nanoseconds(ticks);
            let whole = nanos / NANOS_PER_SECOND;
            let fraction = nanos % NANOS_PER_SECOND;
            let width = HOST_SECONDS_PRECISION;
            format!("{whole}.{fraction:0width$}")
        }
        TimeFormat::HostNanoseconds => rate.to_nanoseconds(ticks).to_string(),
    }
}

/// The host-clock reading to display, honouring the zero-timestamp convention.
///
/// CoreMIDI defines a packet timestamp of zero as "now", and the monitor
/// substitutes the moment it received the message — the third line beneath the
/// `Expert mode` checkbox. Ticking that box suppresses the substitution and
/// shows the zero that arrived.
fn host_ticks(event: &MidiEvent, expert: ExpertMode) -> u64 {
    match (event.arrival.host, expert) {
        (HostTime::Stamped(ticks), _) => ticks.get(),
        (HostTime::ZeroMeaningNow { .. }, ExpertMode::On) => 0,
        (HostTime::ZeroMeaningNow { received }, ExpertMode::Off) => received.get(),
    }
}

/// The Message column's contents.
///
/// # Why `Expert mode` reads the raw bytes rather than the decoder being changed
///
/// [`super::decoder`] converts a `Note On` with velocity zero into
/// [`MidiMessage::NoteOff`], because that is what the convention means and what
/// the filter checkboxes are named for. Someone who ticks `Expert mode` wants to
/// know which one actually came down the wire — and [`MidiEvent::raw`] already
/// retains exactly that, for exactly this reason.
///
/// Moving the conversion out of the decoder instead would have shifted those
/// events from the `Note Off` filter checkbox to the `Note On` one, which would
/// let a *display* setting change which events pass a *filter*. Reading the
/// retained bytes leaves the model, the filter, and the decoder untouched.
///
/// Under running status the wire carries no status byte, so `raw` cannot
/// contradict the interpretation and the decoded message stands.
#[must_use]
pub fn message_display(event: &MidiEvent, settings: DisplaySettings) -> &'static str {
    match settings.expert {
        ExpertMode::Off => event.message.display_name(),
        ExpertMode::On if arrived_as_note_on(event) => "Note On",
        ExpertMode::On => event.message.display_name(),
    }
}

/// Whether this event was decoded as a release but arrived as a `Note On`.
fn arrived_as_note_on(event: &MidiEvent) -> bool {
    matches!(event.message, MidiMessage::NoteOff { .. })
        && event
            .raw
            .first()
            .is_some_and(|status| status & STATUS_MASK == STATUS_NOTE_ON)
}

/// The Data column's contents.
///
/// Each value is governed by the one setting that names it: note numbers by
/// [`NoteFormat`], controller numbers by [`ControllerFormat`], programs by
/// [`ProgramNumbering`](super::display::ProgramNumbering), and everything else
/// by [`DataFormat`](super::display::DataFormat). The velocity
/// beside a note is *everything else*, so changing the note format must not
/// touch it.
///
/// # Why `Expert mode` abandons all of that
///
/// `screenshots/expert-mode.png` states the rule: `Data formatted as raw
/// hexadecimal`. Not "in base sixteen" — *raw*. The cell stops being an
/// interpretation of the message and becomes the bytes that arrived, which is
/// the only reading under which the note, controller, program and data formats
/// all have nothing left to govern.
///
/// This is the same string the row's tooltip carries, deliberately: someone who
/// found the bytes by hovering should see the identical text when they tick the
/// box, rather than having to satisfy themselves that two spellings of the same
/// bytes agree.
///
/// A `System Exclusive` transfer shows its bytes here too, where the decoded
/// form shows a size. That is the point of the mode, and the payload is bounded
/// by [`crate::constants::MAX_SYSEX_BYTES`], so the cell cannot grow without
/// limit — it will simply be wider than the column and truncate.
#[must_use]
pub fn data_display(event: &MidiEvent, settings: DisplaySettings) -> String {
    match settings.expert {
        ExpertMode::On => event.raw_hex(),
        ExpertMode::Off => decoded_display(event, settings),
    }
}

/// The Data column's contents as an interpretation of the message.
///
/// Split from [`data_display`] so the raw path above stays one line and this
/// keeps the exhaustive match the module header promises.
fn decoded_display(event: &MidiEvent, settings: DisplaySettings) -> String {
    let data = settings.data;
    match &event.message {
        MidiMessage::NoteOn { note, velocity, .. }
        | MidiMessage::NoteOff { note, velocity, .. } => format!(
            "{} {}",
            note_display(note.get(), settings),
            data.write(u16::from(velocity.get()))
        ),
        MidiMessage::AftertouchPoly { note, pressure, .. } => format!(
            "{} {}",
            note_display(note.get(), settings),
            data.write(u16::from(pressure.get()))
        ),
        MidiMessage::Control {
            controller, value, ..
        } => format!(
            "{} {}",
            controller_display(controller.get(), settings),
            data.write(u16::from(value.get()))
        ),
        MidiMessage::Program { program, .. } => {
            settings.program.displayed(program.get()).to_string()
        }
        MidiMessage::ChannelPressure { pressure, .. } => data.write(u16::from(pressure.get())),
        MidiMessage::PitchWheel { value, .. } => data.write(value.get()),
        MidiMessage::TimeCode { data: nibbles } => data.write(u16::from(nibbles.get())),
        MidiMessage::SongPositionPointer { position } => data.write(position.get()),
        MidiMessage::SongSelect { song } => data.write(u16::from(song.get())),
        MidiMessage::TuneRequest
        | MidiMessage::Clock
        | MidiMessage::Start
        | MidiMessage::Stop
        | MidiMessage::Continue
        | MidiMessage::ActiveSense
        | MidiMessage::Reset => String::new(),
        MidiMessage::SystemExclusive { payload } => {
            // Framing bytes count toward what the user thinks of as the message.
            // Deliberately a size and not a byte dump: `Hexadecimal number` must
            // not turn a transfer of thousands of bytes into one table cell.
            let total = payload.len() + 2;
            format!("{total} bytes")
        }
        MidiMessage::Invalid { reason, bytes } => {
            format!("{} ({} bytes)", reason.label(), bytes.len())
        }
    }
}

/// Renders one note number per the chosen format.
///
/// Reached only while `Expert mode` is off, since the mode replaces the whole
/// cell rather than reformatting the values inside it.
#[must_use]
pub fn note_display(note: u8, settings: DisplaySettings) -> String {
    match settings.note {
        NoteFormat::NameMiddleC3 => note_name(note, OCTAVE_OFFSET_MIDDLE_C3),
        NoteFormat::NameMiddleC4 => note_name(note, OCTAVE_OFFSET_MIDDLE_C4),
        NoteFormat::Decimal => note.to_string(),
        NoteFormat::Hexadecimal => format!("{note:X}"),
    }
}

/// Renders a note number as a name and octave, such as `C2`.
///
/// # Why the octave convention is a parameter rather than fixed
///
/// It was fixed at middle C = C4 before this feature, justified as the
/// scientific convention and as the reading that made `screenshots/data.png`'s
/// `C2` come out right. That justification is **superseded, not forgotten**:
/// `screenshots/setting.jpg` offers both conventions and selects
/// `Note (Middle C = C3)`, and the user resolved the conflict between the two
/// reference images in favour of the preferences image on 2026-09-23. See
/// `specs/006-display-preferences/plan.md`, Complexity Tracking.
///
/// The practical consequence is that the monitor's default note names now sit
/// one octave below what `screenshots/data.png` shows. That is intended. Do not
/// "fix" it by restoring C4 as the default — select `Note (Middle C = C4)` in
/// the preferences instead, which is what that option is for.
///
/// Every number from 0 to 127 produces a name under both conventions; the octave
/// runs negative at the bottom, so note 0 is `C-2` under the C3 convention.
fn note_name(note: u8, octave_offset: i16) -> String {
    let name = NOTE_NAMES[usize::from(note % SEMITONES_PER_OCTAVE)];
    let octave = i16::from(note / SEMITONES_PER_OCTAVE) - octave_offset;
    format!("{name}{octave}")
}

/// Renders one controller number per the chosen format.
///
/// Under `Standard name` a controller with no assigned name falls back to its
/// number: the name is an addition to the number, never a replacement that
/// leaves the cell blank.
///
/// Reached only while `Expert mode` is off, since the mode replaces the whole
/// cell rather than reformatting the values inside it.
#[must_use]
pub fn controller_display(number: u8, settings: DisplaySettings) -> String {
    match settings.controller {
        ControllerFormat::StandardName => {
            controller::name(number).map_or_else(|| number.to_string(), ToOwned::to_owned)
        }
        ControllerFormat::Decimal => number.to_string(),
        ControllerFormat::Hexadecimal => format!("{number:X}"),
    }
}
