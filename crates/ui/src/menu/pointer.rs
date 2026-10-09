//! A menu page answering a mouse or a finger: where its rows are, and what a
//! move, a click, a wheel and a secondary button do to the cursor.
//!
//! **Ours, with no counterpart on any disc** - see [`crate::pointer`] for
//! the vocabulary and for why. An `impl Menu` of its own rather than more of
//! `menu.rs`, which `scripts/check-file-size.py` baselines and lets shrink
//! but not grow, and because nothing here changes how a pad drives the
//! same page: [`Menu::update`] is untouched, and a `Menu` that is never
//! handed a pointer behaves exactly as it always has.
//!
//! # The rules, and why each is what it is
//!
//! - **Hover selects.** Moving the pointer onto a row puts the cursor on it,
//!   so the row brightens - or, on Wipeout HD, its block grows through
//!   `focus.rs`'s easing - before anything is pressed. Only on a tick the
//!   pointer actually *moved*, so a mouse left resting over a row does not
//!   fight the keyboard for the cursor.
//! - **A click activates what is under it**, whether or not the cursor was
//!   already there: it is one gesture, "that one". On a submenu, an action
//!   or a back row that is what cross does. On a choice or a toggle it
//!   steps the value forward the way cross does, unless the click landed on
//!   one of HD's step arrows, which step the way it points.
//! - **On a page longer than the screen, the wheel scrolls the view** one
//!   row per detent, the way a desktop list does, and **a finger scrolls
//!   it** through [`crate::kinetic`]: the list follows the finger, a flick
//!   coasts on after the lift and comes to rest on a row, and a touch-down
//!   catches a coasting list without activating anything. The window moves
//!   and the cursor is pushed only as far as keeps it inside the new
//!   window, so the selection and the view stay coherent the way a pad walk
//!   keeps them; nothing is selected or activated by either (the
//!   composition root reports no position or click for a drag). Both stop
//!   at the ends rather than wrapping. Until 2026-10-07 the wheel walked
//!   the cursor instead, which the maintainer found wrong on a long page:
//!   the rows moved under a list that should have moved itself.
//!   **Chosen, not measured**: one row per detent. The view moves in whole
//!   rows - the row nearest the finger's offset - because a page draws
//!   from a whole first row and `Draw` has no clip to cut a row at the
//!   header or the footer, so the rubber band past an end is felt (the
//!   pull gives less) rather than seen.
//! - **On a page that fits, the wheel walks the cursor** one row per detent
//!   and stops at the ends, since there is no view to move; a drag there
//!   does nothing.
//! - **The secondary button is back**, which is what circle is on the same
//!   page. On a strip it is the same.
//! - **A disabled row is selectable and inert**, exactly as it is for a pad:
//!   [`Menu::adjust`] refuses it, and a click goes through the same gate.
//!
//! # Where the rows are
//!
//! [`regions`] asks the same two modules that draw the page - [`super::rows`]
//! and [`super::strip`] - for the rects, computed from the same arithmetic
//! the drawing uses. The alternative, a second copy of the layout kept in
//! step by hand, is the drift `rows.rs` already refuses between
//! `Menu::scroll` and the skin. Only the rows on screen get regions, off
//! the same window `draw` shows.

use crate::kinetic::{Extent, Kinetic};
use crate::pointer::{Pointer, contains};

use super::{Frame, Menu, MenuEvent, Skin, strip};

/// What a click on a region does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Part {
    /// The row itself: activate it, the way cross does.
    Row,
    /// A step arrow pointing back: move the row's value one step back.
    StepBack,
    /// A step arrow pointing forward: move the row's value one step on.
    StepForward,
}

/// One rect a pointer can land on, and the row and part it belongs to.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Region {
    /// Index into the page's entries.
    pub row: usize,
    pub part: Part,
    /// `[x, y, width, height]`, in the grid the page is drawn in.
    pub rect: [f32; 4],
}

/// The regions of `menu`'s current page, in the idiom [`super::draw_list`]
/// would draw it in - a strip's tabs, or a column's rows.
///
/// Takes the same `skin`, `frame` and `measure` `draw_list` does, because it
/// is the same decision: which idiom, and at what widths. A caller that has
/// just drawn the page passes what it drew with.
#[must_use]
pub fn regions(
    menu: &Menu,
    skin: &Skin,
    frame: &Frame,
    measure: &dyn Fn(&str) -> f32,
) -> Vec<Region> {
    let page = menu.page();
    match skin.strip().filter(|_| strip::suits(page)) {
        Some(strip) => strip::regions(menu, skin, strip, measure, frame),
        None => super::rows::regions(menu, skin, frame),
    }
}

/// The region under `at`, if any. The first in list order wins, which
/// matters only where two overlap - and a step arrow sits inside its
/// row's label block, so the arrows are listed after the block and this
/// looks for them first.
#[must_use]
pub fn hit(regions: &[Region], at: (f32, f32)) -> Option<Region> {
    regions
        .iter()
        .copied()
        .filter(|region| contains(region.rect, at))
        .max_by_key(|region| match region.part {
            Part::Row => 0,
            Part::StepBack | Part::StepForward => 1,
        })
}

impl Menu {
    /// Consumes a tick of pointer input against `regions`, and says what
    /// happened - the same [`MenuEvent`]s [`Self::update`] reports for a pad.
    ///
    /// `regions` is [`regions`] for the page as it was just drawn. It is
    /// passed in rather than computed here because computing it needs the
    /// skin and the frame, which this type does not hold, and because a
    /// caller that has drawn the page has already paid for the layout.
    pub fn pointer(&mut self, pointer: &Pointer, regions: &[Region]) -> Vec<MenuEvent> {
        let mut out = Vec::new();
        if pointer.is_idle() {
            return out;
        }
        let page = self.current();
        let rows = self.page().entries.len();

        if pointer.scroll != 0 && rows > self.visible {
            self.shift_view(i64::from(pointer.scroll));
            self.scroller.kinetic.set(self.scroll() as f32);
        } else if pointer.scroll != 0 && rows > 0 {
            let last = rows - 1;
            let moved_to = usize::try_from(
                i64::try_from(self.cursor[page]).unwrap_or(0) + i64::from(pointer.scroll),
            )
            .unwrap_or(0)
            .min(last);
            self.move_cursor(page, moved_to);
        }

        let pointer = &self.scroll_gesture(pointer, regions);

        let hit = pointer.at.and_then(|at| hit(regions, at));

        if pointer.moved
            && let Some(region) = hit
        {
            self.move_cursor(page, region.row);
        }

        if pointer.clicked
            && let Some(region) = hit
        {
            self.move_cursor(page, region.row);
            match region.part {
                Part::Row => out.extend(self.activate()),
                Part::StepBack => out.extend(self.adjust(-1)),
                Part::StepForward => out.extend(self.adjust(1)),
            }
        }

        if pointer.back {
            out.extend(self.back());
        }
        out
    }

    /// One tick of a finger on a page longer than the screen: `dy` grid
    /// units of travel down the screen scroll the view up (the content
    /// follows the finger), a touch-down catches a coasting list, and a lift
    /// lets it coast. Returns the pointer as the rows should read it.
    ///
    /// The row pitch is read off `regions` - the same rects the page was
    /// drawn with.
    fn scroll_gesture(&mut self, pointer: &Pointer, regions: &[Region]) -> Pointer {
        let page = self.current();
        let rows = self.definition.pages[page].entries.len();
        if rows <= self.visible {
            return *pointer;
        }
        let mut rects = regions.iter().filter(|r| r.part == Part::Row);
        let pitch = match (rects.next(), rects.next()) {
            (Some(a), Some(b)) if b.rect[1] > a.rect[1] => b.rect[1] - a.rect[1],
            (Some(a), _) => a.rect[3],
            _ => 0.0,
        };
        let travel = if pitch > 0.0 {
            -pointer.drag.1 / pitch
        } else {
            0.0
        };
        if self.scroller.page != page || self.scroller.kinetic.is_still() {
            self.scroller.page = page;
            self.scroller.kinetic.set(self.scroll() as f32);
        }
        let extent = Extent::rows((rows - self.visible) as f32);
        let seen = self.scroller.kinetic.gesture(pointer, travel, &extent);
        self.follow_scroller();
        seen
    }

    /// Advances a coasting list by one tick of `dt` seconds. A pad move, a
    /// page change or a wheel turn since the last tick stops it: the view
    /// is theirs.
    pub fn tick_scroll(&mut self, dt: f32) {
        let page = self.current();
        let rows = self.definition.pages[page].entries.len();
        let stale = self.scroller.page != page || self.scroller.cursor != self.selected();
        if rows <= self.visible || (stale && !self.scroller.kinetic.is_held()) {
            self.scroller.kinetic.set(self.scroll() as f32);
            self.scroller.page = page;
            self.scroller.cursor = self.selected();
            return;
        }
        let extent = Extent::rows((rows - self.visible) as f32);
        self.scroller.kinetic.tick(dt, &extent);
        self.follow_scroller();
    }

    /// Moves the view onto the row nearest the scroller's offset.
    fn follow_scroller(&mut self) {
        let page = self.current();
        let last = self.definition.pages[page].entries.len() - self.visible;
        let want = self
            .scroller
            .kinetic
            .offset()
            .round()
            .clamp(0.0, last as f32) as i64;
        let by = want - self.scroll() as i64;
        if by != 0 {
            self.shift_view(by);
        }
        self.scroller.cursor = self.selected();
    }

    /// Moves a page longer than the screen's window `by` rows (down the list
    /// when positive), clamped at both ends, and pushes the cursor just far
    /// enough to stay inside the new window.
    fn shift_view(&mut self, by: i64) {
        let page = self.current();
        let rows = self.definition.pages[page].entries.len();
        let visible = self.visible;
        if rows <= visible {
            return;
        }
        let last = rows - visible;
        let start = (self.scroll() as i64 + by).clamp(0, last as i64) as usize;
        // The window only holds a cursor with a row of lookahead at each end
        // (`window_start`), so the cursor moves just far enough to stay in it.
        let low = if start == 0 { 0 } else { start + 1 };
        let high = if start == last {
            rows - 1
        } else {
            start + visible - 2
        };
        self.cursor[page] = self.cursor[page].clamp(low, high);
        self.scroll[page] = start;
    }

    /// Puts the cursor on `row` and lets the window follow, the way a pad
    /// move in [`Self::update`] does - stored, so the list scrolls rather
    /// than snapping.
    fn move_cursor(&mut self, page: usize, row: usize) {
        let rows = self.definition.pages[page].entries.len();
        if row >= rows {
            return;
        }
        let dir = if row > self.cursor[page] {
            super::nav::Dir::Down
        } else {
            super::nav::Dir::Up
        };
        self.nav
            .when(self.cursor[page] != row, super::nav::Nav::Moved(Some(dir)));
        self.cursor[page] = row;
        self.scroll[page] = self.scroll();
    }
}

/// A finger's scroll of a long page: the [`Kinetic`] model in rows, and
/// what it was started on, so a page change or a pad move stops it.
#[derive(Debug, Clone, Default, PartialEq)]
pub(super) struct Scroller {
    kinetic: Kinetic,
    /// The page the scroll belongs to.
    page: usize,
    /// The cursor as the scroll last left it.
    cursor: usize,
}
