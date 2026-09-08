//! Where Pulse keeps its built-in campaign: `Data\Plugins\grids\Definition.xml`
//! and the sixteen `grid_NN.xml` files it lists, all inside `Data.wad`.
//!
//! **Names only, no decoding.** How a `PI_Grid`/`PI_Cell` document parses is
//! `oag_formats::race_campaign`'s job - a property of the file's own shape,
//! not of this release - and per [ADR-0022] this crate carries no engine-side
//! type for it either: only the USA PSP pressing has been checked for this
//! path (`docs/ghidra/functions/psp-pulse-usa/race-campaign.md`), and item 4
//! of that ADR reserves `oag-title` axes for a fact measured against two
//! corpora. A second Pulse release found to differ here - EU, PS2, or a
//! different grid count - is exactly the kind of finding this file should stay
//! small enough to not obscure.
//!
//! ```no_run
//! use oag_assets::Archive;
//! use oag_formats::race_campaign;
//!
//! let mut archive = Archive::open("data/images/pulse-psp-usa.chd:PSP_GAME/USRDIR/Data.wad")?;
//! let definition = archive.read_name(oag_pulse::campaign::DEFINITION_ENTRY)?;
//! let expanded = oag_formats::fexml::text(&definition)?;
//! for src in race_campaign::definition_entries(&expanded) {
//!     let blob = archive.read_name(&src)?;
//!     let grid = race_campaign::from_blob(&blob)?;
//!     println!("{}: {} cells", grid.name, grid.cells.len());
//! }
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! [ADR-0022]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0022-title-packages.md

/// The file that lists every grid.
///
/// Resolves only if the path is already known, because the WAD's own name
/// table carries no directory, so a fresh name-mining pass would not find it
/// by guessing shorter prefixes. See `docs/formats/race-setup.md`'s "Settled
/// the same day" section for how this was found: from a
/// `"%s\Definition.xml"` format string in `BOOT.BIN` applied to the literal
/// `"Data\Plugins\grids"`.
pub const DEFINITION_ENTRY: &str = r"Data\Plugins\grids\Definition.xml";

/// How many grids the USA PSP pressing's own `Definition.xml` lists.
///
/// **A measurement of one disc, not a promise about every release.** Reading
/// [`DEFINITION_ENTRY`] itself (via
/// [`oag_formats::race_campaign::definition_entries`](../../oag_formats/race_campaign/fn.definition_entries.html))
/// is what a caller wanting a release-agnostic count should do; this constant
/// exists so a test can assert this disc still says sixteen without decoding
/// the file, and so [`entry_name`] has a range to check `index` against.
pub const GRID_COUNT: u8 = 16;

/// The entry name for one grid, `Data\Plugins\grids\grid_00.xml` through
/// `grid_15.xml` - the exact spelling [`DEFINITION_ENTRY`] itself lists, two
/// digits, zero-padded.
///
/// A caller that already has [`DEFINITION_ENTRY`]'s own `Src=` list (the
/// robust path, since it does not assume the count or the naming scheme) has
/// no need of this; it exists for the common case of wanting one grid by
/// index without reading `Definition.xml` first, and for the ground-truth test
/// that checks the two agree.
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
