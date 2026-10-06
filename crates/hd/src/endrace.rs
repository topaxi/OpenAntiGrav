//! Where the three screens a race ends on live on this disc, and the one
//! piece of art this pass draws beyond the screen's own text and fills.
//!
//! See `docs/formats/hd-endrace-screens.md` for the widget-by-widget read
//! [`oag_ui_screens::endrace::hd`] draws off, and `oag_game::endrace`'s own title
//! dispatch for the loader that reads this module's constants.

/// `Data\Plugins\Frontend\Gui\EndRace_Definition.xml` - see
/// [`oag_title::FrontEnd::endrace_entry`], which restates this same value as
/// the axis a caller reaches it through, and
/// `crate::frontend::names::ENDRACE_DEFINITION` for the evidence (five of
/// seven archives, no two alike by MD5).
pub const SCREEN_ENTRY: &str = crate::frontend::names::ENDRACE_DEFINITION;

/// The grid `EndRace_Definition.xml` is authored in - the same 1920x1080
/// this title's whole front end authors in
/// (`oag_title::MenuSkin::space`, `docs/formats/hd-frontend.md`), measured
/// directly off the file: the grid `Item` that positions `EndRace Results`'
/// own standings table is `OffsetX="375" OffsetY="368"`, well past a 480-wide
/// PSP screen, and `EndRace Rewards`' backdrop is `width="2496"` - a
/// safe-zone-oversized fill for a 1920-wide screen, not a 480-wide one.
pub const AUTHORED_GRID: [f32; 2] = [1920.0, 1080.0];

/// Every texture the HD end screens draw beyond their own fills - the
/// title-bar arrow glyph beside `ResultsTitle`/`FE_MENU`, already spelled
/// `.gtf` on the widget itself (`src="Data\FE\Images\Title_Arrow_HD.gtf"`),
/// unlike [`crate::campaign::HEX_TEXTURES`]'s own `.mip`-to-`.gtf` respell -
/// and `EndRace Podium`'s plinth fill, `dot.gtf`.
/// Every other image either widget names - the ship-badge column, the
/// target/medal/record-notify art - is left unresolved on purpose; see
/// `oag_ui_screens::endrace::hd`'s own module doc for why.
pub const EXTRA_TEXTURES: [&str; 2] = [
    r"Data\FE\Images\Title_Arrow_HD.gtf",
    r"Data\FE\Images\dot.gtf",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_screen_entry_is_a_named_plugin_path_like_the_rest_of_this_front_end() {
        assert_eq!(
            SCREEN_ENTRY,
            r"Data\Plugins\Frontend\Gui\EndRace_Definition.xml"
        );
    }
}
