//! The title's own pause screen over a parked race: when it applies, what it is built from,
//! and what its RESTART RACE and QUIT RACE rows do.
//!
//! The rows are the disc's (`oag_title::PauseMenu`), put at the root of the menu tree by
//! `oag_ui::menu::Definition::open_on_pause_screen`. A title that authors none, a split-screen
//! race and a network race keep what they did: the front end's main page over the parked race.

use log::{debug, info, warn};

use oag_ui::menu::Definition;

use crate::app::race_stage::RaceStage;
use crate::app::stage::Stage;

use super::Session;

/// The key a race's pause screen is chosen by: its mode's name, or `free_play` for a Free Play
/// event (Pure's, which this build races as a single race, `docs/formats/pure-campaign.md`).
fn pause_mode(stage: &RaceStage) -> &str {
    match &stage.campaign_cell {
        Some(cell) if oag_tables::pure_campaign::is_free_play(&cell.name) => "free_play",
        _ => stage.race.core.sim.world.mode().name(),
    }
}

impl Session {
    /// Whether the race in `stage` pauses into the title's own pause screen: the title authors
    /// one, and the race has one view and is not a network race.
    pub(crate) fn pause_screen_applies(&self, stage: &RaceStage) -> bool {
        self.net.is_none()
            && stage.race.view_state().viewport_count() <= 1
            && !stage.race.windowed()
            && self
                .shell
                .as_ref()
                .is_some_and(|shell| shell.title.pause_menu.is_some())
    }

    /// The race the pause screen is for: the live one, or the one parked behind the menus.
    fn pausing_race(&self) -> Option<&RaceStage> {
        let race = match &self.stage {
            // A demo is not paused into: leaving it is the main menu. Nor is a recording being
            // watched: the viewer's exit lands on REPLAYS and nothing is parked to resume or quit.
            // Nor is a benchmark: leaving it ends the run, and the menus opened over it are the
            // main menu's.
            Stage::Race(stage)
                if !stage.race.core.sim.finished()
                    && stage.demo.is_none()
                    && stage.benchmark.is_none()
                    && !stage.watching =>
            {
                Some(&**stage)
            }
            _ => self.suspended_race.as_deref(),
        };
        race.filter(|stage| self.pause_screen_applies(stage))
    }

    /// The disc's own layout of the pause screen for the race held, when the title carries it.
    pub(crate) fn pause_view(&self) -> Option<crate::app::menu_stage::PauseView> {
        let shell = self.shell.as_ref()?;
        let table = shell.title.pause_menu?;
        let stage = self.pausing_race()?;
        let authored = table.screen_for(pause_mode(stage));
        let find = |name: &str| {
            shell
                .pause_screens
                .screens
                .iter()
                .find(|screen| screen.name() == name)
        };
        let screen = find(authored.name)?.clone();
        let opened: Vec<_> = authored
            .rows
            .iter()
            .filter_map(|row| row.opens)
            .filter_map(|name| Some(find(name)?.clone()))
            .collect();
        let sprites = std::iter::once(&screen)
            .chain(&opened)
            .flat_map(oag_ui_screens::pause::PauseScreen::sprite_srcs)
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .filter_map(|src| Some((src.to_string(), shell.sprites.get(src)?)))
            .collect();
        Some(crate::app::menu_stage::PauseView {
            screen,
            opened,
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
        // This project's PHOTO MODE row is not on the disc's screen: a compare shot leaves it out.
        let authored = oag_title::PauseMenu {
            offers_photo: pause.offers_photo && !self.original_menu,
            ..*pause
        };
        let built = if self.original_menu {
            trial.open_on_pause_screen_as_authored(&authored, mode, strings)
        } else {
            trial.open_on_pause_screen(&authored, mode, strings)
        };
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

    /// The pause screen's music list over the race playlist, on the song playing, where the title
    /// authors one (`docs/ui/pause-menu.md`, "Pulse").
    pub(crate) fn pause_music_list(
        &self,
        shell: &super::Shell,
    ) -> Option<oag_ui::menu::music_list::MusicList> {
        let spec = shell.title.pause_menu?.music_list_for(shell.platform)?;
        let playlist = self.audio.race_playlist(&self.music_discs)?;
        Some(oag_ui::menu::music_list::MusicList::new(
            playlist.titles,
            playlist.playing,
            spec.wraps,
        ))
    }

    /// The song the pause screen's music list was left on when the race resumes: the race music
    /// starts it from its beginning (`docs/ui/pause-menu.md`, "Pulse").
    pub(crate) fn take_pause_music_pick(&mut self) {
        let pick = match &self.stage {
            Stage::Menu(stage) => stage
                .menu
                .music_list()
                .and_then(oag_ui::menu::music_list::MusicList::pick),
            _ => None,
        };
        if let Some(index) = pick
            && self.audio.pick_race_song(index)
        {
            debug!("pause music list: the race resumes on playlist song {index}");
        }
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
        if parked.race.core.sim.pilot_assist_level() == level {
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
        let exit =
            super::escape::race_exit(parked.campaign_cell.as_ref(), parked.campaign_difficulty);
        crate::app::race_build::drop_off_thread(parked);
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
