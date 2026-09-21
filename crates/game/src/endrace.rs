//! Reads the EndRace screens off an already-open source: their
//! disc-authored layout and the sprite sheet to draw them from.
//!
//! One function, called from two places that cannot share a module tree -
//! `crate::main::session::endrace` (the live session) and
//! `crate::capture::menu_page` (`--menu-page endrace-results`/
//! `endrace-rewards`/`endrace-menu`) - the same reason [`crate::campaign`]
//! exists for the Race Campaign's own two screens, and this mirrors it
//! outright down to the title dispatch: [`load`] picks Pulse's own reader or
//! [`load_hd`] by `title.name`, the identical shape
//! `crate::campaign::load`/`load_hd` already use.

use anyhow::{Context, Result};
use oag_ui::endrace::Layout;
use oag_ui::language::StringTable;
use oag_ui::picker::FaceScales;

use crate::sprite::Sheet;

/// `Data\Plugins\PI001\GUI\EndRace_Definition.xml` - the file all three of
/// Pulse's own screens are authored in. See [`oag_hd::endrace::SCREEN_ENTRY`]
/// for Wipeout HD/Fury's own copy, at a different path, a different
/// resolution and an almost entirely different widget vocabulary -
/// [`load`] picks between the two by `title`, the same way
/// `crate::campaign::load` picks between [`crate::campaign::SCREEN_ENTRY`]
/// and `oag_hd::campaign::SCREEN_ENTRY`.
///
/// A title that reaches [`load`] and fails says so once per race rather than
/// once per frame - `RaceStage::endrace_unavailable`.
pub const SCREEN_ENTRY: &str = r"Data\Plugins\PI001\GUI\EndRace_Definition.xml";

/// Textures neither screen shares with `Skin.xml`'s own front-end sheet -
/// `EndRace Rewards`' medal/loyalty icons and its hex-grid backdrop. See
/// [`load`]'s own extension step.
const EXTRA_TEXTURES: [&str; 2] = [
    r"Data\FE\Images\reward_icons.mip",
    r"Data\FE\Images\hex_bg.mip",
];

/// Everything a caller needs to draw and drive the EndRace screens this
/// build reads for the open title - Pulse's three, or Wipeout HD/Fury's two
/// (`rewards: None` - not read this pass, see [`load_hd`]'s own doc).
#[derive(Debug)]
pub struct EndRaceScreens {
    pub results: Layout,
    /// `None` for a title whose `EndRace Rewards` this pass does not read -
    /// Wipeout HD/Fury today. **Not the same as "this title has no such
    /// screen"** - `DATA02`'s own copy carries one; it is simply out of this
    /// pass's scope. A caller must not treat `None` here as "skip straight
    /// to Menu" for a reason a decompile would give; it already does, on the
    /// title-agnostic ground that this build never opens a screen it has no
    /// [`Layout`] for.
    pub rewards: Option<Layout>,
    pub menu: Layout,
    /// `base` extended with this title's own extra textures - the same
    /// "front end's own sheet plus this screen's own art" shape
    /// [`crate::campaign::load`] already extends it with.
    pub sprites: Sheet,
}

/// Reads this open title's own EndRace screens, title-dispatched: Pulse's
/// [`SCREEN_ENTRY`], or Wipeout HD/Fury's own [`load_hd`] - picked by
/// `title.name` against [`oag_hd::TITLE`]'s own `name`, the identical check
/// `crate::campaign::load` already makes for the Race Campaign's own two
/// screens, for the identical reason that function's own doc gives (a title
/// package declares its `Title` as a `const`, so `std::ptr::eq` against a
/// promoted temporary is not reliable, but the `name` string is).
///
/// # Errors
///
/// Propagates a missing or unreadable screen entry, or a screen definition
/// missing a screen this title's own reader requires.
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
    title: &'static oag_title::Title,
) -> Result<EndRaceScreens> {
    if title.name == oag_hd::TITLE.name {
        return load_hd(archives, strings, faces, grid, base, fallback_globals);
    }

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
        rewards: Some(rewards),
        menu,
        sprites,
    })
}

/// [`load`]'s Wipeout HD/Fury branch - `oag_hd::endrace::SCREEN_ENTRY`, its
/// own 1920x1080 authored grid, and no `oag_tables::fexml` expansion of the
/// screen XML - it is plain UTF-8 on this title, the same divergence
/// `crate::campaign::load_hd`'s own doc gives for `CellMode_Definition.xml`.
///
/// **Reads `EndRace Results`/`EndRace Menu` only.** `EndRace Rewards` is on
/// the served copy (`DATA02`) but out of this pass's own scope - see
/// `docs/formats/hd-endrace-screens.md` and [`oag_ui::endrace::hd`]'s module
/// doc for what does and does not draw and why. `EndRace Podium` (`DATA05`/
/// `DATA06` only) is not read at all.
fn load_hd(
    archives: &mut oag_assets::Archives,
    strings: &StringTable,
    faces: FaceScales,
    grid: [f32; 2],
    base: &Sheet,
    fallback_globals: &[(&str, &str)],
) -> Result<EndRaceScreens> {
    let blob = archives
        .read_name(oag_hd::endrace::SCREEN_ENTRY)
        .with_context(|| format!("reading {}", oag_hd::endrace::SCREEN_ENTRY))?;
    let xml = String::from_utf8(blob).context("EndRace_Definition.xml is not UTF-8")?;
    let screens = oag_ui::screen::Screens::from_xml_with_fallback_globals(&xml, fallback_globals);
    let results = Layout::read_authored(
        &screens,
        "EndRace Results",
        strings,
        faces,
        grid,
        oag_hd::endrace::AUTHORED_GRID,
    )
    .context("EndRace Results is not on this screen")?;
    let menu = Layout::read_authored(
        &screens,
        "EndRace Menu",
        strings,
        faces,
        grid,
        oag_hd::endrace::AUTHORED_GRID,
    )
    .context("EndRace Menu is not on this screen")?;

    let mut blobs = Vec::new();
    for src in oag_hd::endrace::EXTRA_TEXTURES {
        match archives.read_name(src) {
            Ok(blob) => blobs.push((src.to_string(), blob)),
            Err(error) => log::warn!("{src}: {error:#} - the widget it is for draws nothing"),
        }
    }
    let mut report = Vec::new();
    let sprites = base.extended(&blobs, &mut report);
    for line in report {
        log::info!("endrace sprites {line}");
    }

    Ok(EndRaceScreens {
        results,
        // Not read this pass - see this function's own doc.
        rewards: None,
        menu,
        sprites,
    })
}
