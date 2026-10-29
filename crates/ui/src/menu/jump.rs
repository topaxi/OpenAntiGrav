//! Opening a page by id rather than by walking to it: `--menu-page`'s jump
//! and the touch front end's walk-in. Split out of `menu.rs` under the
//! 1,000-line rule in `scripts/check-file-size.py`; a move, with no
//! behaviour change.

use super::Menu;

impl Menu {
    /// Jumps straight to a page, discarding the stack.
    ///
    /// For looking at one page without walking to it - `--menu-page` - rather
    /// than for navigation, which is why the stack is *replaced*: a page reached
    /// this way was not reached through anything, and pretending otherwise would
    /// give it a back destination it never had.
    ///
    /// Returns whether the page exists.
    pub fn open(&mut self, id: &str) -> bool {
        let Some(at) = self.definition.pages.iter().position(|page| page.id == id) else {
            return false;
        };
        self.stack = vec![at];
        self.snap_focus();
        true
    }

    /// Walks into a page, keeping the stack - what activating a submenu row
    /// does, for a caller that arrived from a screen of its own rather
    /// than from the row: Wipeout 2048's front end opening this build's
    /// race box or RACE REMIX from its own mode grid. BACK from the page
    /// then lands on whatever was under it, the way it would from the row.
    ///
    /// Returns whether the page exists.
    pub fn push(&mut self, id: &str) -> bool {
        let Some(at) = self.definition.pages.iter().position(|page| page.id == id) else {
            return false;
        };
        self.stack.push(at);
        self.snap_focus();
        true
    }
}

/// One page of a [`Menu`]'s stack as [`Menu::trail`] records it: kept by ids, so it
/// reopens on a tree rebuilt with rows added or taken away.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Spot {
    /// The page's id.
    pub page: String,
    /// The selected row's id, or its setting key for a row with no id.
    pub row: Option<String>,
    /// The selected row's index, for a row with neither.
    pub index: usize,
}

impl Menu {
    /// The stack, root first, with the row each page has selected.
    #[must_use]
    pub fn trail(&self) -> Vec<Spot> {
        self.stack
            .iter()
            .map(|&at| {
                let page = &self.definition.pages[at];
                let index = self.cursor[at];
                let row = page
                    .meta
                    .get(index)
                    .and_then(|meta| meta.id.clone())
                    .or_else(|| page.entries.get(index)?.setting().map(str::to_string));
                Spot {
                    page: page.id.clone(),
                    row,
                    index,
                }
            })
            .collect()
    }

    /// Reopens what [`Self::trail`] recorded, on this menu's tree: each page by id, the
    /// cursor on the same row by id or setting, else at the same index clamped to the
    /// page. Stops at the first page the tree lacks, keeping what it reopened. Returns
    /// whether every page was found.
    pub fn restore_trail(&mut self, trail: &[Spot]) -> bool {
        let mut stack = Vec::new();
        for spot in trail {
            let Some(at) = self.definition.pages.iter().position(|p| p.id == spot.page) else {
                break;
            };
            let page = &self.definition.pages[at];
            let by_key = spot.row.as_deref().and_then(|key| {
                page.row_with_id(key).or_else(|| {
                    page.entries
                        .iter()
                        .position(|entry| entry.setting() == Some(key))
                })
            });
            let last = page.entries.len().saturating_sub(1);
            self.cursor[at] = by_key.unwrap_or(spot.index.min(last));
            stack.push(at);
        }
        let whole = stack.len() == trail.len();
        if !stack.is_empty() {
            self.stack = stack;
        }
        self.snap_focus();
        whole
    }
}
