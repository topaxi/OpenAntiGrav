//! `Grid Selection` and `Cell Selection` answering a mouse or a finger.
//!
//! **Ours, with no counterpart on any disc** - see [`crate::pointer`]. An
//! `impl` of its own beside `campaign.rs`, which `scripts/check-file-size.py`
//! ratchets - the same reason [`crate::picker::pointer`] and
//! [`crate::menu::pointer`] each sit beside the module they extend rather
//! than growing it.
//!
//! # Hit-testing the hexagon, not its bounding box
//!
//! The grid is staggered - `docs/ui/campaign-screens.md`'s own measurements
//! for both screens - so neighbouring hexes' bounding boxes overlap at their
//! corners. A rectangle test there picks whichever hex is listed first, not
//! whichever one the player is actually pointing at, so every hex target
//! here is tested with [`crate::pointer::hex_contains`] rather than
//! [`crate::pointer::contains`]. The rect handed to it is the same one the
//! draw would place the sprite at - authored position, sprite's own size -
//! so a hit region can never drift from the picture the way a hand-typed
//! rectangle could.
//!
//! # What is a target
//!
//! - **`Grid Selection`'s `up arrow`/`down arrow` images** page through the
//!   sixteen tiers one at a time, the way the d-pad does.
//! - **Each screen's own hex** - `Grid Selection`'s four tiles for the
//!   current page, `Cell Selection`'s occupied cells - selects on a hover
//!   and confirms on a second click, the same two-tap idiom
//!   [`crate::picker::pointer`] already uses.
//! - **The secondary button** backs out, as circle does. On `Cell Selection`
//!   with `Cell Help` open, a click or the secondary button closes the
//!   overlay instead - the same redirect [`super::CellSelection::update`]
//!   gives every directional press while it is up.

use crate::frontend::Placed;
use crate::pointer::{Pointer, contains, hex_contains};

use super::{CellSelection, Event, GRIDS_PER_PAGE, GridSelection, Layout, hex_rect};

/// What a click on one of the screen's targets does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum What {
    /// Step to the previous tier: `Grid Selection`'s `up arrow`.
    Previous,
    /// Step to the next tier: `Grid Selection`'s `down arrow`.
    Next,
    /// One hex, by its absolute tier index (`Grid Selection`) or cell index
    /// (`Cell Selection`).
    Hex(usize),
}

/// One rect a pointer can land on, and what it does.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Target {
    pub what: What,
    /// `[x, y, width, height]`, in the screen's own grid - a hex's own rect
    /// for [`What::Hex`], tested with [`hex_contains`] rather than
    /// [`contains`].
    pub rect: [f32; 4],
}

/// `Grid Selection`'s targets: the paging arrows, and the current page's
/// own hex tiles - skipping a slot the selected page has no tier for, the
/// same skip [`super::grid_draw_list`] already makes.
#[must_use]
pub fn grid_targets(
    model: &GridSelection,
    layout: &Layout,
    sprites: &dyn Fn(&str) -> Option<Placed>,
) -> Vec<Target> {
    let screen = &layout.screen;
    let mut out = Vec::new();
    for image in &screen.images {
        let what = match image.name.as_deref() {
            Some("up arrow") => What::Previous,
            Some("down arrow") => What::Next,
            _ => continue,
        };
        if let Some(rect) = image_rect(image, sprites) {
            out.push(Target { what, rect });
        }
    }
    let page = model.page();
    for slot in 0..GRIDS_PER_PAGE {
        let index = page * GRIDS_PER_PAGE + slot;
        if index >= model.grids().len() {
            continue;
        }
        if let Some(rect) = hex_rect(screen, slot, 0, sprites) {
            out.push(Target {
                what: What::Hex(index),
                rect,
            });
        }
    }
    out
}

/// `Cell Selection`'s targets: one per cell the selected grid actually
/// carries, at that cell's own `grid_coords` position.
#[must_use]
pub fn cell_targets(
    model: &CellSelection,
    layout: &Layout,
    sprites: &dyn Fn(&str) -> Option<Placed>,
) -> Vec<Target> {
    let screen = &layout.screen;
    let mut out = Vec::new();
    for (index, cell) in model.cells().iter().enumerate() {
        let Some((x, y)) = cell.grid_coords() else {
            continue;
        };
        if let Some(rect) = hex_rect(screen, x as usize, y as usize, sprites) {
            out.push(Target {
                what: What::Hex(index),
                rect,
            });
        }
    }
    out
}

/// Where an image is drawn: its authored size, or its texture's - the same
/// fallback [`crate::picker::pointer::targets`] applies for the same reason.
fn image_rect(
    image: &crate::screen::Image,
    sprites: &dyn Fn(&str) -> Option<Placed>,
) -> Option<[f32; 4]> {
    let placed = sprites(&image.src);
    let width = image
        .width
        .or_else(|| placed.map(|placed| placed.width as f32))?;
    let height = image
        .height
        .or_else(|| placed.map(|placed| placed.height as f32))?;
    Some([image.x, image.y, width, height])
}

/// The first target under `at`, in [`grid_targets`]/[`cell_targets`]' own
/// order - a hex tested against its own hexagon, an arrow against its plain
/// rect.
#[must_use]
pub fn hit(targets: &[Target], at: (f32, f32)) -> Option<Target> {
    targets.iter().copied().find(|target| match target.what {
        What::Hex(_) => hex_contains(target.rect, at),
        What::Previous | What::Next => contains(target.rect, at),
    })
}

impl GridSelection {
    /// Consumes a tick of pointer input against `targets` and says what
    /// happened, on the same [`Event`] terms as [`Self::update`].
    pub fn pointer(&mut self, pointer: &Pointer, targets: &[Target]) -> Vec<Event> {
        let mut out = Vec::new();
        if pointer.is_idle() {
            return out;
        }
        if pointer.scroll != 0 {
            out.extend(self.step(pointer.scroll.signum()));
        }
        let hit = pointer.at.and_then(|at| hit(targets, at));
        // What was selected before this tick's gesture - a tap moves and
        // clicks in one tick, and that click must not confirm the tile the
        // move just landed on.
        let was = self.index();
        if pointer.moved
            && let Some(Target {
                what: What::Hex(index),
                ..
            }) = hit
        {
            out.extend(self.select(index));
        }
        if pointer.clicked
            && let Some(target) = hit
        {
            // `up arrow`/`down arrow` are the click equivalent of the pad's
            // `Up`/`Down` - see `Self::page_step`'s own doc for the
            // measurement this mirrors.
            match target.what {
                What::Previous => out.extend(self.page_step(-1)),
                What::Next => out.extend(self.page_step(1)),
                What::Hex(index) if index == was => out.push(Event::Confirmed),
                What::Hex(index) => out.extend(self.select(index)),
            }
        }
        if pointer.back {
            out.push(Event::Back);
        }
        out
    }

    /// Moves the selection by one tier, wrapping. **HD/Fury only now** -
    /// its own `Grid Selection` pages one flyer at a time
    /// ([`Self::per_page`] `== 1`), reusing this unchanged wrapping idiom
    /// since that pager's own clamp-versus-wrap behaviour was never
    /// independently measured (unlike Pulse's own, see [`Self::page_step`]).
    /// [`Self::page_step`] delegates back to this whenever `per_page <= 1`,
    /// so nothing about HD's own paging (pad or the click arrows in
    /// `super::hd::GridSelection::hd_pointer`) changed when Pulse's own
    /// paging was fixed.
    pub(super) fn step(&mut self, step: i32) -> Option<Event> {
        let count = self.grids.len();
        if count == 0 {
            return None;
        }
        let index = (self.index as i64 + i64::from(step)).rem_euclid(count as i64) as usize;
        self.index = index;
        Some(Event::Moved)
    }

    /// A page turn - `Up`/`Down` on Pulse's own `Grid Selection`, and the
    /// `up arrow`/`down arrow` click targets. **Clamps at the deck's own
    /// ends, does not wrap**, and moves to the same slot within the new
    /// page - all three measured live against PPSSPP on 2026-09-25
    /// (`pulse-psp-usa.chd`, a down press from `grid0` landing on `grid4`'s
    /// own `"GRID 5"`, a further down from the last page **staying on
    /// `"PHANTOM GRID 1"` rather than moving at all** - not sliding to the
    /// deck's own last grid either, which is what clamping the raw index
    /// rather than the page index would give - and a right-then-down from
    /// `grid1` landing on `grid5`'s own `"GRID 6"` rather than resetting to
    /// `"GRID 5"`). See `docs/ui/campaign-screens.md`'s "Measured against
    /// PPSSPP, 2026-09-25" for the full walk. Confidence 90.
    ///
    /// **HD only**: delegates to [`Self::step`] (wraps by one tier)
    /// whenever `per_page <= 1` - see that method's own doc for why this
    /// pass leaves HD's own paging exactly as it was.
    pub(super) fn page_step(&mut self, direction: i32) -> Option<Event> {
        if self.per_page <= 1 {
            return self.step(direction);
        }
        let count = self.grids.len();
        if count == 0 {
            return None;
        }
        let per_page = self.per_page;
        let current_page = self.index / per_page;
        let last_page = (count - 1) / per_page;
        let new_page = i64::from(direction) + i64::try_from(current_page).unwrap_or(0);
        let new_page = new_page.clamp(0, i64::try_from(last_page).unwrap_or(0));
        #[allow(
            clippy::cast_sign_loss,
            reason = "clamp(0, ..) above already rules out negative"
        )]
        let new_page = new_page as usize;
        if new_page == current_page {
            return None;
        }
        let slot = self.index % per_page;
        self.index = (new_page * per_page + slot).min(count - 1);
        Some(Event::Moved)
    }

    /// One tile - `Left`/`Right` on Pulse's own `Grid Selection`. **Clamps
    /// at the current page's own ends, does not cross into the next/previous
    /// page** - measured live: a right from `grid0`'s own last-slot tier
    /// (`grid3`, `"GRID 4"`) stays on `"GRID 4"` rather than crossing to
    /// `grid4`'s `"GRID 5"`, and a left from `grid0` (the deck's own first
    /// slot) stays on `"GRID 1"`. See [`Self::page_step`]'s own doc for the
    /// full measurement citation - the two were read in the same PPSSPP
    /// session. Confidence 90.
    ///
    /// **A no-op wherever [`Self::per_page`] is `1`** (HD's own one-tile
    /// page) - the page a single tile spans is itself, so this never moves
    /// anything there. HD's pad `Left`/`Right` were never bound to
    /// anything before this pass either, so that is not a behaviour change.
    pub(super) fn tile_step(&mut self, direction: i32) -> Option<Event> {
        if self.grids.is_empty() {
            return None;
        }
        let per_page = self.per_page.max(1);
        let page_start = (self.index / per_page) * per_page;
        let page_end = (page_start + per_page - 1).min(self.grids.len() - 1);
        let new_index =
            (self.index as i64 + i64::from(direction)).clamp(page_start as i64, page_end as i64);
        #[allow(
            clippy::cast_sign_loss,
            reason = "clamp(page_start, ..) above already rules out negative"
        )]
        let new_index = new_index as usize;
        if new_index == self.index {
            return None;
        }
        self.index = new_index;
        Some(Event::Moved)
    }

    /// Puts the selection on `index`, if it names a different tier.
    fn select(&mut self, index: usize) -> Option<Event> {
        if index >= self.grids.len() || index == self.index {
            return None;
        }
        self.index = index;
        Some(Event::Moved)
    }
}

impl CellSelection {
    /// Consumes a tick of pointer input against `targets` and says what
    /// happened, on the same [`Event`] terms as [`Self::update`].
    pub fn pointer(&mut self, pointer: &Pointer, targets: &[Target]) -> Vec<Event> {
        let mut out = Vec::new();
        if pointer.is_idle() {
            return out;
        }
        if self.help_open {
            // `Cell Help` takes the tick whole - a click or the secondary
            // button closes it, the same as cross/circle do for
            // `Self::update`, and neither a hover nor a click moves the
            // selection underneath the overlay.
            if pointer.clicked || pointer.back {
                self.help_open = false;
                out.push(Event::Help);
            }
            return out;
        }
        let hit = pointer.at.and_then(|at| hit(targets, at));
        let was = self.index;
        if pointer.moved
            && let Some(Target {
                what: What::Hex(index),
                ..
            }) = hit
        {
            out.extend(self.select(index));
        }
        if pointer.clicked
            && let Some(Target {
                what: What::Hex(index),
                ..
            }) = hit
        {
            if index == was {
                out.push(Event::Confirmed);
            } else {
                out.extend(self.select(index));
            }
        }
        if pointer.back {
            out.push(Event::Back);
        }
        out
    }

    /// Puts the selection on `index`, if it names a different cell.
    fn select(&mut self, index: usize) -> Option<Event> {
        if index >= self.cells.len() || index == self.index {
            return None;
        }
        self.index = index;
        Some(Event::Moved)
    }
}
