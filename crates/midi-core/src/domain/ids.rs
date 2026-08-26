//! Newtypes for the values that would otherwise all be bare integers.
//!
//! # Why wrap them
//!
//! **Newtype pattern.** The problem it solves here is concrete: a channel
//! number, a source id, and a retention limit are all small integers, so a bare
//! `u32` parameter accepts any of them and the compiler stays quiet. Wrapping
//! each one makes the mix-up impossible and gives every invariant exactly one
//! place to be checked — the constructor. With no tests, a validation rule that
//! can be bypassed is a rule that will be.

use crate::application::error::CoreError;
use crate::constants::{
    CHANNEL_RANGE, MAX_DATA_14, MAX_DATA_BYTE, MILLIS_PER_DAY, RETENTION_LIMIT_RANGE,
};
use serde::{Deserialize, Serialize};

/// Identifies one monitored source for the lifetime of the process.
///
/// Opaque and never displayed: the Sources list shows a name, but two entries
/// can legitimately share one (`IAC Driver Bus 1` appears under both
/// `MIDI sources` and `Spy on output to destinations`). Identity is this id, so
/// selecting one of those entries cannot silently select the other.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SourceId(u32);

impl SourceId {
    /// Wraps a raw discriminator produced by the source catalogue.
    #[must_use]
    pub const fn new(raw: u32) -> Self {
        Self(raw)
    }

    /// The underlying discriminator, for crossing the IPC boundary.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// The two named, collapsible groups in `screenshots/sources.png`.
///
/// A source without a group is a standalone row — `Act as a destination for
/// other programs` in the reference image. Modelling that as `Option` rather
/// than a third group variant keeps "has no parent checkbox" unrepresentable
/// as anything else.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SourceGroupId {
    /// `MIDI sources` — the input ports being watched.
    MidiSources,
    /// `Spy on output to destinations` — traffic observed on the way out.
    SpyOnOutput,
}

impl SourceGroupId {
    /// Every group, in the top-to-bottom order of the reference screenshot.
    pub const ALL: [Self; 2] = [Self::MidiSources, Self::SpyOnOutput];

    /// The group's label, copied verbatim from `screenshots/sources.png`.
    ///
    /// Verbatim matters: the constitution makes the screenshot the design
    /// authority, so this string is normative and not a place for rewording.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::MidiSources => "MIDI sources",
            Self::SpyOnOutput => "Spy on output to destinations",
        }
    }
}

/// A monotonically increasing identifier for one observed event.
///
/// Serves two purposes that both need stability: it is the rendered list's key,
/// and it is the high-water mark that lets the webview discard event batches
/// which were already in flight when a settings change replaced the list.
///
/// # Why `u32` rather than `u64`
///
/// The identifier crosses to JavaScript, which has no 64-bit integer. Sending it
/// as a double would make the generated TypeScript type `number | null` —
/// modelling a null that can never occur and forcing every comparison to check
/// for it. A `u32` crosses as a plain `number` and says something true instead.
///
/// The range is not a practical constraint: ids are session-scoped, and at the
/// simulator's peak rate exhausting a `u32` would take months of uninterrupted
/// monitoring.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EventId(u32);

impl EventId {
    /// The first id handed out after startup.
    pub const FIRST: Self = Self(0);

    /// The next id in sequence.
    ///
    /// Saturates rather than wrapping. Wrapping would silently break ordering
    /// and make the high-water mark discard live events; saturating stalls the
    /// counter instead, which is both visible and unreachable in any real
    /// session. Overflow is not a panic path, because the project forbids one.
    #[must_use]
    pub const fn next(self) -> Self {
        Self(self.0.saturating_add(1))
    }

    /// The underlying value, for crossing the IPC boundary.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// A MIDI channel as the user sees it: 1 through 16.
///
/// Stores the **1-based** display value, not the 0-based wire nibble. The
/// conversion lives in exactly one place ([`Self::to_wire_nibble`]), because an
/// off-by-one that only shows up on channel 16 is precisely the bug a project
/// without tests cannot afford to reintroduce.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChannelNumber(u8);

impl ChannelNumber {
    /// Validates a user-facing channel number.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::ChannelOutOfRange`] when `value` falls outside 1–16,
    /// which happens when the One Channel field is typed into directly. The
    /// caller should reject the entry and leave the previous channel in effect.
    pub fn new(value: u8) -> Result<Self, CoreError> {
        if CHANNEL_RANGE.contains(&value) {
            Ok(Self(value))
        } else {
            Err(CoreError::ChannelOutOfRange { value })
        }
    }

    /// Builds a channel from a status byte's low nibble, which cannot fail.
    ///
    /// A nibble is 0–15 by construction, so this is the one path that needs no
    /// validation — it exists so byte decoding does not have to handle an error
    /// that cannot occur.
    #[must_use]
    pub const fn from_wire_nibble(nibble: u8) -> Self {
        Self((nibble & 0x0F) + 1)
    }

    /// The 0-based nibble to place in a status byte.
    #[must_use]
    pub const fn to_wire_nibble(self) -> u8 {
        self.0 - 1
    }

    /// The 1-based number to display in the Chan column.
    #[must_use]
    pub const fn get(self) -> u8 {
        self.0
    }
}

/// How many events the monitor retains before discarding the oldest.
///
/// Validated on construction so that "Remember up to N events" can never hold a
/// value the application will not actually honour.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetentionLimit(usize);

impl RetentionLimit {
    /// Validates a retention limit against the supported range.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::RetentionOutOfRange`] for zero or for values above
    /// the stated ceiling. The caller should refuse the entry and keep the
    /// previous limit, rather than silently accepting a number it will not honour.
    pub fn new(value: usize) -> Result<Self, CoreError> {
        if RETENTION_LIMIT_RANGE.contains(&value) {
            Ok(Self(value))
        } else {
            Err(CoreError::RetentionOutOfRange { value })
        }
    }

    /// The limit as a count.
    #[must_use]
    pub const fn get(self) -> usize {
        self.0
    }
}

impl Default for RetentionLimit {
    /// The `1000` shown in the reference window before the user touches it.
    fn default() -> Self {
        Self(crate::constants::DEFAULT_RETENTION_LIMIT)
    }
}

/// A seven-bit MIDI data value: note number, velocity, controller value, and so on.
///
/// One type covers all of them because they share an invariant (0–127) and no
/// code path benefits from distinguishing a velocity from a controller value at
/// the type level — the surrounding [`super::message::MidiMessage`] variant
/// already names which is which.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DataByte(u8);

impl DataByte {
    /// The lowest data value.
    pub const ZERO: Self = Self(0);

    /// The highest data value, `127`.
    pub const MAX: Self = Self(MAX_DATA_BYTE);

    /// Builds a data value by discarding the high bit.
    ///
    /// Total by construction: masking cannot fail, so decoding and generation
    /// avoid an error branch that could never be taken.
    #[must_use]
    pub const fn from_masked(raw: u8) -> Self {
        Self(raw & MAX_DATA_BYTE)
    }

    /// The value, 0–127.
    #[must_use]
    pub const fn get(self) -> u8 {
        self.0
    }
}

/// A fourteen-bit MIDI quantity carried as two data bytes.
///
/// Used by Pitch Wheel and Song Position Pointer, which transmit a least
/// significant byte followed by a most significant byte.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Data14(u16);

impl Data14 {
    /// Builds a 14-bit value by discarding the two high bits.
    #[must_use]
    pub const fn from_masked(raw: u16) -> Self {
        Self(raw & MAX_DATA_14)
    }

    /// Splits into the wire pair `(least significant, most significant)`.
    #[must_use]
    pub const fn to_wire_pair(self) -> (u8, u8) {
        ((self.0 & 0x7F) as u8, ((self.0 >> 7) & 0x7F) as u8)
    }

    /// The combined value, 0–16383.
    #[must_use]
    pub const fn get(self) -> u16 {
        self.0
    }
}

/// Wall-clock arrival time, as milliseconds since local midnight.
///
/// The Time column shows `HH:MM:SS.mmm` and nothing coarser or finer, so the
/// domain stores exactly the precision the display needs and no date at all.
/// A monitor session that crosses midnight simply wraps, which matches what the
/// reference application shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Timestamp(u64);

impl Timestamp {
    /// Builds a timestamp, wrapping at the end of the day.
    #[must_use]
    pub const fn from_millis_since_midnight(millis: u64) -> Self {
        Self(millis % MILLIS_PER_DAY)
    }

    /// Formats as `HH:MM:SS.mmm`, the form used in `screenshots/main-screen.png`.
    #[must_use]
    pub fn to_display(self) -> String {
        let millis = self.0 % 1000;
        let total_seconds = self.0 / 1000;
        let seconds = total_seconds % 60;
        let minutes = (total_seconds / 60) % 60;
        let hours = (total_seconds / 3600) % 24;
        format!("{hours:02}:{minutes:02}:{seconds:02}.{millis:03}")
    }
}

/// A validated hexadecimal prefix to match against an event's raw bytes.
///
/// # Why it stores a normalised nibble string
///
/// The user may type `b0 07`, `B007`, or `B0 07` and means the same thing every
/// time, so normalisation (uppercase, whitespace stripped) happens once here
/// rather than at each comparison. Storing *nibbles* rather than bytes is what
/// makes an odd-length prefix like `9` work without a special case: matching is
/// a plain string prefix test, and `"9"` matches `"90"` through `"9F"` for free.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HexPrefix(String);

impl HexPrefix {
    /// Parses and normalises a user-entered prefix.
    ///
    /// # Errors
    ///
    /// - [`CoreError::EmptyHexPrefix`] when the entry holds no hex digits at all.
    /// - [`CoreError::MalformedHexPrefix`] when it contains a character that is
    ///   neither a hex digit nor separating whitespace.
    ///
    /// In both cases the caller must leave the previously applied filter in
    /// effect: a typo should not blank the event list.
    pub fn parse(entry: &str) -> Result<Self, CoreError> {
        let mut normalised = String::with_capacity(entry.len());
        for character in entry.chars() {
            if character.is_whitespace() {
                continue;
            }
            if !character.is_ascii_hexdigit() {
                return Err(CoreError::MalformedHexPrefix {
                    entry: entry.to_owned(),
                });
            }
            normalised.push(character.to_ascii_uppercase());
        }

        if normalised.is_empty() {
            return Err(CoreError::EmptyHexPrefix);
        }
        Ok(Self(normalised))
    }

    /// The normalised nibble string, for prefix comparison and for redisplay.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
