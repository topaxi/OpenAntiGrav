//! Where Wipeout HD keeps its campaign: `Data\Plugins\grids\Definition.xml`
//! and the sixteen `grid_NN.xml` files it lists - the same `PI_Grid`/
//! `PI_Cell` schema Pulse ships, read by the same
//! [`oag_tables::race_campaign`] parser, extended additively to cover what
//! this title authors that Pulse does not. See
//! `docs/formats/race-campaign.md`'s HD section for the schema diff, the
//! counts, and the archive table this module's own doc comments summarise.
//!
//! **Names only, no decoding** - the same split [`oag_pulse::campaign`]
//! keeps, per [ADR-0022]. A file's *shape* lives in `oag_tables`; what this
//! title ships lives here.
//!
//! # Split across four archives, and the split does not agree with itself
//!
//! Unlike Pulse's one `Data.wad`, HD's `Data\Plugins\grids\` directory is on
//! four of the seven PSARCs, and they do not all carry the same sixteen
//! grids or the same schema for the grids they share:
//!
//! | Archive | Grids | Schema | `Campaign=` |
//! | --- | --- | --- | --- |
//! | `DATA00.PSARC` | `grid_08`..`grid_15` (Fury-only, absent elsewhere) | per-difficulty (`EasyGold`..`HardBronze`, `NitroElim*`) | `"Fury"` |
//! | `DATA02.PSARC` | `grid_00`..`grid_07` | flat (`Gold`/`Silver`/`Bronze`), Pulse's own shape | absent |
//! | `DATA04.PSARC` | `grid_00`..`grid_07` | per-difficulty | absent |
//! | `DATA06.PSARC` | `grid_00`..`grid_07` | per-difficulty | `"HD"` |
//!
//! [`oag_assets::Archives`]'s own precedence (`data` = `DATA00`, `fe` =
//! `DATA02`, `extra` = `DATA01`, `DATA03`, `DATA04`, `DATA05`, `DATA06` in
//! that order - see [`crate::TITLE`]) reaches `DATA00` for `grid_08`..`15`
//! and falls through to `DATA02` for `grid_00`..`07`, since `DATA00` does not
//! carry those eight at all. So **the campaign this project's own asset
//! layer reads is genuinely mixed-schema**: the first eight grids flat, the
//! last eight per-difficulty - a fact about this ordering, on the same terms
//! `docs/formats/hd-hud.md` and [`crate::names`] already record for the
//! language table and the front-end `Definition.xml`, not a measurement of
//! what a PS3 loads. `DATA04.PSARC`'s and `DATA06.PSARC`'s own per-difficulty
//! copies of `grid_00`..`07` are real, on the disc, and unreached by this
//! precedence; a caller that wants them has to open those two archives
//! directly, the way `crates/hd/tests/campaign_grids_ground_truth.rs` does to
//! measure the schema at all.
//!
//! Sixteen total either way: `DATA00`'s own `Definition.xml` - the one
//! [`oag_assets::Archives`] resolves to for that path, on the same "first
//! archive checked that has it" rule that lands on `DATA00`'s copy of
//! [`crate::names::FRONT_END_PLUGIN_DEFINITION`] too - lists all sixteen
//! `grid_00`..`grid_15`; `DATA02`'s and `DATA04`'s own copies list only the
//! eight they themselves carry.
//!
//! # One grid's own tag is broken on the disc
//!
//! `grid_04.xml`'s `<Values>` opening tag is missing its closing `>` before
//! `BillboardName`'s value in **all three** of its copies (`DATA02`,
//! `DATA04`, `DATA06`, byte-identical at the break) - `...vex"</Values>`
//! where every other grid in the corpus reads `...vex"></Values>`. Read
//! structurally, that swallows the tag meant to close `<Values>` into the
//! start tag's own attribute text, so the five `<PI_Cell>` elements that
//! follow become descendants of `<Values>` rather than of `<PI_Grid>`, and
//! [`oag_tables::race_campaign::parse`] correctly reads zero cells for a
//! grid that authors five. **This is not a bug in that parser to fix** - the
//! file is broken on the disc, in three independent copies, and inventing a
//! recovery would be guessing at which cells the original intended rather
//! than reading what is there. Whether Wipeout HD's own XML reader tolerates
//! the same break is not established here and would need decompiling its
//! executable, out of this pass's scope - see
//! `docs/formats/race-campaign.md`'s HD section.
//!
//! [ADR-0022]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0022-title-packages.md

/// The file that lists every grid. Present on `DATA00`, `DATA02` and
/// `DATA04`; `DATA00`'s own copy is the one [`oag_assets::Archives`]
/// resolves to, and the only one of the three naming all sixteen grids.
pub const DEFINITION_ENTRY: &str = r"Data\Plugins\grids\Definition.xml";

/// How many grids the disc's fullest `Definition.xml` (`DATA00`'s) lists -
/// the same count as Pulse's own campaign, per
/// [`oag_pulse::campaign::GRID_COUNT`], though split across archives
/// differently and not sharing one schema - see the module docs.
pub const GRID_COUNT: u8 = 16;

/// The entry name for one grid, `Data\Plugins\grids\grid_00.xml` through
/// `grid_15.xml` - the same spelling [`oag_pulse::campaign::entry_name`]
/// produces, since both titles zero-pad the same two digits.
#[must_use]
pub fn entry_name(index: u8) -> String {
    format!(r"Data\Plugins\grids\grid_{index:02}.xml")
}

/// The screen `Grid Selection`/`Cell Selection` are both authored in -
/// `Data\Plugins\Frontend\Gui\CellMode_Definition.xml`, HD's counterpart to
/// [`oag_pulse::campaign`]'s `Data\Plugins\PI001\GUI\CellMode_Definition.xml`.
/// Present on `DATA02` (42,548 bytes) and, disagreeing, `DATA06` (59,361
/// bytes) - [`oag_assets::Archives`]'s own precedence reaches `DATA02`'s
/// copy for `oag_game::campaign::load_hd`'s own `Grid Selection`/`Cell
/// Selection` read.
///
/// **`DATA06`'s copy is a later, Fury-era build, not merely a different
/// one - now measured, not just presumed.** It is the only copy carrying a
/// `Campaign Selection` screen and a `Grid Selection Fury` screen at all;
/// `DATA02`'s copy has neither, and its own `Grid Selection`'s `flyerlist`
/// carries eight `<Entry IDString="BLANK">` placeholders where `DATA06`'s
/// names real grids (`"grid0"`..`"grid7"`). Confirmed against a live PS3:
/// RPCS3's own `TTY.log` prints `Grid Selection Fury` as a screen name on
/// `hdfury-ps3-eu-dec.iso`, a name that exists nowhere in `DATA02`'s copy -
/// see `docs/ui/campaign-screens.md`'s "Wipeout HD/Fury: `Campaign
/// Selection`" section for the full comparison, including the one place
/// `DATA06`'s own `Cell Selection` diverges further (a renamed target-medal
/// row, an extra background layer) that this crate does not adopt.
/// `oag_game::campaign::load_hd` reads `Campaign Selection`/`Grid Selection
/// Fury` from `DATA06` directly (by archive label, not precedence) and
/// leaves `Grid Selection`/`Cell Selection` on `DATA02`, since those two are
/// diffed byte-for-byte equivalent between the two copies (bar two
/// `FEGlobals->` colour names this title's own `skin.xml` resolves
/// identically either way).
///
/// **Plain UTF-8, not dictionary-shortened** - unlike [`DEFINITION_ENTRY`],
/// reading this needs no `oag_tables::fexml` expansion first. See
/// `docs/ui/campaign-screens.md`'s HD section.
pub const SCREEN_ENTRY: &str = r"Data\Plugins\Frontend\Gui\CellMode_Definition.xml";

/// `grid0`..`grid7`, the base `Wipeout HD` campaign's own slice of
/// [`DEFINITION_ENTRY`]'s sixteen grids - `Campaign Selection`'s
/// `FE_RC_HD` entry redirects to `Grid Selection`, which pages exactly this
/// range (`DATA06`'s own `flyerlist`, `"grid0"`..`"grid7"`). See
/// `docs/ui/campaign-screens.md`'s "Wipeout HD/Fury: `Campaign Selection`"
/// section.
pub const HD_GRID_RANGE: std::ops::Range<usize> = 0..8;

/// `grid8`..`grid15`, `Fury`'s own slice - `Campaign Selection`'s
/// `FE_RC_FURY` entry (the measured default) redirects to `Grid Selection
/// Fury`, which pages exactly this range. See [`HD_GRID_RANGE`]'s own doc.
pub const FURY_GRID_RANGE: std::ops::Range<usize> = 8..16;

/// The screen `Campaign Selection`/`Grid Selection Fury` are authored in -
/// [`SCREEN_ENTRY`]'s own path, but only `DATA06`'s copy, not the
/// precedence-resolved one. See [`SCREEN_ENTRY`]'s own doc for why.
pub const SELECTION_SCREEN_ARCHIVE: &str = crate::archives::DATA06;

/// `Campaign Selection`'s own screen name, as `DATA06`'s `CellMode_Definition.xml`
/// authors it. See [`SCREEN_ENTRY`]'s own doc.
pub const SELECTION_SCREEN: &str = "Campaign Selection";

/// `Grid Selection Fury`'s own screen name - `Campaign Selection`'s `FE_RC_FURY`
/// entry's own redirect target, and [`FURY_GRID_RANGE`]'s own screen. See
/// [`SCREEN_ENTRY`]'s own doc.
pub const FURY_GRID_SCREEN: &str = "Grid Selection Fury";

/// The grid `CellMode_Definition.xml` is authored in - HD's own screen
/// resolution, unlike Pulse's copy of the same two screen names, which
/// `oag_ui::campaign::Layout`'s own `PSP_GRID` constant is written at
/// 480x272 for. Measured directly off the file: `<Flyer>`'s own
/// `OriginX="960" OriginY="540"` is dead centre of 1920x1080, and widgets
/// place out past 480 on both axes (`DifficultyButton` at `x="944"
/// y="994"`), matching [`oag_title::MenuSkin::space`]'s own reading of
/// `skin.xml`'s `<Movie Width="1920" height="1080">` for the rest of this
/// title's front end. Confidence 90 - a direct read of the file's own
/// numbers, not yet cross-checked against a live capture.
pub const AUTHORED_GRID: [f32; 2] = [1920.0, 1080.0];

/// The hex textures both screens draw from, none of which is part of
/// `skin.xml`'s own front-end sheet - the same "front end's own sheet plus
/// this screen's own art" extension [`oag_pulse::campaign`]'s own
/// `HEX_TEXTURES` makes for Pulse's two, widened to HD's four-layer hex
/// (`Bg_x_y`/`Outline_x_y`/`Lock_x_y`/`Medal_x_y`, each its own texture)
/// plus the `Selector` overlay.
///
/// **`(widget src, archive path)` pairs, not one string.** `CellMode_Definition.xml`
/// spells every one of these `src="Data\FE\Images\Hexagon_HD_OUTLINE.mip"` -
/// the PSP-era extension, same as [`crate::frontend::names::MENU_STRIP_CURSOR`]'s
/// own doc already found for `cursor.mip` - but no archive on this disc
/// carries a `.mip` by that stem; each is a `.gtf`, lower-cased, at
/// `data/fe/images/hexagon_hd_outline.gtf` and siblings. Measured directly:
/// `oag_assets::psarc::Archive::paths` over all seven archives on
/// `hdfury-ps3-eu-dec.iso` lists every stem below as a `.gtf` and none as a
/// `.mip`. `oag_assets::psarc`'s own path normalisation folds case and
/// backslashes for the *read*, but `oag_game::sprite::Sheet::get` keys its
/// placements by an exact string match against a widget's own `image.src` -
/// so a caller has to read the `.gtf` off the archive and then shelve the
/// decoded blob under the `.mip` spelling the widget actually asks for at
/// draw time, which is what the first element of each pair names and the
/// second reads.
pub const HEX_TEXTURES: [(&str, &str); 5] = [
    (
        r"Data\FE\Images\Hexagon_HD_OUTLINE.mip",
        r"Data\FE\Images\Hexagon_HD_OUTLINE.gtf",
    ),
    (
        r"Data\FE\Images\Hexagon_HD.mip",
        r"Data\FE\Images\Hexagon_HD.gtf",
    ),
    (
        r"Data\FE\Images\Hexlock_HD.mip",
        r"Data\FE\Images\Hexlock_HD.gtf",
    ),
    (
        r"Data\FE\Images\Hexmedal_HD.mip",
        r"Data\FE\Images\Hexmedal_HD.gtf",
    ),
    (
        r"Data\FE\Images\Hexagon_HD_THICK_OUT.mip",
        r"Data\FE\Images\Hexagon_HD_THICK_OUT.gtf",
    ),
];

/// Everything else `SCREEN_ENTRY`'s own `src=` attributes name that this
/// build draws, beyond the hex art above - the bullet arrow beside every
/// detail-column row, the lock overlay HD's `Grid Selection` shows over a
/// locked flyer, and the bracket-cornered ticker's placeholder fill. Every
/// one of these is already spelled `.gtf` on both the widget and the
/// archive, unlike [`HEX_TEXTURES`], so a flat list is enough. Read the same
/// way: off the file directly, not invented. A texture that will not decode
/// still leaves the widget it was for undrawn - see
/// `docs/ui/campaign-screens.md`'s HD section.
pub const OTHER_TEXTURES: [&str; 3] = [
    r"Data\FE\Images\NonSelectable_Arrow_HD.gtf",
    r"Data\FE\Images\Subtitle_Arrow_HD.gtf",
    r"Data\FE\Images\Padlock.gtf",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entry_names_are_spelled_the_way_definition_xml_spells_them() {
        assert_eq!(entry_name(0), r"Data\Plugins\grids\grid_00.xml");
        assert_eq!(entry_name(15), r"Data\Plugins\grids\grid_15.xml");
    }

    #[test]
    fn the_two_campaign_ranges_cover_all_sixteen_grids_with_no_overlap() {
        assert_eq!(HD_GRID_RANGE, 0..8);
        assert_eq!(FURY_GRID_RANGE, 8..16);
        assert_eq!(HD_GRID_RANGE.end, FURY_GRID_RANGE.start);
        assert_eq!(FURY_GRID_RANGE.end, usize::from(GRID_COUNT));
    }
}
