//! How the window is painted.
//!
//! # Why this is not a field of [`super::display::DisplaySettings`]
//!
//! Every field of that struct is read by [`super::rendering`], and
//! `set_display_settings` answers with a snapshot because a format change must
//! apply to events captured under the old one. A theme changes no event's text:
//! every colour the window uses is a custom property the stylesheet resolves
//! live, so the rows already on screen repaint without being rebuilt. Putting a
//! theme there would make choosing a colour re-render up to a hundred thousand
//! retained rows to produce byte-identical strings, and would be the one field
//! the renderer ignores.
//!
//! It follows [`super::column`] instead, which is the existing precedent for a
//! setting that shapes the window rather than the text in it: its own module,
//! its own DTO, its own commands, and its own `#[serde(default)]` field on
//! [`crate::application::settings::PersistedSettings`].
//!
//! # Why the labels are here
//!
//! The same reason `display`'s are. The core decides which options exist, so the
//! interface cannot invent one or reword a control. That these two strings are
//! not fixed by a reference image does not make them the webview's to own.

use serde::{Deserialize, Serialize};

/// Which palette the window is painted in.
///
/// # Why the reference look is a named choice rather than the absence of one
///
/// Modelling this as a flag over an unnamed baseline would leave the
/// reproduction of `screenshots/setting.jpg` with no name of its own, and the
/// preferences tab has to offer it as something the user can pick *back*. Two
/// named variants give the radio group two labels and give the stored document a
/// value that says what it means.
///
/// # A note on `Default`
///
/// The variant is called `Default` and the type also derives [`Default`]. That
/// is legal — `Self::Default` inside the inherent impl resolves to the variant,
/// and the derive returns it. It reads oddly for a moment, but the radio button
/// has to say `Default`, so renaming the variant would only move the oddity into
/// [`Self::label`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum Theme {
    /// The macOS light appearance the reference screenshots were captured in.
    #[default]
    Default,
    /// The application icon's palette: neon pink through orange over a deep
    /// plum ground, taken from `app-icon.svg`'s own gradient stops so the window
    /// and the icon are one piece of artwork rather than two guesses at an idea.
    Raver,
}

impl Theme {
    /// Every option, in the order the tab lists them.
    pub const ALL: [Self; 2] = [Self::Default, Self::Raver];

    /// The radio button's label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Default => "Default",
            Self::Raver => "Raver",
        }
    }
}

/// The group label to the left of the radio stack, in the shape
/// [`super::display::group_label`] established.
pub mod group_label {
    /// Labels the [`super::Theme`] group.
    pub const THEME: &str = "Theme";
}

/// Every appearance choice, as one value.
///
/// A struct rather than a bare [`Theme`] because the command that sets it echoes
/// the whole object back, which is what lets the webview reuse the
/// `withOption(settings, groupId, optionId)` shape the `Display` tab already
/// uses. A second appearance setting then extends this rather than changing the
/// command's signature.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct AppearanceSettings {
    /// Which palette the window is painted in.
    ///
    /// Read through [`super::display::forgiving`] for the reason that function
    /// records: a value written by a later version must cost this one field and
    /// not the user's filters, columns, retention, and saved requests.
    #[serde(default, deserialize_with = "crate::domain::display::forgiving")]
    pub theme: Theme,
}
