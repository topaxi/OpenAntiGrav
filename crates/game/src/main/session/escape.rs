//! What escape does, and the other half of leaving a race: parking it in
//! [`Session::suspended_race`] and swapping it back in later.
//!
//! Split out of `session/menus.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py` - a move, with no behaviour change. This is
//! also the seam `oag_game::records`'s own module doc points at as "the
//! other capture site": [`Session::escape`] is where a race that never calls
//! [`oag_raceplay::Race::finished`] - Speed Lap and Zone, since
//! [`oag_race::Mode::laps_target`] is `None` for both - still gets its
//! result recorded, on the way out rather than on a finish transition that
//! for those two modes never comes. See
//! `docs/architecture/adr/0049-race-records-are-a-chosen-schema-captured-outside-the-tick.md`.

use log::{error, info};

use oag_game::records;
use oag_ui::strings;

use crate::hints;
use crate::stage::Stage;

use super::Session;

#[cfg(test)]
#[path = "escape/tests.rs"]
mod tests;

/// Where leaving a live race lands.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum RaceExit {
    /// The menus, with the race parked behind them.
    Menus,
    /// `Cell Selection` on the cell the race was launched from.
    CellSelection {
        cell: String,
        difficulty: Option<oag_tables::race_campaign::Difficulty>,
    },
}

/// A race launched from a campaign cell, in any mode, leaves to that cell's
/// `Cell Selection`; every other race leaves to the menus. See
/// [`Session::quit_campaign_race`].
pub(crate) fn race_exit(
    cell: Option<&oag_tables::race_campaign::Cell>,
    difficulty: Option<oag_tables::race_campaign::Difficulty>,
) -> RaceExit {
    match cell {
        Some(cell) => RaceExit::CellSelection {
            cell: cell.name.clone(),
            difficulty,
        },
        None => RaceExit::Menus,
    }
}

impl Session {
    /// Swaps the menus for the race `escape` parked over them - the other half
    /// of [`Session::open_menus`] parking one. Reached only from
    /// [`Session::handle_menu`], on [`oag_ui::menu::MenuEvent::Closed`] while
    /// [`Session::suspended_race`] holds something.
    ///
    /// A no-op if nothing is parked, which cannot happen through
    /// `handle_menu`'s own guard but is cheap to make true unconditionally
    /// rather than only where it is currently checked.
    pub(crate) fn resume_race(&mut self) {
        let Some(mut stage) = self.suspended_race.take() else {
            return;
        };
        // **Cleared, not left as `escape` set it.** `result_saved` means
        // "saved since this stage last became live", and parking a race does
        // not end its life the way finishing or truly leaving it does - the
        // player is about to keep racing under the same key. Left set, a
        // Speed Lap improved after a resume would never be recorded (the
        // `escape` guard would see it as already saved), and a Time Trial
        // that is escaped mid-race and then resumed to a real finish would
        // have that finish silently dropped by `Session::frame`'s own guard.
        // The race was genuinely captured once, on the way in here - this
        // does not lose that write, only stops it from blocking the next
        // one.
        stage.result_saved = false;
        // Same reasoning as `Session::launch_race`: the outgoing `MenuStage`'s
        // last picture, so a later `escape` does not flash black waiting for
        // the restarted feed's first frame.
        self.held_menu_backdrop = match &mut self.stage {
            Stage::Menu(stage) => stage
                .backdrop
                .as_mut()
                .and_then(|backdrop| backdrop.held.take()),
            _ => None,
        };
        self.stage = Stage::Race(stage);
        // Never left mid-freeze by a pause that predates the trip through the
        // menus - see `Session::launch_race`, which resets this for the same
        // reason on a fresh race.
        self.paused = false;
        // The playlist's own resume: `pause_race_music` (called on the way
        // into the menus) saved the position and left `race_index` alone, so
        // this continues the same track rather than starting a new one - see
        // its doc comment.
        self.audio.start_race_music(
            &self.music_discs,
            self.settings.audio.music_source,
            &oag_source::cache::default_audio_cache_dir(),
        );
        let strings = strings::project_table(self.settings.language.as_deref());
        self.gpu.window.set_title(&hints::race_title(&strings));
        println!(
            "\n{}{}",
            hints::race_keys(&strings),
            hints::esc_to_menu(&strings)
        );
    }

    /// Leaving a campaign cell's race lands on `Cell Selection` with the cell
    /// still highlighted, in every campaign mode.
    ///
    /// The original's pause menu rows all end at `Kill Game Transition` ->
    /// `Kill Game` -> `Show Unlocks`, and `Show Unlocks`' redirect sends
    /// `Main Menu->Mode == FE_RACE_CAM` (and `Racebox->RBMode ==
    /// RB_LOAD_GRID`) to `Cell Selection`, everything else to `Main Menu`
    /// (`InGame_Definition.xml` and the `Show Unlocks` screen, `Data.wad`;
    /// `docs/formats/race-campaign.md`). `EndRace`'s `RETURN TO GRID` takes the
    /// same road, which is `Session::return_to_campaign`.
    ///
    /// **Chosen, not measured:** this build has no pause menu, so escape is
    /// the pause menu's QUIT RACE row, and the race is discarded rather than
    /// parked - a campaign race cannot be resumed from the menus the way a
    /// custom one can. The result capture at the top of [`Session::escape`]
    /// has already run, so a Speed Lap or Time Trial best is kept.
    fn quit_campaign_race(
        &mut self,
        cell: &str,
        difficulty: Option<oag_tables::race_campaign::Difficulty>,
    ) {
        info!("leaving the campaign race for Cell Selection");
        self.leave_finished_race();
        self.suspended_race = None;
        self.reopen_cell_selection(cell, difficulty);
    }

    /// The Android system Back (gesture or key): exactly [`Self::escape`],
    /// except that it never quits.
    ///
    /// **Chosen, not measured.** On the front end's root page, the disc
    /// chooser, the boot movies and `PRESS START` there is no level behind,
    /// and Android's own convention would hand the phone back to its launcher.
    /// A swipe by accident closing the game is worse than one that does
    /// nothing, and the root has a QUIT row for the player who means it, so
    /// those places ignore Back. In a race it is the pause menu: escape parks
    /// the race and opens the menus, and backing out of them resumes it.
    pub(crate) fn back(&mut self) {
        info!("system back");
        let was_quitting = self.quit;
        self.escape();
        self.quit = was_quitting;
    }

    /// What escape does, which is **back one level** and not quit.
    ///
    /// One rule, four places it lands:
    ///
    /// - in the menus, it pops a page, exactly as circle does. On the root page
    ///   `Menu::back` raises `Closed`, which means "there is nothing behind the
    ///   menus" - which quits, unless a race is parked underneath, in which
    ///   case there is, and that fourth place is [`Session::resume_race`];
    /// - in a race, it hands the window back to the menus;
    /// - in the front end, in the disc chooser, or in a `--race` run that never
    ///   had menus, there is no level behind and it quits. The chooser is the
    ///   clearest case of the rule rather than an exception to it: it is the
    ///   first thing a run shows, so behind it is the desktop.
    ///
    /// **Leaving a race parks it, unless the race is over.** A race still
    /// running is kept in [`Session::suspended_race`] rather than dropped, and
    /// backing all the way out of the menus resumes it exactly where it was
    /// left - a player who backs out mid-race expects to be able to get back
    /// to it. A finished race is different: there is a results table behind it
    /// rather than a track, and nothing there to resume into, so that one is
    /// still discarded the way every race used to be.
    pub(crate) fn escape(&mut self) {
        // **The other capture site** - see `oag_game::records`'s own module
        // doc for why there have to be two. This one exists for the three
        // modes `Session::frame`'s finish-transition arm can never reach:
        // `SpeedLap` and `Zone` never set `Race::finished` at all, and an
        // abandoned `TimeTrial` leaves through here too. Guarded by
        // `result_saved` the same way the other site is, so a race that
        // already saved on finishing and is *then* escaped from its own
        // results table does not write the file a second time for nothing
        // new to say.
        //
        // Read into locals and the borrow of `self.stage` ended before
        // `self.records` is touched - the same borrow-splitting
        // `Session::frame`'s own capture uses, and for the same reason:
        // `stage` and `self.records` are different fields of the same
        // `self`, so the first has to stop being borrowed before the second
        // can be.
        if let Stage::Race(stage) = &mut self.stage {
            crate::race_stage::ghost::save(&mut stage.race, &stage.result_key);
        }
        if let Stage::Race(stage) = &mut self.stage
            && !stage.result_saved
        {
            stage.result_saved = true;
            let key = stage.result_key.clone();
            let observation = stage.observation();
            if let Some(cell) = stage.campaign_cell.as_ref() {
                self.records.record_campaign(
                    &key.title,
                    &cell.name,
                    observation.campaign_medal,
                    observation.campaign_difficulty,
                );
            }
            if let Some(progress) = stage.campaign_2048_event.as_ref() {
                self.records.record_campaign(
                    &key.title,
                    &progress.name,
                    observation.campaign_medal,
                    None,
                );
            }
            self.records.record(key, observation);
            if let Err(e) = records::save(&self.records) {
                error!("could not save race records: {e:#}");
            }
        }

        // The chooser's disc-key prompt takes escape whole, the same rule a
        // menu's modal prompt follows below: escape closes it and does not quit.
        if let Stage::Launcher(stage) = &mut self.stage
            && stage.launcher.cancel_entry()
        {
            return;
        }
        // A selection screen takes escape first, as Back - the same rule the
        // modal prompt below follows, one layer up.
        if matches!(&self.stage, Stage::Menu(stage) if stage.picker.is_some()) {
            self.handle_picker(oag_ui_screens::picker::Event::Back);
            return;
        }
        // The Race Campaign's own screens, same rule: `Cell Selection` steps
        // back to `Grid Selection`, which steps back to the menus.
        if matches!(&self.stage, Stage::Menu(stage) if stage.campaign.is_some()) {
            self.handle_campaign(oag_ui_screens::campaign::Event::Back);
            return;
        }
        // Collected before anything else touches `self`: `handle_menu` takes
        // `&mut self` and the events borrow the stage.
        if let Stage::Menu(stage) = &mut self.stage {
            // **A modal prompt takes escape first, and takes it whole.**
            // `Menu::back` is called here out of band with the tick loop -
            // see its own doc - so without this, escape over an open keyboard
            // would pop the page *behind* the overlay and leave the prompt
            // hanging over a page it was never opened from. Cancelling is
            // also what escape means in the one other modal state this build
            // has: `rebind::Capture::Cancel`.
            if stage.prompt.take().is_some() {
                return;
            }
            let events = stage.menu.back();
            for event in events {
                self.handle_menu(&event);
            }
            return;
        }

        if self.shell.is_some()
            && let Stage::Race(stage) = &self.stage
            && let RaceExit::CellSelection { cell, difficulty } =
                race_exit(stage.campaign_cell.as_ref(), stage.campaign_difficulty)
        {
            self.quit_campaign_race(&cell, difficulty);
            return;
        }

        if matches!(self.stage, Stage::Race(_)) && self.shell.is_some() {
            info!("leaving the race");
            // Before `open_menus`, not after: the menu voice this resumes has
            // to be sounding by the time the menus themselves draw. See
            // `Audio::pause_race_music`.
            self.audio.pause_race_music();
            // The engine is a *held* voice - `~ENGINE` - so leaving the race
            // has to release it. Without this the menus hum.
            self.audio.stop_race_sfx();
            match self.open_menus() {
                Ok(()) => println!(
                    "\n{}",
                    hints::shell_keys(&strings::project_table(self.settings.language.as_deref()))
                ),
                // Reported rather than fatal, and then it quits: a race whose
                // menus cannot be rebuilt has nothing left to offer, but a
                // window that vanished with no message would read as a crash.
                Err(e) => {
                    error!("cannot return to the menus: {e:#}");
                    self.quit = true;
                }
            }
            return;
        }

        self.quit = true;
    }
}
