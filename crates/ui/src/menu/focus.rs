//! How a selected entry's block grows: the per-row focus fraction, its
//! easing, and the page's arrival counter the strip's underline reads.
//!
//! An `impl Menu` of its own rather than more of `menu.rs`, which
//! `scripts/check-file-size.py` baselines and lets shrink but not grow.
//! Everything here is state Wipeout HD's menu widgets keep per entry -
//! `HorizMenu_LayoutBlocks`, `VertMenu_LayoutBlocks` and `List_Update` on
//! `docs/ghidra/functions/ps3-hdfury-eu/menu-blocks.md` - carried on every
//! title's [`Menu`] because it is a few floats a page, and read only by a
//! skin that has blocks.
//!
//! # The two fields, and when each moves
//!
//! [`Menu::focus`] is how far each row's block has grown toward its selected
//! width, `0.0` to `1.0`, per page and parallel to the cursor for the reason
//! the cursor is per page. A fraction rather than a width so the drawing
//! side owns what it is worth in units. It eases in [`Menu::tick_focus`]
//! and snaps whenever a page becomes the one on screen - the original
//! resets its widths in the widget's `OnEnable`, so a page arrives with its
//! selected block already wide, and only a cursor *move* within a page is
//! animated.
//!
//! [`Menu::arrival`] counts ticks since that page arrived, or is `None`
//! once [`Menu::settle`] has said the arrival is over. The strip's underline
//! reads it: `HorizMenu_LayoutBlocks` slides the mark in from the right over
//! the first half-second of a page and blinks it on a counter, both reset
//! when the page arrives.

use super::Menu;

impl Menu {
    /// Ticks since the page on screen arrived - zero on the tick it did -
    /// or `None` after [`Self::settle`].
    #[must_use]
    pub fn ticks_since_arrival(&self) -> Option<u32> {
        self.arrival
    }

    /// Ends the page's arrival: every block at its target width, the
    /// underline where it comes to rest, its blink in its visible phase.
    ///
    /// For a still. `--menu-page` runs no clock, so without this it would
    /// draw the tick a page arrives on - the underline off at the strip's
    /// far right, mid-slide - which is a true frame of the original and a
    /// useless one to review a layout by. A live menu never calls it.
    pub fn settle(&mut self) {
        self.snap_focus();
        self.arrival = None;
    }

    /// How far `row`'s block has grown toward its selected width, `0.0` to
    /// `1.0`. See [`Self::focus`].
    ///
    /// `1.0` for the selected row and `0.0` for every other on a page that
    /// has not been ticked since it arrived, which is what a still drawn
    /// through `--menu-page` shows; in between only while
    /// [`Self::tick_focus`] is easing a cursor move.
    #[must_use]
    pub fn focus_of(&self, row: usize) -> f32 {
        self.focus[self.current()].get(row).copied().unwrap_or(0.0)
    }

    /// Eases every block on the page on screen toward its target by `ease`
    /// of the remaining distance: `1.0` for the selected row, `0.0` for the
    /// rest.
    ///
    /// Called once per tick by whoever owns the menu, with the title's own
    /// rate ([`oag_title::MenuBlocks::ease`], a sixth on Wipeout HD) - a
    /// fixed step, never the wall clock, for the reason `anim.rs` gives. A
    /// block within a thousandth of its target snaps to it, so the tail of
    /// an exponential approach does not run forever at sub-pixel widths.
    pub fn tick_focus(&mut self, ease: f32) {
        if let Some(ticks) = &mut self.arrival {
            *ticks = ticks.saturating_add(1);
        }
        let page = self.current();
        let selected = self.cursor[page];
        for (row, focus) in self.focus[page].iter_mut().enumerate() {
            let target = if row == selected { 1.0 } else { 0.0 };
            *focus += (target - *focus) * ease;
            if (target - *focus).abs() < 1e-3 {
                *focus = target;
            }
        }
    }

    /// Snaps the page on screen's blocks to their targets - what the
    /// original's `OnEnable` reset does when a page arrives.
    pub(super) fn snap_focus(&mut self) {
        let page = self.current();
        let rows = self.definition.pages[page].entries.len();
        self.focus[page] = snapped(rows, self.cursor[page]);
        self.arrival = Some(0);
    }
}

/// A page's block-growth fractions with no easing in flight: `1.0` at
/// `selected`, `0.0` everywhere else. See [`Menu::focus_of`].
pub(super) fn snapped(rows: usize, selected: usize) -> Vec<f32> {
    (0..rows)
        .map(|row| if row == selected { 1.0 } else { 0.0 })
        .collect()
}
