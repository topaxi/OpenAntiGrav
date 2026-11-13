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

/// The key a race's pause screen is chosen by: its mode's name, or `free_play` for a Free Play
/// event (Pure's, which this build races as a single race, `docs/formats/pure-campaign.md`).
fn pause_mode(stage: &RaceStage) -> &str {
    match &stage.campaign_cell {
        Some(cell) if oag_tables::pure_campaign::is_free_play(&cell.name) => "free_play",
        _ => stage.race.sim.world.mode().name(),
    }
}

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
            // A demo is not paused into: leaving it is the main menu.
            Stage::Race(stage) if !stage.race.finished() && stage.demo.is_none() => Some(&**stage),
            _ => self.suspended_race.as_deref(),
        };
        race.filter(|stage| self.pause_screen_applies(stage))
    }

    /// The disc's own layout of the pause screen for the race held, when the title carries it.
    pub(crate) fn pause_view(&self) -> Option<crate::menu_stage::PauseView> {
        let shell = self.shell.as_ref()?;
        let table = shell.title.pause_menu?;
        let stage = self.pausing_race()?;
        let name = table.screen_for(pause_mode(stage)).name;
        let screen = shell
            .pause_screens
            .screens
            .iter()
            .find(|screen| screen.name() == name)?
            .clone();
        let sprites = screen
            .sprite_srcs()
            .filter_map(|src| Some((src.to_string(), shell.sprites.get(src)?)))
            .collect();
        Some(crate::menu_stage::PauseView {
            screen,
            table,
            sprites,
        })
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
        let mode = pause_mode(stage);
        let mut trial = definition.clone();
        let strings = if pause.layout.is_some() {
            &shell.pause_screens.strings
        } else {
            &shell.strings
        };
        let built = trial.open_on_pause_screen(pause, mode, strings);
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

    /// A setting the pause screen's own lists changed (PILOT ASSIST, GHOST) reaches the parked
    /// race now rather than at the next one. Ghost visibility is set on it at once and read
    /// again every frame from the settings; Pilot Assist is the level the race flies with, and a mid-race change is
    /// an event in the recording, so a replay or ghost lap flies the laws it was recorded under.
    pub(crate) fn apply_pause_setting(&mut self, setting: &str) {
        let on_pause_page = matches!(
            &self.stage,
            Stage::Menu(stage) if stage.menu.page().id == oag_ui::menu::PAUSE_PAGE
        );
        if !on_pause_page {
            return;
        }
        if setting == "display.ghost_ship_visible" {
            if let (Some(shell), Some(parked)) = (&self.shell, self.suspended_race.as_mut()) {
                parked
                    .race
                    .set_ghost_visible(self.settings.display.ghost_ship_shown(shell.title));
            }
            return;
        }
        if setting != "controls.pilot_assist" {
            return;
        }
        let level = self.settings.controls.pilot_assist;
        let Some(parked) = self.suspended_race.as_mut() else {
            return;
        };
        if parked.race.pilot_assist_level() == level {
            return;
        }
        // Applied and recorded together: the recording keeps going and the change is an
        // event between the same two ticks (`Race::change_pilot_assist`).
        parked.race.change_pilot_assist(level);
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
