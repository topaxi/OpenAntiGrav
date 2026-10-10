//! One frame: the fixed timestep and the stage update. The draw itself is
//! [`Session::draw`], in the sibling `draw.rs` this module doc used to
//! describe alongside - see that file's own doc for why it moved out.

use anyhow::Result;
use log::{error, info, warn};

use oag_display::display;
use oag_display::space::Space;
use oag_game::{records, report, settings};
use oag_gameplay::input::Button;
use oag_physics::pilot_assist::Level;
use oag_raceplay as race;
use oag_ui::frontend::{self};
use oag_ui::strings;
use oag_ui::{font, menu};

use crate::hints;
use crate::stage::Stage;

use super::Session;
use super::pointer;

impl Session {
    pub(crate) fn frame(&mut self) -> Result<()> {
        // `--measure-race-load`'s clock and its `LAUNCH RACE`, first so the
        // frame it times is all of this one. A no-op on every other run.
        self.probe_begin_frame();
        self.refresh_prompts();
        // Whatever the audio callback could not render while this thread held
        // the mixer lock, said out loud from a thread that can afford to
        // allocate a sentence. Here rather than in the tick loop because a
        // dropped buffer is a property of the wall clock, not of the timestep,
        // and `Output::report_health` throttles itself in frames.
        self.audio.output().report_health();
        self.audio.output().flush_tap();

        // Before everything, because it is what makes there be anything: until
        // a disc has been picked there is no boot to finish and no front end to
        // hand a window to. Read a frame after the press for the same reason
        // `Launch Game` is checked here rather than in the tick loop - the row
        // lighting up is drawn before the load stalls the window.
        let picked = match &mut self.stage {
            Stage::Launcher(stage) => stage.picked.take(),
            _ => None,
        };
        if let Some(source) = picked
            && let Err(e) = self.finish_launcher(&source)
        {
            // Reported and stayed on rather than fatal: the chooser is still on
            // screen and the other rows are still there to try. A disc that
            // will not boot is exactly the case this screen exists to survive.
            error!("cannot boot {source}: {e:#}");
        }

        #[cfg(target_arch = "wasm32")]
        self.sync_web_window();

        // Before this frame's ticks, so the front end's own first frame is drawn
        // on the frame after the fade ended rather than a frame later still.
        if let Err(e) = self.finish_loading() {
            error!("cannot open the front end: {e:#}");
            self.quit = true;
            return Ok(());
        }

        // Checked before this frame's ticks rather than after them, so the frame
        // that entered `Launch Game` is drawn once before the load stalls the
        // window.
        // Whatever the picker settled on, remembered for next time. Saved the
        // frame the pick lands, not when the menus open: a player who quits
        // from the title screen (the web build reloads the page) would
        // otherwise be asked again. Here rather than in the picker because
        // `menu.rs` and `frontend.rs` stay ignorant of where settings live.
        if let Stage::Frontend(stage) = &self.stage
            && let Some(language) = stage.frontend.chosen()
            && self.settings.language.as_deref() != Some(language)
        {
            self.settings.language = Some(language.to_string());
            if let Err(e) = settings::save(&self.settings) {
                warn!("could not save the chosen language: {e:#}");
            }
        }

        // `Launch Game` opens **our menus**, not a race. The original has a main
        // menu between the picker and a track and this build now has one too; it
        // is simply not the original's, which is why the state whose transition
        // gets us here is still spelled the way the disc spells it while what it
        // reaches is not a recovered screen at all. See `oag_ui::menu`.
        if !self.launched
            && matches!(&self.stage, Stage::Frontend(stage)
                // `--measure-race-load` times the race path, not the boot
                // sequence, and skips straight past it.
                if stage.frontend.is_finished() || self.load_probe.is_some())
        {
            self.launched = true;
            info!("{}: opening the menus", frontend::states::LAUNCH_GAME);
            // Wipeout 2048's own Team/Options screens, read the same "only
            // if the player actually touched it" way the language above is -
            // `None` on every title without a touch front end, and on this
            // one until `Home`'s own tiles are visited at all. Applied
            // before `open_menus` below, which is what seeds the RACE/REMIX
            // pages' own TEAM row and the Team screen's campaign-launch
            // team from `settings.race.team`/`variant` - after it, the row
            // would already be built off the untouched value. See
            // `oag_ui::frontend::team`'s and `::options2048`'s own module
            // docs for what each of these carries and why the rest (skin,
            // motion sensor) is drawn and not read here.
            if let Stage::Frontend(stage) = &self.stage {
                if let Some((team, variant)) = stage.frontend.team_choice() {
                    self.settings.race.team = team.to_string();
                    self.settings.race.variant = variant.to_string();
                }
                if let Some(index) = stage.frontend.camera_choice() {
                    self.settings.graphics.camera_view = match index {
                        0 => display::CameraView::Close,
                        2 => display::CameraView::Internal,
                        _ => display::CameraView::Far,
                    };
                }
                // `OptionsPilot`'s list is `FE_OFF`, `FE_NORMAL`, `FE_SUPER`.
                if let Some(index) = stage.frontend.pilot_choice() {
                    self.settings.controls.pilot_assist = match index {
                        0 => Level::Off,
                        1 => Level::Normal,
                        _ => Level::Extreme,
                    };
                }
                if let Some(percent) = stage.frontend.music_choice() {
                    self.settings.audio.music_volume =
                        oag_sound::Volume::try_from(percent).unwrap_or_default();
                }
                if let Some(percent) = stage.frontend.sfx_choice() {
                    self.settings.audio.sfx_volume =
                        oag_sound::Volume::try_from(percent).unwrap_or_default();
                }
            }
            let hint_strings = strings::project_table(self.settings.language.as_deref());
            // What Wipeout 2048's own front end asked for, if it was that
            // title: its `Launch 2048` carries a request where every other
            // title's `Launch Game` opens the menus and stops. See
            // `oag_ui::frontend::Launch`.
            let launch = match &self.stage {
                Stage::Frontend(stage) => stage.frontend.launch().cloned(),
                _ => None,
            };
            if let Err(e) = self.open_menus() {
                error!("cannot open the menus: {e:#}");
            } else {
                println!("\n{}", hints::shell_keys(&hint_strings));
                self.follow_launch(launch);
            }
        }

        // As soon as the circuit's own load lands, not only once the fade that
        // covers it has run out - see `Session::advance_race_build`, which is
        // the fix for the gap that otherwise opens up behind the black screen.
        // A no-op on every frame but the one the worker actually finishes on.
        //
        // Before the timing block below and not after it, same as
        // `finish_loading` above: the scene build it does is itself a load,
        // `advance_race_build` sets `self.stalled` for exactly that reason, and
        // a call on the other side of `elapsed` would land the stall in the
        // *next* frame's measurement instead, past the point that flag is read.
        self.advance_race_build();

        // Fixed timestep, per ADR-0007: the simulation steps at exactly 1/60
        // whatever the window is doing. The clock's own catch-up cap is what keeps
        // the race load above from being paid back as a burst of ticks.
        let now = web_time::Instant::now();
        let elapsed = now.duration_since(self.last);
        self.last = now;
        self.schedule_next_frame(now);
        // The presentation layer's only reader of the clock, and it reads the
        // same value the timestep does rather than taking its own.
        //
        // **A load is not a frame time.** Opening the menus or building a race
        // stalls the loop for a few hundred milliseconds, and that stall lands
        // in exactly one `elapsed` - the same one, whichever side of this the
        // stage change happened on, because a stage only ever changes above or
        // below here. Recording it would put one 1000 ms column across the
        // graph for the two seconds a player is most likely to be looking at
        // it, so the frame that carries a load is dropped instead.
        // Counted here, before anything this frame is measured against it. A
        // GPU reading names the frame it was taken on rather than the frame it
        // arrives on, which is what `stall_frame` below is then able to
        // exclude - see [`oag_gpu::timing::PassTimer`].
        self.frame_index += 1;
        // **How long this frame took, or nothing at all on a stalled one.**
        // The same value `meter` is fed, handed to the same guard: a load is
        // not a frame time, and the residual `drs::Residual` learns from is
        // `frame - timed`, so a 300 ms stall paired with a 3 ms scene would
        // teach it that this machine spends 297 ms on work nothing times. The
        // `Option` is how "the caller has no reading" is said, rather than a
        // zero that would read as a frame that took no time.
        let frame_seconds = (!self.stalled).then_some(elapsed.as_secs_f32());
        if self.stalled {
            self.meter.clear();
            // The same guard, reaching a measurement that has not come back
            // yet: `clear` empties what was recorded, and the reading for this
            // frame is still somewhere between the queue and the map. Every
            // reading up to and including this one is dropped when it lands.
            self.scene_cost.clear();
            // The two wall-clock meters, on the same guard and for the same
            // reason: a 300 ms track build recorded as this frame's CPU cost
            // would sit in the `CPU` row for the two seconds a player is most
            // likely to be reading it.
            self.cpu_cost.clear();
            self.present_cost.clear();
            // The controller's own half of the same guard. It holds the scale
            // rather than resetting it - see `drs::Controller::reset` for why
            // snapping back to the ceiling on a load is the wrong move.
            self.drs.reset();
            self.stall_frame = self.frame_index;
            self.stalled = false;
        } else {
            self.meter.record(elapsed.as_secs_f32());
            // **A stutter, named where it happened.** An audio underrun and a
            // frame that took 90 ms are the same event seen from two threads,
            // and the only way to tell that from an audio path that is late on
            // its own is to have both lines in one terminal. 40 ms is two and a
            // half frames at 60 Hz - past any ordinary jitter and well under
            // the load stalls `self.stalled` already excludes.
            if elapsed > std::time::Duration::from_millis(40) {
                warn!("frame: {:.1} ms", elapsed.as_secs_f32() * 1000.0);
            }
        }
        let nanos = u64::try_from(elapsed.as_nanos()).unwrap_or(u64::MAX);
        let steps = self.clock.advance(nanos);
        let dt = f64::from(self.clock.rate().dt());

        // Before the snapshot below, because it is what makes the snapshot
        // start meaning anything: `--prefetch` may not run until the boot's own
        // movies are done with the cache, so this is the frame that starts it.
        // A no-op on every frame but one, and on every run without the flag.
        // `race.is_none()` because the race path puts the same stage up again
        // long after the boot, and starting a conversion from *there* would put
        // ten minutes of `ffmpeg` behind a two-second screen. `--prefetch` is
        // the boot's, and the boot's screen is the one that waits for it.
        if matches!(&self.stage, Stage::Loading(stage) if stage.media_ready() && stage.race.is_none())
        {
            self.start_prefetch();
        }

        // One snapshot for the whole frame, taken outside the tick loop: it is a
        // lock and a clone, and the ticks in one frame cannot have seen the
        // worker at different points anyway. `phase` itself is unused here -
        // `Session::draw` reads it off a second, equally cheap call instead,
        // now that drawing is split into its own file - but `progress` is
        // read again below, inside the tick loop.
        let (_phase, progress) = self.loading_progress();
        // Where the picture is on the surface this frame, for the pointer:
        // the same rectangle `Session::draw` fits every stage into, so a
        // click maps back through exactly the transform the rows went out
        // through. See `session::pointer::in_grid`.
        let rect = display::viewport(self.gpu.size(), self.settings.display.aspect);

        for _ in 0..steps {
            // Ends the devices' tick for both stages. A race reads the snapshot's
            // axes; the front end reads the button edges the same call computed,
            // through `buttons_mut`, because it needs `consume_press` and a
            // snapshot is a value.
            // One snapshot per grid slot, from whichever device
            // `oag_input::pad::Assignment` says drives it - every device on
            // slot 0 until something assigns one elsewhere, which is the single
            // merged stream this was until 2026-09-16. The front end reads the
            // button edges this same call computed, through `buttons_mut`, for
            // the one slot the keyboard drives; it has one menu and one cursor
            // however many people are racing.
            self.feed_touch();
            let inputs = self.controls.player_snapshots();
            self.pad_dir = super::menu_sound::pad_direction(self.controls.buttons());
            // The pad spoke: the drawn cursor goes until the mouse moves
            // again, the same rule a key press applies in `app.rs`. Read
            // off the pad's own contribution rather than the merged
            // snapshot, because a click synthesises a press through the
            // keyboard's latch and that press is the mouse speaking.
            if self.controls.pad_spoke() {
                self.pointer.other_device();
            }
            // And the pointer's, on the same latch terms: one take per tick,
            // in window pixels until a stage says which grid it draws in.
            let pointer = self.pointer.take();
            // The audio's whole tick, and it is inside this loop rather than
            // beside it on purpose. Cue emission and mixer control are driven by
            // the tick count, exactly as the exhaust and the chase camera are
            // (see `race::Race::tick`), so a headless capture and a window
            // produce the same sound at the same tick. There is deliberately no
            // per-frame counterpart: with a device attached `cpal` drains the
            // mixer from its own callback thread, and with none the offline
            // dump below is the only reader.
            //
            // The movie's playhead is read **before** that call, so that this
            // loop and `capture::run` pace the picture against the same
            // measurement - where the sound had got to at the end of the
            // previous tick - rather than differing by one tick depending on
            // which side of `tick` each happened to sit. See
            // `frontend::Player::follow`.
            let movie_playhead = self.audio.movie_playhead();
            self.audio.tick();
            // The in-race camera cycle, read off the shared `Input` and consumed,
            // exactly as the front end and the menus consume their own presses -
            // a press seen on two devices is one press and there is one place to
            // clear it.
            //
            // Deliberately here and **not** inside `Race::tick`. The tick takes an
            // `InputSnapshot` by value and the selected view is not part of one, so
            // keeping the cycle outside is what makes "cycling the camera cannot
            // move a simulation bit" true by construction rather than by argument.
            // It also puts the settings file - which `Race` cannot see - in reach,
            // which is what persists the choice across a restart. Before the tick
            // rather than after, so the frame this tick produces is already drawn
            // from the new view.
            // **Not on `Race End Photo`**, where the legend names this button
            // for photo mode, which this build does not have: the press is
            // spent and the camera stays as the spectator director has it.
            let in_photo = matches!(&self.stage, Stage::Race(stage)
                if stage.endrace.as_ref().is_some_and(
                    crate::race_stage::endrace_touch::EndRace::is_photo));
            if matches!(self.stage, Stage::Race(_))
                && self.controls.buttons().is_pressed(Button::Select)
            {
                self.controls.buttons_mut().consume_press(Button::Select);
                if !in_photo {
                    self.cycle_camera_view();
                }
            }
            // Pause, on the same button the original spends on "leave the
            // results table" - free during a still-running race because
            // nothing else there reads `Start` yet. **No menu, no overlay**:
            // this only decides whether `Race::tick` below runs, so the world
            // is exactly as the freezing tick left it, the way the finished
            // race arm below already leaves it under the results table.
            if matches!(&self.stage, Stage::Race(stage) if !stage.race.finished())
                && self.controls.buttons().is_pressed(Button::Start)
            {
                self.controls.buttons_mut().consume_press(Button::Start);
                self.paused = !self.paused;
            }
            // Circle is "back" everywhere else on screen - the menus, the
            // front end - and a paused race is the one place a still-running
            // one reads it too. Gated on `self.paused` rather than on the
            // stage alone, so an unpaused race keeps Circle for
            // `spend_pickup`'s absorb - see `race::weapons`. Same shape as
            // the results-table block below: hand the window back to the
            // menus, exactly as `escape` does from any race.
            if matches!(self.stage, Stage::Race(_))
                && self.paused
                && self.controls.buttons().is_pressed(Button::Circle)
            {
                self.controls.buttons_mut().consume_press(Button::Circle);
                self.escape();
                // The remaining steps this frame owed belong to whatever is on
                // screen now, and it has not been drawn once yet.
                break;
            }
            // What leaves the built-in results table, and it is the same thing
            // escape does from a race: hand the window back to the menus, or
            // quit a `--race` run that never had any. See
            // `oag_game::scoreboard`. **Only while no EndRace flow is built**:
            // once it is, Cross/Start belong to `Session::tick_endrace` below,
            // which walks `Results` -> `Rewards` -> `Menu` - see
            // `race_stage::endrace::results_table_takes_confirm` for the bug
            // this guard fixed.
            //
            // **A rising edge**, so the thrust the player was holding as they
            // crossed the line cannot dismiss the board they have not read yet.
            if matches!(&self.stage, Stage::Race(stage)
            if crate::race_stage::endrace::results_table_takes_confirm(
                stage.race.finished(),
                stage.endrace.is_some(),
            )) && [Button::Cross, Button::Start]
                .into_iter()
                .any(|press| self.controls.buttons().is_pressed(press))
            {
                for press in [Button::Cross, Button::Start] {
                    self.controls.buttons_mut().consume_press(press);
                }
                self.escape();
                // The remaining steps this frame owed belong to whatever is on
                // screen now, and it has not been drawn once yet.
                break;
            }
            match &mut self.stage {
                // Stepped in the tick loop with everything else, so the wave's
                // heartbeat runs at the simulation's fixed 60 Hz rather than at
                // whatever the window is managing. The original's own loading
                // thread ran it at 30; ours is one beat per 24 ticks either way,
                // and a frame-rate-dependent heartbeat is exactly the thing
                // ADR-0007 fixed the timestep to avoid.
                // Two waits, one screen: the boot's own movies, and the
                // `--prefetch` conversion when there is one. The fade starts
                // when both are done - `Screen::advance` holds at full opacity
                // until then - and `finish_loading` hands the window on when it
                // has run out.
                // In the tick loop with everything else, so a held key is read
                // as one press at the same 60 Hz the menus read theirs at.
                Stage::Launcher(stage) => {
                    stage.update(self.controls.buttons_mut());
                    // The chooser draws in the PSP's grid, having no source
                    // to take one from - see `Stage::launcher`.
                    stage.pointer(&pointer::in_grid(pointer, Space::PSP, rect));
                }
                Stage::Loading(stage) => {
                    stage.screen.set_load_stage(progress.load_stage);
                    stage
                        .screen
                        .advance(progress.finished && stage.media_ready() && stage.race_ready());
                }
                Stage::Frontend(stage) => {
                    // The language picker takes the pointer itself; every
                    // other screen in the chain waits for a button and
                    // reads a click as one. See `session::pointer`.
                    let grid = pointer::in_grid(pointer, stage.frontend.space(), rect);
                    if !stage.frontend.pointer(&grid) {
                        pointer::press_for_click(&mut self.controls, &grid);
                    }
                    let events =
                        stage
                            .frontend
                            .update(dt, self.controls.buttons_mut(), movie_playhead);
                    // One set of planes and one voice serve every movie the front
                    // end draws, so each is installed on the tick its own screen
                    // is entered - picture and sound together, which is what makes
                    // `movie_playhead` report *that* movie's position for the
                    // screen's own update to pace against.
                    //
                    // Every entered state is offered rather than one named screen:
                    // which screens play movies came out of the title's own chain,
                    // and Pure's two are neither its boot step nor the step after
                    // its picker.
                    for event in &events {
                        if let oag_ui::state_machine::Event::Enter(name) = event {
                            stage.install_movie(name, &mut self.audio);
                        }
                    }
                    report(&events, stage.trace);
                    oag_raceplay::loader_log::lines(stage.frontend.take_notes());
                    // The movie's sound outlives neither leg, and a skip leaves
                    // the state without finishing the player - see
                    // `Frontend::is_playing_movie`.
                    //
                    if !stage.frontend.is_playing_movie() {
                        self.audio.stop_movie();
                    }
                    // The menu music's cue is **no movies left**, not "not in one
                    // right now". The two are the same question only on a title
                    // whose boot opens on its movie: Pure opens on its language
                    // picker, so "not in a movie" is true before its reel has
                    // played at all, and starting the loop there put it under the
                    // reel - which is the overlap this test was written to stop.
                    // `pending` empties as each movie is installed, so it is
                    // exactly "none still to come".
                    //
                    // Every tick rather than on the edge - the state is what is
                    // asked, not a transition - which `start_music` absorbs by
                    // being idempotent.
                    if !stage.frontend.is_playing_movie() && stage.pending.is_empty() {
                        self.audio.start_music(
                            &self.music_discs,
                            self.settings.audio.music_source,
                            &oag_source::cache::default_audio_cache_dir(),
                        );
                    }
                }
                Stage::Menu(stage) => {
                    stage.tick(dt);
                    // A selection screen takes the tick whole, the way a
                    // modal prompt does below: the rows behind it are a
                    // picture. Confirming may hand the window to a loading
                    // screen, so the catch-up loop stops the same way it does
                    // for a race the menu just started.
                    let grid = pointer::in_grid(pointer, stage.skin.space(), rect);
                    if stage.picker.is_some() {
                        self.tick_picker(&grid);
                        if !matches!(self.stage, Stage::Menu(_)) {
                            break;
                        }
                        continue;
                    }
                    // The Race Campaign's own screens, same shape: they take
                    // the tick whole rather than sitting behind the rows.
                    if stage.campaign.is_some() {
                        self.tick_campaign(&grid);
                        continue;
                    }
                    // Snapshotted *before* the input is consumed, because the
                    // page being left stops existing the moment the model
                    // moves. Compared by page id rather than by stack depth:
                    // `back` and `open` both change the page, and a jump
                    // between two pages at the same depth is still a change.
                    let before = stage.menu.page().id.clone();
                    let leaving = menu::draw_list(
                        &stage.menu,
                        &stage.skin,
                        &|button| self.controls.bound_keys(button),
                        &|text| font::measure(&stage.text_atlas, text),
                        None,
                        &stage.frame,
                        // The same fact `Session::draw`'s own call reads off
                        // `suspended_race` - a page change mid-pause has to
                        // drop this snapshot's own full-screen chrome too, or
                        // the outgoing half of the tween flashes it back on.
                        self.suspended_race.is_some(),
                    );
                    // Off the raw `Input`, ahead of `Menu::update` - see
                    // `Session::maybe_begin_binding`'s own doc comment for why
                    // it takes its fields apart rather than borrowing `self`.
                    Session::maybe_begin_binding(
                        &mut self.awaiting_binding,
                        &mut self.controls,
                        &stage.menu,
                    );
                    // A modal prompt eats the tick's input outright: while an
                    // on-screen keyboard or a confirm is up, the rows behind
                    // it are a picture. Ahead of the menus rather than beside
                    // them, and `Input::take` is why finishing on this very
                    // tick is safe for a *button* - the edge that closed the
                    // prompt is consumed, so `Menu::update` below sees
                    // nothing. A `Pointer` is a value nothing consumes, so
                    // the menus are gated on whether a prompt was up *before*
                    // this tick: the click that answered KEEP, or the
                    // right-click that cancelled a keyboard, must not also
                    // land on the row beneath it.
                    let prompt_was_open = stage.prompt.is_some();
                    let finished = Session::tick_prompt(
                        stage,
                        self.controls.buttons_mut(),
                        &grid,
                        &self.pilot_roster,
                        self.shell.as_ref().map(|shell| &shell.strings),
                    );
                    // Frozen while a capture is in flight: `app.rs` diverts
                    // every keyboard event away from `Controls::set_key` for
                    // as long as `awaiting_binding` is `Some`, so there is
                    // nothing new here for the menus to navigate with anyway
                    // - and the pointer is held back with it, so a click
                    // cannot move the page under a prompt or a capture.
                    let events = if self.awaiting_binding.is_some() || prompt_was_open {
                        Vec::new()
                    } else {
                        let mut events = stage.menu.update(self.controls.buttons_mut());
                        events.extend(pointer::menu_pointer(
                            stage,
                            &grid,
                            &mut self.awaiting_binding,
                            &mut self.controls,
                        ));
                        events
                    };
                    if stage.menu.page().id != before {
                        stage.begin_change(leaving);
                    }
                    let navs = stage.menu.take_nav();
                    // After the stage borrow above is over, which is the whole
                    // reason `tick_prompt` hands the answer out rather than
                    // acting on it: renaming reloads the roster and re-supplies
                    // the very menu it was reading.
                    if let Some(finished) = finished {
                        self.finish_prompt(finished);
                    }
                    for event in events {
                        self.handle_menu(&event);
                    }
                    self.play_navs(navs);
                    // **Break if the menu just started or resumed a race**,
                    // the same way the results table above breaks when it
                    // hands the window back. `LaunchRace` itself only reaches
                    // `Stage::Loading` synchronously - the race proper waits
                    // behind that screen's fade - but `Session::resume_race`
                    // (on `MenuEvent::Closed` over a parked race, see
                    // `Session::suspended_race`) swaps `Stage::Menu` straight
                    // for `Stage::Race`, and without this the frame's
                    // remaining catch-up steps would tick it with whatever
                    // closed the menu still held - Circle, say, read as the
                    // in-race absorb it also means. Bounded by the catch-up
                    // cap, so usually zero or one step, which is exactly why
                    // the `LaunchRace` shape of this went unnoticed the first
                    // time: finding G1 of the 2026-08-18 review.
                    if matches!(self.stage, Stage::Race(_)) {
                        break;
                    }
                }
                Stage::Race(stage) if stage.race.finished() => {
                    // The built-in results table waits for cross or start
                    // and draws nothing to aim at, so a click is the press -
                    // the rising edge the block above this match reads next
                    // tick. The EndRace flow answers the pointer directly
                    // instead (`Session::tick_endrace`, called after this
                    // match ends) - `EndRace Menu`'s own rows need a real
                    // hit-test, which a synthesised Cross press on every
                    // click would fight with.
                    if stage.endrace.is_none() {
                        pointer::press_for_click(&mut self.controls, &pointer);
                    }
                    // **A race that ended any way but the line is over, so
                    // nothing is stepped.** The world is left exactly as the
                    // finishing tick left it and the frame loop goes on drawing it
                    // under the results table.
                    //
                    // Stopping here rather than inside `Race::tick` is
                    // deliberate: the tick is what the trace harness, the
                    // headless capture and the disc-backed AI tests all drive,
                    // and a tick that quietly became a no-op would change what
                    // every one of them produces. See `race::results`.
                    //
                    // **The audio still advances**, because it is not the
                    // simulation: `~ENGINE` is a held voice with a spin-down
                    // law, and leaving it unticked here would hold the engine
                    // at racing pitch under the results table for as long as
                    // the player looked at it. Nothing this call touches is
                    // `World` state, so "nothing is stepped" is still true of
                    // the thing that sentence is about.
                    // **Except when the player crossed the line**, where the
                    // original keeps the race running behind the panels: the
                    // opponents go on lapping and the player's craft is flown by
                    // the AI. Neutral input, because the panels consume the pad
                    // and a craft that is not the player's must not read it. The
                    // standings and the board were frozen at the line, so
                    // nothing the panels show moves. See
                    // `Race::runs_on_after_the_line`.
                    //
                    // A Single Race wreck steps the whole world too
                    // (`Race::runs_on_after_the_wreck`); any other ending (a Zone
                    // run) steps only what is seen: the explosion, the shake, the
                    // destroy camera. See `Race::tick_cosmetics`.
                    stage.race.tick_finished();
                    oag_game::sound::race_tick(&mut self.audio, &mut stage.race);
                    // `stage`'s own last use - the reborrow inside
                    // `Session::tick_endrace` is legal from here on, the
                    // same NLL shape `Stage::Menu`'s own arm relies on for
                    // `Session::tick_campaign`.
                    let space = stage
                        .endrace
                        .as_ref()
                        .map(crate::race_stage::endrace_touch::EndRace::space);
                    if let Some(space) = space {
                        let grid = pointer::in_grid(pointer, space, rect);
                        self.tick_endrace(&grid);
                    }
                }
                Stage::Race(stage) if self.paused => {
                    // A paused race reads circle as "back to the menus";
                    // the secondary button is circle everywhere else on
                    // screen, so it is here too.
                    if pointer.back {
                        self.controls.tap(Button::Circle);
                    }
                    // Same shape as the finished arm above and for the same
                    // reason: the audio is not simulation state, so ticking
                    // it is what keeps the engine note honest while `World`
                    // itself sits frozen at whatever tick `Start` caught it on.
                    oag_game::sound::race_tick(&mut self.audio, &mut stage.race);
                }
                // **The pre-race flyby steps instead of the world**: the circuit's own
                // camera animation, held to its end or a held Cross, with the grid
                // standing at its first tick. The countdown's 272 ticks start when it
                // ends - the original's does too. See `race::intro_camera`. After the
                // paused and finished arms, so Start still pauses it; and never reached
                // by a headless run, whose loop drives `Race::tick` from tick 0.
                Stage::Race(stage) if !self.no_intro && stage.race.intro_to_play() => {
                    stage.race.begin_intro();
                    if pointer.clicked {
                        stage.race.skip_intro();
                    }
                    stage.race.tick_intro(&inputs);
                }
                Stage::Race(stage) => {
                    // Outside `Race::tick`, so this cannot reach the hash: it
                    // tops the slot up the way a pad grant would, and only when
                    // the slot is already empty, so firing still spends it.
                    if let Some(weapon) = self.give {
                        let ship = &mut stage.race.sim.world.ships[0];
                        if ship.pickup.weapon.is_none() {
                            ship.pickup.weapon = Some(weapon);
                        }
                    }
                    // Every slot's snapshot, straight off the devices'
                    // assignment - see `oag_input::pad::Assignment`. Under its
                    // default every device is on slot 0, so this is the one
                    // snapshot the single-argument signature used to take, in
                    // the slot it used to go to.
                    stage.race.tick(&inputs);
                    stage.tick_messages();
                    // Immediately after the tick and inside this loop, so a cue
                    // lands on the tick that raised it whether the frame
                    // stepped once, twice or not at all. See
                    // `oag_sound::Audio::race_tick`.
                    oag_game::sound::race_tick(&mut self.audio, &mut stage.race);
                    if self.log_every > 0
                        && stage.race.sim.world.tick % u64::from(self.log_every) == 0
                    {
                        println!("{}", race::describe(&stage.race.telemetry()));
                    }
                }
            }

            // **The one moment a *finished* race's result is captured** -
            // never from inside `Race::tick` above, and never on every frame
            // the results table sits on screen: `result_saved` makes this a
            // one-time transition even though the finished arm re-enters
            // every tick for as long as the player looks at the board.
            // `escape` is the other capture site, for the three modes that
            // may never reach this one at all - see `oag_game::records`'s own
            // module doc for why both exist.
            //
            // Outside the `match` above on purpose: that borrows `self.stage`
            // mutably through `stage`, and `self.records` is a different
            // field of the same `self` - ending the first borrow before this
            // runs is what lets the second start. See `Session::escape`,
            // which draws the same borrow apart the same way.
            // A ghost is written on the tick a lap beats it, not only at the
            // finish: Speed Lap never finishes, and a lap set before a crash
            // should survive it. A no-op on every tick without a new best.
            if let Stage::Race(stage) = &mut self.stage {
                crate::race_stage::ghost::save(&mut stage.race, &stage.result_key);
            }
            if let Stage::Race(stage) = &mut self.stage
                && stage.race.finished()
                && !stage.result_saved
            {
                stage.result_saved = true;
                let key = stage.result_key.clone();
                // Before `stage.observation()` below reads
                // `RaceStage::tournament_final_rank` for the medal: a
                // Tournament cell's own rank is not known until the leg
                // that just finished is folded into the running standings.
                // See `super::tournament::record_finished_leg`'s own doc
                // for why this is a plain function taking `stage` rather
                // than a `Session` method.
                super::tournament::record_finished_leg(&mut self.tournament, stage);
                let observation = stage.observation();
                // Read before `self.records.record` below folds `observation`
                // into the row: `PersonalBest::compare` wants the row as it
                // stood *before* this race, and `Store::record` mutates it in
                // place - so this is the last point the "before" value can
                // still be read. See `crate::scoreboard`'s own "personal
                // best" line, the one reader of this field.
                stage.personal_best = Some(records::PersonalBest::compare(
                    self.records.get(&key),
                    &observation,
                ));
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
        }

        // Once per frame, not once per catch-up step above - the pointer
        // input itself is handled per-step, inside the loop's own finished-
        // race arm (`Session::tick_endrace`'s call site there), the same
        // grid-resolution shape `Stage::Menu`'s own arm uses for
        // `Session::tick_campaign`. `build_endrace` is idempotent past its
        // first call - `RaceStage::endrace`'s own `is_some()` guard for the
        // built case, `RaceStage::endrace_unavailable` for the failed one -
        // so calling it every frame the race sits finished costs nothing
        // once it has either built the flow or given up. The second guard
        // is not decoration: without it a source whose screens do not read
        // reopened its disc image once a frame and warned once a frame.
        self.build_endrace();

        let presented = self.draw(now, frame_seconds)?;
        self.probe_end_frame(elapsed);
        // **What the `OUTSIDE` row is derived from**, and the anchor matters:
        // measured from `now`, which is the same instant `elapsed` above is
        // taken from, so this frame's body and the gap after it partition the
        // interval between one frame's `now` and the next exactly. What lands
        // in the gap is the frame limiter's `ControlFlow::WaitUntil` (see
        // `App::about_to_wait`), the event loop, and the handful of cheap
        // calls this function makes before it reads the clock at all - which
        // is why the row is not called `SLEEP`.
        //
        // Recorded after the draw rather than around it, so the reading covers
        // the whole frame - ticks, stage update and draw together - which is
        // what has to be subtracted from the wall clock for the remainder to
        // be the gap. Under the same load guard as `meter` above, and under
        // `draw`'s own answer besides: a frame that never got a surface
        // texture recorded nothing on `present_cost`, and recording it here
        // would leave the two meters averaging different sets of frames.
        if frame_seconds.is_some() && presented {
            self.cpu_cost.record(now.elapsed().as_secs_f32());
        }
        Ok(())
    }
}
