//! The fixed set of virtual sources, named to match the reference screenshot.

use crate::domain::ids::{SourceGroupId, SourceId};
use crate::domain::source::Source;

/// `IAC Driver Bus 1` under `MIDI sources`.
pub const IAC_BUS_INPUT: SourceId = SourceId::new(1);
/// `MidiKeys` under `MIDI sources` — the busiest source, as in the reference window.
pub const MIDI_KEYS: SourceId = SourceId::new(2);
/// The standalone `Act as a destination for other programs` row.
pub const DESTINATION: SourceId = SourceId::new(3);
/// `IAC Driver Bus 1` under `Spy on output to destinations`.
pub const IAC_BUS_SPY: SourceId = SourceId::new(4);

/// Builds the source list shown in `screenshots/sources.png`.
///
/// # Two entries share a name on purpose
///
/// `IAC Driver Bus 1` appears under both `MIDI sources` and `Spy on output to
/// destinations`, exactly as the reference image shows. They are separate
/// [`SourceId`] values, so ticking one does not tick the other — identity is the
/// id, and the name is presentation only.
///
/// Every source starts selected so a first launch shows traffic immediately.
#[must_use]
pub fn build() -> Vec<Source> {
    vec![
        Source {
            id: IAC_BUS_INPUT,
            name: "IAC Driver Bus 1".to_owned(),
            group: Some(SourceGroupId::MidiSources),
            selected: true,
        },
        Source {
            id: MIDI_KEYS,
            name: "MidiKeys".to_owned(),
            group: Some(SourceGroupId::MidiSources),
            selected: true,
        },
        Source {
            id: DESTINATION,
            name: "Act as a destination for other programs".to_owned(),
            group: None,
            selected: true,
        },
        Source {
            id: IAC_BUS_SPY,
            name: "IAC Driver Bus 1".to_owned(),
            group: Some(SourceGroupId::SpyOnOutput),
            selected: true,
        },
    ]
}
