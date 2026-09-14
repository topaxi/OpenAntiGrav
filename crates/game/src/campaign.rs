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
