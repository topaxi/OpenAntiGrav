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

/// `Data\Plugins\PI001\GUI\CellMode_Definition.xml` - the file
/// `Grid Selection` and `Cell Selection` are both authored in. Pulse-only:
/// no other title has been checked for this path - see
/// `docs/formats/race-setup.md`. A source that does not carry it is exactly
/// [`load`]'s `Err` case.
pub const SCREEN_ENTRY: &str = r"Data\Plugins\PI001\GUI\CellMode_Definition.xml";

/// The two hex textures both screens draw from, neither of which is part of
/// `Skin.xml`'s own front-end sheet - see [`load`]'s own extension step.
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
    /// `base` extended with [`HEX_TEXTURES`] - the same "front end's own
    /// sheet plus this screen's own art" shape
    /// `oag_ui::picker::slideshow`'s stills already extend it with.
    pub sprites: Sheet,
}

/// Reads [`SCREEN_ENTRY`] and every grid `Data\Plugins\grids\Definition.xml`
/// lists off `archives`.
///
/// A grid file that will not parse is skipped and logged - the same
/// per-row tolerance `oag_tables::race_campaign_ground_truth` itself expects
/// of the disc's own sixteen; one bad file should not blank a screen that
/// has fifteen good ones to show.
///
/// # Errors
///
/// Propagates a missing or unreadable `SCREEN_ENTRY`, a screen definition
/// missing `Grid Selection`/`Cell Selection`, or a `Definition.xml` that
/// yields no grid at all.
pub fn load(
    archives: &mut oag_assets::Archives,
    strings: &StringTable,
    faces: FaceScales,
    grid: [f32; 2],
    base: &Sheet,
) -> Result<Campaign> {
    let blob = archives
        .read_name(SCREEN_ENTRY)
        .with_context(|| format!("reading {SCREEN_ENTRY}"))?;
    let xml = oag_tables::fexml::text(&blob).context("expanding CellMode_Definition.xml")?;
    let screens = oag_ui::screen::Screens::from_xml(&xml);
    let grid_layout = Layout::read(&screens, "Grid Selection", strings, faces, grid)
        .context("Grid Selection is not on this screen")?;
    let cell_layout = Layout::read(&screens, "Cell Selection", strings, faces, grid)
        .context("Cell Selection is not on this screen")?;

    let definition_blob = archives
        .read_name(oag_pulse::campaign::DEFINITION_ENTRY)
        .with_context(|| format!("reading {}", oag_pulse::campaign::DEFINITION_ENTRY))?;
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
        anyhow::bail!(
            "no grid in {} parsed",
            oag_pulse::campaign::DEFINITION_ENTRY
        );
    }

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
