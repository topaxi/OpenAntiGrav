//! Builds and drives the EndRace flow a finished race holds open - the same
//! split `crate::main::session::campaign` makes for the Race Campaign's own
//! screens. See `crate::main::race_stage::endrace` for the runtime this
//! module builds and drives, and `oag_ui_screens::endrace` for the model.

use log::warn;

use oag_ui_screens::endrace::{Event, MenuOption};

use crate::race_stage::endrace::{
    EndRaceRuntime, LoyaltyInputs, ResultsModel, elimination_results, hd_field_rows, headline,
    loyalty_award, menu_options, to_campaign_medal, tournament_results, zone_results,
};
use crate::race_stage::endrace_touch::EndRace;
use crate::race_stage::hd_loyalty::{HdRace, hd_award};
use crate::stage::Stage;

use super::Session;

#[path = "endrace_touch.rs"]
mod touch;

impl Session {
    /// Attempts to build the EndRace flow for the race that just finished in
    /// `Stage::Race`. A no-op once `stage.endrace` is already built, and on
    /// a source with no `EndRace_Definition.xml` or whose read failed -
    /// `RaceStage::draw_hud` falls back to `scoreboard::Overlay` exactly as
    /// before this existed. See `RaceStage::endrace`'s own doc.
    ///
    /// **Both halves of "no-op" are carried by a flag of their own**, because
    /// `Session::frame` calls this every frame a finished race sits on
    /// screen: `stage.endrace` for the built case and
    /// `RaceStage::endrace_unavailable` for the failed one. Everything past
    /// the early guards - `dlc::packs_from_defaults`, `open_source`, reading
    /// the screens - is expensive enough that retrying it per frame is a
    /// performance bug rather than a log one.
    pub(crate) fn build_endrace(&mut self) {
        let Stage::Race(stage) = &self.stage else {
            return;
        };
        if !stage.race.finished() || stage.endrace.is_some() || stage.endrace_unavailable {
            return;
        }
        let Some(shell) = self.shell.as_ref() else {
            return;
        };
        let Some(race_options) = self.race_options.as_ref() else {
            return;
        };
        // Past the guards that make this run once per finished race, so the
        // line is the race ending, not a per-frame retry.
        log::info!("race finished");
        if oag_game::endrace::dialect(shell.title) == Some(oag_title::EndRaceDialect::Touch) {
            self.build_endrace_touch();
            return;
        }

        let mode = race_options.mode;
        let source = race_options.source.clone();
        let dlc = race_options.dlc.clone();
        let team = race_options.team.clone();
        let campaign = stage.campaign_cell.is_some();
        let title = stage.result_key.title.clone();
        // The actual title package, not the display name above - what
        // `oag_game::endrace::load`'s own title dispatch needs, and what
        // this function's own dispatch (Pulse's per-lap `Results` versus
        // Wipeout HD/Fury's whole-field `FieldResults`) needs too. Read out
        // of `shell` before the mutable borrows below, the same "clone now,
        // `shell` cannot survive past this point" idiom
        // `Session::open_campaign`'s own `title_ref` already uses.
        let title_ref = shell.title;
        let board = stage.race.results().cloned();
        // The roster this race drew, slot 0 the player's - read off the race
        // rather than rebuilt, because the draw is seeded and only the race
        // knows its seed (`race::load::roster`).
        let roster = stage.race.slot_teams().to_vec();
        let observation = stage.observation();
        let standing = &stage.race.sim.world.ships[0].standing;
        let boosts = stage.race.run_stats().boosts_by_lap;
        let laps: Vec<oag_ui_screens::endrace::LapSplit> = (0..oag_race::MAX_RECORDED_LAPS)
            .filter_map(|index| {
                standing.lap_splits[index].map(|ticks| oag_ui_screens::endrace::LapSplit {
                    lap: (index + 1) as u32,
                    ticks,
                    boosts: Some(boosts[index]),
                })
            })
            .collect();
        let new_best_lap_ticks = standing.best_lap_ticks;
        // `Race End Photo` follows a finish by the line (measured) and a Single
        // Race wreck (the maintainer's "hold, then results" of 2026-10-02: the
        // original never leaves `InGame` after one, so that the legend shows at
        // all is chosen). Both are the endings whose world keeps running; an
        // Eliminator or Zone ending was not looked at and keeps its panels at
        // once (`docs/gameplay/after-the-finish.md`).
        let runs_on_after_the_end = stage.race.runs_on_after_the_end();

        // `Race_ComputeLoyaltyAward`'s own inputs - see `LoyaltyInputs`'s own
        // doc for why `perfect_laps`/`perfect_zones`/`suggested_ship` are
        // always `0`/`false` and `difficulty` always `None` in this build.
        let award = loyalty_award(LoyaltyInputs {
            mode,
            laps: observation.laps_completed,
            perfect_laps: 0,
            kills: standing.kills,
            zones: u32::from(stage.race.sim.world.primary_race().zone),
            perfect_zones: 0,
            difficulty: None,
            suggested_ship: false,
        });

        let faces = oag_ui_screens::picker::FaceScales {
            default: shell.menu_font.as_ref().map_or(
                oag_ui_screens::picker::FaceScales::default().default,
                |menu| shell.font.line_height / menu.line_height,
            ),
            ..oag_ui_screens::picker::FaceScales::default()
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
            oag_source::dlc::packs_from_defaults(&dlc, &oag_source::cache::default_dlc_cache_dir());
        for problem in problems {
            warn!("{problem}");
        }
        let mut archives = match oag_source::title::open_source(&source, packs, pure_packs) {
            Ok(opened) => opened.archives,
            Err(error) => {
                warn!("cannot open {source} for the EndRace screens: {error:#}");
                self.mark_endrace_unavailable();
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
            title_ref,
        ) {
            Ok(screens) => screens,
            Err(error) => {
                // Not "this source has no EndRace screens": the entry name
                // asked for is Pulse's own, and a title that authors its
                // own elsewhere reaches here too. Wipeout HD ships
                // `/data/plugins/frontend/gui/endrace_definition.xml` in
                // five of its seven archives, in a widget vocabulary this
                // build does not read yet - see
                // `oag_game::endrace::SCREEN_ENTRY`'s own doc.
                warn!(
                    "{error:#} - this title's EndRace screens are not read by this build; \
                     keeping the built-in results table"
                );
                self.mark_endrace_unavailable();
                return;
            }
        };

        let is_hd = oag_game::endrace::dialect(title_ref) == Some(oag_title::EndRaceDialect::Field);

        // HD shows its loyalty on `Results` itself. The law is HD's own
        // (`oag_hd::loyalty`), not the Pulse `award` above, which only the
        // Pulse `Rewards` row below reads. `None` when this launch named no
        // team, which draws the block absent rather than inventing one.
        let hd_loyalty = if is_hd {
            team.clone().map(|team| {
                let award = hd_award(HdRace {
                    mode,
                    laps: observation.laps_completed,
                    kills: standing.kills,
                    zones: u32::from(stage.race.sim.world.primary_race().zone),
                    difficulty: stage.campaign_difficulty,
                });
                let total = self.records.record_loyalty(&title, &team, award);
                if let Err(e) = oag_game::records::save(&self.records) {
                    warn!("could not save the loyalty total: {e:#}");
                }
                oag_ui_screens::endrace::HdLoyalty { award, total }
            })
        } else {
            None
        };

        // HD's original never enters its own `EndRace Rewards` (no redirect
        // names it and the executable registers no screen class for it -
        // `docs/formats/hd-endrace-screens.md`), so this flow does not
        // either: no `Rewards` model means `EndRaceRuntime::advance` goes
        // Results -> Menu, the route HD's own `EndRaceMenuRedirect` authors.
        // Its loyalty is `hd_loyalty` above, on `Results`, not a Rewards row.
        let rewards = if is_hd {
            None
        } else {
            // The loyalty row - `None` when this launch named no team at all
            // (a `--race` run with no `--team`), which draws the row absent
            // rather than a blank name. A real launch through `Team
            // Selection` always names one.
            let loyalty = team.clone().map(|team| {
                let total = self.records.record_loyalty(&title, &team, award);
                if let Err(e) = oag_game::records::save(&self.records) {
                    warn!("could not save the loyalty total: {e:#}");
                }
                oag_ui_screens::endrace::Loyalty {
                    team_name: team,
                    award,
                    total,
                }
            });
            Some(oag_ui_screens::endrace::Rewards {
                medal: observation.campaign_medal.map(to_campaign_medal),
                campaign,
                loyalty,
            })
        };

        // A Tournament leg's own standings, off `self.tournament` (already
        // folded with this leg's own points -
        // `Session::record_finished_leg` runs before `build_endrace` in the
        // same frame, see `tournament_results`'s own doc) and this leg's
        // grid roster, as the race itself drew it. `None` (no team named at
        // all) draws every row's own name absent rather than invented.
        // The ids are folder names; the draw resolves each to its display
        // name through the string table, as the original's own `localise` does.
        let slot_teams = (!is_hd
            && matches!(
                mode,
                oag_race::Mode::Tournament | oag_race::Mode::Eliminator
            )
            && team.is_some())
        .then_some(roster);
        let pulse_tournament = (!is_hd && mode == oag_race::Mode::Tournament)
            .then_some(self.tournament.as_ref())
            .flatten()
            .and_then(|progress| {
                tournament_results(
                    board.as_ref(),
                    progress,
                    slot_teams.as_deref(),
                    progress.is_last_leg(),
                )
            });

        let results = if let Some(tournament) = pulse_tournament {
            ResultsModel::PulseTournament(tournament)
        } else if !is_hd && mode == oag_race::Mode::Eliminator {
            let world = &stage.race.sim.world;
            ResultsModel::PulseElimination(elimination_results(
                &world.ships[..usize::from(world.ship_count)],
                slot_teams.as_deref(),
            ))
        } else if !is_hd && mode == oag_race::Mode::Zone {
            ResultsModel::PulseZone(zone_results(
                stage.race.sim.world.primary_race(),
                stage.race.run_stats(),
            ))
        } else if is_hd {
            ResultsModel::Hd(oag_ui_screens::endrace::FieldResults {
                headline: headline(mode, observation.place),
                rows: hd_field_rows(board.as_ref()),
                loyalty: hd_loyalty,
            })
        } else {
            ResultsModel::Pulse(oag_ui_screens::endrace::Results {
                headline: headline(mode, observation.place),
                laps,
                total_ticks: observation.tick,
            })
        };
        let menu = oag_ui_screens::endrace::EndRaceMenu::new(
            menu_options(campaign, self.tournament_has_next_leg()),
            new_best_lap_ticks,
        );

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
            runs_on_after_the_end,
            self.anisotropy,
        ) {
            Ok(runtime) => {
                if let Stage::Race(stage) = &mut self.stage {
                    stage.endrace = Some(EndRace::Disc(Box::new(runtime)));
                }
            }
            Err(error) => {
                warn!("cannot build the EndRace screens' own renderer: {error:#}");
                self.mark_endrace_unavailable();
            }
        }
    }

    /// Records that this race's EndRace flow could not be built, so that
    /// [`Self::build_endrace`] stops at its own guard from the next frame
    /// on. See `RaceStage::endrace_unavailable`.
    pub(super) fn mark_endrace_unavailable(&mut self) {
        if let Stage::Race(stage) = &mut self.stage {
            stage.endrace_unavailable = true;
        }
    }

    /// One tick of an open EndRace flow: pad and pointer input, and what
    /// came of either. A no-op unless `Stage::Race`'s own `endrace` is
    /// built.
    pub(crate) fn tick_endrace(&mut self, pointer: &oag_ui::pointer::Pointer) {
        let mut confirmed_option = None;
        let mut navs = Vec::new();
        {
            let Stage::Race(stage) = &mut self.stage else {
                return;
            };
            let Some(endrace) = stage.endrace.as_mut() else {
                return;
            };
            let EndRace::Disc(endrace) = endrace else {
                // 2048's pages answer a pointer and pad of their own - see
                // `Session::tick_endrace_touch`.
                self.tick_endrace_touch(pointer);
                return;
            };
            endrace.tick_flow();
            endrace.tick_tournament_table();
            if endrace.is_menu() {
                let mut events = endrace.menu_mut().update(self.controls.buttons_mut());
                // No pointer-target list here: `EndRace Menu`'s own row
                // rects need the layout `endrace` already owns, which
                // `EndRaceRuntime::menu_targets` reads directly, dispatched
                // to the right title's own row geometry - see that method's
                // own doc for why `Results`/`Rewards` need no target list of
                // their own.
                let targets = endrace.menu_targets();
                events.extend(endrace.menu_mut().pointer(pointer, &targets));
                for event in events {
                    navs.push(event.nav());
                    match event {
                        Event::Confirmed => confirmed_option = endrace.menu().selected(),
                        Event::Moved | Event::Back => {}
                    }
                }
                // After this tick's move, so a newly focused option starts
                // easing and blinking on the same tick it is picked - the
                // order `Block_Update` sees focus in. Wipeout HD/Fury's
                // option blocks draw it; Pulse's list ignores it.
                endrace.menu_mut().tick();
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
                // Read either way, so a press made before `Race End Photo`
                // is entered is spent there rather than carried into the
                // panels as an edge of its own.
                if confirmed && endrace.takes_confirm() {
                    endrace.advance();
                    navs.push(oag_ui::menu::nav::Nav::Accept);
                }
                // `RaceManager_Update`'s d-pad on `Race End Photo`, once it is entered: the
                // spectator camera's mode and the craft it watches.
                if endrace.is_photo() && endrace.takes_confirm() {
                    use oag_gameplay::input::Button;
                    for button in [Button::Up, Button::Down, Button::Left, Button::Right] {
                        if self.controls.buttons_mut().take(button) {
                            stage.race.spectator_press(button);
                        }
                    }
                }
            }
        }
        self.play_navs(navs);
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
                    && let Some(EndRace::Disc(endrace)) = stage.endrace.as_mut()
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
                (self.campaign_cell, self.campaign_difficulty) = match &self.stage {
                    Stage::Race(stage) => (stage.campaign_cell.clone(), stage.campaign_difficulty),
                    _ => (None, None),
                };
                self.finish_launch();
            }
            MenuOption::ReturnToGrid => self.return_to_campaign(),
            MenuOption::ReturnToMenu => self.leave_finished_race(),
            MenuOption::NextRace => self.advance_tournament_leg(),
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
        self.reopen_cell_selection(&cell.name, None);
    }

    /// Opens the campaign and lands `Cell Selection` on the cell named
    /// `cell`, on `difficulty` where one is given - `RETURN TO GRID`'s
    /// half, and Wipeout HD/Fury's `Team Selection` Back
    /// (`Session::handle_picker`), whose `TeamRedirectBack` authors no
    /// `goto` and so returns to the screen it came from.
    pub(crate) fn reopen_cell_selection(
        &mut self,
        cell: &str,
        difficulty: Option<oag_tables::race_campaign::Difficulty>,
    ) {
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
            .position(|grid| grid.cells.iter().any(|c| c.name == cell))
        else {
            warn!(
                "{cell} is not on any of this source's own grids any more - staying on Grid Selection"
            );
            return;
        };
        if campaign.open_cell_selection(which)
            && let crate::campaign_stage::Screen::Cell { model, .. } = &mut campaign.screen
        {
            model.select_by_name(cell);
            if let Some(difficulty) = difficulty {
                model.set_difficulty(difficulty);
            }
        }
    }

    /// The shared half of `RETURN TO GRID`/`RETURN TO MENU`: hands the
    /// window back to the menus, the same way `Session::escape` does for a
    /// live race - see that function's own doc on why a *finished* race is
    /// discarded rather than parked.
    pub(super) fn leave_finished_race(&mut self) {
        // Whether the tournament just finished its last leg or is being
        // abandoned early, there is nothing to carry past this point - see
        // `Session::tournament`'s own doc for why this engine parks no
        // save/resume state the way the original's `Tournament_SaveProgress`
        // does.
        self.abandon_tournament();
        self.audio.pause_race_music();
        self.audio.stop_race_sfx();
        if let Err(error) = self.open_menus() {
            log::error!("cannot return to the menus: {error:#}");
            self.quit = true;
        }
    }
}
