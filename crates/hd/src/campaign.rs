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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entry_names_are_spelled_the_way_definition_xml_spells_them() {
        assert_eq!(entry_name(0), r"Data\Plugins\grids\grid_00.xml");
        assert_eq!(entry_name(15), r"Data\Plugins\grids\grid_15.xml");
    }
}
