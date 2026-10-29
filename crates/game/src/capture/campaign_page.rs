//! `--menu-page campaign-select`/`grid-select`/`cell-select`: the Race
//! Campaign's own screens, split out of `menu_page.rs` under the 1,000-line
//! rule (`scripts/check-file-size.py`) once `campaign-select`'s own arm
//! pushed that file over - a move, with `campaign_kind`/`campaign_page`
//! unchanged. See `oag_game::campaign` for the read this is a still of, and
//! `crate::campaign_stage::Screen::Selection`'s own doc for `Campaign
//! Selection` itself.

use anyhow::{Context, Result};

/// Which Race Campaign screen a `--menu-page` name asks for, and whose: a
/// trailing `=<mod id>/<campaign id>` draws a mod campaign (ADR-0077) found on
/// the mod search path, e.g. `cell-select@1=oag.example-campaign/night-run`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CampaignPage {
    pub(super) kind: CampaignKind,
    /// The mod campaign's key, `None` for the disc's own.
    pub(super) custom: Option<String>,
}

/// Which Race Campaign screen a `--menu-page` name asks for, if either.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CampaignKind {
    /// **HD/Fury only.** `Campaign Selection`, ahead of `Grid` - see
    /// `crate::campaign_stage::Screen::Selection`'s own doc. Refused on
    /// every other title, the same way [`campaign_page`] already refuses a
    /// source with no campaign at all. `selected` is `0` for Fury (the
    /// default) and `1` for `Wipeout HD`, spelled `campaign-select` and
    /// `campaign-select@1`.
    Selection { selected: usize },
    /// `hd_base` picks HD's base `grid0`..`grid7` over Fury's (the default,
    /// see [`campaign_page`]), and `tier` the page within them. Spelled
    /// `grid-select`, `grid-select-hd` and `grid-select-hd@3`.
    Grid { hd_base: bool, tier: usize },
    /// `hd_base` as on `Grid`: `cell-select-hd` draws the base campaign's first
    /// grid, `cell-select` Fury's. `tier` picks another grid (`cell-select@2`).
    Cell { hd_base: bool, tier: usize },
}

#[must_use]
pub(super) fn campaign_kind(page: &str) -> Option<CampaignPage> {
    let (page, custom) = match page.split_once('=') {
        Some((page, key)) => (page, Some(key.to_string())),
        None => (page, None),
    };
    campaign_kind_of(page).map(|kind| CampaignPage { kind, custom })
}

fn campaign_kind_of(page: &str) -> Option<CampaignKind> {
    let (name, tier) = match page.split_once('@') {
        Some((name, tier)) => (name, tier.parse().ok()?),
        None => (page, 0),
    };
    match name {
        "campaign-select" | "campaign_select" => Some(CampaignKind::Selection { selected: tier }),
        "grid-select" | "grid_select" => Some(CampaignKind::Grid {
            hd_base: false,
            tier,
        }),
        "grid-select-hd" | "grid_select_hd" => Some(CampaignKind::Grid {
            hd_base: true,
            tier,
        }),
        "cell-select" | "cell_select" => Some(CampaignKind::Cell {
            hd_base: false,
            tier,
        }),
        "cell-select-hd" | "cell_select_hd" => Some(CampaignKind::Cell {
            hd_base: true,
            tier,
        }),
        _ => None,
    }
}

/// Renders a `--menu-page` frame: `list` through `renderer`, with
/// `shot`'s flyer card between its backdrop and its widgets when it has one.
/// Exactly `Renderer::render` when it has none.
#[allow(clippy::too_many_arguments)]
pub(super) fn render_frame(
    shot: Option<&FlyerShot>,
    renderer: &mut crate::render::Renderer,
    gpu: (&wgpu::Device, &wgpu::Queue, wgpu::TextureFormat),
    encoder: &mut wgpu::CommandEncoder,
    view: &wgpu::TextureView,
    list: &[oag_ui::frontend::Draw],
    viewport: (f32, f32, f32, f32),
    target: ((u32, u32), oag_display::space::Space),
    clip: Option<(usize, f32, f32)>,
) {
    let (device, queue, format) = gpu;
    match shot {
        Some(shot) => shot.render(
            renderer, device, queue, format, encoder, view, list, viewport, target.0, target.1,
        ),
        None => renderer.render(device, queue, encoder, view, list, viewport, clip),
    }
}

/// The flyer card `--menu-page grid-select` owes the frame between its
/// backdrop and its widgets - see [`crate::flyer::render_list`].
pub(super) struct FlyerShot {
    flyers: crate::flyer::Flyers,
    shows: Vec<crate::flyer::Show>,
    /// How many of the list's first draws are the backdrop.
    split: usize,
}

impl FlyerShot {
    /// Renders `list` into `view` with the card in between.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn render(
        &self,
        renderer: &mut crate::render::Renderer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        list: &[oag_ui::frontend::Draw],
        viewport: (f32, f32, f32, f32),
        target_size: (u32, u32),
        space: oag_display::space::Space,
    ) {
        crate::flyer::render_list(
            renderer,
            wgpu::LoadOp::Clear(wgpu::Color::BLACK),
            (device, queue, format),
            encoder,
            view,
            (list, self.split),
            (viewport, target_size, space),
            Some((&self.flyers, &self.shows)),
            None,
        );
    }
}

use crate::unlock::{to_campaign_difficulty, to_campaign_medal};

/// Draws `Campaign Selection`, `Grid Selection` on its first tier, or `Cell
/// Selection` on that tier's own cells - the same shape
/// [`super::menu_page::picker_page`] draws the race box's two screens in,
/// minus a preview mesh: neither campaign screen authors one. See
/// `oag_game::campaign` for the read this is a still of.
#[allow(
    clippy::too_many_arguments,
    reason = "the same argument menu_page and picker_page both make: each is a separate fact \
              the page needs"
)]
pub(super) fn campaign_page(
    page: CampaignPage,
    archives: &mut oag_assets::Archives,
    strings: &oag_ui::language::StringTable,
    // The boot's own circuit-name column - the one `crate::main::session::campaign::open_campaign`
    // hands the live session - so `Track` on HD's `Cell Selection` names
    // the circuit rather than falling back to its raw id.
    circuit_names: &oag_ui::language::CircuitNames,
    // **HD/Fury only, `None` on every other title.** The chosen language's
    // own `entries.xml` path - resolves `Campaign Selection`'s own four
    // idstrings off `DATA06`'s copy, the same overlay
    // `crate::main::session::campaign::open_campaign` applies for a live
    // session; see `oag_game::campaign::hd_selection_string_overlay`'s own
    // doc for why this is not already in `strings`.
    entries_path: Option<&str>,
    faces: oag_ui_screens::picker::FaceScales,
    grid: [f32; 2],
    backdrop: Option<oag_ui::menu::Picture>,
    skin: &oag_ui::menu::Skin,
    frame: &oag_ui::menu::Frame,
    // `&mut`, extended in place with the hex textures neither screen's XML
    // shares with `Skin.xml` - the same `*sprites = sprites.extended(...)`
    // idiom `picker_stills` already uses, so the sheet `Renderer::new` builds
    // from further down `run` is the one these two textures actually landed
    // on.
    sprites: &mut oag_hud::sprite::Sheet,
    // The front-end root's own `FEGlobals` - see `crate::campaign::load`'s
    // own doc for why a still needs this too, not only the live session.
    fallback_globals: &[(&str, &str)],
    title: &'static oag_title::Title,
    platform: oag_disc::Platform,
    tracks: &[oag_raceplay::catalogue::Track],
    // Read-only, off whatever `<config dir>/oag/records.toml` already holds -
    // the same "read, never write" rule the `records` `--menu-page` and
    // `crate::race_capture::CaptureOptions::previous_best` both already follow, applied
    // here so a locked/unlocked grid or cell in this still matches what a
    // live session (`crate::main::campaign_stage::CampaignStage`) would show
    // for the same file, rather than always drawing the fresh-profile
    // reading `GridSummary::from_grid`/`CellSelection::new` give on their
    // own. A capture with no file, or a title/cell this machine has never
    // raced under, draws exactly the fresh-profile numbers either way.
    records: &crate::records::Store,
    // `--menu-picker-seconds`: how long after the screen opened `Selector`'s
    // pulse is drawn at (`SelectorPulse`); `None` draws it at clock 0.
    pulse_seconds: Option<f32>,
) -> Result<(Vec<oag_ui::frontend::Draw>, Option<FlyerShot>)> {
    let campaign = crate::campaign::load(
        archives,
        strings,
        faces,
        grid,
        sprites,
        fallback_globals,
        title,
        tracks,
    )
    .context("this source has no Race Campaign to show")?;
    let kind = page.kind;
    // A mod campaign: the disc's screens, its grids replaced, as
    // `Session::open_campaign` does live.
    let custom = match &page.custom {
        Some(key) => {
            let mods = crate::mods::Mods::load_for_profile()?;
            let installed = mods.installed_for(title, &|_| true);
            for (mod_id, reason) in &installed.failed {
                log::error!("mod {mod_id}: campaign left out: {reason}");
            }
            Some(
                installed
                    .get(key)
                    .cloned()
                    .with_context(|| format!("no mod campaign {key} for {}", title.name))?,
            )
        }
        None => None,
    };
    let mut campaign = campaign;
    if let Some(custom) = &custom {
        campaign.grids.clone_from(&custom.grids);
        campaign.selection_layout = None;
        campaign.grid_layout_fury = None;
    }
    // No per-session tip rotation, the same gap `circuit_names` below has -
    // see `crate::campaign::static_footer_overlay`. Read before
    // `campaign.sprites` moves out below.
    let footer_overlay = crate::campaign::static_footer_overlay(&campaign, &faces);
    *sprites = campaign.sprites;
    let is_hd = crate::campaign::draws_hd_campaign(title);
    // A cell's own best saved medal, by `name` - see `records`' own doc
    // above for why this reads the real file rather than always `None`.
    let campaign_title = custom.as_ref().map_or_else(
        || crate::records::campaign_title(title, platform),
        |custom| custom.records_title.clone(),
    );
    // A mod campaign's `open`/`requires` locks and hidden cells, as
    // `CampaignStage::open_cell_selection` applies them.
    let cell_model = |cells: Vec<oag_tables::race_campaign::Cell>, which: usize| {
        let build = |cells: Vec<oag_tables::race_campaign::Cell>| {
            let model = oag_ui_screens::campaign::CellSelection::with_medals(cells, &|name| {
                records
                    .campaign_medal(&campaign_title, name)?
                    .best_medal
                    .map(to_campaign_medal)
            });
            match &custom {
                Some(custom) => model.with_fixed_locks(&|name| {
                    custom.fixed_lock(
                        name,
                        &crate::unlock::campaign_medal_of(records, &campaign_title),
                    )
                }),
                None => model,
            }
        };
        let mut model = build(cells.clone());
        if let Some(custom) = &custom
            && custom.reveal(which) == oag_mod::campaign::Reveal::Hidden
        {
            let locked = model.locked_names();
            if !locked.is_empty() && locked.len() < cells.len() {
                model = build(
                    cells
                        .into_iter()
                        .filter(|c| !locked.contains(&c.name))
                        .collect(),
                );
            }
        }
        model
    };
    let medal_of = |cell_name: &str| {
        records
            .campaign_medal(&campaign_title, cell_name)?
            .best_medal
            .map(to_campaign_medal)
    };
    // [`medal_of`]'s own sibling for `oag_ui_screens::campaign::CellSelection::with_difficulty`
    // - **HD only**, per that method's own doc; read unconditionally here
    // the same way `medal_of` is, since it answers `None` on every non-HD
    // row regardless.
    let difficulty_of = |cell_name: &str| {
        records
            .campaign_medal(&campaign_title, cell_name)?
            .best_difficulty
            .map(to_campaign_difficulty)
    };
    // See `entries_path`'s own doc: HD only, and only when the chosen
    // language actually has an `entries.xml` to overlay `DATA06`'s copy of
    // `Campaign Selection`'s own four idstrings onto.
    let overlaid_strings = is_hd.then_some(entries_path).flatten().map(|path| {
        let overlay = crate::campaign::hd_selection_string_overlay(archives, path);
        let mut merged = strings.clone();
        merged.merge(overlay);
        merged
    });
    let strings = overlaid_strings.as_ref().unwrap_or(strings);
    // **HD/Fury only.** A still has no `Campaign Selection` step to narrow
    // `campaign.grids` the way `CampaignStage::open_grid_selection` always
    // does before a live session ever draws `Grid Selection`/`Cell
    // Selection` - undoing that narrowing here (`Grid`/`Cell` below used to
    // read `campaign.grids` whole, all sixteen) drew a page counter no real
    // screen ever shows: `Event 01/16` where RPCS3's own frame reads `Event
    // 01/08` (`docs/ui/campaign-screens.md`'s "measured on RPCS3" section).
    // Defaults to `Fury` - the measured default `CampaignSelection::new`
    // itself starts on and the campaign every RPCS3 reference frame this
    // pass found is actually of (a scratch directory, not kept,
    // `lane-hd-sel/rpcs3-campaign-selection`) - so a `--menu-page`
    // `grid-select`/`cell-select` still is directly comparable to those
    // frames rather than to a base-campaign state nothing on disk shows.
    // `campaign.grid_layout_fury` is `Fury`'s own screen (a `pub` field,
    // already loaded by `crate::campaign::load_hd`); falls back to the base
    // `Wipeout HD` campaign only when a source's `DATA06` copy is missing or
    // incomplete and `grid_layout_fury` is `None` (`Campaign::selection_layout`'s
    // own doc: the two are `Some`/`None` together).
    let base_hd = matches!(
        kind,
        CampaignKind::Grid { hd_base: true, .. } | CampaignKind::Cell { hd_base: true, .. }
    );
    let (hd_grids, hd_grid_layout) = match campaign.grid_layout_fury.as_ref().filter(|_| !base_hd) {
        // A mod campaign pages all of its own grids on the base layout.
        _ if custom.is_some() => (&campaign.grids[..], &campaign.grid_layout),
        Some(layout) => (
            campaign
                .grids
                .get(oag_hd::campaign::FURY_GRID_RANGE)
                .unwrap_or(&[]),
            layout,
        ),
        None => (
            campaign
                .grids
                .get(oag_hd::campaign::HD_GRID_RANGE)
                .unwrap_or(&[]),
            &campaign.grid_layout,
        ),
    };
    let mut flyer_name = None;
    let mut cell_page = false;
    let mut selected_campaign = None;
    let layers = if is_hd {
        match kind {
            CampaignKind::Selection { selected } => {
                let Some(layout) = campaign.selection_layout.as_ref() else {
                    anyhow::bail!(
                        "this source has no Campaign Selection screen - DATA06's own copy of \
                         {} is missing or incomplete, see oag_hd::campaign::SCREEN_ENTRY's own doc",
                        oag_hd::campaign::SCREEN_ENTRY
                    );
                };
                let model =
                    oag_ui_screens::campaign::selection::CampaignSelection::at(if selected == 0 {
                        oag_ui_screens::campaign::selection::Campaign::Fury
                    } else {
                        oag_ui_screens::campaign::selection::Campaign::Hd
                    });
                selected_campaign = Some(model.selected());
                // No progress source in a still - `0` earned, the same
                // fresh-profile reading `GridSelection::new`'s own bare
                // `from_grid` call gives every other number below. The
                // denominator is not progress, though - a campaign's own
                // total cell count never depends on a save file - so it is
                // summed here the same way `CampaignStage::campaign_gold_medals`
                // does live.
                let total = |range: std::ops::Range<usize>| {
                    campaign
                        .grids
                        .get(range)
                        .unwrap_or(&[])
                        .iter()
                        .map(|grid| u32::try_from(grid.cells.len()).unwrap_or(u32::MAX))
                        .sum::<u32>()
                };
                oag_ui_screens::campaign::selection::draw_list(
                    &model,
                    layout,
                    skin,
                    frame,
                    strings,
                    (0, total(oag_hd::campaign::FURY_GRID_RANGE)),
                    (0, total(oag_hd::campaign::HD_GRID_RANGE)),
                    backdrop,
                    false,
                    &|src| sprites.get(src),
                    &footer_overlay,
                    campaign
                        .flyers
                        .as_ref()
                        .is_some_and(|flyers| !flyers.selection_shows(model.selected()).is_empty()),
                )
            }
            CampaignKind::Grid { tier, .. } => {
                let mut model = oag_ui_screens::campaign::GridSelection::new(
                    hd_grids
                        .iter()
                        .map(|grid| {
                            oag_ui_screens::campaign::GridSummary::from_grid_with_medals(
                                grid, &medal_of,
                            )
                        })
                        .collect(),
                );
                model.set_index(tier);
                flyer_name = model.selected().and_then(|grid| grid.flyer_name.clone());
                oag_ui_screens::campaign::hd::hd_grid_draw_list(
                    &model,
                    hd_grid_layout,
                    skin,
                    frame,
                    strings,
                    backdrop,
                    false,
                    &|src| sprites.get(src),
                    &footer_overlay,
                )
            }
            CampaignKind::Cell { tier, .. } => {
                // `crate::campaign::load` already refuses an empty grid
                // list (`"no grid in ... parsed"`), so `get` is `None`
                // only on a tier past the end; the fallback below is a zeroed
                // summary rather than a panic, on the same "a still draws
                // something honest rather than crashing" terms the rest of
                // this module follows.
                let grid = hd_grids.get(tier);
                flyer_name = grid.and_then(|grid| grid.flyer_name.clone());
                cell_page = true;
                let cells = grid.map(|grid| grid.cells.clone()).unwrap_or_default();
                // HD/Fury's own fresh-profile default rung is `Easy`, not
                // Pulse's `Medium` - see
                // `oag_ui_screens::campaign::CellSelection::difficulty`'s own doc for
                // the RPCS3 measurement. Already inside `is_hd` here, so
                // unconditional.
                let model = crate::campaign::with_opening_difficulty(
                    cell_model(cells, tier).with_difficulty(&difficulty_of),
                    records,
                    &campaign_title,
                    true,
                );
                let grid_summary =
                    grid.map_or(oag_ui_screens::campaign::GridSummary::empty(), |grid| {
                        oag_ui_screens::campaign::GridSummary::from_grid_with_medals(
                            grid, &medal_of,
                        )
                    });
                oag_ui_screens::campaign::hd::hd_cell_draw_list(
                    &model,
                    &campaign.cell_layout,
                    skin,
                    frame,
                    strings,
                    circuit_names,
                    tier,
                    hd_grids.len().max(1),
                    &grid_summary,
                    &oag_ui_screens::campaign::hd::CellArt {
                        next_flyer: hd_grids
                            .get(tier + 1)
                            .and_then(|grid| grid.flyer_name.as_deref()),
                        fury: !base_hd && campaign.grid_layout_fury.is_some(),
                        track_emblems: &campaign.circuit_emblems,
                    },
                    backdrop,
                    false,
                    &|src| sprites.get(src),
                    &footer_overlay,
                )
            }
        }
    } else {
        match kind {
            CampaignKind::Selection { .. } => {
                anyhow::bail!(
                    "Campaign Selection is Wipeout HD/Fury's own screen - this source has none"
                );
            }
            CampaignKind::Grid { .. } => {
                let model = oag_ui_screens::campaign::GridSelection::new(
                    campaign
                        .grids
                        .iter()
                        .map(|grid| {
                            oag_ui_screens::campaign::GridSummary::from_grid_with_medals(
                                grid, &medal_of,
                            )
                        })
                        .collect(),
                );
                let mut model = model;
                model.pulse_mut().set_seconds(pulse_seconds.unwrap_or(0.0));
                oag_ui_screens::campaign::grid_draw_list(
                    &model,
                    &campaign.grid_layout,
                    skin,
                    frame,
                    strings,
                    backdrop,
                    false,
                    &|src| sprites.get(src),
                    &footer_overlay,
                )
            }
            CampaignKind::Cell { tier, .. } => {
                // The grid `@N` names, the first by default.
                let cells = campaign
                    .grids
                    .get(tier)
                    .map(|grid| grid.cells.clone())
                    .unwrap_or_default();
                let model = crate::campaign::with_opening_difficulty(
                    cell_model(cells, tier),
                    records,
                    &campaign_title,
                    false,
                );
                let mut model = model;
                model.pulse_mut().set_seconds(pulse_seconds.unwrap_or(0.0));
                oag_ui_screens::campaign::cell_draw_list(
                    &model,
                    &campaign.cell_layout,
                    skin,
                    frame,
                    strings,
                    backdrop,
                    false,
                    &|src| sprites.get(src),
                    &footer_overlay,
                )
            }
        }
    };
    let layers = crate::campaign::gate_glow(campaign.frontend_glow, layers);
    let split = layers.backdrop.len();
    let shot = campaign.flyers.and_then(|flyers| {
        let shows = match (flyer_name, selected_campaign) {
            (Some(name), _) if cell_page => vec![crate::flyer::Flyers::cell_show(&name)],
            (Some(name), _) => vec![flyers.grid_show(&name)],
            (None, Some(selected)) => flyers.selection_shows(selected),
            (None, None) => return None,
        };
        Some(FlyerShot {
            flyers,
            shows,
            split,
        })
    });
    Ok((layers.flatten(), shot))
}
