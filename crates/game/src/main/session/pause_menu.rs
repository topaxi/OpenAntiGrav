//! The title's own pause screen over a parked race: when it applies, what it is built from,
//! and what its RESTART RACE and QUIT RACE rows do.
//!
//! The rows are the disc's (`oag_title::PauseMenu`), put at the root of the menu tree by
//! `oag_ui::menu::Definition::open_on_pause_screen`. A title that authors none, a split-screen
//! race and a network race keep what they did: the front end's main page over the parked race.

use log::{debug, info, warn};

use oag_ui::menu::Definition;

use crate::race_stage::RaceStage;
use crate::stage::Stage;

use super::Session;

impl Session {
    /// Whether the race in `stage` pauses into the title's own pause screen: the title authors
    /// one, and the race has one view and is not a network race.
    pub(crate) fn pause_screen_applies(&self, stage: &RaceStage) -> bool {
        self.net.is_none()
            && stage.race.viewport_count() <= 1
            && !stage.race.windowed()
            && self
                .shell
                .as_ref()
                .is_some_and(|shell| shell.title.pause_menu.is_some())
    }

    /// The race the pause screen is for: the live one, or the one parked behind the menus.
    fn pausing_race(&self) -> Option<&RaceStage> {
        let race = match &self.stage {
            Stage::Race(stage) if !stage.race.finished() => Some(&**stage),
            _ => self.suspended_race.as_deref(),
        };
        race.filter(|stage| self.pause_screen_applies(stage))
    }

    /// Puts the pause screen at the root of `definition` when a race that pauses into one is
    /// held. Returns whether it did; nothing changes otherwise, and when the disc's words for
    /// the rows cannot be found (no CONTINUE row), which falls back to the main page.
    pub(crate) fn add_pause_screen(&self, definition: &mut Definition) -> bool {
        let Some(shell) = self.shell.as_ref() else {
            return false;
        };
        let (Some(pause), Some(stage)) = (shell.title.pause_menu, self.pausing_race()) else {
            return false;
        };
        let mode = stage.race.sim.world.mode().name();
        let mut trial = definition.clone();
        let built = trial.open_on_pause_screen(pause, mode, &shell.strings);
        let resumes = trial
            .page(oag_ui::menu::PAUSE_PAGE)
            .is_some_and(|page| page.row_with_id("pause_resume").is_some());
        if !resumes {
            warn!(
                "{}: the disc's words for {:?} did not resolve, so the main page is the pause \
                 menu",
                built.screen, built.unworded
            );
            return false;
        }
        debug!(
            "pause screen {:?} for {mode}: not built {:?}, unworded {:?}",
            built.screen, built.unbuilt, built.unworded
        );
        *definition = trial;
        true
    }

    /// RESTART RACE: the parked race again, on the options it ran with, the way the end of a
    /// race restarts it (`MenuOption::RaceAgain`).
    pub(crate) fn restart_parked_race(&mut self) {
        let Some(stage) = self.suspended_race.as_ref() else {
            return;
        };
        (
            self.campaign_cell,
            self.campaign_difficulty,
            self.campaign_ai_skill_scale,
            self.campaign_custom,
        ) = (
            stage.campaign_cell.clone(),
            stage.campaign_difficulty,
            stage.campaign_ai_skill_scale,
            stage.campaign_custom.clone(),
        );
        info!("restarting the race from the pause menu");
        self.finish_launch();
    }

    /// QUIT RACE: the parked race is dropped. A campaign race lands on its cell, any other on
    /// the front end's main page (`Session::quit_campaign_race`).
    pub(crate) fn quit_parked_race(&mut self) {
        let Some(parked) = self.suspended_race.take() else {
            return;
        };
        let exit = super::escape::race_exit(parked.campaign_cell.as_ref(), parked.campaign_difficulty);
        crate::race_build::drop_off_thread(parked);
        match exit {
            super::escape::RaceExit::CellSelection { cell, difficulty } => {
                self.quit_campaign_race(&cell, difficulty);
            }
            super::escape::RaceExit::Menus => {
                info!("leaving the race for the menus");
                self.leave_finished_race();
            }
        }
    }
}
