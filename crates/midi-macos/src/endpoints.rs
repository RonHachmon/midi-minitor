//! Reading the machine's real MIDI endpoints and naming them as the system does.
//!
//! # Why names are taken verbatim
//!
//! Every name here comes from the operating system and is passed through
//! untouched — no trimming, no title-casing, no deduplicating, no appending an
//! index to tell two identical names apart. Someone reading this list is trying
//! to match it against Audio MIDI Setup or against the label printed on a box,
//! and a "tidier" name is a name that no longer matches anything.
//!
//! Two ports genuinely may share a display name. They stay separate rows,
//! distinguished by identity rather than by text, which is exactly why identity
//! and name are different fields on a source.

use coremidi::{
    Destination, Destinations, Properties, PropertyGetter, Source as CoreSource, Sources,
};
use midi_core::domain::ids::{SourceGroupId, SourceId, SourceKey, TargetId, TargetKey};
use midi_core::domain::source::{Availability, Source};
use midi_core::domain::target::{Target, TargetKind};

/// One endpoint as the operating system describes it.
///
/// # Why this exists rather than mapping inline
///
/// Reading properties can fail per property, and the fallbacks differ: a missing
/// name is cosmetic, a missing unique id is not. Naming the intermediate step
/// gives those decisions one place to live instead of scattering them through an
/// enumeration loop.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EndpointSnapshot {
    /// The system-assigned identifier, stable across replug and reboot.
    pub unique_id: i32,
    /// The system's own display name, used verbatim.
    pub display_name: String,
    /// Whether the system reports the device as currently offline.
    pub offline: bool,
}

impl EndpointSnapshot {
    /// Reads one endpoint's properties.
    ///
    /// Returns [`None`] when the endpoint has no unique id. That should not
    /// happen — the platform assigns one to every object — but an endpoint that
    /// cannot be identified cannot have a selection saved against it either, so
    /// skipping it is better than inventing an identity that will not survive
    /// the next launch.
    fn read(endpoint: &CoreSource) -> Option<Self> {
        let unique_id: i32 = Properties::unique_id().value_from(endpoint).ok()?;

        // A nameless endpoint is unusual but harmless: the row still selects and
        // still attributes events correctly, it just reads awkwardly. Failing the
        // whole endpoint over a cosmetic property would hide a working device.
        let display_name: String = Properties::display_name()
            .value_from(endpoint)
            .or_else(|_| Properties::name().value_from(endpoint))
            .unwrap_or_else(|_| format!("Unnamed endpoint {unique_id}"));

        let offline: bool = Properties::offline().value_from(endpoint).unwrap_or(false);

        Some(Self {
            unique_id,
            display_name,
            offline,
        })
    }

    /// Turns this endpoint into the domain's view of a source.
    #[must_use]
    pub fn to_source(&self, id: SourceId) -> Source {
        Source {
            id,
            key: SourceKey::Endpoint(self.unique_id),
            name: self.display_name.clone(),
            group: Some(SourceGroupId::MidiSources),
            selected: false,
            availability: if self.offline {
                Availability::Absent
            } else {
                Availability::Open
            },
        }
    }
}

/// Every MIDI input endpoint the system currently reports.
///
/// One entry per *port*, not per physical device: an interface exposing four
/// ports appears as four selectable rows, which is how the system presents them
/// and how anyone routing MIDI thinks about them.
#[must_use]
pub fn enumerate() -> Vec<(EndpointSnapshot, CoreSource)> {
    Sources
        .into_iter()
        .filter_map(|source| EndpointSnapshot::read(&source).map(|snapshot| (snapshot, source)))
        .collect()
}

/// One destination as the operating system describes it.
///
/// # Why this is separate from [`EndpointSnapshot`]
///
/// The two read the same properties, but they describe different sets and are
/// consumed by different ports: a source is something to listen to, and a
/// destination is somewhere to send. Sharing one type would mean a value that
/// could be handed to the wrong half of the adapter and only fail at run time.
/// The properties they read overlapping is not a reason to conflate them.
///
/// Note what is missing: no `offline` flag. A destination that is not there is
/// simply absent from the next scan, because the send screen shows one chosen
/// target rather than a remembered list, and telling the user it has gone is the
/// send's job rather than the list's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DestinationSnapshot {
    /// The system-assigned identifier, stable across replug and reboot.
    pub unique_id: i32,
    /// The system's own display name, used verbatim, for the same reason source
    /// names are.
    pub display_name: String,
}

impl DestinationSnapshot {
    /// Reads one destination's properties.
    ///
    /// Returns [`None`] when the destination has no unique id, for the reason
    /// [`EndpointSnapshot::read`] does: a target that cannot be identified cannot
    /// have a choice saved against it either.
    fn read(endpoint: &Destination) -> Option<Self> {
        let unique_id: i32 = Properties::unique_id().value_from(endpoint).ok()?;

        let display_name: String = Properties::display_name()
            .value_from(endpoint)
            .or_else(|_| Properties::name().value_from(endpoint))
            .unwrap_or_else(|_| format!("Unnamed destination {unique_id}"));

        Some(Self {
            unique_id,
            display_name,
        })
    }

    /// Turns this destination into the domain's view of a target.
    #[must_use]
    pub fn to_target(&self, id: TargetId) -> Target {
        Target::new(
            id,
            TargetKey::Endpoint(self.unique_id),
            self.display_name.clone(),
            TargetKind::Destination,
        )
    }
}

/// Every MIDI destination the system currently reports.
///
/// One entry per *port*, as with sources, and for the same reason: that is how
/// the system presents them and how anyone routing MIDI thinks about them.
#[must_use]
pub fn enumerate_destinations() -> Vec<(DestinationSnapshot, Destination)> {
    Destinations
        .into_iter()
        .filter_map(|destination| {
            DestinationSnapshot::read(&destination).map(|snapshot| (snapshot, destination))
        })
        .collect()
}
