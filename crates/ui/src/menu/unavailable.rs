//! A row whose current option cannot be used: drawn dimmed, with the reason.
//!
//! Neither [`Menu::is_disabled`] nor a [`super::Warning`]. A disabled row cannot
//! be stepped, and this one must be, or a player could never leave a broken
//! option; a warning is about a combination of rows, this is about one option.
//! The option stays in the list, so a custom track that will not load is shown
//! with its reason rather than missing.

use super::{Entry, Menu};

impl Entry {
    /// Why the option this row is on cannot be used, if it cannot.
    #[must_use]
    pub fn unavailable(&self) -> Option<&str> {
        match self {
            Self::Choice {
                values, current, ..
            } => values.get(*current)?.unavailable.as_deref(),
            _ => None,
        }
    }
}

impl Menu {
    /// [`Entry::unavailable`], for the renderers: dim the value and show this
    /// where a warning's message goes.
    #[must_use]
    pub fn unavailable<'a>(&self, entry: &'a Entry) -> Option<&'a str> {
        entry.unavailable()
    }

    /// Whether the current page has a row that can land on an unusable option,
    /// so its note line is kept clear of the rows whether or not one is shown
    /// now (a note that moved the rows when it appeared would shift the list
    /// under the player's cursor). Joins `reserve_note` at `visible_rows`.
    #[must_use]
    pub fn reserves_unavailable_note(&self) -> bool {
        self.page().entries.iter().any(|entry| match entry {
            Entry::Choice { values, .. } => values.iter().any(|v| v.unavailable.is_some()),
            _ => false,
        })
    }
}
