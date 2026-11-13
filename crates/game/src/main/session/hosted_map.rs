//! The 2048 campaign map, opened from a title's own menus.
//!
//! Omega's `Campaign Selection` offers 2048 beside HD and Fury; choosing it
//! parks the menus and shows the same `oag_ui::frontend::Frontend` map Vita
//! 2048 boots into, built from Omega's own data (`oag_game::boot::map_seed`).
//! Back returns to the chooser with 2048 still selected, and an event
//! launched from the map starts its race through [`Session::follow_launch`],
//! as 2048's own does.

use log::{debug, warn};

use oag_game::boot;
use oag_game::records;

use crate::frontend_stage::FrontendStage;
use crate::stage::Stage;

use super::Session;
use super::menus::combine_variant;

impl Session {
    /// Parks the menus and shows the campaign map.
    pub(crate) fn open_hosted_map(&mut self) {
        let Some(shell) = self.shell.clone() else {
            return;
        };
        let (Some(seed), Some(options)) = (shell.map.as_ref(), self.race_options.as_ref()) else {
            return;
        };
        let title_name = records::campaign_title(shell.title, shell.platform);
        let craft = combine_variant(shell.title, &self.settings.race.team, &self.settings.race.variant).0;
        let frontend = seed.open(&self.records, &title_name, &options.source, craft);
        let mut renderer = match oag_game::render::Renderer::new(
            &self.gpu.device,
            &self.gpu.queue,
            self.gpu.output.config.format,
            None,
            shell.font.clone(),
            &shell.sprites,
        ) {
            Ok(renderer) => renderer,
            Err(error) => {
                warn!("cannot show the campaign map: {error:#}");
                return;
            }
        };
        renderer.set_space(shell.space);
        renderer.set_inset(
            shell
                .title
                .hud_art
                .safe_area_scale(self.settings.graphics.hud_safe_area),
        );
        boot::fonts::install_faces(
            &mut renderer,
            &self.gpu.device,
            &self.gpu.queue,
            shell.menu_skin,
            &shell.font,
            shell.title_font.clone(),
            shell.buttons_font.clone(),
        );
        let Stage::Menu(menu) = std::mem::replace(
            &mut self.stage,
            Stage::Frontend(Box::new(FrontendStage::hosted(renderer, frontend))),
        ) else {
            return;
        };
        debug!("{}: opening the campaign map", shell.title.name);
        self.parked_menu = Some(menu);
    }

    /// Hands the window back to the menus when the map was backed out of or
    /// launched an event; a no-op while no map is open.
    pub(crate) fn poll_hosted_map(&mut self) {
        if self.parked_menu.is_none() {
            return;
        }
        let Stage::Frontend(stage) = &self.stage else {
            return;
        };
        let launch = if stage.frontend.is_finished() {
            Some(stage.frontend.launch().cloned())
        } else if stage.frontend.left_map() {
            None
        } else {
            return;
        };
        if let Some(menu) = self.parked_menu.take() {
            self.stage = Stage::Menu(menu);
        }
        if let Some(launch) = launch {
            self.follow_launch(launch);
        }
    }
}
