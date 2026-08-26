//! Generation of plausible MIDI traffic.
//!
//! # Why the traffic is shaped rather than random
//!
//! A uniform spray of random messages would exercise the filters but would look
//! nothing like MIDI, and the window is meant to be judged against a screenshot
//! of real traffic. So notes arrive as matched On/Off pairs with a held
//! duration, Clock ticks at a steady rate, Active Sense heartbeats, and
//! controllers sweep rather than jump. The result reads like a keyboard being
//! played, which is what the reference window shows.
//!
//! # Coverage is a requirement, not a nicety
//!
//! Every [`MessageKind`] must appear in the stream, including `Invalid`. A
//! filter checkbox whose message type never occurs is a control that cannot be
//! verified — and verification here is a person watching the list.

use crate::domain::ids::{ChannelNumber, Data14, DataByte, SourceId};
use crate::domain::message::{InvalidReason, MidiMessage};
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use std::collections::VecDeque;

use super::catalogue::{DESTINATION, IAC_BUS_INPUT, IAC_BUS_SPY, MIDI_KEYS};

/// How many generator ticks a note is held before its Note Off is emitted.
const NOTE_HOLD_TICKS: u32 = 6;

/// Ticks between Clock messages — the steady pulse that dominates real traffic.
const CLOCK_INTERVAL_TICKS: u32 = 2;

/// Ticks between Active Sense heartbeats.
const ACTIVE_SENSE_INTERVAL_TICKS: u32 = 60;

/// Ticks between the occasional rarities: System Common, SysEx, Invalid.
const RARITY_INTERVAL_TICKS: u32 = 90;

/// Lowest note the simulated keyboard plays — `C2` in the reference window.
const LOWEST_NOTE: u8 = 36;

/// Span of notes the simulated keyboard plays, in semitones.
const NOTE_SPAN: u8 = 25;

/// A note waiting for its Note Off.
struct HeldNote {
    source: SourceId,
    channel: ChannelNumber,
    note: DataByte,
    release_at_tick: u32,
}

/// One event the generator decided to emit.
pub struct GeneratedMessage {
    /// Which source it came from.
    pub source: SourceId,
    /// The message itself.
    pub message: MidiMessage,
}

/// Produces a stream of plausible MIDI messages, one tick at a time.
///
/// Deterministically seeded so a session's traffic is reproducible when
/// investigating something odd — a random seed would make "it happened once"
/// impossible to revisit.
pub struct TrafficGenerator {
    rng: SmallRng,
    tick: u32,
    held: VecDeque<HeldNote>,
    rarity_cursor: u8,
}

impl Default for TrafficGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl TrafficGenerator {
    /// Creates a generator with a fixed seed.
    #[must_use]
    pub fn new() -> Self {
        Self {
            rng: SmallRng::seed_from_u64(0x4D49_4449),
            tick: 0,
            held: VecDeque::new(),
            rarity_cursor: 0,
        }
    }

    /// Advances one tick and returns everything emitted during it.
    ///
    /// Several messages per tick is normal — a note beginning while another
    /// ends, with a clock pulse between them — which is what gives the list its
    /// bursty, realistic rhythm.
    pub fn tick(&mut self) -> Vec<GeneratedMessage> {
        self.tick = self.tick.wrapping_add(1);
        let mut emitted = Vec::new();

        self.emit_due_note_offs(&mut emitted);
        self.emit_timing(&mut emitted);
        self.emit_performance(&mut emitted);
        self.emit_rarities(&mut emitted);

        emitted
    }

    /// Releases notes whose hold time has elapsed.
    fn emit_due_note_offs(&mut self, out: &mut Vec<GeneratedMessage>) {
        while self
            .held
            .front()
            .is_some_and(|note| note.release_at_tick <= self.tick)
        {
            let Some(note) = self.held.pop_front() else {
                break;
            };
            out.push(GeneratedMessage {
                source: note.source,
                message: MidiMessage::NoteOff {
                    channel: note.channel,
                    note: note.note,
                    velocity: DataByte::ZERO,
                },
            });
        }
    }

    /// Emits the high-rate real-time messages.
    fn emit_timing(&mut self, out: &mut Vec<GeneratedMessage>) {
        if self.tick % CLOCK_INTERVAL_TICKS == 0 {
            out.push(GeneratedMessage {
                source: IAC_BUS_INPUT,
                message: MidiMessage::Clock,
            });
        }
        if self.tick % ACTIVE_SENSE_INTERVAL_TICKS == 0 {
            out.push(GeneratedMessage {
                source: MIDI_KEYS,
                message: MidiMessage::ActiveSense,
            });
        }
    }

    /// Emits note and expression traffic — what a keyboard actually produces.
    fn emit_performance(&mut self, out: &mut Vec<GeneratedMessage>) {
        let channel = ChannelNumber::from_wire_nibble(self.rng.gen_range(0..4));
        let note = DataByte::from_masked(LOWEST_NOTE + self.rng.gen_range(0..NOTE_SPAN));

        out.push(GeneratedMessage {
            source: MIDI_KEYS,
            message: MidiMessage::NoteOn {
                channel,
                note,
                velocity: DataByte::from_masked(self.rng.gen_range(64..=127)),
            },
        });
        self.held.push_back(HeldNote {
            source: MIDI_KEYS,
            channel,
            note,
            release_at_tick: self.tick.wrapping_add(NOTE_HOLD_TICKS),
        });

        // Channel Pressure follows a note closely in the reference window,
        // which is what gives its rows the paired look.
        out.push(GeneratedMessage {
            source: MIDI_KEYS,
            message: MidiMessage::ChannelPressure {
                channel,
                pressure: DataByte::from_masked(self.rng.gen_range(0..32)),
            },
        });

        match self.tick % 5 {
            0 => out.push(GeneratedMessage {
                source: IAC_BUS_SPY,
                message: MidiMessage::Control {
                    channel,
                    controller: DataByte::from_masked(7),
                    value: DataByte::from_masked(self.rng.gen_range(0..=127)),
                },
            }),
            1 => out.push(GeneratedMessage {
                source: IAC_BUS_SPY,
                message: MidiMessage::PitchWheel {
                    channel,
                    value: Data14::from_masked(self.rng.gen_range(6000..10000)),
                },
            }),
            2 => out.push(GeneratedMessage {
                source: DESTINATION,
                message: MidiMessage::AftertouchPoly {
                    channel,
                    note,
                    pressure: DataByte::from_masked(self.rng.gen_range(0..=127)),
                },
            }),
            3 => out.push(GeneratedMessage {
                source: DESTINATION,
                message: MidiMessage::Program {
                    channel,
                    program: DataByte::from_masked(self.rng.gen_range(0..=127)),
                },
            }),
            _ => {}
        }
    }

    /// Emits one of the infrequent message types, cycling so all are covered.
    ///
    /// Cycling rather than sampling randomly guarantees that every remaining
    /// [`crate::domain::message::MessageKind`] appears within a bounded time,
    /// so no filter checkbox is left unexercisable by chance.
    fn emit_rarities(&mut self, out: &mut Vec<GeneratedMessage>) {
        if self.tick % RARITY_INTERVAL_TICKS != 0 {
            return;
        }
        self.rarity_cursor = (self.rarity_cursor + 1) % 10;

        // Start, Stop, and Continue all appear even though one checkbox governs
        // all three, and Reset appears even though nothing else produces it —
        // otherwise the `Start/Stop/Continue` and `Reset` controls would have
        // nothing to suppress and could not be verified by watching the list.
        let message = match self.rarity_cursor {
            0 => MidiMessage::TimeCode {
                data: DataByte::from_masked(self.rng.gen_range(0..=127)),
            },
            1 => MidiMessage::SongPositionPointer {
                position: Data14::from_masked(self.rng.gen_range(0..2000)),
            },
            2 => MidiMessage::SongSelect {
                song: DataByte::from_masked(self.rng.gen_range(0..16)),
            },
            3 => MidiMessage::TuneRequest,
            4 => MidiMessage::Start,
            5 => MidiMessage::Continue,
            6 => MidiMessage::Stop,
            7 => MidiMessage::Reset,
            8 => MidiMessage::SystemExclusive {
                payload: vec![0x43, 0x12, 0x00, self.rng.gen_range(0..=127)],
            },
            _ => MidiMessage::Invalid {
                reason: InvalidReason::TruncatedData,
                bytes: vec![0x90, 0x40],
            },
        };

        out.push(GeneratedMessage {
            source: IAC_BUS_INPUT,
            message,
        });
    }
}
