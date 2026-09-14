//! Opens and drives the Race Campaign's two screens over the menus - the
//! same shape `session::picker` opens the race box's selection screens in.
//! See `oag_ui::campaign` for the model and `crate::campaign_stage` for what
//! this holds open.

use log::warn;
use oag_ui::campaign::Event;

use crate::campaign_stage::{CampaignStage, Screen};
use crate::stage::Stage;

use super::Session;

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
        match oag_game::campaign::load(&mut archives, &strings, faces, grid, &shell.sprites) {
            Ok(campaign) => {
                // The extended sheet - `hex_filled.mip`/`hex_outline.mip`,
                // neither of which `Skin.xml`'s own sheet carries - has to
                // reach the renderer once before anything drawn from it is
                // on screen, the same upload `PickerStage::take_sheet` does
                // for a slideshow's own stills.
                self.renderer_set_sprites(&campaign.sprites);
                if let Stage::Menu(menu_stage) = &mut self.stage {
                    menu_stage.campaign = Some(CampaignStage::new(
                        campaign.grids,
                        campaign.grid_layout,
                        campaign.cell_layout,
                        strings,
                        campaign.sprites,
                    ));
                }
            }
            Err(error) => warn!("{error:#} - RACE CAMPAIGN has nothing to show"),
        }
    }

    /// Uploads `sheet` to the menu stage's own renderer - `MenuStage::render`
    /// otherwise has no chance to, since [`Self::open_campaign`] runs before
    /// the next frame's draw and a campaign screen carries no `take_sheet`
    /// dance of its own the way a picker's slideshow does.
    fn renderer_set_sprites(&mut self, sheet: &oag_game::sprite::Sheet) {
        if let Stage::Menu(stage) = &mut self.stage {
            stage
                .renderer
                .set_sprites(&self.gpu.device, &self.gpu.queue, sheet);
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
            // doc. Confirming a cell logs and stays on `Cell Selection`
            // rather than opening `Team Selection`/`Launch Game`.
            (Screen::Cell { .. }, Event::Confirmed) => {
                log::info!(
                    "Cell Selection confirmed - launch is not wired yet, see oag_ui::campaign's \
                     own module doc"
                );
            }
            (Screen::Cell { .. }, Event::Back) => campaign.back_to_grid_selection(),
            (_, Event::Moved | Event::Help) => {}
        }
    }
}
