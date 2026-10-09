//! A selection screen answering a mouse or a finger.
//!
//! **Ours, with no counterpart on any disc** - see [`oag_ui::pointer`]. An
//! `impl Picker` of its own beside `picker.rs`, which is a few lines under
//! the file-size rule; nothing here changes what a pad does on the same
//! screen, and [`Picker::update`] now steps through the same two helpers a
//! pointer does so the two cannot drift.
//!
//! # What is a target
//!
//! The screen's own widgets, where it has them, and this build's two
//! rectangles where it has not:
//!
//! - **The `up arrow` and `down arrow` images** step the entry, the way the
//!   d-pad does. Both selection screens on both PSP titles author the pair
//!   by those names (`Selection_Definition.xml`, read 2026-09-14 off
//!   `pulse-psp-eu.chd`'s `Data.wad`), so the target is the disc's own
//!   glyph at the disc's own place.
//! - **The `skin left arrow` and `skin right arrow` images** step the
//!   livery, on a ship picker with more than one.
//! - **Pure's `<Menu>` rows** - the entry list down the left - select the
//!   row under the pointer on a hover, and a click on the row already
//!   selected confirms it. Pulse shows one entry at a time and has no such
//!   list; its rows are simply absent.
//! - **The info panel and the preview** confirm on a click. These are
//!   [`Layout::panel`] and [`Layout::preview`], the two rectangles the screen
//!   is built around - the panel the disc's `Infogradient` widget, the
//!   preview this build's own - and they are the one place a Pulse picker
//!   with no row list can be confirmed from by pointing. A click on the
//!   backdrop outside both is nothing, so a stray click does not start a
//!   race.
//! - **A horizontal finger drag** over HD's hex grids pans the columns,
//!   and a flick coasts them, through [`oag_ui::kinetic`] - see
//!   [`Picker::pan`]. **Chosen, not measured**: the original is a pad-only
//!   PS3 game.
//! - **The secondary button** backs out, as circle does; **the wheel** steps
//!   the entry, up for previous.

use oag_ui::frontend::Placed;
use oag_ui::kinetic::Extent;
use oag_ui::menu::Skin;
use oag_ui::pointer::{Pointer, contains, hex_contains};
use oag_ui::screen::Image;

use super::{Event, Layout, Picker};

/// How far off its column, in columns, the grid must be before the column
/// beside it is the selection. **Chosen, not measured**: past half a column
/// the neighbour is nearer the centre, and the tenth beyond that keeps a
/// finger resting on the halfway line from flipping the selection - and
/// reloading the preview - every time it trembles.
pub const STEP_AT: f32 = 0.6;

/// What a click on one of the screen's targets does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum What {
    /// Step to the previous entry: the `up arrow`.
    Previous,
    /// Step to the next entry: the `down arrow`.
    Next,
    /// Step to the previous livery: the `skin left arrow`.
    PreviousVariant,
    /// Step to the next livery: the `skin right arrow`.
    NextVariant,
    /// One row of a listed entry column, by entry index.
    Entry(usize),
    /// A cell of HD's hex column: team `entry`, and the livery it is when
    /// the model is open. Selects, and confirms when it is already chosen.
    Cell { entry: usize, variant: usize },
    /// The panel or the preview: confirm the selection.
    Confirm,
}

/// One rect a pointer can land on, and what it does.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Target {
    pub what: What,
    /// `[x, y, width, height]`, in the screen's own grid.
    pub rect: [f32; 4],
}

/// The screen's targets, in the order they are tested - the arrows first,
/// then the rows, then the two confirm rectangles, so an arrow drawn over
/// the panel is the arrow.
///
/// `sprites` resolves an image the way [`super::draw_list`] does, because an
/// `Image` with no authored size is its texture's size, and only the sheet
/// knows that.
#[must_use]
pub fn targets(
    picker: &Picker,
    layout: &Layout,
    skin: &Skin,
    sprites: &dyn Fn(&str) -> Option<Placed>,
) -> Vec<Target> {
    if let Some(extra) = layout.hd.as_deref() {
        return hd_targets(picker, layout, extra);
    }
    if let Some(extra) = layout.hd_track.as_deref() {
        return super::hd::track::targets(picker, layout, extra);
    }
    let screen = &layout.screen;
    let mut out = Vec::new();
    let variants = picker.variants().len();
    for image in &screen.images {
        let what = match image.name.as_deref() {
            Some("up arrow") => What::Previous,
            Some("down arrow") => What::Next,
            Some("skin left arrow") if variants > 1 => What::PreviousVariant,
            Some("skin right arrow") if variants > 1 => What::NextVariant,
            _ => continue,
        };
        if let Some(rect) = image_rect(image, sprites) {
            out.push(Target { what, rect });
        }
    }
    if let Some(menu) = screen.menu.as_ref() {
        // The same pitch `entry_rows` steps by: one line of the widget's own
        // font at the widget's own scale. Its width is not authored - the
        // widget lists names of any length - so the band is the run from the
        // widget's x to the panel's left edge, which is the column the names
        // are drawn down.
        let scale = layout.face_scale(&menu.font) * menu.scale;
        let pitch = skin.line_height() * scale;
        let width = (layout.panel[0] - menu.x).max(pitch);
        // Centred on the face's capitals rather than laid from the pen, for
        // the reason `RowInk` gives: the face keeps room above its cap line.
        let top = skin.row_band_top(menu.y, scale);
        for index in 0..picker.entries().len() {
            out.push(Target {
                what: What::Entry(index),
                rect: [menu.x, top + index as f32 * pitch, width, pitch],
            });
        }
    }
    out.push(Target {
        what: What::Confirm,
        rect: layout.panel,
    });
    out.push(Target {
        what: What::Confirm,
        rect: layout.preview,
    });
    out
}

/// Wipeout HD/Fury's `Team Selection` targets, off its own `Bracket` rects,
/// since the screen authors no arrows to click. **Chosen, not measured**, like
/// every pointer target on a console screen: the `CHOOSE TEAM` frame's left
/// half steps to the previous team and its right half to the next, the
/// way left/right do on the pad there; the `NAVIGATE TEAM` frame's top half
/// steps the livery back and its bottom half forward, as up/down do; the
/// `SHIP MODEL` frame confirms.
fn hd_targets(picker: &Picker, layout: &Layout, extra: &super::hd::TeamScreen) -> Vec<Target> {
    let mut out = Vec::new();
    if let Some(grid) = &extra.hex {
        // Open cells only: a padlocked cell is nothing, and its neighbour
        // hexagons are tested as hexagons, not boxes (`hex_contains`).
        for cell in grid.cells(picker.index(), picker.entries().len(), picker.pan()) {
            let Some(variant) = super::hd::hex::variant_at(picker, &cell) else {
                continue;
            };
            out.push(Target {
                what: What::Cell {
                    entry: cell.entry,
                    variant,
                },
                rect: cell.rect,
            });
        }
    }
    let [x, y, w, h] = layout.panel;
    out.push(Target {
        what: What::Previous,
        rect: [x, y, w / 2.0, h],
    });
    out.push(Target {
        what: What::Next,
        rect: [x + w / 2.0, y, w / 2.0, h],
    });
    if picker.variants().len() > 1
        && extra.hex.is_none()
        && let Some(grid) = extra.brackets.get(1).filter(|bracket| !bracket.middle)
    {
        let [x, y, w, h] = grid.rect;
        out.push(Target {
            what: What::PreviousVariant,
            rect: [x, y, w, h / 2.0],
        });
        out.push(Target {
            what: What::NextVariant,
            rect: [x, y + h / 2.0, w, h / 2.0],
        });
    }
    out.push(Target {
        what: What::Confirm,
        rect: layout.preview,
    });
    out
}

/// Where `image` is drawn: its authored size, or its texture's.
fn image_rect(image: &Image, sprites: &dyn Fn(&str) -> Option<Placed>) -> Option<[f32; 4]> {
    let placed = sprites(&image.src);
    let width = image
        .width
        .or_else(|| placed.map(|placed| placed.width as f32))?;
    let height = image
        .height
        .or_else(|| placed.map(|placed| placed.height as f32))?;
    Some([image.x, image.y, width, height])
}

/// The first target under `at`, in [`targets`]' own order.
#[must_use]
pub fn hit(targets: &[Target], at: (f32, f32)) -> Option<Target> {
    targets.iter().copied().find(|target| match target.what {
        What::Cell { .. } => hex_contains(target.rect, at),
        _ => contains(target.rect, at),
    })
}

impl Picker {
    /// Consumes a tick of pointer input against `targets` and says what
    /// happened, on the same [`Event`] terms as [`Self::update`].
    pub fn pointer(&mut self, pointer: &Pointer, targets: &[Target]) -> Vec<Event> {
        let mut out = Vec::new();
        if pointer.is_idle() {
            return out;
        }
        if pointer.scroll != 0 {
            out.extend(self.step_entry(pointer.scroll.signum()));
            self.scroll.settle(&Extent::wrapping());
        }
        let grid = targets
            .iter()
            .any(|target| matches!(target.what, What::Cell { .. }));
        let travel = if grid {
            pointer.drag.0 / super::hd::hex::PITCH[0]
        } else {
            0.0
        };
        // A touch-down catches a coasting grid, and its tap selects nothing.
        let pointer = &self.scroll.gesture(pointer, travel, &Extent::wrapping());
        out.extend(self.fold_pan());
        let hit = pointer.at.and_then(|at| hit(targets, at));
        // What was selected before this tick's gesture: a tap is a move and
        // a click in one tick, and the click must not confirm the row the
        // move just landed on. Only a row that was already selected confirms.
        let was = self.index;
        if pointer.moved
            && let Some(Target {
                what: What::Entry(index),
                ..
            }) = hit
        {
            out.extend(self.select(index));
        }
        if pointer.clicked
            && let Some(target) = hit
        {
            match target.what {
                What::Previous => out.extend(self.step_entry(-1)),
                What::Next => out.extend(self.step_entry(1)),
                What::PreviousVariant => out.extend(self.step_vertical(-1)),
                What::NextVariant => out.extend(self.step_vertical(1)),
                What::Entry(index) if index == was => out.push(Event::Confirmed),
                What::Entry(index) => out.extend(self.select(index)),
                What::Cell { entry, variant } => {
                    let chosen = entry == was && variant == self.variant;
                    if chosen {
                        out.push(Event::Confirmed);
                    } else {
                        out.extend(self.select_cell(entry, variant));
                    }
                }
                What::Confirm => out.push(Event::Confirmed),
            }
        }
        if pointer.back {
            out.push(Event::Back);
        }
        out
    }

    /// The grid's fractional column offset while a finger drags it or it
    /// coasts: positive slides the columns right, so the column on the left
    /// is coming to the centre. `0.0` at rest and always inside
    /// `-STEP_AT..=STEP_AT` once [`Self::fold_pan`] has run.
    #[must_use]
    pub fn pan(&self) -> f32 {
        self.scroll.offset()
    }

    /// Folds whole columns the grid has travelled out of the pan, stepping
    /// the selection once per column the way left/right do on the pad -
    /// drag right brings the previous entry to the centre - wrapping at the
    /// ends, so each `Event::Moved` is the one the pad's step makes.
    ///
    /// The offset itself is [`oag_ui::kinetic`]'s: it follows the finger,
    /// coasts after a flick and settles onto the nearest column, and this
    /// only keeps the selection on whichever column is nearest the centre
    /// (past [`STEP_AT`]). Called from [`Self::pointer`] and from
    /// [`Self::update`], which runs every tick, so a coast steps as it goes.
    pub(super) fn fold_pan(&mut self) -> Vec<Event> {
        let mut out = Vec::new();
        while self.scroll.offset() > STEP_AT {
            self.scroll.shift(-1.0);
            out.extend(self.step_entry(-1));
        }
        while self.scroll.offset() < -STEP_AT {
            self.scroll.shift(1.0);
            out.extend(self.step_entry(1));
        }
        out
    }

    /// Puts the selection on team `entry` and its livery `variant`: the
    /// move a click on a hex of HD's team column makes.
    fn select_cell(&mut self, entry: usize, variant: usize) -> Option<Event> {
        let moved = self.select(entry);
        let wanted = variant.min(self.variants().len().saturating_sub(1));
        let changed = wanted != self.variant;
        self.variant = wanted;
        moved.or(changed.then_some(Event::VariantChanged))
    }

    /// Moves the selection by `step` entries, wrapping, and restarts the
    /// livery and the slideshow the way the original does on every
    /// selection - a single-entry list included, where the step lands on
    /// the same entry and still counts as a move. `None` on an empty picker.
    pub(super) fn step_entry(&mut self, step: i32) -> Option<Event> {
        let count = self.entries.len();
        if count == 0 {
            return None;
        }
        let index = match self.columns {
            0 => (self.index as i64 + i64::from(step)).rem_euclid(count as i64) as usize,
            columns => {
                // Along the row, wrapping at its end.
                let row = self.index / columns;
                let width = columns.min(count - row * columns);
                let column = (self.index % columns) as i64 + i64::from(step);
                row * columns + column.rem_euclid(width as i64) as usize
            }
        };
        self.land_on(index);
        Some(Event::Moved)
    }

    /// What up and down do: the other row of a grid, or the livery on a list.
    pub(super) fn step_vertical(&mut self, step: i32) -> Option<Event> {
        if self.columns == 0 {
            return self.step_variant(step);
        }
        let rows = self.entries.len().div_ceil(self.columns);
        let (row, column) = (self.index / self.columns, self.index % self.columns);
        let target = (row as i64 + i64::from(step)).rem_euclid(rows as i64) as usize;
        let index = target * self.columns + column;
        if rows < 2 || index >= self.entries.len() {
            return None;
        }
        self.land_on(index);
        Some(Event::Moved)
    }

    /// Puts the selection on `index`, if it is an entry. A no-op on the
    /// entry already selected: the slideshow is not restarted by pointing
    /// at what is already showing.
    fn select(&mut self, index: usize) -> Option<Event> {
        if index >= self.entries.len() || index == self.index {
            return None;
        }
        self.land_on(index);
        Some(Event::Moved)
    }

    fn land_on(&mut self, index: usize) {
        self.index = index;
        // Wipeout HD/Fury's team screen keeps the model row across a team
        // step - measured on RPCS3 2026-09-29: `Right` from `concept1` lands on
        // the next team's `concept1` (Qirex `085`/`080`), from `normal` on its
        // `normal`. Every other picker restarts the livery.
        if !self.across || self.variant >= self.variants().len() {
            self.variant = 0;
        }
        self.since_selection = 0.0;
    }

    /// Moves the livery by `step`, wrapping, on a picker with more than one.
    pub(super) fn step_variant(&mut self, step: i32) -> Option<Event> {
        let variants = self.variants().len();
        if variants < 2 {
            return None;
        }
        self.variant = (self.variant as i64 + i64::from(step)).rem_euclid(variants as i64) as usize;
        Some(Event::VariantChanged)
    }
}
