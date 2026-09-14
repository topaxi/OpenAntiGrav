//! Where the language picker's rows go.
//!
//! One function, in a module of its own, because two things read it: the
//! drawing (`draw.rs`) to place the rows, and the pointer (`pointer.rs`) to
//! find the one under a point. Written once so the two cannot disagree -
//! the same drift rule `crate::menu::rows` keeps between `Menu::scroll` and
//! the skin.

use super::{Align, font_line_height};

/// The `Menu` widget's own anchor, scale and alignment, and a pitch of one
/// line of its font at that scale.
///
/// A screen with no `Menu` widget gets the origin, which is what the drawing
/// always fell back to.
pub(super) struct LanguageRows {
    pub(super) x: f32,
    pub(super) y: f32,
    pub(super) scale: f32,
    pub(super) pitch: f32,
    pub(super) align: Align,
}

pub(super) fn language_rows(menu: Option<&crate::screen::Menu>) -> LanguageRows {
    let scale = menu.map_or(1.0, |m| m.scale).max(0.5);
    LanguageRows {
        x: menu.map_or(0.0, |m| m.x),
        y: menu.map_or(0.0, |m| m.y),
        scale,
        pitch: font_line_height(menu.map_or("Default", |m| m.font.as_str())) * scale,
        align: Align::parse(menu.map_or("left", |m| m.align.as_str())),
    }
}
