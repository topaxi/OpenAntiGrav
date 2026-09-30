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

impl super::Frontend {
    /// One line of `font`, in grid units, for a row that states no pitch.
    ///
    /// The `Default` face's own height, read off the atlas this build loaded,
    /// once [`Self::set_picker_line_height`] has said the title wants it; every
    /// other role is that height times the ratio [`Self::face_scale`] holds, and
    /// a role with no measured ratio falls back to the table rather than guess.
    /// Without the setting it is the table, as before.
    fn picker_line_height(&self, font: &str) -> f32 {
        match self.picker_line_height {
            Some(height) if font.eq_ignore_ascii_case("default") => height,
            Some(height)
                if self
                    .face_scales
                    .iter()
                    .any(|(r, _)| r.eq_ignore_ascii_case(font)) =>
            {
                height * self.face_scale(font)
            }
            _ => font_line_height(font),
        }
    }

    /// How wide the selected row's band is, in grid units, at `scale`.
    ///
    /// **Chosen, not measured.** The table case keeps the 220 the drawing and
    /// the pointer already shared; with a face-derived pitch the rows are set in
    /// HD's 33-unit face, whose longest native name (`SimplifiedChinese`) is
    /// about 270 wide, so the band is 300.
    pub(super) fn band_width(&self, scale: f32) -> f32 {
        if self.picker_line_height.is_some() {
            300.0 * scale
        } else {
            super::pointer::ROW_WIDTH * scale
        }
    }

    pub(super) fn language_rows(&self, menu: Option<&crate::screen::Menu>) -> LanguageRows {
        language_rows(
            menu,
            self.picker_line_height(menu.map_or("Default", |m| m.font.as_str())),
        )
    }
}

fn language_rows(menu: Option<&crate::screen::Menu>, line_height: f32) -> LanguageRows {
    let scale = menu.map_or(1.0, |m| m.scale).max(0.5);
    LanguageRows {
        x: menu.map_or(0.0, |m| m.x),
        y: menu.map_or(0.0, |m| m.y),
        scale,
        pitch: line_height * scale,
        align: Align::parse(menu.map_or("left", |m| m.align.as_str())),
    }
}
