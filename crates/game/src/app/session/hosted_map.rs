//! The 2048 campaign map, opened from a title's own menus.
//!
//! Omega's `Campaign Selection` offers 2048 beside HD and Fury; choosing it
//! pushes over the menus and shows the same `oag_ui::frontend::Frontend` map Vita
//! 2048 boots into, built from Omega's own data (`crate::boot::map_seed`).
//! Back returns to the chooser with 2048 still selected, and an event
//! launched from the map starts its race through [`Session::follow_launch`],
//! as 2048's own does.

use log::{debug, warn};

use crate::boot;
use crate::records;

use crate::app::frontend_stage::FrontendStage;
use crate::app::stage::Stage;

use super::Session;
use super::menus::combine_variant;

impl Session {
    /// Shows the campaign map over the menus.
    pub(crate) fn open_hosted_map(&mut self) {
        if !matches!(self.stack.top(), Stage::Menu(_)) {
            return;
        }
        let Some(shell) = self.shell.clone() else {
            return;
        };
        let (Some(seed), Some(options)) = (shell.map.as_ref(), self.race_options.as_ref()) else {
            return;
        };
        let title_name = records::campaign_title(shell.title, shell.platform);
        let craft = combine_variant(
            shell.title,
            &self.settings.race.team,
            &self.settings.race.variant,
        )
        .0;
        let frontend = seed.open(&self.records, &title_name, &options.source, craft);
        let mut renderer = match crate::render::Renderer::new(
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
        debug!("{}: opening the campaign map", shell.title.name);
        self.stack.push(
            Stage::Frontend(Box::new(FrontendStage::hosted(renderer, frontend))),
            None,
        );
    }

    /// Hands the window back to the menus when the map was backed out of or
    /// launched an event; a no-op while no map is open.
    pub(crate) fn poll_hosted_map(&mut self) {
        let Stage::Frontend(stage) = self.stack.top() else {
            return;
        };
        if !matches!(self.stack.below(1), Some(Stage::Menu(_))) {
            return;
        }
        let mut card = None;
        let launch = if stage.frontend.is_finished() {
            card = stage.frontend.launched_card();
            Some(stage.frontend.launch().cloned())
        } else if stage.frontend.left_map() {
            None
        } else {
            return;
        };
        self.pop_activity();
        if let Some(launch) = launch {
            self.follow_launch(launch, card);
        }
    }
}
