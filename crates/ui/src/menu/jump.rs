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
