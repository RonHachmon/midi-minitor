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
    CHANNEL_RANGE, DEFAULT_PUBLISHED_NAME, MAX_DATA_14, MAX_DATA_BYTE, MAX_PUBLISHED_NAME_LEN,
    MAX_REQUEST_NAME_LEN, MILLIS_PER_DAY, RETENTION_LIMIT_RANGE,
};
use serde::{Deserialize, Serialize};

/// Identifies one monitored source for the lifetime of the process.
///
/// Opaque and never displayed: the Sources list shows a name, but two attached
/// devices can legitimately report the same one — two identical controllers, or
/// two ports of one interface. Identity is this id, so selecting one of them
/// cannot silently select the other.
///
/// # Why this is a session handle and not the device's real identity
///
/// This value is minted as ports are discovered and is reassigned freely across
/// launches. It exists to be small, dense, and cheap to send to the webview,
/// which is why it is `u32`. The identity that must survive a quit, a replug,
/// and a reboot is [`SourceKey`], and the two are deliberately different types
/// so neither can be used for the other's job.
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

/// The identity of a source that outlives the session.
///
/// # The problem this solves
///
/// A saved selection must find the same physical port on the next launch, even
/// if the device was moved to a different socket or the machine was rebooted.
/// [`SourceId`] cannot do that: it is minted in discovery order and changes
/// whenever the set of attached devices changes. Persisting it would mean
/// restoring yesterday's choices onto today's arbitrary numbering.
///
/// # Why this never crosses the IPC boundary
///
/// Its variants carry whatever shape each platform uses for a port's identity —
/// a signed integer on one, an opaque system string on another. Sending that to
/// the webview would put a negative number, or a path-like string, into a
/// contract where [`SourceId`] is deliberately a small unsigned integer, and the
/// webview has no use for either — it keys rows on [`SourceId`]. Keeping this
/// type off the wire is precisely what allows the wire type to stay simple.
///
/// # Why the platforms get separate variants rather than one shared shape
///
/// Because no shared shape is true of both. macOS supplies a signed unique id;
/// Windows supplies a device-interface string and no integer equivalent. Forcing
/// one representation would make macOS stringify a number or Windows hash a
/// string, and a hash collision restores the wrong device's selection silently.
///
/// Widening [`Self::Endpoint`] instead of adding a variant was rejected for a
/// harder reason: this type is serialized into the settings file, so changing an
/// existing variant's payload invalidates every settings file already written.
/// Serde's externally tagged default means a *new* variant leaves existing
/// entries deserializing exactly as before — the compatibility is structural
/// rather than something a migration step has to get right.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SourceKey {
    /// A real endpoint, keyed on the identifier the operating system assigns it.
    ///
    /// Signed because the platform's unique-id property is signed, and sparse
    /// rather than sequential — it is a handle to compare, never to count with.
    Endpoint(i32),

    /// A real port, keyed on the device-interface string the system reports.
    ///
    /// # Why a string, when the sibling variant is an integer
    ///
    /// Because the platform offers nothing else that survives a quit, a replug
    /// into a different socket, and a reboot. The enumeration index does not —
    /// it is positional and shifts as devices come and go, so persisting it
    /// would restore yesterday's choices onto today's numbering, which is the
    /// exact failure this whole type exists to prevent.
    ///
    /// The string is opaque and is never displayed or parsed. It is compared,
    /// nothing more. It is stored verbatim so that a human inspecting the
    /// settings file can recognise which device a line refers to — a hashed
    /// stand-in would be smaller and unreadable, and would trade a compile-time
    /// convenience for a silent runtime lie when two hashes collide.
    ///
    /// A platform that reports no interface string for a port cannot form this
    /// key at all; the name-matching fallback that covers it is the caller's
    /// responsibility, not this type's.
    DeviceInterface(String),

    /// A real port that the system offers no identity for beyond its name.
    ///
    /// # Why this is a variant and not a `DeviceInterface` holding a name
    ///
    /// Because it is not one. A device-interface string is an identity the
    /// system guarantees; a display name is a label two ports can share. Storing
    /// a name in the variant that promises stability would make the settings
    /// file claim something untrue, and would leave nothing able to tell the two
    /// cases apart — which is exactly what the weaker case needs, because it
    /// requires different handling.
    ///
    /// **The weakness is the point of naming it.** When two ports present at the
    /// same moment carry this key with the same name and nothing else separates
    /// them, a remembered selection must be applied to *neither*, and both rows
    /// must say so. Guessing one would silently monitor a device the user never
    /// chose — the quiet wrong answer this whole type exists to prevent. A key
    /// that cannot distinguish must therefore be recognisable as such, and only
    /// a distinct variant makes it so.
    PortName(String),

    /// The endpoint this application publishes for other programs to send to.
    ///
    /// # Why a named variant rather than an endpoint id
    ///
    /// An endpoint created at runtime is assigned a fresh unique id on every
    /// launch unless one is set explicitly, so keying it like a real device
    /// would mean persisting an id in order to look up a persisted id. There is
    /// exactly one such endpoint, so naming it makes its identity stable for
    /// free — and makes "the virtual destination" unrepresentable as anything
    /// else.
    VirtualDestination,
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

/// Identifies one send target for the lifetime of the process.
///
/// The counterpart of [`SourceId`], and separate from it for the same reason the
/// two lists are separate: a machine's inputs and its outputs are different sets,
/// and a number that meant one in a call about the other would be a mix-up the
/// compiler could not see.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TargetId(u32);

impl TargetId {
    /// Wraps a raw discriminator produced while scanning targets.
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

/// The identity of a send target that outlives the session.
///
/// # Why this splits three ways
///
/// It mirrors [`SourceKey`], and for the same platform reasons. CoreMIDI gives
/// every endpoint a stable integer that survives a replug, so macOS can key on
/// that. WinMM output devices have no such identifier — they are addressed by an
/// index that shifts as devices come and go — so Windows must key on the name it
/// reports. The published source is neither: it is this application's own
/// endpoint, so its identity is a constant rather than anything the machine
/// assigns.
///
/// # Why it never crosses the IPC boundary
///
/// The webview addresses targets by [`TargetId`], which is small and dense. This
/// type exists to be written to disk and matched against a later scan, and
/// sending it to the interface would invite the interface to reason about it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TargetKey {
    /// A CoreMIDI endpoint, keyed by its stable unique identifier.
    Endpoint(i32),
    /// A WinMM output device, keyed by the name the platform reports.
    DeviceName(String),
    /// This application's own published source.
    PublishedSource,
}

/// Identifies one entry in the session's record of sends.
///
/// Monotonic across the session, so a re-send names exactly the entry the user
/// pointed at even after older entries have been evicted and the list has
/// shifted. Keying on position would break precisely when the list is longest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SendRecordId(u32);

impl SendRecordId {
    /// Wraps a raw discriminator minted when a send is recorded.
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

/// The name of a request in the library, and its identity.
///
/// # Why the name is the identity
///
/// Names are unique across the whole library, so nothing is gained by minting an
/// id beside them — and a great deal is lost. Keying on the name means deletion
/// needs no index and no positional contract, and a view asking to delete
/// something already gone gets [`CoreError::UnknownRequest`] rather than deleting
/// whatever now sits in that position. This is the reasoning the data prefix
/// rules already use for their prefix.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RequestName(String);

impl RequestName {
    /// Parses and trims a user-entered request name.
    ///
    /// # Errors
    ///
    /// [`CoreError::MalformedRequestName`] when the entry is empty once trimmed,
    /// or longer than [`MAX_REQUEST_NAME_LEN`]. The caller keeps the library as
    /// it was: a name that cannot be shown is refused rather than silently
    /// shortened into one the user did not choose.
    pub fn parse(entry: &str) -> Result<Self, CoreError> {
        let trimmed = entry.trim();
        if trimmed.is_empty() || trimmed.chars().count() > MAX_REQUEST_NAME_LEN {
            return Err(CoreError::MalformedRequestName {
                max: MAX_REQUEST_NAME_LEN,
            });
        }
        Ok(Self(trimmed.to_owned()))
    }

    /// Wraps a name this crate itself defines.
    ///
    /// Used only for the built-in library, whose names are literals in this
    /// crate rather than anything a user typed, so there is nothing to validate.
    #[must_use]
    pub fn built_in(name: &'static str) -> Self {
        Self(name.to_owned())
    }

    /// The name, for display and for comparison.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The name the published source carries, as other programs see it.
///
/// # Why this is validated and [`String`] is not enough
///
/// Programs that receive from this source remember their settings against this
/// string. An empty name would give them nothing to remember, and one longer than
/// their device lists can show would be truncated somewhere this application
/// cannot see. Both are refused at the one place the value is built.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublishedName(String);

impl PublishedName {
    /// Parses and trims a user-entered publication name.
    ///
    /// # Errors
    ///
    /// [`CoreError::MalformedPublicationName`] when the entry is empty once
    /// trimmed, or longer than [`MAX_PUBLISHED_NAME_LEN`]. The caller keeps the
    /// name already in force.
    pub fn parse(entry: &str) -> Result<Self, CoreError> {
        let trimmed = entry.trim();
        if trimmed.is_empty() || trimmed.chars().count() > MAX_PUBLISHED_NAME_LEN {
            return Err(CoreError::MalformedPublicationName {
                max: MAX_PUBLISHED_NAME_LEN,
            });
        }
        Ok(Self(trimmed.to_owned()))
    }

    /// The name, for display and for the platform call that publishes it.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for PublishedName {
    /// The product name, matching what the virtual destination already uses.
    fn default() -> Self {
        Self(DEFAULT_PUBLISHED_NAME.to_owned())
    }
}
