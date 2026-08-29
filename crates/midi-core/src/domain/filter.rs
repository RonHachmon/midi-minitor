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
use crate::application::error::CoreError;
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

/// One prefix and what it does to the events that match it.
///
/// # Why the prefix is the identity
///
/// [`DataPrefixFilter::add`] refuses a prefix that is already listed, under
/// either kind, so no two rules in a list can carry the same one. That makes the
/// prefix a key: deletion names it directly, and needs neither a generated id —
/// which would exist to tell apart rows that cannot be alike — nor a position,
/// which would make the interface's contract depend on ordering it does not show.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DataPrefixRule {
    /// The normalised nibble prefix this rule tests.
    pub prefix: HexPrefix,
    /// Whether matching events are the only ones shown, or the only ones hidden.
    pub kind: PrefixMode,
}

impl DataPrefixRule {
    /// Builds a rule from an already-parsed prefix.
    #[must_use]
    pub const fn new(prefix: HexPrefix, kind: PrefixMode) -> Self {
        Self { prefix, kind }
    }

    /// Whether this rule's prefix begins an event's raw hex.
    ///
    /// Matching is on nibbles rather than bytes, which is what lets an
    /// odd-length prefix like `9` match `90` through `9F` without a special case.
    #[must_use]
    pub fn matches(&self, raw_hex: &str) -> bool {
        raw_hex.starts_with(self.prefix.as_str())
    }

    /// Whether these two rules' prefixes can ever match the same bytes.
    ///
    /// True exactly when one prefix begins the other, equality included. No other
    /// pair can share a match: if two prefixes both begin the same string, then
    /// the shorter necessarily begins the longer. That fact is what makes the
    /// clash rule sufficient rather than merely helpful — it is the whole reason
    /// [`DataPrefixFilter::admits`] can do without a precedence rule.
    #[must_use]
    fn overlaps(&self, other: &Self) -> bool {
        let (this, that) = (self.prefix.as_str(), other.prefix.as_str());
        this.starts_with(that) || that.starts_with(this)
    }
}

/// The data prefix rules currently in force.
///
/// # Why a list of rules rather than one mode over many prefixes
///
/// The field this replaced applied a single `Show only` / `Hide` choice to every
/// prefix in it, so dropping one prefix meant retyping the rest and nothing on
/// screen said what was actually in effect. A rule carries its own kind, is added
/// on its own, and is deleted on its own.
///
/// # The invariant every method here depends on
///
/// No two rules share a prefix, and no two rules of *different* kinds have
/// overlapping prefixes. [`Self::add`] is the only way in, and it enforces both.
/// See [`Self::admits`] for what that buys.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "StoredDataPrefixFilter")]
pub struct DataPrefixFilter {
    /// The rules in force, in the order they were added.
    pub rules: Vec<DataPrefixRule>,
}

impl DataPrefixFilter {
    /// Adds a rule, or refuses it because it clashes with one already listed.
    ///
    /// # Why a clash is refused rather than resolved
    ///
    /// The alternative is a precedence rule — "hide beats show only", or the
    /// reverse — which every reader then has to hold in their head to predict
    /// what the list does. Refusing the pair at the point of adding keeps the
    /// list free of contradictions instead, so the question never arises. It also
    /// puts the explanation where the user can act on it: at the moment they
    /// typed the rule, not later when rows are missing.
    ///
    /// # What counts as a clash
    ///
    /// Two things, and deliberately not a third:
    ///
    /// - **The same prefix, under either kind.** Repeating it says nothing new,
    ///   and pairing it with the opposite kind contradicts outright. Either way
    ///   the prefix would stop identifying one rule.
    /// - **Overlapping prefixes of opposite kinds** — `Show only 90` beside
    ///   `Hide 9`, or beside `Hide 9012`. One rule would swallow the other.
    /// - **Overlapping prefixes of the same kind is permitted.** `Show only 9`
    ///   beside `Show only 90` is redundant, not contradictory, and refusing it
    ///   would stop a user narrowing or broadening a set they are building.
    ///
    /// The rule is symmetric: both tests it performs are symmetric in the two
    /// rules, so adding B to a list holding A clashes exactly when adding A to a
    /// list holding B would.
    ///
    /// # Errors
    ///
    /// - [`CoreError::DuplicatePrefixRule`] when the prefix is already listed
    ///   under either kind.
    /// - [`CoreError::ContradictoryPrefixRule`] when it overlaps a listed prefix
    ///   of the opposite kind.
    ///
    /// Nothing is mutated on either path: the checks run to completion before the
    /// rule is pushed, so a refusal leaves the list exactly as it was and the
    /// filter in force keeps running.
    pub fn add(&mut self, rule: DataPrefixRule) -> Result<(), CoreError> {
        if let Some(existing) = self
            .rules
            .iter()
            .find(|listed| listed.prefix == rule.prefix)
        {
            return Err(CoreError::DuplicatePrefixRule {
                prefix: rule.prefix.as_str().to_owned(),
                existing_kind: existing.kind,
            });
        }

        if let Some(existing) = self
            .rules
            .iter()
            .find(|listed| listed.kind != rule.kind && listed.overlaps(&rule))
        {
            return Err(CoreError::ContradictoryPrefixRule {
                prefix: rule.prefix.as_str().to_owned(),
                kind: rule.kind,
                existing_prefix: existing.prefix.as_str().to_owned(),
                existing_kind: existing.kind,
            });
        }

        self.rules.push(rule);
        Ok(())
    }

    /// Removes the rule carrying this prefix.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::UnknownPrefixRule`] when no rule carries it, which
    /// means the caller is working from a stale copy of the list.
    pub fn remove(&mut self, prefix: &HexPrefix) -> Result<(), CoreError> {
        let before = self.rules.len();
        self.rules.retain(|rule| &rule.prefix != prefix);
        if self.rules.len() == before {
            return Err(CoreError::UnknownPrefixRule {
                prefix: prefix.as_str().to_owned(),
            });
        }
        Ok(())
    }

    /// Whether this event survives the rules.
    ///
    /// An event is shown when it matches no `Exclude` rule and — if any
    /// `Include` rule exists — matches one of them.
    ///
    /// # Why there is no precedence between the two kinds
    ///
    /// Because the two tests below can never disagree about one event. Two
    /// prefixes match the same bytes only when one begins the other
    /// ([`DataPrefixRule::overlaps`]), and [`Self::add`] refuses exactly that
    /// pair across kinds. So no event can match both an `Include` rule and an
    /// `Exclude` rule, and asking which wins is asking about a state the list
    /// cannot reach. The contradiction is prevented where the user can be told
    /// about it, rather than resolved silently here.
    ///
    /// # A consequence, and it is intended
    ///
    /// While any `Include` rule is in the list, the `Exclude` rules cannot change
    /// what is shown: anything they would remove has already been excluded for
    /// matching no `Include` rule. That follows from the same invariant, and it is
    /// the accepted cost of preventing contradictions rather than ranking them.
    /// It is **not** a defect to be repaired by introducing the precedence rule
    /// the paragraph above explains away.
    ///
    /// # The empty case
    ///
    /// With no rules, everything passes. The naive reading of a show-only list
    /// would hide every event when the list is empty, which would look like the
    /// application had broken; an empty list means "no opinion", not "match
    /// nothing".
    #[must_use]
    pub fn admits(&self, event: &MidiEvent) -> bool {
        if self.rules.is_empty() {
            return true;
        }
        let hex = event.raw_hex();

        let hidden = self
            .rules
            .iter()
            .any(|rule| rule.kind == PrefixMode::Exclude && rule.matches(&hex));
        if hidden {
            return false;
        }

        let mut shown = self
            .rules
            .iter()
            .filter(|rule| rule.kind == PrefixMode::Include)
            .peekable();
        shown.peek().is_none() || shown.any(|rule| rule.matches(&hex))
    }
}

/// The two shapes this filter has ever had on disk.
///
/// # Why the older one is still read
///
/// [`crate::application::settings::PersistedSettings`] is one document holding
/// the user's source selections, hidden columns, and retention limit as well as
/// this filter. The settings repository treats a document it cannot parse as
/// absent — the right rule for one that is genuinely unreadable, and the wrong
/// one here, where it would silently discard all of that to avoid converting two
/// fields. Untagged rather than versioned because the two shapes share no field
/// names, so neither can satisfy the other's requirements and no version number
/// has to be threaded through a document that has never carried one.
#[derive(Deserialize)]
#[serde(untagged)]
enum StoredDataPrefixFilter {
    /// Written by this version onwards: a kind per rule.
    Rules {
        /// The rules as stored.
        rules: Vec<DataPrefixRule>,
    },
    /// Written before rules existed: one mode over a list of prefixes.
    Legacy {
        /// The mode that applied to every prefix.
        mode: PrefixMode,
        /// The prefixes it applied to.
        prefixes: Vec<HexPrefix>,
    },
}

impl From<StoredDataPrefixFilter> for DataPrefixFilter {
    /// Converts either stored shape into the current one.
    ///
    /// # Why the legacy arm deduplicates
    ///
    /// The old field was a free-text entry and accepted `90 90`, which the rule
    /// list may not hold: a repeated prefix would break the identity that
    /// deletion relies on. Keeping the first occurrence is enough — the entries
    /// were identical in every respect, including their mode.
    ///
    /// A migrated list can never breach the clash rule for any other reason:
    /// every rule takes the same kind, and same-kind overlaps are permitted.
    fn from(stored: StoredDataPrefixFilter) -> Self {
        match stored {
            StoredDataPrefixFilter::Rules { rules } => Self { rules },
            StoredDataPrefixFilter::Legacy { mode, prefixes } => {
                let mut rules: Vec<DataPrefixRule> = Vec::with_capacity(prefixes.len());
                for prefix in prefixes {
                    if !rules.iter().any(|rule| rule.prefix == prefix) {
                        rules.push(DataPrefixRule::new(prefix, mode));
                    }
                }
                Self { rules }
            }
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
