//! Values fixed by the reference screenshots and by the MIDI specification.
//!
//! # Why these are constants rather than literals at their use sites
//!
//! The screenshots are the design authority for this application. A value that
//! appears inline in three files gets re-guessed three times and drifts; a value
//! that lives here is checked against the image once. When a reviewer asks "why
//! 1000?", the answer is at the definition rather than in a commit message.

use std::ops::RangeInclusive;

/// Retention default shown in `screenshots/main-screen.png`.
///
/// The reference window reads "Remember up to 1000 events" on a fresh launch,
/// so this is what the field must contain before the user touches it.
pub const DEFAULT_RETENTION_LIMIT: usize = 1000;

/// Accepted range for the retention field.
///
/// The lower bound is 1 because retaining zero events would make the monitor
/// useless while looking merely empty. The upper bound is a deliberate promise:
/// the specification requires a *stated* supported range, and at roughly 64
/// bytes per retained event this ceiling costs a few megabytes and stays
/// smooth behind a virtualized list.
pub const RETENTION_LIMIT_RANGE: RangeInclusive<usize> = 1..=100_000;

/// Valid MIDI channel numbers, as the user sees them.
///
/// These are 1-based display values, not the 0-based nibble carried on the
/// wire. Conversion happens once at the byte-encoding boundary so that no
/// display or filtering code can forget the offset.
pub const CHANNEL_RANGE: RangeInclusive<u8> = 1..=16;

/// Largest value a single MIDI data byte can carry.
///
/// The high bit distinguishes status bytes from data bytes, leaving seven bits.
pub const MAX_DATA_BYTE: u8 = 127;

/// Largest value a paired 14-bit MIDI quantity can carry (pitch wheel, song position).
pub const MAX_DATA_14: u16 = 16_383;

/// Milliseconds in a day, the modulus for a wall-clock timestamp.
pub const MILLIS_PER_DAY: u64 = 24 * 60 * 60 * 1000;

/// Largest System Exclusive transfer retained in full.
///
/// # Why there is a ceiling at all
///
/// A System Exclusive transfer is framed by its end byte, not by a declared
/// length, so a device that starts one and never finishes it would otherwise
/// grow a buffer without bound. A monitor that exhausts memory watching a
/// misbehaving device has failed at the one job it had.
///
/// The value matches the practical packet-buffer ceiling the operating system's
/// own MIDI stack works to, so a transfer this application truncates is one the
/// platform was not going to deliver intact either.
///
/// Passing the ceiling is reported, never silent: the transfer is listed with
/// its **true** size so the user learns how big it actually was.
pub const MAX_SYSEX_BYTES: usize = 65_536;
