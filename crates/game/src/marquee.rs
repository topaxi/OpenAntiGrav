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

use crate::frontend::{Align, Draw};
use crate::menu::{Menu, Skin};

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

/// Slides the focused row's value, if it is too wide for [`MAX_WIDTH`], and
/// says where to clip it - `(y, left, right)`, for `Renderer::render`.
///
/// Every other row is left exactly as `menu::draw_list` drew it, and so is
/// this one when its value already fits - `Align::Right` against the same
/// edge it always drew against, which is what makes a row that stops
/// overflowing (a shorter adapter name chosen, a narrower language picked)
/// look identical to one that never did, with `None` back instead of a clip.
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
) -> (Vec<Draw>, Option<(f32, f32, f32)>) {
    if focus(menu).is_none() {
        return (list, None);
    }
    let row_y = skin.first_row_y()
        + menu.selected().saturating_sub(menu.scroll()) as f32 * skin.row_pitch();
    let Some(draw) = list.iter_mut().find(|draw| {
        matches!(draw, Draw::Text { y, align: Align::Right, .. } if (*y - row_y).abs() < f32::EPSILON)
    }) else {
        return (list, None);
    };
    let Draw::Text {
        x,
        scale,
        align,
        text,
        ..
    } = draw
    else {
        unreachable!("just matched Draw::Text above");
    };
    let width = measure(text) * *scale;
    let overflow = width - MAX_WIDTH;
    if overflow <= 0.0 {
        return (list, None);
    }
    let right = *x;
    let left = right - MAX_WIDTH;
    // A continuous pixel offset, not a character count: the clip in
    // `Renderer::push_text` trims a glyph's own quad at the box edge, so
    // there is nothing here that needs to land on a character boundary.
    *x = left - crate::anim::marquee_offset(elapsed, overflow);
    *align = Align::Left;
    (list, Some((row_y, left, right)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn measure(text: &str) -> f32 {
        text.chars().count() as f32 * 6.0
    }

    fn fixture(value: &str) -> Menu {
        let definition = crate::menu::Definition::parse(&format!(
            r#"
            version = 1
            root = "main"
            [[page]]
            id = "main"
            [[page.entry]]
            kind = "choice"
            label = "RENDERER"
            setting = "graphics.renderer"
            values = ["{value}"]
            "#
        ))
        .expect("parse");
        Menu::new(definition)
    }

    fn skin() -> Skin {
        Skin::new(oag_pulse::TITLE.menu, 22.0)
    }

    #[test]
    fn a_row_that_fits_is_left_exactly_as_drawn() {
        let menu = fixture("off");
        let before = crate::menu::draw_list(&menu, &skin(), &|_| Vec::new(), None).flatten();
        let (after, clip) = apply(before.clone(), &menu, &skin(), &measure, 5.0);
        assert_eq!(before, after);
        assert_eq!(clip, None, "nothing to scroll, so nothing to clip either");
    }

    /// The row this test names is exactly the one the machine's own real
    /// adapter name broke: 200 pixels does not fit `vulkan: llvmpipe (LLVM
    /// 22.1.8, 256 bits) (cpu)`-length strings, which is the whole reason
    /// this module exists.
    #[test]
    fn an_overflowing_row_slides_and_reports_where_to_clip() {
        let long = "vulkan: a very generationally long laptop gpu name";
        let menu = fixture(long);
        let before = crate::menu::draw_list(&menu, &skin(), &|_| Vec::new(), None).flatten();
        let (after, clip) = apply(before.clone(), &menu, &skin(), &measure, 5.0);
        assert_ne!(before, after, "an overflowing value has to move");
        let Some((_, left, right)) = clip else {
            panic!("an overflowing value needs a clip window");
        };
        assert!((right - left - MAX_WIDTH).abs() < f32::EPSILON);
    }

    /// [`Timer`] starts at zero and only resets when the identity it is
    /// handed actually changes.
    #[test]
    fn the_timer_keeps_running_while_focus_is_unchanged() {
        let mut timer = Timer::default();
        let key = Some(("graphics".to_string(), 2, "vulkan: a gpu".to_string()));
        timer.tick(0.5, key.clone());
        timer.tick(0.5, key);
        assert!((timer.elapsed() - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn changing_the_value_under_an_unmoved_cursor_resets_the_timer() {
        let mut timer = Timer::default();
        timer.tick(
            0.5,
            Some(("graphics".to_string(), 2, "vulkan: a gpu".to_string())),
        );
        timer.tick(
            0.5,
            Some(("graphics".to_string(), 2, "vulkan: another gpu".to_string())),
        );
        assert_eq!(
            timer.elapsed(),
            0.5,
            "a new value is a fresh row to read, timed from this tick rather than \
             carrying the old row's clock forward"
        );
    }
}
