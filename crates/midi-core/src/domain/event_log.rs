//! The bounded store of retained events.

use super::event::MidiEvent;
use super::filter::FilterSettings;
use super::ids::{RetentionLimit, SourceId};
use std::collections::VecDeque;

/// A fixed-capacity, oldest-first-eviction log of observed events.
///
/// # What this holds, and what it does not
///
/// It holds events from **selected sources**, capped by the retention limit.
/// Message-type, channel, and prefix filters are applied by [`Self::view`] when
/// reading, never when writing. That split is the reason "Remember up to 1000
/// events" describes memory rather than the viewport: narrowing a filter does
/// not throw retained events away, so widening it again brings them back.
#[derive(Debug, Clone)]
pub struct EventLog {
    entries: VecDeque<MidiEvent>,
    limit: RetentionLimit,
}

impl Default for EventLog {
    fn default() -> Self {
        Self::new(RetentionLimit::default())
    }
}

impl EventLog {
    /// Creates an empty log with the given cap.
    #[must_use]
    pub fn new(limit: RetentionLimit) -> Self {
        Self {
            entries: VecDeque::with_capacity(limit.get()),
            limit,
        }
    }

    /// Appends an event, evicting the oldest if that would exceed the cap.
    pub fn push(&mut self, event: MidiEvent) {
        self.entries.push_back(event);
        self.evict_overflow();
    }

    /// Replaces the cap, discarding the excess oldest events immediately.
    ///
    /// Immediacy is the point: trimming lazily as new events arrive would leave
    /// the list showing more rows than the field says it holds.
    pub fn set_limit(&mut self, limit: RetentionLimit) {
        self.limit = limit;
        self.evict_overflow();
    }

    /// The current cap.
    #[must_use]
    pub const fn limit(&self) -> RetentionLimit {
        self.limit
    }

    /// Discards every retained event.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// How many events are retained, before any filter is applied.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether nothing is retained.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Drops retained events belonging to sources that are no longer selected.
    ///
    /// Called when a source is switched off, so that the visible list matches
    /// the current selection rather than showing history from a source the user
    /// has just stopped watching.
    pub fn retain_selected<F>(&mut self, is_selected: F)
    where
        F: Fn(SourceId) -> bool,
    {
        self.entries.retain(|event| is_selected(event.source));
    }

    /// The retained events that pass the filter, oldest first.
    ///
    /// Oldest first is display order: the reference window's timestamps ascend
    /// downward, newest at the bottom.
    pub fn view<'log>(
        &'log self,
        filter: &'log FilterSettings,
    ) -> impl Iterator<Item = &'log MidiEvent> + 'log {
        self.entries.iter().filter(|event| filter.admits(event))
    }

    /// Removes oldest entries until the cap is satisfied.
    fn evict_overflow(&mut self) {
        while self.entries.len() > self.limit.get() {
            self.entries.pop_front();
        }
    }
}
