//! What the user is currently looking at, as opposed to what is being monitored.
//!
//! # Why filtering is a view rather than an admission gate
//!
//! Source selection decides what enters the retained log; the settings in this
//! module decide what is *shown* from it. That distinction is what lets a filter
//! change re-reveal events that are already retained: unchecking `Clock` and
//! checking it again brings the retained clock events straight back rather than
//! waiting for new ones, which is what a monitor is expected to do.

use super::event::MidiEvent;
use super::ids::{ChannelNumber, HexPrefix};
use super::message::MessageKind;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Whether the channel radio pair is on All Channels or a single one.
///
/// # Why a sum type rather than a flag plus a number
///
/// "One Channel is selected but no channel is set" is not a state the interface
/// can produce and not one the domain should be able to represent. Carrying the
/// channel inside the variant that needs it makes the invalid combination
/// unspellable instead of merely unlikely.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChannelMode {
    /// Every channel passes, and channel-less messages pass too.
    AllChannels,
    /// Only this channel passes; channel-less messages are excluded entirely.
    OneChannel(ChannelNumber),
}

impl Default for ChannelMode {
    /// `All Channels` is the reference window's default.
    fn default() -> Self {
        Self::AllChannels
    }
}

/// Whether hex prefix matches are the only events shown, or the only ones hidden.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrefixMode {
    /// Show only matching events.
    Include,
    /// Hide only matching events.
    Exclude,
}

impl Default for PrefixMode {
    /// Include reads as the natural default: the user types what they want to see.
    fn default() -> Self {
        Self::Include
    }
}

/// Matches events by a hexadecimal prefix of their raw bytes.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DataPrefixFilter {
    /// Whether matches are kept or dropped.
    pub mode: PrefixMode,
    /// The prefixes to test. An event matches if it matches *any* of them.
    pub prefixes: Vec<HexPrefix>,
}

impl DataPrefixFilter {
    /// Whether this event survives the prefix filter.
    ///
    /// # The empty case
    ///
    /// With no prefixes entered, everything passes in **both** modes. The naive
    /// reading of Include would hide every event when the field is empty, which
    /// would look like the application had broken; an empty filter means "no
    /// opinion", not "match nothing".
    #[must_use]
    pub fn admits(&self, event: &MidiEvent) -> bool {
        if self.prefixes.is_empty() {
            return true;
        }
        let hex = event.raw_hex();
        let matched = self
            .prefixes
            .iter()
            .any(|prefix| hex.starts_with(prefix.as_str()));
        match self.mode {
            PrefixMode::Include => matched,
            PrefixMode::Exclude => !matched,
        }
    }
}

/// Everything the Filter panel currently admits.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FilterSettings {
    /// The message kinds whose checkboxes are ticked.
    pub kinds: HashSet<MessageKind>,
    /// The channel radio pair's state.
    pub channel_mode: ChannelMode,
    /// The hexadecimal prefix filter.
    pub data_filter: DataPrefixFilter,
}

impl Default for FilterSettings {
    /// Every checkbox ticked and `All Channels` selected, matching the reference
    /// filter panel on a fresh launch.
    fn default() -> Self {
        Self {
            kinds: MessageKind::ALL.into_iter().collect(),
            channel_mode: ChannelMode::AllChannels,
            data_filter: DataPrefixFilter::default(),
        }
    }
}

impl FilterSettings {
    /// Whether this event should be shown.
    ///
    /// The three tests are independent and all must pass: the message's kind is
    /// ticked, the channel rule is satisfied, and the prefix filter admits it.
    #[must_use]
    pub fn admits(&self, event: &MidiEvent) -> bool {
        self.kinds.contains(&event.message.kind())
            && self.admits_channel(event)
            && self.data_filter.admits(event)
    }

    /// Whether the channel rule admits this event.
    ///
    /// # Why channel-less messages are excluded under One Channel
    ///
    /// Asking for channel 5 is asking for one instrument's traffic. Clock,
    /// System Exclusive, and the System Common messages belong to no channel, so
    /// letting them through would defeat the point of narrowing — the user would
    /// still be reading the high-rate system stream they were trying to escape.
    fn admits_channel(&self, event: &MidiEvent) -> bool {
        match self.channel_mode {
            ChannelMode::AllChannels => true,
            ChannelMode::OneChannel(wanted) => event.channel() == Some(wanted),
        }
    }

    /// Whether every kind in a category is ticked, none are, or some are.
    ///
    /// Drives the parent checkboxes at the head of each Filter column.
    #[must_use]
    pub fn category_state(
        &self,
        category: super::message::MessageCategory,
    ) -> super::source::CheckState {
        super::source::CheckState::from_children(
            MessageKind::ALL
                .into_iter()
                .filter(|kind| kind.category() == Some(category))
                .map(|kind| self.kinds.contains(&kind)),
        )
    }
}
