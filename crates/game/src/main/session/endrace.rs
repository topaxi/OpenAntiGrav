//! Builds and drives the EndRace flow a finished race holds open - the same
//! split `crate::main::session::campaign` makes for the Race Campaign's own
//! screens. See `crate::main::race_stage::endrace` for the runtime this
//! module builds and drives, and `oag_ui::endrace` for the model.

use log::warn;

use oag_ui::endrace::{Event, MenuOption};

use crate::race_stage::endrace::{
    EndRaceRuntime, LoyaltyInputs, headline, loyalty_award, menu_options, to_campaign_medal,
};
use crate::stage::Stage;

use super::Session;

impl Session {
    /// Attempts to build the EndRace flow for the race that just finished in
    /// `Stage::Race`. A no-op once `stage.endrace` is already built, and on
    /// a source with no `EndRace_Definition.xml` or whose read failed -
    /// `RaceStage::draw_hud` falls back to `scoreboard::Overlay` exactly as
    /// before this existed. See `RaceStage::endrace`'s own doc.
    pub(crate) fn build_endrace(&mut self) {
        let Stage::Race(stage) = &self.stage else {
            return;
        };
        if !stage.race.finished() || stage.endrace.is_some() {
            return;
        }
        let Some(shell) = self.shell.as_ref() else {
            return;
        };
        let Some(race_options) = self.race_options.as_ref() else {
            return;
        };

        let mode = race_options.mode;
        let source = race_options.source.clone();
        let dlc = race_options.dlc.clone();
        let team = race_options.team.clone();
        let campaign = stage.campaign_cell.is_some();
        let title = stage.result_key.title.clone();
        let observation = stage.observation();
        let standing = &stage.race.sim.world.ships[0].standing;
        let laps: Vec<oag_ui::endrace::LapSplit> = (0..oag_race::MAX_RECORDED_LAPS)
            .filter_map(|index| {
                standing.lap_splits[index].map(|ticks| oag_ui::endrace::LapSplit {
                    lap: (index + 1) as u32,
                    ticks,
                })
            })
            .collect();
        let new_best_lap_ticks = standing.best_lap_ticks;

        // `Race_ComputeLoyaltyAward`'s own inputs - see `LoyaltyInputs`'s own
        // doc for why `perfect_laps`/`perfect_zones`/`suggested_ship` are
        // always `0`/`false` and `difficulty` always `None` in this build.
        let award = loyalty_award(LoyaltyInputs {
            mode,
            laps: observation.laps_completed,
            perfect_laps: 0,
            kills: standing.kills,
            zones: u32::from(stage.race.sim.world.race.zone),
            perfect_zones: 0,
            difficulty: None,
            suggested_ship: false,
        });

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
        let globals: Vec<(String, String)> = shell
            .globals
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect();
        let skin_line_height = shell.menu_font.as_ref().unwrap_or(&shell.font).line_height;
        let skin = oag_ui::menu::Skin::new(shell.menu_skin, shell.space, skin_line_height);
        let frame = shell.frame.clone();
        let atlas = shell
            .menu_font
            .clone()
            .unwrap_or_else(|| shell.font.clone());
        let base_sprites = shell.sprites.clone();
        let format = self.gpu.config.format;

        let (packs, pure_packs, problems) =
            oag_game::dlc::packs_from_defaults(&dlc, &oag_game::boot::default_dlc_cache_dir());
        for problem in problems {
            warn!("{problem}");
        }
        let mut archives = match oag_game::title::open_source(&source, packs, pure_packs) {
            Ok(opened) => opened.archives,
            Err(error) => {
                warn!("cannot open {source} for the EndRace screens: {error:#}");
                return;
            }
        };
        let global_refs: Vec<(&str, &str)> = globals
            .iter()
            .map(|(key, value)| (key.as_str(), value.as_str()))
            .collect();
        let screens = match oag_game::endrace::load(
            &mut archives,
            &strings,
            faces,
            grid,
            &base_sprites,
            &global_refs,
        ) {
            Ok(screens) => screens,
            Err(error) => {
                warn!(
                    "{error:#} - this source has no EndRace screens to show; keeping the \
                     built-in results table"
                );
                return;
            }
        };

        let results = oag_ui::endrace::Results {
            headline: headline(mode, observation.place),
            laps,
            total_ticks: observation.tick,
        };
        // The loyalty row - `None` when this launch named no team at all
        // (a `--race` run with no `--team`), which draws the row absent
        // rather than a blank name. A real launch through `Team Selection`
        // always names one.
        let loyalty = team.map(|team| {
            let total = self.records.record_loyalty(&title, &team, award);
            if let Err(e) = oag_game::records::save(&self.records) {
                warn!("could not save the loyalty total: {e:#}");
            }
            oag_ui::endrace::Loyalty {
                team_name: team,
                award,
                total,
            }
        });
        let rewards = oag_ui::endrace::Rewards {
            medal: observation.campaign_medal.map(to_campaign_medal),
            campaign,
            loyalty,
        };
        let menu = oag_ui::endrace::EndRaceMenu::new(menu_options(campaign), new_best_lap_ticks);

        match EndRaceRuntime::new(
            &self.gpu.device,
            &self.gpu.queue,
            format,
            screens,
            skin,
            frame,
            strings,
            atlas,
            results,
            rewards,
            menu,
        ) {
            Ok(runtime) => {
                if let Stage::Race(stage) = &mut self.stage {
                    stage.endrace = Some(runtime);
                }
            }
            Err(error) => warn!("cannot build the EndRace screens' own renderer: {error:#}"),
        }
    }

    /// One tick of an open EndRace flow: pad and pointer input, and what
    /// came of either. A no-op unless `Stage::Race`'s own `endrace` is
    /// built.
    pub(crate) fn tick_endrace(&mut self, pointer: &oag_ui::pointer::Pointer) {
        let mut confirmed_option = None;
        {
            let Stage::Race(stage) = &mut self.stage else {
                return;
            };
            let Some(endrace) = stage.endrace.as_mut() else {
                return;
            };
            if endrace.is_menu() {
                let mut events = endrace.menu_mut().update(self.controls.buttons_mut());
                // No pointer-target list here: `EndRace Menu`'s own row
                // rects need the layout `endrace` already owns, which
                // `oag_ui::endrace::pointer::menu_targets` reads directly -
                // see that module's own doc for why `Results`/`Rewards`
                // need no target list of their own.
                let targets =
                    oag_ui::endrace::pointer::menu_targets(endrace.menu(), endrace.menu_layout());
                events.extend(endrace.menu_mut().pointer(pointer, &targets));
                for event in events {
                    match event {
                        Event::Confirmed => confirmed_option = endrace.menu().selected(),
                        Event::Moved | Event::Back => {}
                    }
                }
            } else {
                // `Results`/`Rewards` answer only a confirm - cross, start
                // or a click anywhere, the same `ContinueButton` press
                // either screen's own XML authors and nothing else.
                let confirmed = self
                    .controls
                    .buttons_mut()
                    .take(oag_gameplay::input::Button::Cross)
                    || self
                        .controls
                        .buttons_mut()
                        .take(oag_gameplay::input::Button::Start)
                    || pointer.clicked;
                if confirmed {
                    endrace.advance();
                }
            }
        }
        if let Some(option) = confirmed_option {
            self.handle_endrace_menu_option(option);
        }
    }

    /// What confirming a row on `EndRace Menu` does - the disc's own
    /// `<Redirect>` table (`docs/formats/endrace-screens.md`), reproduced
    /// through this project's existing campaign/relaunch machinery rather
    /// than a parallel path of its own.
    fn handle_endrace_menu_option(&mut self, option: MenuOption) {
        match option {
            MenuOption::ViewResultsAgain => {
                if let Stage::Race(stage) = &mut self.stage
                    && let Some(endrace) = stage.endrace.as_mut()
                {
                    endrace.view_results_again();
                }
            }
            MenuOption::RaceAgain => {
                // The same cell has to be re-armed before `finish_launch`
                // relaunches: `Session::launch_campaign_cell`'s own drain
                // (`crate::main::session::load::finish_loading`) already
                // cleared `self.campaign_cell` into the *first* `RaceStage`,
                // so a second launch with nothing set here would lose the
                // medal evaluation and `EndRace Rewards`' own `campaign`
                // flag on the replay.
                self.campaign_cell = match &self.stage {
                    Stage::Race(stage) => stage.campaign_cell.clone(),
                    _ => None,
                };
                self.finish_launch();
            }
            MenuOption::ReturnToGrid => self.return_to_campaign(),
            MenuOption::ReturnToMenu => self.leave_finished_race(),
            // Not offered by `crate::race_stage::endrace::menu_options` -
            // this engine implements no Tournament mode.
            MenuOption::NextRace => {}
        }
    }

    /// `RETURN TO GRID`: leaves the race and reopens `Cell Selection` on the
    /// same cell - the flow `Session::launch_campaign_cell` already opened
    /// it from, run in reverse, rather than a parallel "go back to this
    /// cell" mechanism of its own.
    fn return_to_campaign(&mut self) {
        let cell = match &self.stage {
            Stage::Race(stage) => stage.campaign_cell.clone(),
            _ => None,
        };
        self.leave_finished_race();
        let Some(cell) = cell else {
            return;
        };
        self.open_campaign();
        let Stage::Menu(menu_stage) = &mut self.stage else {
            return;
        };
        let Some(campaign) = menu_stage.campaign.as_mut() else {
            return;
        };
        let Some(which) = campaign
            .grids()
            .iter()
            .position(|grid| grid.cells.iter().any(|c| c.name == cell.name))
        else {
            warn!(
                "{} is not on any of this source's own grids any more - staying on Grid Selection",
                cell.name
            );
            return;
        };
        if campaign.open_cell_selection(which)
            && let crate::campaign_stage::Screen::Cell { model, .. } = &mut campaign.screen
        {
            model.select_by_name(&cell.name);
        }
    }

    /// The shared half of `RETURN TO GRID`/`RETURN TO MENU`: hands the
    /// window back to the menus, the same way `Session::escape` does for a
    /// live race - see that function's own doc on why a *finished* race is
    /// discarded rather than parked.
    fn leave_finished_race(&mut self) {
        self.audio.pause_race_music();
        self.audio.stop_race_sfx();
        if let Err(error) = self.open_menus() {
            log::error!("cannot return to the menus: {error:#}");
            self.quit = true;
        }
    }
}
