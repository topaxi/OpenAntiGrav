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
    /// `Cell Selection`'s own `Cell Help` overlay - `triangle` toggles it
    /// (`CellSelection::help_open`) - resolved the identical way
    /// `cell_layout` is, off the same `CellMode_Definition.xml`. `None` on
    /// a title whose screen definition authors no such screen (Wipeout
    /// HD/Fury's own copy does not).
    pub cell_help: Option<Layout>,
    /// The shared front-end root's own `NavigationController` - `Confirm`/
    /// `Back`, see [`oag_ui::campaign::footer::NavigationLegend`]'s own doc.
    /// **Pulse only** - reading Wipeout HD/Fury's equivalent is a different
    /// lane's own thread (`docs/ui/campaign-screens.md`'s `## Open` HD
    /// section), so [`load_hd`]/[`load_omega`] leave this `None`.
    pub nav_legend: Option<oag_ui::campaign::footer::NavigationLegend>,
    /// The shared front-end root's own scrolling tip ticker - see
    /// [`oag_ui::campaign::footer::TickerLayout`]'s own doc. **Pulse only**,
    /// for the same reason [`Self::nav_legend`] is.
    pub ticker: Option<oag_ui::campaign::footer::TickerLayout>,
    /// `base` extended with the title's own hex/other textures - the same
    /// "front end's own sheet plus this screen's own art" shape
    /// `oag_ui::picker::slideshow`'s stills already extend it with.
    pub sprites: Sheet,
    /// **HD/Fury only.** `Campaign Selection`'s own layout, ahead of
    /// [`Self::grid_layout`] - `None` on every other title, and on an HD
    /// source whose `DATA06.PSARC` this pass could not read (a base,
    /// non-Fury pressing, say). See [`load_hd`]'s own doc for where this is
    /// read from and why it differs from [`Self::grid_layout`]'s own
    /// archive.
    pub selection_layout: Option<Layout>,
    /// **HD/Fury only.** `Grid Selection Fury`'s own layout - the screen
    /// `Campaign Selection`'s `FE_RC_FURY` entry opens, which pages
    /// `grid8`..`grid15` (`oag_hd::campaign::FURY_GRID_RANGE`). `None` on
    /// the same terms as [`Self::selection_layout`] - either both are
    /// `Some` or both are `None`, since they are read off the same archive
    /// in the same call.
    pub grid_layout_fury: Option<Layout>,
}

/// [`Campaign::nav_legend`]/[`Campaign::ticker`]: both live on the front-end
/// root (`Data\Plugins\PI001\GUI\Skin.xml`, [`oag_pulse::names::FRONTEND_ROOT`]),
/// not on `CellMode_Definition.xml` - a second, small archive read and parse
/// alongside the screen's own, since neither
/// [`oag_ui::screen::Screens::collect_widgets`] nor this crate's existing
/// `Screens` value for that file (`Shell::screens`, already merged with
/// every `LoadXML` include) keeps the raw node tree
/// [`oag_ui::campaign::footer`] needs. Errors are logged and treated as "not
/// authored" rather than failing the whole campaign screen over a footer.
fn read_footer(
    archives: &mut oag_assets::Archives,
    strings: &StringTable,
    fallback_globals: &[(&str, &str)],
) -> (
    Option<oag_ui::campaign::footer::NavigationLegend>,
    Option<oag_ui::campaign::footer::TickerLayout>,
) {
    let blob = match archives.read_name(oag_pulse::names::FRONTEND_ROOT) {
        Ok(blob) => blob,
        Err(error) => {
            log::warn!(
                "{}: {error:#} - the footer's Confirm/Back legend and tip ticker draw nothing",
                oag_pulse::names::FRONTEND_ROOT
            );
            return (None, None);
        }
    };
    let xml = match oag_tables::fexml::text(&blob) {
        Ok(xml) => xml,
        Err(error) => {
            log::warn!(
                "expanding {}: {error:#} - the footer's Confirm/Back legend and tip ticker draw nothing",
                oag_pulse::names::FRONTEND_ROOT
            );
            return (None, None);
        }
    };
    let globals =
        oag_ui::screen::Screens::from_xml_with_fallback_globals(&xml, fallback_globals).globals;
    let root = oag_ui::screen::parse(&xml);
    let nav_legend = oag_ui::campaign::footer::NavigationLegend::read(&root, &globals, strings);
    let ticker = oag_ui::campaign::footer::TickerLayout::read(&root, &globals);
    (nav_legend, ticker)
}

/// [`Campaign::nav_legend`]/[`Campaign::ticker`]'s own draw list for a
/// caller with no [`crate::records::Store`] to rotate the ticker's tips off
/// of - `crate::capture::menu_page`'s `--menu-page grid-select`/
/// `cell-select`, the same "no live session to fold through" gap this
/// module's own `load_hd` leaves `circuit_names` with. The ticker draws its
/// own layout but an empty rotation (nothing to show, honestly, rather than
/// a guessed one) at a frozen `elapsed`; the `Confirm`/`Back` legend needs
/// neither and draws in full.
#[must_use]
pub fn static_footer_overlay(
    campaign: &Campaign,
    faces: &FaceScales,
    measure: &dyn Fn(&str) -> f32,
) -> Vec<oag_ui::frontend::Draw> {
    let mut overlay = campaign
        .nav_legend
        .as_ref()
        .map_or_else(Vec::new, |legend| legend.draw(faces, measure));
    if let Some(ticker) = &campaign.ticker {
        overlay.extend(oag_ui::campaign::footer::ticker_draw(
            ticker,
            0.0,
            &[],
            faces,
            &|_| 0.0,
        ));
    }
    overlay
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
/// measured**. See `docs/ui/campaign-screens.md`'s HD section for the
/// discriminating measurement this pass took (`DATA02`'s own flat
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
    if title.name == oag_omega::TITLE.name {
        return load_omega(archives, strings, faces, grid, base, fallback_globals);
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
    // `None` rather than an error: `Cell Help`'s own overlay is a bonus
    // screen, not one either caller has ever required to exist the way the
    // two selection screens above are.
    let cell_help = Layout::read(&screens, "Cell Help", strings, faces, grid);

    let grids = read_grids(archives, oag_pulse::campaign::DEFINITION_ENTRY)?;
    let (nav_legend, ticker) = read_footer(archives, strings, fallback_globals);

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
        cell_help,
        nav_legend,
        ticker,
        sprites,
        selection_layout: None,
        grid_layout_fury: None,
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
    let (selection_layout, grid_layout_fury) =
        load_hd_campaign_selection(archives, strings, faces, grid, fallback_globals);

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
        // HD/Omega's own `Cell Help`/`NavigationController`/ticker are a
        // different lane's own thread - see `Campaign::nav_legend`'s own
        // doc.
        cell_help: None,
        nav_legend: None,
        ticker: None,
        sprites,
        selection_layout,
        grid_layout_fury,
    })
}

/// `Campaign Selection`/`Grid Selection Fury`, read off `DATA06.PSARC`
/// directly - by archive label, not [`oag_assets::Archives::read_name`]'s
/// own precedence, which reaches `DATA02`'s copy of the same path and has
/// neither screen at all. See [`oag_hd::campaign::SCREEN_ENTRY`]'s own doc
/// for the full measurement (RPCS3's own `TTY.log` names `Grid Selection
/// Fury` as a live screen, which only `DATA06`'s copy authors) and
/// `docs/ui/campaign-screens.md`'s "Wipeout HD/Fury: `Campaign Selection`"
/// section.
///
/// `(None, None)`, logged, when this source has no `DATA06` copy of
/// [`oag_hd::campaign::SCREEN_ENTRY`] at all, or that copy's own XML does
/// not carry both screens - a base, non-Fury HD pressing this project has
/// not seen, say. [`load_hd`] still returns its own `Campaign` in that
/// case: the base `Grid Selection`/`Cell Selection` this function does not
/// touch are unaffected, and `crate::main::session::campaign::open_campaign`
/// falls back to opening straight on `Grid Selection`, the pre-this-pass
/// behaviour, rather than refusing the whole campaign over one missing
/// screen.
/// `Campaign Selection`'s own four idstrings - [`oag_ui::campaign::selection::TITLE_ID`]/
/// `SUBTITLE_ID` and both campaigns' own [`oag_ui::campaign::selection::Campaign::entry_id`] -
/// resolved off `entries_path`'s own `DATA06` copy rather than whichever
/// archive `oag_assets::Archives::read_name`'s precedence would otherwise
/// serve. **The reason this exists at all**: confirmed directly against
/// `hdfury-ps3-eu-dec.iso` that `DATA00`/`DATA01`/`DATA02`/`DATA03`/`DATA05`
/// carry none of the four ids in their own copy of `entries_path` - only
/// `DATA06`'s does, the same archive [`oag_hd::campaign::SELECTION_SCREEN_ARCHIVE`]
/// already names for the screen's own layout XML. A caller merges the
/// result into its own [`StringTable`] (`StringTable::merge`); an empty map
/// when `entries_path`'s `DATA06` copy is missing or not UTF-8, so a
/// caller's own `strings.get_or_id` falls back to the bare id rather than
/// panicking.
///
/// **Shared by both readers of this screen** - `crate::main::session::campaign::open_campaign`
/// (the live session) and `crate::capture::campaign_page` (`--menu-page
/// campaign-select`) - so the two cannot resolve these four ids differently.
/// Before this function existed, only the live path applied any overlay at
/// all (just [`oag_ui::campaign::selection::TITLE_ID`]/`SUBTITLE_ID`, not
/// the two entry names), so a `--menu-page campaign-select` capture showed
/// raw ids where a player's own session showed real text - the gap this
/// closes.
#[must_use]
pub fn hd_selection_string_overlay(
    archives: &mut oag_assets::Archives,
    entries_path: &str,
) -> std::collections::HashMap<String, String> {
    archives
        .read_every_name(entries_path)
        .into_iter()
        .find(|(label, _)| label.ends_with(oag_hd::campaign::SELECTION_SCREEN_ARCHIVE))
        .and_then(|(_, blob)| oag_tables::fexml::text(&blob).ok())
        .map(|xml| {
            let table = StringTable::from_xml(&xml);
            [
                oag_ui::campaign::selection::TITLE_ID,
                oag_ui::campaign::selection::SUBTITLE_ID,
                oag_ui::campaign::selection::Campaign::Fury.entry_id(),
                oag_ui::campaign::selection::Campaign::Hd.entry_id(),
            ]
            .into_iter()
            .filter_map(|id| {
                table
                    .get(id)
                    .map(|value| (id.to_string(), value.to_string()))
            })
            .collect()
        })
        .unwrap_or_default()
}

fn load_hd_campaign_selection(
    archives: &mut oag_assets::Archives,
    strings: &StringTable,
    faces: FaceScales,
    grid: [f32; 2],
    fallback_globals: &[(&str, &str)],
) -> (Option<Layout>, Option<Layout>) {
    let copies = archives.read_every_name(oag_hd::campaign::SCREEN_ENTRY);
    let Some((_, blob)) = copies
        .into_iter()
        .find(|(label, _)| label.ends_with(oag_hd::campaign::SELECTION_SCREEN_ARCHIVE))
    else {
        log::warn!(
            "{}: no {} copy of {} - Campaign Selection stays unmodelled, RACE CAMPAIGN opens \
             straight on the base Grid Selection",
            oag_hd::campaign::SELECTION_SCREEN_ARCHIVE,
            oag_hd::campaign::SELECTION_SCREEN_ARCHIVE,
            oag_hd::campaign::SCREEN_ENTRY,
        );
        return (None, None);
    };
    let xml = match String::from_utf8(blob) {
        Ok(xml) => xml,
        Err(error) => {
            log::warn!(
                "{}'s own copy of {} is not UTF-8 ({error}) - Campaign Selection stays unmodelled",
                oag_hd::campaign::SELECTION_SCREEN_ARCHIVE,
                oag_hd::campaign::SCREEN_ENTRY,
            );
            return (None, None);
        }
    };
    let screens = oag_ui::screen::Screens::from_xml_with_fallback_globals(&xml, fallback_globals);
    let selection = Layout::read_authored(
        &screens,
        oag_hd::campaign::SELECTION_SCREEN,
        strings,
        faces,
        grid,
        oag_hd::campaign::AUTHORED_GRID,
    );
    let fury = Layout::read_authored(
        &screens,
        oag_hd::campaign::FURY_GRID_SCREEN,
        strings,
        faces,
        grid,
        oag_hd::campaign::AUTHORED_GRID,
    );
    match (selection, fury) {
        (Some(selection), Some(fury)) => (Some(selection), Some(fury)),
        _ => {
            log::warn!(
                "{}'s own copy of {} is missing {} or {} - Campaign Selection stays unmodelled",
                oag_hd::campaign::SELECTION_SCREEN_ARCHIVE,
                oag_hd::campaign::SCREEN_ENTRY,
                oag_hd::campaign::SELECTION_SCREEN,
                oag_hd::campaign::FURY_GRID_SCREEN,
            );
            (None, None)
        }
    }
}

/// [`load`]'s Omega branch - the same shape as [`load_hd`], off
/// [`oag_omega::campaign`]'s own entry names and nineteen grids rather than
/// HD's sixteen (`oag_omega::campaign::GRID_COUNT`, confirmed by direct
/// listing against `data09.psarc` rather than assumed equal to HD's).
///
/// **Reuses [`oag_hd::campaign::HEX_TEXTURES`]/`OTHER_TEXTURES`** rather than
/// declaring Omega's own copies: Omega's front end is HD's `PI001` plugin
/// carried forward (`docs/formats/omega-frontend.md`), so these are the same
/// widget/archive-path pairs, not a new measurement. **Every one of them
/// will report "not found" and draw nothing** - the shipped files are `.gnf`
/// now, not `.gtf`, and this function reads by exact name the way
/// `oag_hd::campaign`'s own texture step always has; see
/// `crate::boot::sprites::gnf_sibling_report` for the front end's own sheet,
/// which does name the `.gnf` sibling. Extending that same naming to this
/// screen's own textures is future work, not attempted here - the hex grid
/// still lays out and draws its cells with no texture rather than failing to
/// load at all.
fn load_omega(
    archives: &mut oag_assets::Archives,
    strings: &StringTable,
    faces: FaceScales,
    grid: [f32; 2],
    base: &Sheet,
    fallback_globals: &[(&str, &str)],
) -> Result<Campaign> {
    let blob = archives
        .read_name(oag_omega::campaign::SCREEN_ENTRY)
        .with_context(|| format!("reading {}", oag_omega::campaign::SCREEN_ENTRY))?;
    let xml = String::from_utf8(blob).context("CellMode_Definition.xml is not UTF-8")?;
    let screens = oag_ui::screen::Screens::from_xml_with_fallback_globals(&xml, fallback_globals);
    let grid_layout = Layout::read_authored(
        &screens,
        "Grid Selection",
        strings,
        faces,
        grid,
        oag_omega::campaign::AUTHORED_GRID,
    )
    .context("Grid Selection is not on this screen")?;
    let cell_layout = Layout::read_authored(
        &screens,
        "Cell Selection",
        strings,
        faces,
        grid,
        oag_omega::campaign::AUTHORED_GRID,
    )
    .context("Cell Selection is not on this screen")?;

    let grids = read_grids(archives, oag_omega::campaign::DEFINITION_ENTRY)?;

    let mut blobs = Vec::new();
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
        // HD/Omega's own `Cell Help`/`NavigationController`/ticker are a
        // different lane's own thread - see `Campaign::nav_legend`'s own
        // doc.
        cell_help: None,
        nav_legend: None,
        ticker: None,
        sprites,
        // Omega's own front end is HD's `PI001` plugin carried forward, but
        // no `Campaign Selection`/`Grid Selection Fury` has been measured on
        // it - Omega's racing is out of scope entirely
        // (`docs/formats/omega-status.md`), so this is left unmodelled
        // rather than assumed to carry HD's own screen unread.
        selection_layout: None,
        grid_layout_fury: None,
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
