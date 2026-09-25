//! `--menu-page campaign-select`/`grid-select`/`cell-select`: the Race
//! Campaign's own screens, split out of `menu_page.rs` under the 1,000-line
//! rule (`scripts/check-file-size.py`) once `campaign-select`'s own arm
//! pushed that file over - a move, with `campaign_kind`/`campaign_page`
//! unchanged. See `oag_game::campaign` for the read this is a still of, and
//! `crate::campaign_stage::Screen::Selection`'s own doc for `Campaign
//! Selection` itself.

use anyhow::{Context, Result};

/// Which Race Campaign screen a `--menu-page` name asks for, if either.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CampaignKind {
    /// **HD/Fury only.** `Campaign Selection`, ahead of `Grid` - see
    /// `crate::campaign_stage::Screen::Selection`'s own doc. Refused on
    /// every other title, the same way [`campaign_page`] already refuses a
    /// source with no campaign at all.
    Selection,
    Grid,
    Cell,
}

#[must_use]
pub(super) fn campaign_kind(page: &str) -> Option<CampaignKind> {
    match page {
        "campaign-select" | "campaign_select" => Some(CampaignKind::Selection),
        "grid-select" | "grid_select" => Some(CampaignKind::Grid),
        "cell-select" | "cell_select" => Some(CampaignKind::Cell),
        _ => None,
    }
}

/// [`crate::records::Medal`] restated as [`oag_tables::race_campaign::Medal`],
/// the same duplicated-on-purpose conversion `crate::campaign_stage::to_campaign_medal`
/// makes for the live session, redone here rather than shared: that one
/// lives in the `oag-game` *binary* crate (`main.rs`'s own `mod campaign_stage`),
/// this file in the *library* crate `pub mod capture` is built from, and
/// nothing crosses that split.
fn to_campaign_medal(medal: crate::records::Medal) -> oag_tables::race_campaign::Medal {
    match medal {
        crate::records::Medal::Gold => oag_tables::race_campaign::Medal::Gold,
        crate::records::Medal::Silver => oag_tables::race_campaign::Medal::Silver,
        crate::records::Medal::Bronze => oag_tables::race_campaign::Medal::Bronze,
    }
}

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
    kind: CampaignKind,
    archives: &mut oag_assets::Archives,
    strings: &oag_ui::language::StringTable,
    // **HD/Fury only, `None` on every other title.** The chosen language's
    // own `entries.xml` path - resolves `Campaign Selection`'s own four
    // idstrings off `DATA06`'s copy, the same overlay
    // `crate::main::session::campaign::open_campaign` applies for a live
    // session; see `oag_game::campaign::hd_selection_string_overlay`'s own
    // doc for why this is not already in `strings`.
    entries_path: Option<&str>,
    faces: oag_ui::picker::FaceScales,
    grid: [f32; 2],
    backdrop: Option<oag_ui::menu::Picture>,
    skin: &oag_ui::menu::Skin,
    frame: &oag_ui::menu::Frame,
    // `&mut`, extended in place with the hex textures neither screen's XML
    // shares with `Skin.xml` - the same `*sprites = sprites.extended(...)`
    // idiom `picker_stills` already uses, so the sheet `Renderer::new` builds
    // from further down `run` is the one these two textures actually landed
    // on.
    sprites: &mut crate::sprite::Sheet,
    // The front-end root's own `FEGlobals` - see `crate::campaign::load`'s
    // own doc for why a still needs this too, not only the live session.
    fallback_globals: &[(&str, &str)],
    title: &'static oag_title::Title,
    // The `Confirm`/`Back` fit-to-gap shrink's own text-width function.
    measure: &dyn Fn(&str) -> f32,
    // Read-only, off whatever `<config dir>/oag/records.toml` already holds -
    // the same "read, never write" rule the `records` `--menu-page` and
    // `race::CaptureOptions::previous_best` both already follow, applied
    // here so a locked/unlocked grid or cell in this still matches what a
    // live session (`crate::main::campaign_stage::CampaignStage`) would show
    // for the same file, rather than always drawing the fresh-profile
    // reading `GridSummary::from_grid`/`CellSelection::new` give on their
    // own. A capture with no file, or a title/cell this machine has never
    // raced under, draws exactly the fresh-profile numbers either way.
    records: &crate::records::Store,
) -> Result<Vec<oag_ui::frontend::Draw>> {
    let campaign = crate::campaign::load(
        archives,
        strings,
        faces,
        grid,
        sprites,
        fallback_globals,
        title,
    )
    .context("this source has no Race Campaign to show")?;
    // No per-session tip rotation, the same gap `circuit_names` below has -
    // see `crate::campaign::static_footer_overlay`. Read before
    // `campaign.sprites` moves out below.
    let footer_overlay = crate::campaign::static_footer_overlay(&campaign, &faces, measure);
    *sprites = campaign.sprites;
    let is_hd = title.name == oag_hd::TITLE.name;
    // A cell's own best saved medal, by `name` - see `records`' own doc
    // above for why this reads the real file rather than always `None`.
    let medal_of = |cell_name: &str| {
        records
            .campaign_medal(title.name, cell_name)?
            .best_medal
            .map(to_campaign_medal)
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
    // **HD's own `Track Line` fold is not built here.** `CircuitNames::choose`
    // needs every copy of the track-name table across the source's own
    // archives, which this still has no ready list of - see
    // `oag_ui::campaign::hd::hd_track_line`'s own doc for the plain fallback
    // this leaves a captured HD `Cell Selection` with (the raw track id, the
    // same gap `picker_page`'s own RACE-page capture already has). The live
    // session's `crate::main::session::campaign::open_campaign` does carry
    // one (`Shell::circuit_names`) and is the path that matters for a
    // player.
    let circuit_names = oag_ui::language::CircuitNames::default();
    // **HD/Fury only.** A still has no `Campaign Selection` step to narrow
    // `campaign.grids` the way `CampaignStage::open_grid_selection` always
    // does before a live session ever draws `Grid Selection`/`Cell
    // Selection` - undoing that narrowing here (`Grid`/`Cell` below used to
    // read `campaign.grids` whole, all sixteen) drew a page counter no real
    // screen ever shows: `Event 01/16` where RPCS3's own frame reads `Event
    // 01/08` (`docs/ui/campaign-screens.md`'s "measured on RPCS3" section).
    // Defaults to the base `Wipeout HD` campaign (`HD_GRID_RANGE`, grid0..7)
    // rather than `Fury` - `campaign.grid_layout` is that campaign's own
    // screen; `Fury`'s is a separate, differently-sourced
    // `campaign.grid_layout_fury` this capture path does not thread through,
    // so picking `Fury` here would draw `Fury`'s grids under `Wipeout HD`'s
    // own widget layout instead.
    let hd_grids = campaign
        .grids
        .get(oag_hd::campaign::HD_GRID_RANGE)
        .unwrap_or(&[]);
    let layers = if is_hd {
        match kind {
            CampaignKind::Selection => {
                let Some(layout) = campaign.selection_layout.as_ref() else {
                    anyhow::bail!(
                        "this source has no Campaign Selection screen - DATA06's own copy of \
                         {} is missing or incomplete, see oag_hd::campaign::SCREEN_ENTRY's own doc",
                        oag_hd::campaign::SCREEN_ENTRY
                    );
                };
                let model = oag_ui::campaign::selection::CampaignSelection::new();
                // No progress source in a still - `0`/`0`, the same
                // fresh-profile reading `GridSelection::new`'s own bare
                // `from_grid` call gives every other number below.
                oag_ui::campaign::selection::draw_list(
                    &model,
                    layout,
                    skin,
                    frame,
                    strings,
                    0,
                    0,
                    backdrop,
                    false,
                    &|src| sprites.get(src),
                )
            }
            CampaignKind::Grid => {
                let model = oag_ui::campaign::GridSelection::new(
                    hd_grids
                        .iter()
                        .map(|grid| {
                            oag_ui::campaign::GridSummary::from_grid_with_medals(grid, &medal_of)
                        })
                        .collect(),
                );
                oag_ui::campaign::hd::hd_grid_draw_list(
                    &model,
                    &campaign.grid_layout,
                    skin,
                    frame,
                    strings,
                    backdrop,
                    false,
                    &|src| sprites.get(src),
                )
            }
            CampaignKind::Cell => {
                // `crate::campaign::load` already refuses an empty grid
                // list (`"no grid in ... parsed"`), so `first()` is `None`
                // only if that changes; the fallback below is a zeroed
                // summary rather than a panic, on the same "a still draws
                // something honest rather than crashing" terms the rest of
                // this module follows.
                let grid = hd_grids.first();
                let cells = grid.map(|grid| grid.cells.clone()).unwrap_or_default();
                let model = oag_ui::campaign::CellSelection::with_medals(cells, &medal_of);
                let grid_summary = grid.map_or(
                    oag_ui::campaign::GridSummary {
                        name: String::new(),
                        cell_count: 0,
                        max_points: 0,
                        required_points: 0,
                        gold_medals: 0,
                        points_earned: 0,
                        locked: false,
                    },
                    |grid| oag_ui::campaign::GridSummary::from_grid_with_medals(grid, &medal_of),
                );
                oag_ui::campaign::hd::hd_cell_draw_list(
                    &model,
                    &campaign.cell_layout,
                    skin,
                    frame,
                    strings,
                    &circuit_names,
                    0,
                    hd_grids.len().max(1),
                    &grid_summary,
                    backdrop,
                    false,
                    &|src| sprites.get(src),
                )
            }
        }
    } else {
        match kind {
            CampaignKind::Selection => {
                anyhow::bail!(
                    "Campaign Selection is Wipeout HD/Fury's own screen - this source has none"
                );
            }
            CampaignKind::Grid => {
                let model = oag_ui::campaign::GridSelection::new(
                    campaign
                        .grids
                        .iter()
                        .map(|grid| {
                            oag_ui::campaign::GridSummary::from_grid_with_medals(grid, &medal_of)
                        })
                        .collect(),
                );
                oag_ui::campaign::grid_draw_list(
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
            CampaignKind::Cell => {
                // The first grid `Definition.xml` lists - `--menu-page` has
                // no way to name a tier, and a still needs something to
                // show.
                let cells = campaign
                    .grids
                    .first()
                    .map(|grid| grid.cells.clone())
                    .unwrap_or_default();
                let model = oag_ui::campaign::CellSelection::with_medals(cells, &medal_of);
                oag_ui::campaign::cell_draw_list(
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
    Ok(layers.flatten())
}
