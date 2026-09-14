//! Opens and drives the Race Campaign's two screens over the menus - the
//! same shape `session::picker` opens the race box's selection screens in.
//! See `oag_ui::campaign` for the model and `crate::campaign_stage` for what
//! this holds open.

use anyhow::{Context, Result};
use log::warn;
use oag_tables::race_campaign;
use oag_ui::campaign::{Event, Layout};

use crate::campaign_stage::{CampaignStage, Screen};
use crate::stage::Stage;

use super::Session;

/// `Data\Plugins\PI001\GUI\CellMode_Definition.xml` - the file
/// `Grid Selection` and `Cell Selection` are both authored in. Pulse-only:
/// no other title has been checked for this path (`docs/formats/race-setup.md`
/// leaves Pure's/HD's own Race Campaign screens unread), so a source that
/// does not carry it just logs and opens nothing - the same "let the data
/// answer" shape `Shell::track_select` already gives a title with no race
/// box.
const SCREEN_ENTRY: &str = r"Data\Plugins\PI001\GUI\CellMode_Definition.xml";

impl Session {
    /// `RACE CAMPAIGN`'s own action: opens `Grid Selection` over the menus.
    ///
    /// Reads the screen definition and all sixteen grid files fresh, the
    /// same way [`Self::open_track_picker`] opens the race box's archives on
    /// every press rather than keeping them from boot - see that function's
    /// own doc. A source with no campaign at all logs why and leaves the
    /// menus exactly as they were.
    pub(crate) fn open_campaign(&mut self) {
        let Some(options) = self.race_options.as_ref() else {
            return;
        };
        let (packs, pure_packs, problems) = oag_game::dlc::packs_from_defaults(
            &options.dlc,
            &oag_game::boot::default_dlc_cache_dir(),
        );
        for problem in problems {
            warn!("{problem}");
        }
        let mut archives = match oag_game::title::open_source(&options.source, packs, pure_packs) {
            Ok(opened) => opened.archives,
            Err(error) => {
                warn!(
                    "cannot open {} for RACE CAMPAIGN: {error:#}",
                    options.source
                );
                return;
            }
        };
        let Some(shell) = self.shell.as_ref() else {
            return;
        };
        let faces = oag_ui::picker::FaceScales {
            default: shell
                .menu_font
                .as_ref()
                .map_or(oag_ui::picker::FaceScales::default().default, |menu| {
                    shell.font.line_height / menu.line_height
                }),
            ..oag_ui::picker::FaceScales::default()
        };
        let grid = [shell.space.size.0, shell.space.size.1];
        let strings = shell.strings.clone();
        let sprites = shell.sprites.clone();
        match load_campaign(&mut archives, &strings, faces, grid, sprites) {
            Ok(stage) => {
                if let Stage::Menu(menu_stage) = &mut self.stage {
                    menu_stage.campaign = Some(stage);
                }
            }
            Err(error) => warn!("{error:#} - RACE CAMPAIGN has nothing to show"),
        }
    }

    /// One tick of an open campaign screen: its input, and what came of it.
    pub(crate) fn tick_campaign(&mut self) {
        let Stage::Menu(stage) = &mut self.stage else {
            return;
        };
        let Some(campaign) = stage.campaign.as_mut() else {
            return;
        };
        let events = match &mut campaign.screen {
            Screen::Grid(model) => model.update(self.controls.buttons_mut()),
            Screen::Cell { model, .. } => model.update(self.controls.buttons_mut()),
        };
        for event in events {
            self.handle_campaign(event);
        }
    }

    pub(crate) fn handle_campaign(&mut self, event: Event) {
        let Stage::Menu(stage) = &mut self.stage else {
            return;
        };
        let Some(campaign) = stage.campaign.as_mut() else {
            return;
        };
        match (&campaign.screen, event) {
            (Screen::Grid(model), Event::Confirmed) => {
                let index = model.index();
                if !campaign.open_cell_selection(index) {
                    warn!("grid {index} has no cells - staying on Grid Selection");
                }
            }
            (Screen::Grid(_), Event::Back) => stage.campaign = None,
            // **Not wired past here** - see `oag_ui::campaign`'s own module
            // doc and the `campaign` handover thread. Confirming a cell logs
            // and stays on `Cell Selection` rather than opening `Team
            // Selection`/`Launch Game`.
            (Screen::Cell { .. }, Event::Confirmed) => {
                log::info!(
                    "Cell Selection confirmed - launch is not wired yet, see \
                     handover/frontend/the-campaign-grid-draws-and-does-not-launch.md"
                );
            }
            (Screen::Cell { .. }, Event::Back) => campaign.back_to_grid_selection(),
            (_, Event::Moved | Event::Help) => {}
        }
    }
}

fn load_campaign(
    archives: &mut oag_assets::Archives,
    strings: &oag_ui::language::StringTable,
    faces: oag_ui::picker::FaceScales,
    grid: [f32; 2],
    sprites: oag_game::sprite::Sheet,
) -> Result<CampaignStage> {
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
            Err(error) => warn!("{src}: {error:#} - grid skipped"),
        }
    }
    if grids.is_empty() {
        anyhow::bail!(
            "no grid in {} parsed",
            oag_pulse::campaign::DEFINITION_ENTRY
        );
    }
    log::info!(
        "Race Campaign: {} grid(s), {} cell(s)",
        grids.len(),
        grids.iter().map(|g| g.cells.len()).sum::<usize>()
    );
    Ok(CampaignStage::new(
        grids,
        grid_layout,
        cell_layout,
        strings.clone(),
        sprites,
    ))
}
