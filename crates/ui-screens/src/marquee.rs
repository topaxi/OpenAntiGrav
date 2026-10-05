//! Scrolling a menu row's value when it is too wide to read.
//!
//! Every other title with a PC-shaped options screen does this for a device
//! name or a resolution string too long for its column - Pulse never had to,
//! because it never had a value column at all (see `menu::VALUE_RIGHT`). So
//! this is entirely ours, and it is a post-process over [`menu::draw_list`]'s
//! finished [`Draw`] list rather than a change to that function: identifying
//! the one row to scroll needs nothing [`Menu`] does not already expose
//! through its own public API (`page`, `selected`, `scroll`), and keeping
//! this separate keeps `menu.rs` and `frontend.rs` - both already at the
//! ceiling `scripts/check-file-size.py` freezes them at - untouched by a
//! feature the disc's own menus never needed.
//!
//! [`Timer`] is the clock; [`focus`] says which row it is timing; [`apply`]
//! slides that row's text and reports where to clip it. The clip itself is
//! `Renderer::push_text`'s job, not this module's: sliding a whole glyph past
//! the box edge is only half-visible for part of a frame, and drawing that
//! honestly needs the glyph's own quad trimmed, which needs the font atlas
//! this module deliberately never touches. See `crate::render`.

use oag_ui::frontend::{Align, Draw};
use oag_ui::menu::{Menu, Skin};

/// How wide a row's value is allowed to get before it starts scrolling.
///
/// **Ours**, picked the same way `menu::VALUE_RIGHT` was: nothing on the disc
/// states a value column's width, because the disc has no value column. Wide
/// enough that no label this build's own `menu.toml` declares collides with
/// a value sitting in the last 200 pixels of the row.
pub const MAX_WIDTH: f32 = 200.0;

/// Which row's value is being scrolled, and for how long it has been.
///
/// Lives on the stage that owns the tick (`MenuStage`), not on [`Menu`]
/// itself: `Menu` is a pure input-in, events-out state machine with no clock
/// of any kind - see its own module docs - and this is a clock, for the same
/// reason `anim::Tween` lives beside the page-change code that drives it
/// rather than inside `Menu`.
#[derive(Debug, Default)]
pub struct Timer {
    on: Option<(String, usize, String)>,
    elapsed: f32,
}

impl Timer {
    /// Advances the clock by `dt`, resetting it to zero whenever `focus`
    /// names a different row - or the same row holding a different value,
    /// which happens every time a player adjusts the very row they are
    /// reading. Without the value in the key, cycling to a new adapter would
    /// carry on scrolling mid-string instead of starting the new name from
    /// the beginning.
    pub fn tick(&mut self, dt: f32, focus: Option<(String, usize, String)>) {
        if focus != self.on {
            self.on = focus;
            self.elapsed = 0.0;
        }
        self.elapsed += dt.max(0.0);
    }

    /// How long the current row has held focus.
    #[must_use]
    pub fn elapsed(&self) -> f32 {
        self.elapsed
    }
}

/// The row a value marquee should be scrolling, if any: the selected row,
/// keyed by its page, index and current value so a change to any of the
/// three restarts [`Timer`]. `None` for a row with nothing adjustable to
/// show - a submenu, a disabled row, a binding - which is exactly the set
/// `menu::draw_list` itself declines to draw in the selected colour.
#[must_use]
pub fn focus(menu: &Menu) -> Option<(String, usize, String)> {
    let page = menu.page();
    let row = menu.selected();
    let entry = page.entries.get(row)?;
    if !entry.is_adjustable() || menu.is_disabled(entry) {
        return None;
    }
    Some((page.id.clone(), row, entry.value()?.to_string()))
}

/// Fixes up every overflowing value on the page, and says where to clip the
/// one that is also scrolling - `(y, left, right)`, for `Renderer::render`.
///
/// **Every overflowing row is fixed, not only the selected one.** The bug
/// this module exists for - a long adapter name burying its own label - is
/// there whether or not the cursor happens to be on that row; a marquee that
/// only ever touched the focused row would leave every *other* row exactly
/// as broken as before the moment a player looked away from it. So a row
/// that overflows but is not focused gets a static excerpt instead of
/// motion: its tail, clipped to [`MAX_WIDTH`] the same way the scrolling row
/// eventually reads at rest, just never animated - nothing here should catch
/// a player's eye that they are not currently adjusting.
///
/// Every row whose value already fits is left exactly as `menu::draw_list`
/// drew it - `Align::Right` against the same edge it always drew against -
/// which is what makes a row that stops overflowing (a shorter adapter name
/// chosen, a narrower language picked) look identical to one that never did.
///
/// `measure` is `font::measure` behind the same closure indirection
/// `menu::draw_list`'s own `bindings` argument already uses, so this module
/// never has to know what a font atlas is.
#[must_use]
pub fn apply(
    mut list: Vec<Draw>,
    menu: &Menu,
    skin: &Skin,
    measure: &dyn Fn(&str) -> f32,
    elapsed: f32,
) -> (Vec<Draw>, Option<(usize, f32, f32)>) {
    let focused_y = focus(menu).map(|_| {
        skin.first_row_y() + menu.selected().saturating_sub(menu.scroll()) as f32 * skin.row_pitch()
    });
    let mut clip = None;
    for (index, draw) in list.iter_mut().enumerate() {
        let Draw::Text {
            x,
            y,
            scale,
            align,
            text,
            ..
        } = draw
        else {
            continue;
        };
        if *align != Align::Right {
            continue;
        }
        let overflow = measure(text) * *scale - MAX_WIDTH;
        if overflow <= 0.0 {
            continue;
        }
        let right = *x;
        let left = right - MAX_WIDTH;
        if focused_y == Some(*y) {
            // A continuous pixel offset, not a character count: the clip in
            // `Renderer::push_text` trims a glyph's own quad at the box
            // edge, so nothing here needs to land on a character boundary.
            *x = left - oag_ui::anim::marquee_offset(elapsed, overflow);
            *align = Align::Left;
            // The list's own index, not `y`: the label shares this row's
            // `y` with its value, and a clip keyed on `y` alone would have
            // culled every one of the label's glyphs right along with the
            // value's overflow - which is `RENDERER` disappearing outright,
            // not just crowded. `menu_stage.rs` hands this `Vec` straight to
            // `Renderer::render` with nothing in between that would reorder
            // it, so the index still names this same entry there.
            clip = Some((index, left, right));
        } else {
            *text = fit_right(text, measure, MAX_WIDTH / *scale).to_string();
        }
    }
    (list, clip)
}

/// The longest suffix of `text` whose measured width (at `measure`'s scale of
/// 1) is no more than `budget`.
///
/// Static, not scrolled - a row nobody is reading right now shows its tail
/// and stops there, exactly where the scrolling row rests at the end of its
/// own cycle. `O(n^2)` in `measure` calls, which is fine for a menu row: this
/// runs on at most a handful of characters, once a frame, for rows that are
/// not the one actually animating.
fn fit_right<'a>(text: &'a str, measure: &dyn Fn(&str) -> f32, budget: f32) -> &'a str {
    text.char_indices()
        .find(|(i, _)| measure(&text[*i..]) <= budget)
        .map_or("", |(i, _)| &text[i..])
}

#[cfg(test)]
mod tests;
