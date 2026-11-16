//! A choice row whose source offers nothing to choose is left out, not drawn dead, and so is
//! an action row whose action is not on offer ([`Menu::offer`]).
//!
//! Not [`Menu::is_disabled`]: that greys a row because *another setting* is
//! off, and stays on screen so the player can find what took it away. This is
//! for a choice the player's discs or the booted title leave empty (MUSIC
//! SOURCE with one disc, VARIANT on a team with one directory); a greyed row
//! there promises a choice that does not exist.
//!
//! [`Menu::supply`] re-decides it on every call, because what a source offers
//! can change mid-session (VARIANT follows the TEAM row). The row leaves
//! [`Page::entries`] itself, and not only the drawing, so the row count, the
//! scroll window, pad movement and the pointer hit-test (all of which read
//! `entries`) agree that it is not there. It waits in [`Page::parked`] and
//! returns to its old place when the source offers a choice again.

use super::{Action, Entry, Menu, RowMeta, ValueSource};

/// A row taken out of a page, with where it sat.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Parked {
    /// Its index among the page's rows with every parked row in place.
    at: usize,
    entry: Entry,
    subtitle: Option<String>,
    meta: RowMeta,
}

impl Menu {
    /// Puts back every parked row that edits `source`, in its old place.
    pub(super) fn unhide(&mut self, source: ValueSource) {
        self.unhide_where(
            |entry| matches!(entry, Entry::Choice { source: from, .. } if *from == Some(source)),
        );
    }

    /// Puts back every parked row `matches` names, in its old place.
    fn unhide_where(&mut self, matches: impl Fn(&Entry) -> bool) {
        for index in 0..self.definition.pages.len() {
            let page = &mut self.definition.pages[index];
            let (back, wait): (Vec<_>, Vec<_>) = std::mem::take(&mut page.parked)
                .into_iter()
                .partition(|row| matches(&row.entry));
            page.parked = wait;
            if back.is_empty() {
                continue;
            }
            for row in back {
                let at = row.at.min(page.entries.len());
                page.entries.insert(at, row.entry);
                page.subtitles.insert(at, row.subtitle);
                page.meta.insert(at, row.meta);
                if self.cursor[index] >= at {
                    self.cursor[index] += 1;
                }
            }
            self.refocus(index);
        }
    }

    /// Takes out of every page the rows that edit `source`.
    pub(super) fn hide(&mut self, source: ValueSource) {
        self.hide_where(
            |entry| matches!(entry, Entry::Choice { source: from, .. } if *from == Some(source)),
        );
    }

    /// Whether the rows that fire `action` are on their pages or parked. A row that is
    /// only there while something is true (PHOTO MODE while a race is parked) is offered
    /// by the caller whenever that changes; parked, it leaves the row count, the cursor
    /// window and the pointer's hit-test, as a choice with nothing to offer does.
    pub fn offer(&mut self, action: Action, offered: bool) {
        let fires = move |entry: &Entry| matches!(entry, Entry::Run { action: fired, .. } if *fired == action);
        if offered {
            self.unhide_where(fires);
        } else {
            self.hide_where(fires);
        }
    }

    /// Takes out of every page the rows `matches` names.
    fn hide_where(&mut self, matches: impl Fn(&Entry) -> bool) {
        for index in 0..self.definition.pages.len() {
            let mut moved = false;
            while let Some(row) = self.definition.pages[index]
                .entries
                .iter()
                .position(&matches)
            {
                let page = &mut self.definition.pages[index];
                let mut at = row;
                for parked in &page.parked {
                    if parked.at <= at {
                        at += 1;
                    }
                }
                let parked = Parked {
                    at,
                    entry: page.entries.remove(row),
                    subtitle: page.subtitles.remove(row),
                    meta: page.meta.remove(row),
                };
                let place = page.parked.partition_point(|other| other.at < at);
                page.parked.insert(place, parked);
                let cursor = &mut self.cursor[index];
                if *cursor > row {
                    *cursor -= 1;
                }
                *cursor = (*cursor).min(page.entries.len().saturating_sub(1));
                moved = true;
            }
            if moved {
                self.refocus(index);
            }
        }
    }

    /// Re-sizes a page's block-growth fractions after its rows changed.
    fn refocus(&mut self, page: usize) {
        let rows = self.definition.pages[page].entries.len();
        self.focus[page] = (0..rows).map(|row| self.focus_target(page, row)).collect();
    }
}
