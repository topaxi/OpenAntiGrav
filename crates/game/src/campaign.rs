//! Reads the Race Campaign off an already-open source: the two screens'
//! layout and every grid tier's own cells.
//!
//! One function, called from two places that cannot share a module tree -
//! `crate::main::session::campaign` (the live session) and
//! `crate::capture::menu_page` (`--menu-page grid-select`/`cell-select`) -
//! the same reason [`crate::preview`] exists for the race box's own
//! selection screens.

use anyhow::{Context, Result};
use oag_tables::race_campaign::{self, Grid};
use oag_ui::campaign::Layout;
use oag_ui::language::StringTable;
use oag_ui::picker::FaceScales;

use crate::sprite::Sheet;

/// `Data\Plugins\PI001\GUI\CellMode_Definition.xml` - the file Pulse's own
/// `Grid Selection` and `Cell Selection` are authored in. See
/// [`oag_hd::campaign::SCREEN_ENTRY`] for Wipeout HD/Fury's own copy, at a
/// different path and a different authored resolution - [`load`] picks
/// between the two by `title`, the same way the rest of this crate picks a
/// title's tables. A source that carries neither is [`load`]'s `Err` case.
pub const SCREEN_ENTRY: &str = r"Data\Plugins\PI001\GUI\CellMode_Definition.xml";

/// The two hex textures both of Pulse's screens draw from, neither of which
/// is part of `Skin.xml`'s own front-end sheet - see [`load`]'s own
/// extension step. [`oag_hd::campaign::HEX_TEXTURES`]/`OTHER_TEXTURES` are
/// HD's own, five and three respectively rather than two, since HD layers a
/// fourth hex texture (`Bg_x_y`) under Pulse's three.
const HEX_TEXTURES: [&str; 2] = [
    r"Data\FE\Images\hex_filled.mip",
    r"Data\FE\Images\hex_outline.mip",
];

/// Everything a caller needs to draw and drive the campaign: every grid's
/// own cells, both screens' disc-authored layout, and the sprite sheet to
/// draw them from.
#[derive(Debug)]
pub struct Campaign {
    pub grids: Vec<Grid>,
    pub grid_layout: Layout,
    pub cell_layout: Layout,
    /// `base` extended with the title's own hex/other textures - the same
    /// "front end's own sheet plus this screen's own art" shape
    /// `oag_ui::picker::slideshow`'s stills already extend it with.
    pub sprites: Sheet,
}

/// Reads the campaign screen off `archives`, title-dispatched: Pulse's
/// `Data\Plugins\PI001\GUI\CellMode_Definition.xml` and
/// `Data\Plugins\grids\Definition.xml`, or Wipeout HD/Fury's own copies of
/// the same two roles ([`oag_hd::campaign::SCREEN_ENTRY`]/
/// `DEFINITION_ENTRY`) - picked by `title.name` against
/// [`oag_hd::TITLE`]'s own `name`, the same identity check
/// `crate::main::session::remix` already uses to tell HD's own craft
/// roster apart from the other two titles', for the reason that module's
/// own doc gives: a title package declares its `Title` as a `const`, so
/// `std::ptr::eq` against a promoted temporary is not reliable, but the
/// `name` string is.
///
/// Every grid file that will not parse is skipped and logged - the same
/// per-row tolerance `oag_tables::race_campaign_ground_truth` itself expects
/// of the disc's own sixteen; one bad file should not blank a screen that
/// has fifteen good ones to show. **This project's own read precedence,
/// unchanged**: which archive copy of a shared HD grid path
/// (`Data\Plugins\grids\grid_00.xml`..`grid_07.xml`, flat on `DATA02`,
/// per-difficulty on `DATA04`/`DATA06`) is read is whatever
/// `oag_assets::Archives::read_name` already resolves to - **chosen, not
/// measured**, per the driving brief; see
/// `handover/gameplay/hd-furys-campaign-grids-parse-and-the-screen-is-next.md`
/// for the discriminating measurement this pass took (`DATA02`'s own flat
/// `Gold`/`Silver`/`Bronze` values equal `DATA04`/`DATA06`'s **hard** rung,
/// not the medium one `oag_tables::race_campaign::Cell::gold`'s own doc
/// otherwise assumes) and why it stopped short of an RPCS3 capture to
/// settle which rung the screen itself shows by default.
///
/// # Errors
///
/// Propagates a missing or unreadable screen entry, a screen definition
/// missing `Grid Selection`/`Cell Selection`, or a `Definition.xml` that
/// yields no grid at all.
pub fn load(
    archives: &mut oag_assets::Archives,
    strings: &StringTable,
    faces: FaceScales,
    grid: [f32; 2],
    base: &Sheet,
    // The front-end root's own `FEGlobals` (`shell.screens.globals` live,
    // `frontend.screens().globals` in a `--menu-page` capture) - fallbacks
    // for a `FEGlobals->` reference this file declares no `<Variable>` for
    // itself, the same `Screens::from_xml_with_fallback_globals` idiom
    // `crate::boot::screens::load_included_screens` already uses for every
    // other `LoadXML` include. Without this, `Outline_x_y`'s own
    // `i="FEGlobals->CM_HEX_Outline"` (`Data\Plugins\PI001\GUI\Skin.xml`,
    // `0x7F34ACC2`) never resolves and the hex outline draws opaque white
    // instead of its authored translucent teal.
    fallback_globals: &[(&str, &str)],
    title: &'static oag_title::Title,
) -> Result<Campaign> {
    if title.name == oag_hd::TITLE.name {
        return load_hd(archives, strings, faces, grid, base, fallback_globals);
    }

    let blob = archives
        .read_name(SCREEN_ENTRY)
        .with_context(|| format!("reading {SCREEN_ENTRY}"))?;
    let xml = oag_tables::fexml::text(&blob).context("expanding CellMode_Definition.xml")?;
    let screens = oag_ui::screen::Screens::from_xml_with_fallback_globals(&xml, fallback_globals);
    let grid_layout = Layout::read(&screens, "Grid Selection", strings, faces, grid)
        .context("Grid Selection is not on this screen")?;
    let cell_layout = Layout::read(&screens, "Cell Selection", strings, faces, grid)
        .context("Cell Selection is not on this screen")?;

    let grids = read_grids(archives, oag_pulse::campaign::DEFINITION_ENTRY)?;

    let mut blobs = Vec::new();
    for src in HEX_TEXTURES {
        match archives.read_name(src) {
            Ok(blob) => blobs.push((src.to_string(), blob)),
            Err(error) => log::warn!("{src}: {error:#} - the hex grid draws without it"),
        }
    }
    let mut report = Vec::new();
    let sprites = base.extended(&blobs, &mut report);
    for line in report {
        log::info!("campaign sprites {line}");
    }

    Ok(Campaign {
        grids,
        grid_layout,
        cell_layout,
        sprites,
    })
}

/// [`load`]'s Wipeout HD/Fury branch - the same shape, off
/// [`oag_hd::campaign`]'s own entry names, HD's own authored grid
/// ([`oag_hd::campaign::AUTHORED_GRID`], not the PSP's `PSP_GRID`
/// [`oag_ui::campaign::Layout::read`] assumes), and no `oag_tables::fexml`
/// expansion of the screen XML - it is plain UTF-8 on this title, unlike
/// Pulse's dictionary-shortened copy. See `oag_ui::campaign::hd`'s own
/// module doc for what the two screens draw once resolved this way.
fn load_hd(
    archives: &mut oag_assets::Archives,
    strings: &StringTable,
    faces: FaceScales,
    grid: [f32; 2],
    base: &Sheet,
    fallback_globals: &[(&str, &str)],
) -> Result<Campaign> {
    let blob = archives
        .read_name(oag_hd::campaign::SCREEN_ENTRY)
        .with_context(|| format!("reading {}", oag_hd::campaign::SCREEN_ENTRY))?;
    let xml = String::from_utf8(blob).context("CellMode_Definition.xml is not UTF-8")?;
    let screens = oag_ui::screen::Screens::from_xml_with_fallback_globals(&xml, fallback_globals);
    let grid_layout = Layout::read_authored(
        &screens,
        "Grid Selection",
        strings,
        faces,
        grid,
        oag_hd::campaign::AUTHORED_GRID,
    )
    .context("Grid Selection is not on this screen")?;
    let cell_layout = Layout::read_authored(
        &screens,
        "Cell Selection",
        strings,
        faces,
        grid,
        oag_hd::campaign::AUTHORED_GRID,
    )
    .context("Cell Selection is not on this screen")?;

    let grids = read_grids(archives, oag_hd::campaign::DEFINITION_ENTRY)?;

    let mut blobs = Vec::new();
    // `HEX_TEXTURES`' own `(widget src, archive path)` pairs - read off the
    // second, shelved under the first, since that is the spelling
    // `image.src` carries at draw time. See that constant's own doc for why
    // the two differ.
    for (widget_src, archive_path) in oag_hd::campaign::HEX_TEXTURES {
        match archives.read_name(archive_path) {
            Ok(blob) => blobs.push((widget_src.to_string(), blob)),
            Err(error) => {
                log::warn!("{archive_path}: {error:#} - the widget it is for draws nothing");
            }
        }
    }
    for src in oag_hd::campaign::OTHER_TEXTURES {
        match archives.read_name(src) {
            Ok(blob) => blobs.push((src.to_string(), blob)),
            Err(error) => log::warn!("{src}: {error:#} - the widget it is for draws nothing"),
        }
    }
    let mut report = Vec::new();
    let sprites = base.extended(&blobs, &mut report);
    for line in report {
        log::info!("campaign sprites {line}");
    }

    Ok(Campaign {
        grids,
        grid_layout,
        cell_layout,
        sprites,
    })
}

/// Every grid `definition_entry` lists, off `archives`' own read
/// precedence - shared by [`load`]'s Pulse path and [`load_hd`], since both
/// titles author the same `PI_Grid`/`PI_Cell` schema
/// ([`oag_tables::race_campaign`]) behind a `Definition.xml` naming the same
/// shape of `Src=` list, dictionary-shortened on both.
fn read_grids(archives: &mut oag_assets::Archives, definition_entry: &str) -> Result<Vec<Grid>> {
    let definition_blob = archives
        .read_name(definition_entry)
        .with_context(|| format!("reading {definition_entry}"))?;
    let definition_xml =
        oag_tables::fexml::text(&definition_blob).context("expanding grids/Definition.xml")?;
    let mut grids = Vec::new();
    for src in race_campaign::definition_entries(&definition_xml) {
        match archives
            .read_name(&src)
            .map_err(anyhow::Error::from)
            .and_then(|blob| race_campaign::from_blob(&blob).map_err(anyhow::Error::from))
        {
            Ok(grid) => grids.push(grid),
            Err(error) => log::warn!("{src}: {error:#} - grid skipped"),
        }
    }
    if grids.is_empty() {
        anyhow::bail!("no grid in {definition_entry} parsed");
    }
    Ok(grids)
}

/// Which [`oag_race::Mode`] a campaign cell's own [`race_campaign::Mode`]
/// launches as - `None` for the four this engine cannot run at all.
///
/// **Not a spelling mismatch to resolve, a scope one.** `oag_race::Mode` has
/// five variants because that is what `crates/race` implements; a campaign
/// cell's own mode is one of nine, read straight off the disc
/// (`docs/ghidra/functions/psp-pulse-usa/race-campaign.md`). `Tournament`
/// carries per-leg standings this engine has no state for at all;
/// `Head2Head` has no [`oag_race::Mode`] variant to map onto - a two-craft
/// race is a design question (how many opponents, which HUD), not a naming
/// one, and inventing an answer here is exactly what `CLAUDE.md`'s "never
/// invent what the assets author" forbids; `Custom Grid`/`AI Race` are not
/// authored by any shipped `grid_NN.xml` cell at all. A cell whose mode maps
/// to `None` must not launch - the caller logs why and stays on `Cell
/// Selection` rather than substituting an implemented mode for an
/// unimplemented one.
#[must_use]
pub fn race_mode_for_cell(mode: race_campaign::Mode) -> Option<oag_race::Mode> {
    match mode {
        race_campaign::Mode::Race => Some(oag_race::Mode::SingleRace),
        race_campaign::Mode::TimeTrial => Some(oag_race::Mode::TimeTrial),
        race_campaign::Mode::Zone => Some(oag_race::Mode::Zone),
        race_campaign::Mode::Elimination => Some(oag_race::Mode::Eliminator),
        race_campaign::Mode::SpeedLap => Some(oag_race::Mode::SpeedLap),
        // `Other` is a mode name with no Pulse ordinal at all (HD's
        // `NitroBattle`/`Detonator`) - nothing to map onto, so it refuses
        // the same way the four unimplemented Pulse modes do.
        race_campaign::Mode::Tournament
        | race_campaign::Mode::Head2Head
        | race_campaign::Mode::CustomGrid
        | race_campaign::Mode::AiRace
        | race_campaign::Mode::Other(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::race_mode_for_cell;
    use oag_tables::race_campaign::Mode as CampaignMode;

    #[test]
    fn the_five_implemented_modes_map_onto_their_oag_race_mode() {
        assert_eq!(
            race_mode_for_cell(CampaignMode::Race),
            Some(oag_race::Mode::SingleRace)
        );
        assert_eq!(
            race_mode_for_cell(CampaignMode::TimeTrial),
            Some(oag_race::Mode::TimeTrial)
        );
        assert_eq!(
            race_mode_for_cell(CampaignMode::Zone),
            Some(oag_race::Mode::Zone)
        );
        assert_eq!(
            race_mode_for_cell(CampaignMode::Elimination),
            Some(oag_race::Mode::Eliminator)
        );
        assert_eq!(
            race_mode_for_cell(CampaignMode::SpeedLap),
            Some(oag_race::Mode::SpeedLap)
        );
    }

    #[test]
    fn the_four_unimplemented_modes_refuse_to_map_at_all() {
        for mode in [
            CampaignMode::Tournament,
            CampaignMode::Head2Head,
            CampaignMode::CustomGrid,
            CampaignMode::AiRace,
        ] {
            assert_eq!(
                race_mode_for_cell(mode.clone()),
                None,
                "{mode} should not launch"
            );
        }
    }
}
