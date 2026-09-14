//! A selection screen answering a mouse or a finger.
//!
//! **Ours, with no counterpart on any disc** - see [`crate::pointer`]. An
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
//! - **The secondary button** backs out, as circle does; **the wheel** steps
//!   the entry, up for previous.

use crate::frontend::Placed;
use crate::menu::Skin;
use crate::pointer::{Pointer, contains};
use crate::screen::Image;

use super::{Event, Layout, Picker};

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
    targets
        .iter()
        .copied()
        .find(|target| contains(target.rect, at))
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
        }
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
                What::PreviousVariant => out.extend(self.step_variant(-1)),
                What::NextVariant => out.extend(self.step_variant(1)),
                What::Entry(index) if index == was => out.push(Event::Confirmed),
                What::Entry(index) => out.extend(self.select(index)),
                What::Confirm => out.push(Event::Confirmed),
            }
        }
        if pointer.back {
            out.push(Event::Back);
        }
        out
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
        let index = (self.index as i64 + i64::from(step)).rem_euclid(count as i64) as usize;
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
        self.variant = 0;
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
