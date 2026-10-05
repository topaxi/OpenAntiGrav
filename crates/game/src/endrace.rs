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
/// build reads for the open title - Pulse's three, or Wipeout HD/Fury's
/// three on a copy that authors `EndRace Rewards` (two on `DATA06`'s, which
/// does not - see [`load_hd`]'s own doc).
#[derive(Debug)]
pub struct EndRaceScreens {
    pub results: Layout,
    /// `None` when the served copy of the screen file authors no `EndRace
    /// Rewards` - Wipeout HD/Fury's `DATA06` copy, which this build does not
    /// serve today. **`Some` does not mean the live flow opens it**: on HD
    /// the original never enters this screen at all
    /// (`docs/formats/hd-endrace-screens.md`), so `Session::build_endrace`
    /// builds no rewards model on HD and the layout is drawn only by
    /// `--menu-page endrace-rewards`.
    pub rewards: Option<Layout>,
    pub menu: Layout,
    /// `Race End Photo`, the state a Pulse race sits in between the flag and
    /// `EndRace Results` - read off `InGame_Definition.xml`, not
    /// [`SCREEN_ENTRY`], so it is its own `Option`: a pressing whose copy
    /// will not read still gets its three panels, straight away, and says so
    /// once in the log. `None` on Wipeout HD/Fury, whose equivalent was not
    /// read.
    pub photo: Option<Layout>,
    /// `base` extended with this title's own extra textures - the same
    /// "front end's own sheet plus this screen's own art" shape
    /// [`crate::campaign::load`] already extends it with.
    pub sprites: Sheet,
    /// `EndRace Rewards`' `TrophyPanel` models, decoded - see [`Trophy`].
    /// Empty on Wipeout HD/Fury, whose `MedalBlock` is an `<ImageModel>`
    /// this build does not read, and on a Pulse source whose trophy `.vex`
    /// will not decode (logged, never substituted).
    pub trophies: Vec<Trophy>,
}

/// One of `EndRace Rewards`' three `TrophyPanel` models: which medal it is,
/// where the disc places it, and its decoded mesh.
///
/// **Which medal a model is comes off its widget name**, the switch
/// `EndRaceRewards_OnEnter` (`0x088dbbd4`) makes on the medal ordinal:
/// `0` shows `g_trophy`, `1` `s_trophy`, `2` `b_trophy`
/// (`docs/ghidra/functions/psp-pulse-usa/endrace-screens.md`, confidence 80).
pub struct Trophy {
    pub medal: oag_tables::race_campaign::Medal,
    pub placement: oag_ui::screen::Mode3dModel,
    pub mesh: oag_mesh::mesh::Model,
}

impl std::fmt::Debug for Trophy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Trophy")
            .field("medal", &self.medal)
            .field("src", &self.placement.model.src)
            .finish_non_exhaustive()
    }
}

/// The medal a `TrophyPanel` model's widget name stands for - see
/// [`Trophy`].
fn trophy_medal(name: &str) -> Option<oag_tables::race_campaign::Medal> {
    use oag_tables::race_campaign::Medal;
    match name {
        "g_trophy" => Some(Medal::Gold),
        "s_trophy" => Some(Medal::Silver),
        "b_trophy" => Some(Medal::Bronze),
        _ => None,
    }
}

/// Decodes every `TrophyPanel` model `rewards` places, through the same
/// [`crate::preview::model`] a picker's ship preview loads with. A model
/// that will not read or decode is logged and left out: the medal it stands
/// for then draws no trophy, rather than another one in its place.
fn load_trophies(archives: &mut oag_assets::Archives, rewards: &Layout) -> Vec<Trophy> {
    let mut out = Vec::new();
    for placement in &rewards.screen.models {
        let Some(medal) = placement.name.as_deref().and_then(trophy_medal) else {
            continue;
        };
        match crate::preview::model(archives, &placement.model.src) {
            Ok(mesh) => out.push(Trophy {
                medal,
                placement: placement.clone(),
                mesh,
            }),
            Err(error) => log::warn!(
                "{}: {error:#} - the {medal:?} trophy draws nothing",
                placement.model.src
            ),
        }
    }
    out
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
    crate::loader_log::lines(report.iter().map(|line| format!("endrace sprites {line}")));

    let trophies = load_trophies(archives, &rewards);
    let photo = load_photo(archives, strings, faces, grid, fallback_globals);
    Ok(EndRaceScreens {
        results,
        rewards: Some(rewards),
        menu,
        photo,
        sprites,
        trophies,
    })
}

/// Pulse's `Race End Photo`, off `InGame_Definition.xml`. A file that will not
/// read, or a screen that is not in it, is logged and answered with `None`:
/// the legend is then not drawn and the race goes straight to its panels, as
/// it did before this screen was read.
fn load_photo(
    archives: &mut oag_assets::Archives,
    strings: &StringTable,
    faces: FaceScales,
    grid: [f32; 2],
    fallback_globals: &[(&str, &str)],
) -> Option<Layout> {
    let entry = oag_pulse::names::INGAME_DEFINITION;
    let mut read = || -> Result<Layout> {
        let blob = archives
            .read_name(entry)
            .with_context(|| format!("reading {entry}"))?;
        let xml = oag_tables::fexml::text(&blob).context("expanding InGame_Definition.xml")?;
        let screens =
            oag_ui::screen::Screens::from_xml_with_fallback_globals(&xml, fallback_globals);
        Layout::read(
            &screens,
            oag_ui::endrace::photo::SCREEN,
            strings,
            faces,
            grid,
        )
        .context("Race End Photo is not on this screen")
    };
    match read() {
        Ok(layout) => Some(layout),
        Err(error) => {
            log::warn!("{error:#} - the race goes straight to its results, with no legend");
            None
        }
    }
}

/// [`load`]'s Wipeout HD/Fury branch - `oag_hd::endrace::SCREEN_ENTRY`, its
/// own 1920x1080 authored grid, and no `oag_tables::fexml` expansion of the
/// screen XML - it is plain UTF-8 on this title, the same divergence
/// `crate::campaign::load_hd`'s own doc gives for `CellMode_Definition.xml`.
///
/// `EndRace Results`/`EndRace Menu` are required; `EndRace Rewards` is read
/// when the served copy authors it (`DATA02`-`05` do, `DATA06` does not) and
/// left `None` otherwise, never failing the other two. See
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
    let rewards = Layout::read_authored(
        &screens,
        "EndRace Rewards",
        strings,
        faces,
        grid,
        oag_hd::endrace::AUTHORED_GRID,
    );
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
    crate::loader_log::lines(report.iter().map(|line| format!("endrace sprites {line}")));

    Ok(EndRaceScreens {
        results,
        rewards,
        menu,
        photo: None,
        sprites,
        trophies: Vec::new(),
    })
}
