//! How the event table renders what it has received.
//!
//! # Why these live in the domain rather than the webview
//!
//! The same argument [`super::rendering`] makes, one level up: what belongs in
//! the Time, Message, and Data cells is a fact about MIDI, and a *setting* that
//! selects between two such facts is still one. Putting these enums in the
//! webview would place a MIDI rule on the far side of a serialization boundary
//! and duplicate the 128-entry controller table there.
//!
//! # Why the labels are here
//!
//! `screenshots/setting.jpg` is the design authority, and its strings are
//! normative content rather than styling. Keeping them beside the variants they
//! name is what lets the interface render a label it cannot reword — the same
//! reason [`super::filter`]'s checkbox labels come from Rust. Note the en dash
//! (U+2013) in the two [`ProgramNumbering`] labels: it is what the reference
//! image shows, and a hyphen is a fidelity defect.
//!
//! # Why an unrecognised stored value does not fail the load
//!
//! `StoreSettingsRepository::load` treats an unreadable document as absent, so a
//! single unparseable field would silently discard the user's filters, columns,
//! retention, source selections, and saved requests along with it. Every field
//! here is therefore read through [`forgiving`], which falls back to that one
//! field's default and leaves the rest of the document intact.

use serde::{Deserialize, Deserializer, Serialize};

/// What the Time column shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum TimeFormat {
    /// Wall-clock arrival time, `HH:MM:SS.mmm`.
    #[default]
    ClockTime,
    /// The host clock's own tick count, as a whole number.
    ///
    /// The platform's raw value rather than a normalised one, because the reason
    /// to choose this format is to compare against another tool printing the
    /// same counter.
    HostInteger,
    /// The host clock expressed in seconds.
    HostSeconds,
    /// The host clock expressed in nanoseconds.
    HostNanoseconds,
}

impl TimeFormat {
    /// Every option, in the order `screenshots/setting.jpg` lists them.
    pub const ALL: [Self; 4] = [
        Self::ClockTime,
        Self::HostInteger,
        Self::HostSeconds,
        Self::HostNanoseconds,
    ];

    /// The radio button's label, verbatim from the reference image.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::ClockTime => "Clock time",
            Self::HostInteger => "Host time (integer)",
            Self::HostSeconds => "Host time (seconds)",
            Self::HostNanoseconds => "Host time (nanoseconds)",
        }
    }
}

/// How a note number is written.
///
/// # Why the default is the C3 convention
///
/// `screenshots/setting.jpg` shows `Note (Middle C = C3)` selected, and it is
/// the authority for this tab's defaults. This differs from the convention the
/// monitor used before this feature — see [`super::rendering::note_name`], which
/// records why the earlier reading was superseded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum NoteFormat {
    /// A name and octave placing middle C (note 60) at `C3`.
    #[default]
    NameMiddleC3,
    /// A name and octave placing middle C (note 60) at `C4`.
    NameMiddleC4,
    /// The raw note number, base ten.
    Decimal,
    /// The raw note number, base sixteen.
    Hexadecimal,
}

impl NoteFormat {
    /// Every option, in the order `screenshots/setting.jpg` lists them.
    pub const ALL: [Self; 4] = [
        Self::NameMiddleC3,
        Self::NameMiddleC4,
        Self::Decimal,
        Self::Hexadecimal,
    ];

    /// The radio button's label, verbatim from the reference image.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::NameMiddleC3 => "Note (Middle C = C3)",
            Self::NameMiddleC4 => "Note (Middle C = C4)",
            Self::Decimal => "Decimal number",
            Self::Hexadecimal => "Hexadecimal number",
        }
    }
}

/// How a Control Change's controller number is written.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum ControllerFormat {
    /// The controller's conventional name, falling back to its number when it
    /// has none — see [`super::controller`].
    #[default]
    StandardName,
    /// The controller number, base ten.
    Decimal,
    /// The controller number, base sixteen.
    Hexadecimal,
}

impl ControllerFormat {
    /// Every option, in the order `screenshots/setting.jpg` lists them.
    pub const ALL: [Self; 3] = [Self::StandardName, Self::Decimal, Self::Hexadecimal];

    /// The radio button's label, verbatim from the reference image.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::StandardName => "Standard name",
            Self::Decimal => "Decimal number",
            Self::Hexadecimal => "Hexadecimal number",
        }
    }
}

/// The base every remaining data value is written in.
///
/// Velocities, controller values, pressures, pitch bend amounts, song positions,
/// song selections, and time-code values. Deliberately *not* note numbers,
/// controller numbers, or program numbers: each of those has its own setting,
/// and the reference image gives them separate rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum DataFormat {
    /// Base ten.
    #[default]
    Decimal,
    /// Base sixteen.
    Hexadecimal,
}

impl DataFormat {
    /// Every option, in the order `screenshots/setting.jpg` lists them.
    pub const ALL: [Self; 2] = [Self::Decimal, Self::Hexadecimal];

    /// The radio button's label, verbatim from the reference image.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Decimal => "Decimal number",
            Self::Hexadecimal => "Hexadecimal number",
        }
    }

    /// Writes a value in this base.
    ///
    /// Hexadecimal is uppercase and unpadded, matching how the existing raw-byte
    /// view and the hexadecimal prefix filter already present bytes.
    #[must_use]
    pub fn write(self, value: u16) -> String {
        match self {
            Self::Decimal => value.to_string(),
            Self::Hexadecimal => format!("{value:X}"),
        }
    }
}

/// Whether a Program Change's program is counted from one or from zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum ProgramNumbering {
    /// `1 – 128`: the wire value plus one.
    #[default]
    FromOne,
    /// `0 – 127`: the wire value as it arrived.
    FromZero,
}

impl ProgramNumbering {
    /// Every option, in the order `screenshots/setting.jpg` lists them.
    pub const ALL: [Self; 2] = [Self::FromOne, Self::FromZero];

    /// The radio button's label, verbatim from the reference image.
    ///
    /// The separator is an en dash (U+2013) with a space either side, as the
    /// image shows. A hyphen here is a fidelity defect, not a typographical
    /// preference.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::FromOne => "1 – 128 (Standard)",
            Self::FromZero => "0 – 127 (Less common)",
        }
    }

    /// The number to display for a wire program value.
    #[must_use]
    pub const fn displayed(self, wire: u8) -> u16 {
        match self {
            Self::FromOne => wire as u16 + 1,
            Self::FromZero => wire as u16,
        }
    }
}

/// Whether the monitor's three interpretive conveniences are applied.
///
/// # Why this is an enum rather than a `bool`
///
/// It governs three separate suppressions, and each site that consults it should
/// read as a statement about the mode rather than as `if flag`. An enum also
/// makes those sites exhaustive matches, so a third mode — were one ever
/// added — would be a compile error rather than a silently unhandled `else`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum ExpertMode {
    /// The conveniences are applied: values are formatted per the settings
    /// above, a `Note On` with velocity zero reads as a release, and a zero
    /// timestamp shows the time of receipt.
    #[default]
    Off,
    /// The conveniences are suppressed and what arrived is shown.
    On,
}

impl ExpertMode {
    /// The checkbox's label, verbatim from the reference image.
    pub const LABEL: &'static str = "Expert mode";

    /// The three lines while the box is **unticked**, verbatim from
    /// `screenshots/setting.jpg` and in its order.
    const NOTES_OFF: [&'static str; 3] = [
        "Data formatted according to settings above",
        "Note On with velocity 0 shows as Note Off",
        "Zero timestamp shows time received",
    ];

    /// The three lines while the box is **ticked**, verbatim from
    /// `screenshots/expert-mode.png` and in its order.
    const NOTES_ON: [&'static str; 3] = [
        "Data formatted as raw hexadecimal",
        "Note On with velocity 0 shows as Note On",
        "Zero timestamp shows 0",
    ];

    /// The three lines beneath the checkbox, for the mode currently selected.
    ///
    /// # Why the lines change rather than being fixed
    ///
    /// `screenshots/setting.jpg` and `screenshots/expert-mode.png` capture the
    /// same three lines with different text, so they are not a static caption
    /// describing the checkbox — they are a readout of what the monitor is doing
    /// right now. Each line states the behaviour currently in force, which is
    /// why ticking the box rewrites all three rather than striking them out.
    ///
    /// The two arrays are parallel on purpose: line *n* of one is the opposite
    /// of line *n* of the other, so the list never reorders under the reader.
    #[must_use]
    pub const fn notes(self) -> [&'static str; 3] {
        match self {
            Self::Off => Self::NOTES_OFF,
            Self::On => Self::NOTES_ON,
        }
    }

    /// Whether the box is ticked.
    #[must_use]
    pub const fn is_on(self) -> bool {
        matches!(self, Self::On)
    }
}

impl From<bool> for ExpertMode {
    /// Crosses the IPC boundary as the checkbox it is.
    fn from(ticked: bool) -> Self {
        if ticked {
            Self::On
        } else {
            Self::Off
        }
    }
}

/// The group label to the left of each radio stack, verbatim from the reference
/// image.
pub mod group_label {
    /// Labels the [`super::TimeFormat`] group.
    pub const TIME: &str = "Time format";
    /// Labels the [`super::NoteFormat`] group.
    pub const NOTE: &str = "Note format";
    /// Labels the [`super::ControllerFormat`] group.
    pub const CONTROLLER: &str = "Controller format";
    /// Labels the [`super::DataFormat`] group.
    pub const DATA: &str = "Data format";
    /// Labels the [`super::ProgramNumbering`] group.
    pub const PROGRAM: &str = "Program number";
    /// The second line of the [`super::ProgramNumbering`] group's label, which
    /// the reference image sets beneath the first.
    pub const PROGRAM_SECOND_LINE: &str = "(Decimal)";
}

/// Every display choice, as one value.
///
/// # Why this is `Copy`
///
/// Six enum discriminants. Passing it by value into a renderer costs nothing and
/// spares every rendering signature a lifetime parameter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DisplaySettings {
    /// What the Time column shows.
    #[serde(default, deserialize_with = "forgiving")]
    pub time: TimeFormat,
    /// How note numbers are written.
    #[serde(default, deserialize_with = "forgiving")]
    pub note: NoteFormat,
    /// How controller numbers are written.
    #[serde(default, deserialize_with = "forgiving")]
    pub controller: ControllerFormat,
    /// The base for every remaining value.
    #[serde(default, deserialize_with = "forgiving")]
    pub data: DataFormat,
    /// Whether programs are counted from one or from zero.
    #[serde(default, deserialize_with = "forgiving")]
    pub program: ProgramNumbering,
    /// Whether the monitor's conveniences are suppressed.
    #[serde(default, deserialize_with = "forgiving")]
    pub expert: ExpertMode,
}

impl Default for DisplaySettings {
    /// The states `screenshots/setting.jpg` depicts.
    fn default() -> Self {
        Self {
            time: TimeFormat::ClockTime,
            note: NoteFormat::NameMiddleC3,
            controller: ControllerFormat::StandardName,
            data: DataFormat::Decimal,
            program: ProgramNumbering::FromOne,
            expert: ExpertMode::Off,
        }
    }
}

/// Reads one setting, falling back to its default when the stored form is not
/// recognised.
///
/// # Why this exists rather than `#[serde(default)]` alone
///
/// `default` answers a *missing* key. It does nothing for a key that is present
/// and unparseable — a value written by a later version, or a hand-edited
/// document — which fails the whole struct and, because
/// `StoreSettingsRepository::load` treats an unreadable document as absent,
/// would take every unrelated setting down with it.
///
/// The untagged enum is what makes recovery possible: serde buffers the value,
/// tries the real type, and falls through to [`serde::de::IgnoredAny`], which
/// accepts anything. One bad field therefore costs that field and nothing else.
fn forgiving<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de> + Default,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Outcome<T> {
        Recognised(T),
        Unrecognised(serde::de::IgnoredAny),
    }

    Ok(match Outcome::<T>::deserialize(deserializer)? {
        Outcome::Recognised(value) => value,
        Outcome::Unrecognised(_) => T::default(),
    })
}
