//! A choice row whose source offers nothing to choose is left out, not drawn dead.
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

use super::{Entry, Menu, RowMeta, ValueSource};

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
        for index in 0..self.definition.pages.len() {
            let page = &mut self.definition.pages[index];
            let (back, wait): (Vec<_>, Vec<_>) = std::mem::take(&mut page.parked)
                .into_iter()
                .partition(|row| matches!(&row.entry, Entry::Choice { source: from, .. } if *from == Some(source)));
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
        for index in 0..self.definition.pages.len() {
            let mut moved = false;
            while let Some(row) = self.definition.pages[index].entries.iter().position(|entry| {
                matches!(entry, Entry::Choice { source: from, .. } if *from == Some(source))
            }) {
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
