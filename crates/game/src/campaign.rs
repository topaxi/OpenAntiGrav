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
use oag_ui::language::StringTable;
use oag_ui_screens::campaign::Layout;
use oag_ui_screens::picker::FaceScales;

use oag_hud::sprite::Sheet;

mod emblems;
pub mod hit;
pub mod launch;

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
    /// `Back`, see [`oag_ui_screens::campaign::footer::NavigationLegend`]'s own doc.
    /// Read on every title now: Pulse's own `Data\Plugins\PI001\GUI\Skin.xml`
    /// and Wipeout HD/Fury's and Omega's shared
    /// `Data\Plugins\Frontend\Gui\Skin.xml` alike, off [`read_footer`]. `None`
    /// when that read fails, or the root authors no `NavigationController`
    /// with either half this build draws.
    pub nav_legend: Option<oag_ui_screens::campaign::footer::NavigationLegend>,
    /// The shared front-end root's own scrolling tip ticker - see
    /// [`oag_ui_screens::campaign::footer::TickerLayout`]'s own doc. Read the same
    /// way [`Self::nav_legend`] is, on every title - `None` in practice on
    /// Wipeout HD/Fury and Omega regardless, since neither's own `Skin.xml`
    /// authors the `TextInfoIsAlwaysLast` viewport that read looks for
    /// (confirmed by direct read, not assumed).
    pub ticker: Option<oag_ui_screens::campaign::footer::TickerLayout>,
    /// `base` extended with the title's own hex/other textures - the same
    /// "front end's own sheet plus this screen's own art" shape
    /// `oag_ui_screens::picker::slideshow`'s stills already extend it with.
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
    /// **HD/Fury only.** The flyer cards `Grid Selection` draws behind its
    /// widgets, decoded - see [`crate::flyer`]. `None` on every other title,
    /// and on an HD source whose screen file authors no `<Flyer
    /// name="FlyerModel">`; a card that will not decode is left out of it
    /// and logged, which draws nothing for that grid.
    pub flyers: Option<crate::flyer::Flyers>,
    /// **HD/Fury only.** A circuit's white emblem for `Cell Selection`, keyed
    /// by the lowercased circuit id (`17_track`) and holding the `src` the
    /// sheet carries it under - see `oag_ui_screens::campaign::hd::cell_emblems`.
    /// Empty on every other title.
    pub circuit_emblems: std::collections::HashMap<String, String>,
}

/// [`Campaign::nav_legend`]/[`Campaign::ticker`]: both live on the front-end
/// root (`front_end_root` - `Data\Plugins\PI001\GUI\Skin.xml`,
/// [`oag_pulse::names::FRONTEND_ROOT`], on Pulse; `Data\Plugins\Frontend\Gui\Skin.xml`,
/// [`oag_hd::frontend::names::FRONTEND_ROOT`]/[`oag_omega::frontend::names::FRONTEND_ROOT`],
/// on Wipeout HD/Fury and Omega), not on `CellMode_Definition.xml` - a
/// second, small archive read and parse alongside the screen's own, since
/// neither [`oag_ui::screen::Screens::collect_widgets`] nor this crate's
/// existing `Screens` value for that file (`Shell::screens`, already merged
/// with every `LoadXML` include) keeps the raw node tree
/// [`oag_ui_screens::campaign::footer`] needs. `oag_tables::fexml::text` reads
/// either encoding this file comes in - Pulse's own dictionary-shortened
/// copy or Wipeout HD/Fury and Omega's plain UTF-8 one - so this needs no
/// dispatch of its own, unlike [`load_hd`]'s screen-XML read a few
/// functions down. Errors are logged and treated as "not authored" rather
/// than failing the whole campaign screen over a footer.
fn read_footer(
    archives: &mut oag_assets::Archives,
    front_end_root: &str,
    strings: &StringTable,
    fallback_globals: &[(&str, &str)],
) -> (
    Option<oag_ui_screens::campaign::footer::NavigationLegend>,
    Option<oag_ui_screens::campaign::footer::TickerLayout>,
) {
    let blob = match archives.read_name(front_end_root) {
        Ok(blob) => blob,
        Err(error) => {
            log::warn!(
                "{front_end_root}: {error:#} - the footer's Confirm/Back legend and tip ticker draw nothing"
            );
            return (None, None);
        }
    };
    let xml = match oag_tables::fexml::text(&blob) {
        Ok(xml) => xml,
        Err(error) => {
            log::warn!(
                "expanding {front_end_root}: {error:#} - the footer's Confirm/Back legend and tip ticker draw nothing"
            );
            return (None, None);
        }
    };
    let globals =
        oag_ui::screen::Screens::from_xml_with_fallback_globals(&xml, fallback_globals).globals;
    let root = oag_ui::screen::parse(&xml);
    let nav_legend =
        oag_ui_screens::campaign::footer::NavigationLegend::read(&root, &globals, strings);
    let ticker = oag_ui_screens::campaign::footer::TickerLayout::read(&root, &globals);
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
        overlay.extend(oag_ui_screens::campaign::footer::ticker_draw(
            ticker,
            0.0,
            &[],
            faces,
            &|_| 0.0,
        ));
    }
    overlay
}

/// Whether `title` draws HD's campaign screens - HD itself, and Omega, whose
/// front end is HD's `PI001` plugin carried forward
/// (`docs/formats/omega-frontend.md`): [`oag_title::CampaignDialect::draws_hd_screens`]
/// of the title's own [`oag_title::Campaign`].
///
/// Every dispatch that picks `oag_ui_screens::campaign::hd`'s draw list over Pulse's
/// asks this. Omega used to fall through to Pulse's, which has no arm for
/// HD's widget names (`Event`, `RC Laps`, `NextPoints`, `EPoints Title`), so
/// their raw ids and the `%d` template drew on screen.
#[must_use]
pub fn draws_hd_campaign(title: &oag_title::Title) -> bool {
    title.campaign.dialect.draws_hd_screens()
}

/// Reads the campaign screen off `archives`, title-dispatched: Pulse's
/// `Data\Plugins\PI001\GUI\CellMode_Definition.xml` and
/// `Data\Plugins\grids\Definition.xml`, or Wipeout HD/Fury's own copies of
/// the same two roles ([`oag_hd::campaign::SCREEN_ENTRY`]/
/// `DEFINITION_ENTRY`) - picked by the title's own
/// [`oag_title::Campaign::dialect`].
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
#[allow(
    clippy::too_many_arguments,
    reason = "each is a separate fact the load reads: the archives, the strings, the face scales, \
              the grid, the base sheet, the globals, the title and the circuits"
)]
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
    // The circuits' own folders, for `Cell Selection`'s emblem per track. HD
    // only; the others read none.
    tracks: &[oag_raceplay::catalogue::Track],
) -> Result<Campaign> {
    match title.campaign.dialect {
        oag_title::CampaignDialect::Hd => {
            return load_hd(
                title,
                archives,
                strings,
                faces,
                grid,
                base,
                fallback_globals,
                tracks,
            );
        }
        oag_title::CampaignDialect::Omega => {
            return load_omega(archives, strings, faces, grid, base, fallback_globals);
        }
        oag_title::CampaignDialect::Pulse => {}
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

    let grids = read_grids(
        archives,
        oag_pulse::campaign::DEFINITION_ENTRY,
        title.campaign.grid_archive,
    )?;
    let (nav_legend, ticker) = read_footer(
        archives,
        oag_pulse::names::FRONTEND_ROOT,
        strings,
        fallback_globals,
    );

    let mut blobs = Vec::new();
    for src in HEX_TEXTURES {
        // `oag_pulse::read_image`, not `archives.read_name` - this source's
        // compiled texture container is not always the declared `.mip`
        // (`Texture_FindOrLoad` rewrites to `.pct` on the PS2 build; see that
        // function's own doc). `hex_bg.mip`/`hex_bg.pct` already needed this
        // for the selection screens' own hex tile
        // (`docs/ui/selection-screens.md`'s "The hex grid is a 32x16 tile");
        // these two names went through the archive directly instead and so
        // never got the same rewrite, which is why the PS2 campaign grid
        // drew its lock icons with no hex cell under them at all where the
        // PSP draws both.
        match oag_pulse::read_image(archives, src) {
            Ok(blob) => blobs.push((src.to_string(), blob)),
            Err(error) => log::warn!("{src}: {error:#} - the hex grid draws without it"),
        }
    }
    let mut report = Vec::new();
    let sprites = base.extended(&blobs, &mut report);
    oag_raceplay::loader_log::lines(report.iter().map(|line| format!("campaign sprites {line}")));

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
        flyers: None,
        circuit_emblems: std::collections::HashMap::new(),
    })
}

/// Reads one HD campaign texture by archive path, `Data\FE\Images\Hexmedal_HD.gtf`
/// alone routed through [`oag_hd::campaign::PER_DIFFICULTY_MEDAL_ARCHIVE`]'s
/// copy rather than [`oag_assets::Archives::read_name`]'s ordinary
/// precedence.
///
/// **Both of [`load_hd`]'s own texture loops need this, not just one.**
/// `Medal_{x}_{y}` and `Target0/1/2 Medal` name the identical archive file
/// two different ways (`oag_hd::campaign::HEX_TEXTURES`'s own `.mip`
/// rewrite versus `oag_hd::campaign::OTHER_TEXTURES`'s literal `.gtf`) -
/// missing either loop here would leave one of the two widgets drawing off
/// `DATA02`'s shorter, flat atlas while the other correctly draws off
/// `DATA04`'s taller, per-difficulty one, exactly the split outcome a live
/// `--menu-page cell-select` capture caught: the `Target0/1/2 Medal` row
/// showed the right per-difficulty icon shape while `Medal_{x}_{y}` drew
/// nothing at all, `v` past the shorter atlas's own edge. Falls back to
/// [`oag_assets::Archives::read_name`]'s own copy for every other path,
/// and for this one too if `DATA04` somehow lacks the entry - the same
/// honest-absence shape every texture read in [`load_hd`] already takes.
fn read_hd_texture(
    archives: &mut oag_assets::Archives,
    path: &str,
) -> Result<Vec<u8>, oag_assets::Error> {
    if path == r"Data\FE\Images\Hexmedal_HD.gtf"
        && let Some((_, blob)) = archives
            .read_every_name(path)
            .into_iter()
            .find(|(label, _)| same_archive(label, oag_hd::campaign::PER_DIFFICULTY_MEDAL_ARCHIVE))
    {
        return Ok(blob);
    }
    archives.read_name(path)
}

/// Whether `label` names the archive `wanted` does, by file name and ignoring
/// case: a disc labels its archives `PS3_GAME/USRDIR/DATA04.PSARC`, the PSN
/// install `.../USRDIR/data04.psarc`.
fn same_archive(label: &str, wanted: &str) -> bool {
    let name = |path: &str| path.rsplit('/').next().unwrap_or(path).to_ascii_uppercase();
    name(label) == name(wanted)
}

/// [`load`]'s Wipeout HD/Fury branch - the same shape, off
/// [`oag_hd::campaign`]'s own entry names, HD's own authored grid
/// ([`oag_hd::campaign::AUTHORED_GRID`], not the PSP's `PSP_GRID`
/// [`oag_ui_screens::campaign::Layout::read`] assumes), and no `oag_tables::fexml`
/// expansion of the screen XML - it is plain UTF-8 on this title, unlike
/// Pulse's dictionary-shortened copy. See `oag_ui_screens::campaign::hd`'s own
/// module doc for what the two screens draw once resolved this way.
#[allow(
    clippy::too_many_arguments,
    reason = "each is a separate fact the load reads: the archives, the strings, the face scales, \
              the grid, the base sheet, the globals, the title and the circuits"
)]
fn load_hd(
    title: &oag_title::Title,
    archives: &mut oag_assets::Archives,
    strings: &StringTable,
    faces: FaceScales,
    grid: [f32; 2],
    base: &Sheet,
    fallback_globals: &[(&str, &str)],
    tracks: &[oag_raceplay::catalogue::Track],
) -> Result<Campaign> {
    // `DATA06`'s copy, not `oag_assets::Archives::read_name`'s own
    // precedence (which lands on `DATA02`'s) - **switched 2026-09-27**, see
    // `oag_ui_screens::campaign::hd`'s own module doc ("The winning archive is
    // `DATA06`") for the full archive-precedence argument and
    // `oag_hd::campaign::SCREEN_ENTRY`'s doc for the file-level summary.
    // One parse now covers all four screens `CellMode_Definition.xml`
    // authors on this archive - `Grid Selection`/`Cell Selection` below,
    // plus `Campaign Selection`/`Grid Selection Fury` via
    // [`hd_selection_screens`] - where a separate `load_hd_campaign_selection`
    // used to re-read and re-parse the same file a second time.
    let blob = match title.campaign.screen_archive {
        Some(label) => archives
            .read_every_name(oag_hd::campaign::SCREEN_ENTRY)
            .into_iter()
            .find(|(copy, _)| copy.ends_with(label))
            .map(|(_, blob)| blob)
            .with_context(|| {
                format!(
                    "no {label} copy of {} - Wipeout HD/Fury's own Cell Selection cannot draw",
                    oag_hd::campaign::SCREEN_ENTRY,
                )
            })?,
        None => archives
            .read_name(oag_hd::campaign::SCREEN_ENTRY)
            .with_context(|| format!("reading {}", oag_hd::campaign::SCREEN_ENTRY))?,
    };
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

    let grids = read_grids(
        archives,
        oag_hd::campaign::DEFINITION_ENTRY,
        title.campaign.grid_archive,
    )?;
    let (selection_layout, grid_layout_fury) =
        hd_selection_screens(title, &screens, strings, faces, grid);
    let flyer_names: Vec<String> = grids
        .iter()
        .filter_map(|grid| grid.flyer_name.clone())
        .collect();
    // Every card: the base campaign's eight, Fury's eight and the two
    // `Campaign Selection` cards, each with the window of its camera's image
    // it shows.
    let grid_cards = grids.iter().enumerate().filter_map(|(index, grid)| {
        let (window, gain) = if oag_hd::campaign::HD_GRID_RANGE.contains(&index) {
            (crate::flyer::HD_WINDOW, crate::flyer::BASE_GAIN)
        } else {
            (crate::flyer::FURY_WINDOW, crate::flyer::FURY_GAIN)
        };
        Some(crate::flyer::CardSpec {
            flyer: grid.flyer_name.clone()?,
            side: crate::flyer::Side::Front,
            window,
            stretch: 1.0,
            gain,
        })
    });
    let back_cards = grids.iter().enumerate().filter_map(|(index, grid)| {
        let gain = if oag_hd::campaign::HD_GRID_RANGE.contains(&index) {
            crate::flyer::BASE_GAIN
        } else {
            crate::flyer::FURY_GAIN
        };
        Some(crate::flyer::CardSpec {
            flyer: grid.flyer_name.clone()?,
            side: crate::flyer::Side::Back,
            window: crate::flyer::BACK_WINDOW,
            stretch: crate::flyer::BACK_STRETCH,
            gain,
        })
    });
    let campaign_cards = [
        oag_ui_screens::campaign::flyer::FURY_CAMPAIGN_FLYER,
        oag_ui_screens::campaign::flyer::HD_CAMPAIGN_FLYER,
    ]
    .map(|name| crate::flyer::CardSpec {
        flyer: name.to_string(),
        side: crate::flyer::Side::Front,
        window: crate::flyer::FURY_WINDOW,
        stretch: crate::flyer::CAMPAIGN_STRETCH,
        gain: crate::flyer::CAMPAIGN_GAIN,
    });
    // The two `Campaign Selection` cards exist only where that screen does.
    let campaign_cards = selection_layout.is_some().then_some(campaign_cards);
    let cards: Vec<crate::flyer::CardSpec> = grid_cards
        .chain(back_cards)
        .chain(campaign_cards.into_iter().flatten())
        .collect();
    let widgets = oag_ui_screens::campaign::flyer::read(&xml, &screens);
    let flyers = widgets
        .iter()
        .any(|widget| widget.name == oag_ui_screens::campaign::flyer::GRID_WIDGET)
        .then(|| crate::flyer::Flyers::load(archives, widgets, &cards));
    if let Some(flyers) = &flyers {
        for line in &flyers.report {
            log::warn!("{line}");
        }
    } else {
        log::warn!(
            "{} authors no <Flyer name=\"FlyerModel\"> - Grid Selection draws no flyer",
            oag_hd::campaign::SCREEN_ENTRY
        );
    }
    // `Cell Selection`'s own `Confirm`/`Back` legend, off the shared
    // front-end root - see [`read_footer`]'s own doc. `Cell Help` and the
    // ticker are still unmodelled: `CellMode_Definition.xml` authors no
    // `Cell Help` screen on HD at all (unlike Pulse's own copy), and
    // `Skin.xml` authors no `TextInfoIsAlwaysLast` viewport either
    // (confirmed by direct read, `TickerLayout::read` answers `None`
    // regardless of that - this call just never invents the gap).
    let (nav_legend, ticker) = read_footer(
        archives,
        oag_hd::frontend::names::FRONTEND_ROOT,
        strings,
        fallback_globals,
    );

    let mut blobs = Vec::new();
    // `HEX_TEXTURES`' own `(widget src, archive path)` pairs - read off the
    // second, shelved under the first, since that is the spelling
    // `image.src` carries at draw time. See that constant's own doc for why
    // the two differ.
    for (widget_src, archive_path) in oag_hd::campaign::HEX_TEXTURES {
        match read_hd_texture(archives, archive_path) {
            Ok(blob) => blobs.push((widget_src.to_string(), blob)),
            Err(error) => {
                log::warn!("{archive_path}: {error:#} - the widget it is for draws nothing");
            }
        }
    }
    for src in oag_hd::campaign::OTHER_TEXTURES {
        match read_hd_texture(archives, src) {
            Ok(blob) => blobs.push((src.to_string(), blob)),
            Err(error) => log::warn!("{src}: {error:#} - the widget it is for draws nothing"),
        }
    }
    // Each grid's own `Logo.gtf`, which the unlock box under the card names
    // by grid - see `oag_ui_screens::campaign::hd`'s `UnlockBox`. Keyed by the same
    // spelling the screen's `flyerlogo` widget authors.
    for name in &flyer_names {
        let entry = oag_ui_screens::campaign::flyer::logo_entry(name);
        match read_hd_texture(archives, &entry) {
            Ok(blob) => blobs.push((entry, blob)),
            Err(error) => log::warn!("{entry}: {error:#} - its unlock-box logo draws nothing"),
        }
    }
    let circuit_emblems = emblems::circuit_emblems(tracks);
    emblems::push_blobs(archives, &grids, &circuit_emblems, &mut blobs);
    let mut report = Vec::new();
    let sprites = base.extended(&blobs, &mut report);
    oag_raceplay::loader_log::lines(report.iter().map(|line| format!("campaign sprites {line}")));

    Ok(Campaign {
        grids,
        grid_layout,
        cell_layout,
        // HD's own `Cell Help` is still a different lane's own thread - see
        // `Campaign::nav_legend`'s own doc. The legend and the ticker are
        // read above.
        cell_help: None,
        nav_legend,
        ticker,
        sprites,
        selection_layout,
        grid_layout_fury,
        flyers,
        circuit_emblems,
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
/// `Campaign Selection`'s own four idstrings - [`oag_ui_screens::campaign::selection::TITLE_ID`]/
/// `SUBTITLE_ID` and both campaigns' own [`oag_ui_screens::campaign::selection::Campaign::entry_id`] -
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
/// all (just [`oag_ui_screens::campaign::selection::TITLE_ID`]/`SUBTITLE_ID`, not
/// the two entry names), so a `--menu-page campaign-select` capture showed
/// raw ids where a player's own session showed real text - the gap this
/// closes.
#[must_use]
pub fn hd_selection_string_overlay(
    archives: &mut oag_assets::Archives,
    entries_path: &str,
) -> std::collections::HashMap<String, String> {
    hd_data06_strings(
        archives,
        entries_path,
        &[
            oag_ui_screens::campaign::selection::TITLE_ID,
            oag_ui_screens::campaign::selection::SUBTITLE_ID,
            oag_ui_screens::campaign::selection::Campaign::Fury.entry_id(),
            oag_ui_screens::campaign::selection::Campaign::Hd.entry_id(),
        ],
    )
}

/// `ids`, resolved off `entries_path`'s own `DATA06` copy - the general form
/// of [`hd_selection_string_overlay`], for another `DATA06`-only screen with
/// ids of its own the precedence-served table lacks: `Team Selection`'s
/// `RC_NAV_TEAM` ("NAVIGATE TEAM") is in `DATA06`'s English `entries.xml`
/// and in none of `DATA02`..`DATA05`'s, confirmed directly against
/// `hdfury-ps3-eu-dec.iso`. An id the copy lacks is left out.
#[must_use]
pub fn hd_data06_strings(
    archives: &mut oag_assets::Archives,
    entries_path: &str,
    ids: &[&str],
) -> std::collections::HashMap<String, String> {
    archives
        .read_every_name(entries_path)
        .into_iter()
        .find(|(label, _)| label.ends_with(oag_hd::campaign::SELECTION_SCREEN_ARCHIVE))
        .and_then(|(_, blob)| oag_tables::fexml::text(&blob).ok())
        .map(|xml| {
            let table = StringTable::from_xml(&xml);
            ids.iter()
                .filter_map(|id| {
                    table
                        .get(id)
                        .map(|value| ((*id).to_string(), value.to_string()))
                })
                .collect()
        })
        .unwrap_or_default()
}

/// `Campaign Selection`/`Grid Selection Fury`, off the same already-parsed
/// `DATA06` copy of [`oag_hd::campaign::SCREEN_ENTRY`] [`load_hd`] reads its
/// base `Grid Selection`/`Cell Selection` from - **one parse serving all
/// four screens now**, where this function used to open and parse the file
/// a second time on its own (`load_hd_campaign_selection`, before
/// 2026-09-27 switched `load_hd`'s own screen source to `DATA06` too, making
/// the second read redundant - see `oag_ui_screens::campaign::hd`'s own module doc).
///
/// `(None, None)` when this copy's own XML does not carry both screens - a
/// base, non-Fury HD pressing this project has not seen, say.
/// [`load_hd`] still returns its own `Campaign` in that case: the base
/// `Grid Selection`/`Cell Selection` are unaffected, and
/// `crate::main::session::campaign::open_campaign` falls back to opening
/// straight on `Grid Selection`, the pre-`Campaign Selection` behaviour,
/// rather than refusing the whole campaign over one missing screen.
fn hd_selection_screens(
    title: &oag_title::Title,
    screens: &oag_ui::screen::Screens,
    strings: &StringTable,
    faces: FaceScales,
    grid: [f32; 2],
) -> (Option<Layout>, Option<Layout>) {
    let selection = Layout::read_authored(
        screens,
        oag_hd::campaign::SELECTION_SCREEN,
        strings,
        faces,
        grid,
        oag_hd::campaign::AUTHORED_GRID,
    );
    let fury = Layout::read_authored(
        screens,
        oag_hd::campaign::FURY_GRID_SCREEN,
        strings,
        faces,
        grid,
        oag_hd::campaign::AUTHORED_GRID,
    );
    match (selection, fury) {
        (Some(selection), Some(fury)) => (Some(selection), Some(fury)),
        _ => {
            // A title with no `screen_archive` has no chooser to find, which is
            // how its campaign is authored, not a gap worth a warning.
            if let Some(label) = title.campaign.screen_archive {
                log::warn!(
                    "{label}'s own copy of {} is missing {} or {} - Campaign Selection stays unmodelled",
                    oag_hd::campaign::SCREEN_ENTRY,
                    oag_hd::campaign::SELECTION_SCREEN,
                    oag_hd::campaign::FURY_GRID_SCREEN,
                );
            }
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
/// reads by the literal HD `.gtf` path first and falls back to
/// [`crate::boot::sprites::gnf_sibling`]** the same way the front end's own
/// sheet does (`crate::boot::sprites::load`) - the shipped files are `.gnf`
/// now, not `.gtf`, so the first read always misses on this title and the
/// fallback is what actually decodes and names them. A texture whose `.gnf`
/// will not decode either (`docs/formats/gnf.md`'s "Tiling" section) still
/// leaves the widget it was for undrawn rather than failing the whole screen
/// load.
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

    let grids = read_grids(
        archives,
        oag_omega::campaign::DEFINITION_ENTRY,
        oag_omega::TITLE.campaign.grid_archive,
    )?;
    // See [`load_hd`]'s own identical call - Omega's front end is HD's
    // `PI001` plugin carried forward, at the same relative root path.
    let (nav_legend, ticker) = read_footer(
        archives,
        oag_omega::frontend::names::FRONTEND_ROOT,
        strings,
        fallback_globals,
    );

    let mut blobs = Vec::new();
    for (widget_src, archive_path) in oag_hd::campaign::HEX_TEXTURES {
        match archives.read_name(archive_path) {
            Ok(blob) => blobs.push((widget_src.to_string(), blob)),
            Err(error) => match crate::boot::sprites::gnf_sibling(archives, archive_path) {
                Some(Ok(blob)) => blobs.push((widget_src.to_string(), blob)),
                Some(Err(reason)) => log::warn!("campaign {reason}"),
                None => {
                    log::warn!("{archive_path}: {error:#} - the widget it is for draws nothing");
                }
            },
        }
    }
    for src in oag_hd::campaign::OTHER_TEXTURES {
        match archives.read_name(src) {
            Ok(blob) => blobs.push((src.to_string(), blob)),
            Err(error) => match crate::boot::sprites::gnf_sibling(archives, src) {
                Some(Ok(blob)) => blobs.push((src.to_string(), blob)),
                Some(Err(reason)) => log::warn!("campaign {reason}"),
                None => log::warn!("{src}: {error:#} - the widget it is for draws nothing"),
            },
        }
    }
    let mut report = Vec::new();
    let sprites = base.extended(&blobs, &mut report);
    oag_raceplay::loader_log::lines(report.iter().map(|line| format!("campaign sprites {line}")));

    Ok(Campaign {
        grids,
        grid_layout,
        cell_layout,
        // Omega's own `Cell Help` is still unmodelled - see `load_hd`'s own
        // identical note; the legend and the ticker are read above.
        cell_help: None,
        nav_legend,
        ticker,
        sprites,
        // Omega's own front end is HD's `PI001` plugin carried forward, but
        // no `Campaign Selection`/`Grid Selection Fury` has been measured on
        // it - Omega's racing is out of scope entirely
        // (`docs/formats/omega-status.md`), so this is left unmodelled
        // rather than assumed to carry HD's own screen unread.
        selection_layout: None,
        grid_layout_fury: None,
        flyers: None,
        circuit_emblems: std::collections::HashMap::new(),
    })
}

/// Every grid `definition_entry` lists - shared by [`load`]'s Pulse path and
/// [`load_hd`], since both titles author the same `PI_Grid`/`PI_Cell` schema
/// ([`oag_tables::race_campaign`]) behind a `Definition.xml` naming the same
/// shape of `Src=` list, dictionary-shortened on both.
///
/// Each grid is read off `archives`' own precedence, except one `grid_archive`
/// ([`oag_title::Campaign::grid_archive`]) carries a copy of: that copy wins.
pub fn read_grids(
    archives: &mut oag_assets::Archives,
    definition_entry: &str,
    grid_archive: Option<&str>,
) -> Result<Vec<Grid>> {
    let definition_blob = archives
        .read_name(definition_entry)
        .with_context(|| format!("reading {definition_entry}"))?;
    let definition_xml =
        oag_tables::fexml::text(&definition_blob).context("expanding grids/Definition.xml")?;
    let mut grids = Vec::new();
    for src in race_campaign::definition_entries(&definition_xml) {
        let preferred = grid_archive.and_then(|wanted| {
            archives
                .read_every_name(&src)
                .into_iter()
                .find(|(label, _)| same_archive(label, wanted))
                .map(|(_, blob)| blob)
        });
        let blob = match preferred {
            Some(blob) => Ok(blob),
            None => archives.read_name(&src).map_err(anyhow::Error::from),
        };
        match blob.and_then(|blob| race_campaign::from_blob(&blob).map_err(anyhow::Error::from)) {
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
/// launches as - `None` for the two this engine cannot run at all.
///
/// **Not a spelling mismatch to resolve, a scope one.** `oag_race::Mode` has
/// seven variants because that is what `crates/race` implements; a campaign
/// cell's own mode is one of nine, read straight off the disc
/// (`docs/ghidra/functions/psp-pulse-usa/race-campaign.md`). `Custom
/// Grid`/`AI Race` are not authored by any shipped `grid_NN.xml` cell at
/// all - nothing to map onto. A cell whose mode maps to `None` must not
/// launch - the caller logs why and stays on `Cell Selection` rather than
/// substituting an implemented mode for an unimplemented one.
///
/// **`Tournament` now maps too**, since `docs/ghidra/functions/psp-pulse-usa/tournament.md`
/// read the per-leg scoring and standings law in full -
/// `Session::launch_campaign_cell` is what actually carries the leg list and
/// the running totals across a relaunch; this function only says which
/// per-leg rules a Tournament leg races under (`oag_race::Mode::Tournament`
/// mirrors [`oag_race::Mode::SingleRace`] exactly - see that variant's own
/// doc comment).
///
/// **`Head2Head` maps too**, since `docs/ghidra/functions/psp-pulse-usa/head2head.md`
/// read its own law in full: a field of two (measured off `AICount="1"` on
/// all 23 authored cells, not this project's invention), weapons locked
/// off, and a win-or-nothing medal - see [`oag_race::Mode::Head2Head`]'s own
/// doc comment for the evidence.
#[must_use]
pub fn race_mode_for_cell(mode: race_campaign::Mode) -> Option<oag_race::Mode> {
    match mode {
        race_campaign::Mode::Race => Some(oag_race::Mode::SingleRace),
        race_campaign::Mode::TimeTrial => Some(oag_race::Mode::TimeTrial),
        race_campaign::Mode::Zone => Some(oag_race::Mode::Zone),
        race_campaign::Mode::Elimination => Some(oag_race::Mode::Eliminator),
        race_campaign::Mode::SpeedLap => Some(oag_race::Mode::SpeedLap),
        race_campaign::Mode::Tournament => Some(oag_race::Mode::Tournament),
        race_campaign::Mode::Head2Head => Some(oag_race::Mode::Head2Head),
        // `Other` is a mode name with no Pulse ordinal at all (HD's
        // `NitroBattle`/`Detonator`) - nothing to map onto, so it refuses
        // the same way the two unimplemented Pulse modes do.
        race_campaign::Mode::CustomGrid
        | race_campaign::Mode::AiRace
        | race_campaign::Mode::Other(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{race_mode_for_cell, same_archive};
    use oag_tables::race_campaign::Mode as CampaignMode;

    #[test]
    fn the_seven_implemented_modes_map_onto_their_oag_race_mode() {
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
        assert_eq!(
            race_mode_for_cell(CampaignMode::Tournament),
            Some(oag_race::Mode::Tournament)
        );
        assert_eq!(
            race_mode_for_cell(CampaignMode::Head2Head),
            Some(oag_race::Mode::Head2Head)
        );
    }

    #[test]
    fn the_two_unimplemented_modes_refuse_to_map_at_all() {
        for mode in [CampaignMode::CustomGrid, CampaignMode::AiRace] {
            assert_eq!(
                race_mode_for_cell(mode.clone()),
                None,
                "{mode} should not launch"
            );
        }
    }

    #[test]
    fn an_archive_is_named_by_file_name_whatever_the_case_or_folder() {
        let wanted = "PS3_GAME/USRDIR/DATA04.PSARC";
        assert!(same_archive("img.iso:PS3_GAME/USRDIR/DATA04.PSARC", wanted));
        assert!(same_archive("/x/USRDIR/data04.psarc", wanted));
        assert!(!same_archive("/x/USRDIR/data02.psarc", wanted));
    }
}
