//! The faces the sequence draws in: one atlas, and how every other font
//! role is sized against it.
//!
//! Split out of `frontend.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change.

use super::Frontend;

impl Frontend {
    /// Tells the sequence how big each font role's own face is against the
    /// `Default` one, as a ratio of line heights read off the `.fnt` files.
    ///
    /// Every screen widget is rasterised in the one `Default` atlas, so a
    /// `Text` authored in another role - 2048's legal footer in
    /// `NEOSANS_BOLD`, 22 units tall against `NEOSANS_BOLD_LARGE`'s 37 -
    /// draws at the wrong size unless its scale is corrected by that ratio.
    /// The ratio is the files' own; the face is still the wrong one, which
    /// on 2048 costs nothing visible (the two are one typeface at two sizes).
    ///
    /// `default_line_height` is the `Default` face's own, in grid units, for
    /// a `vertalign="middle"` widget to centre its line on its `y`.
    pub fn set_face_scales(&mut self, scales: Vec<(String, f32)>, default_line_height: f32) {
        self.face_scales = scales
            .into_iter()
            .map(|(role, scale)| (role.to_ascii_lowercase(), scale))
            .collect();
        self.default_line_height = Some(default_line_height);
    }

    /// Steps the language picker's rows by one line of the `Default` face this
    /// build loaded (`line_height`, in grid units) rather than by the table
    /// [`font_line_height`] holds, which is Pulse's.
    ///
    /// The `<Menu>` states no pitch of its own, so a line of its font is the
    /// inference either way; this only makes the line the disc's own.
    pub fn set_picker_line_height(&mut self, line_height: f32) {
        self.picker_line_height = Some(line_height);
    }

    /// The scale a widget authored in font `role` draws at, against
    /// `Default`. `1.0` for a role nothing measured.
    pub(crate) fn face_scale(&self, role: &str) -> f32 {
        self.face_scales
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case(role))
            .map_or(1.0, |(_, scale)| *scale)
    }
}

/// The line height of a font id, in the PSP's own pixel units.
///
/// Matches the `.fnt` files Pulse's language plugins resolve each role to:
/// `Default` is `pulse_text.fnt` (13px), `Menu` is `Pulse_20.fnt` (22px),
/// `Title`, `Small`, `InGame` and `Stats` are `Pulse_14.fnt` (17px), `HUD` is
/// `PulseHud.fnt` (25px) and `HUDSmall` is `small.fnt` (10px). The XML is
/// inconsistent about case (`font="menu"` and `font="Menu"` both appear), so
/// this matches case-insensitively.
///
/// # This is Pulse's table, and it is wrong on Pure
///
/// **A known, bounded gap, recorded rather than papered over.** These numbers
/// are the line heights of *Pulse's* faces, and a role does not resolve to the
/// same file on both discs - see [`crate::language::roles`]. Measured on
/// `pure-psp-eu.chd`, Pure's `Default` is `FX300ANG.fnt` at **15px** against
/// the 13 here, and its `Title` is the same file rather than a 17px one. So a
/// Pure front-end screen with more than one line of text spaces those lines by
/// up to a couple of pixels wrong.
///
/// The fix is not another table: it is reading the height back off the atlas the
/// way `boot::load_menu_font`'s caller already does for menu rows
/// (`menu::Skin::new(skin, rows_face.line_height)`), which needs each role's
/// `.fnt` actually loaded rather than only the `Default` one. That is real work
/// and it is not what the boot path is blocked on, so it is named here and left.
/// Nothing about it is title-specific once done - it deletes this function.
pub(crate) fn font_line_height(font: &str) -> f32 {
    match font.to_ascii_lowercase().as_str() {
        "menu" => 22.0,
        "title" | "small" | "ingame" | "stats" => 17.0,
        "hud" => 25.0,
        "hudsmall" => 10.0,
        // "Default", and anything this build does not otherwise recognise.
        _ => 13.0,
    }
}

/// Raises a colour to something visible on black, keeping its hue.
///
/// The picker's title is `0xFF000000` in the XML because the real screen has a
/// lit background behind it. Drawing black on black would look like a bug in
/// this code rather than a missing background, so near-black is lifted.
pub(crate) fn lighten(rgba: [f32; 4]) -> [f32; 4] {
    let luma = 0.299 * rgba[0] + 0.587 * rgba[1] + 0.114 * rgba[2];
    if luma < 0.15 {
        [0.85, 0.9, 0.95, rgba[3]]
    } else {
        rgba
    }
}
