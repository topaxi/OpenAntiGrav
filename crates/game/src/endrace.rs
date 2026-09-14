//! Reads the three EndRace screens off an already-open source: their
//! disc-authored layout and the sprite sheet to draw them from.
//!
//! One function, called from two places that cannot share a module tree -
//! `crate::main::session::endrace` (the live session) and
//! `crate::capture::menu_page` (`--menu-page endrace-results`/
//! `endrace-rewards`/`endrace-menu`) - the same reason [`crate::campaign`]
//! exists for the Race Campaign's own two screens, and this mirrors it
//! outright: same shape, same error handling, same sprite-extension idiom.

use anyhow::{Context, Result};
use oag_ui::endrace::Layout;
use oag_ui::language::StringTable;
use oag_ui::picker::FaceScales;

use crate::sprite::Sheet;

/// `Data\Plugins\PI001\GUI\EndRace_Definition.xml` - the file all three
/// screens are authored in. Pulse-only, like [`crate::campaign::SCREEN_ENTRY`]:
/// no other title has been checked for this path.
pub const SCREEN_ENTRY: &str = r"Data\Plugins\PI001\GUI\EndRace_Definition.xml";

/// Textures neither screen shares with `Skin.xml`'s own front-end sheet -
/// `EndRace Rewards`' medal/loyalty icons and its hex-grid backdrop. See
/// [`load`]'s own extension step.
const EXTRA_TEXTURES: [&str; 2] = [
    r"Data\FE\Images\reward_icons.mip",
    r"Data\FE\Images\hex_bg.mip",
];

/// Everything a caller needs to draw and drive the three EndRace screens.
#[derive(Debug)]
pub struct EndRaceScreens {
    pub results: Layout,
    pub rewards: Layout,
    pub menu: Layout,
    /// `base` extended with [`EXTRA_TEXTURES`] - the same "front end's own
    /// sheet plus this screen's own art" shape [`crate::campaign::load`]
    /// already extends it with.
    pub sprites: Sheet,
}

/// Reads [`SCREEN_ENTRY`] off `archives`.
///
/// # Errors
///
/// Propagates a missing or unreadable `SCREEN_ENTRY`, or a screen definition
/// missing one of the three named screens.
pub fn load(
    archives: &mut oag_assets::Archives,
    strings: &StringTable,
    faces: FaceScales,
    grid: [f32; 2],
    base: &Sheet,
    // See `crate::campaign::load`'s own doc on this parameter - the
    // identical fallback-globals need, for the identical reason
    // (`Outline_x_y`'s `FEGlobals->CM_HEX_Outline`-shaped indirection has a
    // counterpart on this file too).
    fallback_globals: &[(&str, &str)],
) -> Result<EndRaceScreens> {
    let blob = archives
        .read_name(SCREEN_ENTRY)
        .with_context(|| format!("reading {SCREEN_ENTRY}"))?;
    let xml = oag_tables::fexml::text(&blob).context("expanding EndRace_Definition.xml")?;
    let screens = oag_ui::screen::Screens::from_xml_with_fallback_globals(&xml, fallback_globals);
    let results = Layout::read(&screens, "EndRace Results", strings, faces, grid)
        .context("EndRace Results is not on this screen")?;
    let rewards = Layout::read(&screens, "EndRace Rewards", strings, faces, grid)
        .context("EndRace Rewards is not on this screen")?;
    let menu = Layout::read(&screens, "EndRace Menu", strings, faces, grid)
        .context("EndRace Menu is not on this screen")?;

    let mut blobs = Vec::new();
    for src in EXTRA_TEXTURES {
        match archives.read_name(src) {
            Ok(blob) => blobs.push((src.to_string(), blob)),
            Err(error) => log::warn!("{src}: {error:#} - the EndRace screens draw without it"),
        }
    }
    let mut report = Vec::new();
    let sprites = base.extended(&blobs, &mut report);
    for line in report {
        log::info!("endrace sprites {line}");
    }

    Ok(EndRaceScreens {
        results,
        rewards,
        menu,
        sprites,
    })
}
