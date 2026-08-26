//! Which of the event table's five columns are shown.

use crate::application::error::CoreError;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// One field of an event row.
///
/// # Why declaration order matters
///
/// The variants are declared in the left-to-right order of
/// `screenshots/main-screen.png`, and display order is derived from that order
/// rather than stored. A column that is hidden and shown again therefore
/// returns to its original position for free — there is no saved position to
/// get out of step with the header.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Column {
    /// Arrival time, `HH:MM:SS.mmm`.
    Time,
    /// Originating source's display name.
    Source,
    /// Human-readable message name.
    Message,
    /// Channel number, blank for messages that carry none.
    Chan,
    /// The message payload, rendered per message type.
    Data,
}

impl Column {
    /// Every column, in display order.
    pub const ALL: [Self; 5] = [
        Self::Time,
        Self::Source,
        Self::Message,
        Self::Chan,
        Self::Data,
    ];

    /// The header label, copied verbatim from the reference window.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Time => "Time",
            Self::Source => "Source",
            Self::Message => "Message",
            Self::Chan => "Chan",
            Self::Data => "Data",
        }
    }

    /// A stable identifier for crossing the IPC boundary.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Time => "time",
            Self::Source => "source",
            Self::Message => "message",
            Self::Chan => "chan",
            Self::Data => "data",
        }
    }
}

/// The set of columns currently shown.
///
/// # Why this never reaches the filtering code
///
/// Hiding a column is a display choice and must not change which events are
/// listed — hiding Chan while One Channel is active still filters by channel.
/// Keeping this type out of the log's view function makes that structural
/// rather than a rule someone has to remember.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ColumnVisibility {
    hidden: HashSet<Column>,
}

impl Default for ColumnVisibility {
    /// All five columns shown, as in the reference window.
    fn default() -> Self {
        Self {
            hidden: HashSet::new(),
        }
    }
}

impl ColumnVisibility {
    /// Builds a visibility set from the columns that should be shown.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::LastColumnVisible`] when `visible` is empty. The
    /// rule lives here rather than in the interface so that no caller — present
    /// or future — can produce a table with no columns.
    pub fn from_visible(visible: &[Column]) -> Result<Self, CoreError> {
        if visible.is_empty() {
            return Err(CoreError::LastColumnVisible);
        }
        let hidden = Column::ALL
            .into_iter()
            .filter(|column| !visible.contains(column))
            .collect();
        Ok(Self { hidden })
    }

    /// Whether this column is displayed.
    #[must_use]
    pub fn is_visible(&self, column: Column) -> bool {
        !self.hidden.contains(&column)
    }

    /// The visible columns, in display order.
    #[must_use]
    pub fn visible(&self) -> Vec<Column> {
        Column::ALL
            .into_iter()
            .filter(|column| self.is_visible(*column))
            .collect()
    }
}
