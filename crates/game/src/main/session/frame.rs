//! One frame: the fixed timestep, the stage update, and the draw.

use anyhow::Result;
use log::{debug, error, info, warn};

use oag_game::frontend::{self};
use oag_game::keys;
use oag_game::{boot, display, drs, font, menu, movie, perf, race, report, settings, upscale};
use oag_gameplay::input::Button;

use crate::hints::SHELL_KEYS;
use crate::stage::Stage;

use super::Session;

impl Session {
    /// What bounds the dynamic-resolution controller this frame.
    ///
    /// The floor is a [`display::Scale`] and so a percentage of the **aspect
    /// rectangle**, exactly as `render_scale` is - not a fraction of the
    /// allocation. Read the other way it would mean a different pixel count at
    /// every render scale, which is not what a row spelled `50` beside a row
    /// spelled `100` says.
    fn drs_limits(&self) -> drs::Limits {
        let rect = display::viewport(self.gpu.size(), self.settings.display.aspect);
        // Off the **render profile**, like the ceiling it pairs with: a target
        // Pulse holds comfortably is not one HD/Fury holds, which is the whole
        // reason those rows are per title.
        let profile = self.render_profile();
        let floor = upscale::target_size(
            rect,
            profile.minimum_resolution,
            self.gpu.device.limits().max_texture_dimension_2d,
        );
        drs::Limits::new(
            self.framebuffer.allocation(),
            floor,
            // Held under the limiter: aiming above a rate the loop is not
            // allowed to produce would drop the resolution permanently to buy
            // frames that are never presented. The menu warns about the same
            // pairing; this is what stops it costing anything.
            profile.target_fps.at_most(self.limiter_hz()),
        )
    }

    pub(crate) fn frame(&mut self) -> Result<()> {
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
        // `Launch Game` opens **our menus**, not a race. The original has a main
        // menu between the picker and a track and this build now has one too; it
        // is simply not the original's, which is why the state whose transition
        // gets us here is still spelled the way the disc spells it while what it
        // reaches is not a recovered screen at all. See `oag_game::menu`.
        if !self.launched
            && matches!(&self.stage, Stage::Frontend(stage) if stage.frontend.is_finished())
        {
            self.launched = true;
            info!("{}: opening the menus", frontend::states::LAUNCH_GAME);
            // Whatever the picker settled on, remembered for next time. Taken
            // here rather than in the picker because this is where the front
            // end is known to be finished with it, and because `menu.rs` and
            // `frontend.rs` both stay ignorant of where settings live.
            if let Stage::Frontend(stage) = &self.stage
                && let Some(language) = stage.frontend.chosen()
                && self.settings.language.as_deref() != Some(language)
            {
                self.settings.language = Some(language.to_string());
                if let Err(e) = settings::save(&self.settings) {
                    warn!("could not save the chosen language: {e:#}");
                }
            }
            if let Err(e) = self.open_menus() {
                error!("cannot open the menus: {e:#}");
            } else {
                println!("\n{SHELL_KEYS}");
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
        let now = std::time::Instant::now();
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
        // exclude - see [`oag_render::timing::PassTimer`].
        self.frame_index += 1;
        if self.stalled {
            self.meter.clear();
            // The same guard, reaching a measurement that has not come back
            // yet: `clear` empties what was recorded, and the reading for this
            // frame is still somewhere between the queue and the map. Every
            // reading up to and including this one is dropped when it lands.
            self.scene_cost.clear();
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
        // worker at different points anyway.
        let (phase, progress) = self.loading_progress();

        for _ in 0..steps {
            // Ends the devices' tick for both stages. A race reads the snapshot's
            // axes; the front end reads the button edges the same call computed,
            // through `buttons_mut`, because it needs `consume_press` and a
            // snapshot is a value.
            let snapshot = self.controls.snapshot();
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
            // `movie::Player::follow`.
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
            if matches!(self.stage, Stage::Race(_))
                && self.controls.buttons().is_pressed(Button::Select)
            {
                self.controls.buttons_mut().consume_press(Button::Select);
                self.cycle_camera_view();
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
            // What leaves the results table, and it is the same thing escape
            // does from a race: hand the window back to the menus, or quit a
            // `--race` run that never had any. There is nothing else the table
            // can do - the original's `Race End Proceed` chain, which is where a
            // photo, a save and the records go, is not built. See
            // `oag_game::scoreboard`.
            //
            // **A rising edge**, so the thrust the player was holding as they
            // crossed the line cannot dismiss the board they have not read yet.
            if matches!(&self.stage, Stage::Race(stage) if stage.race.finished())
                && [Button::Cross, Button::Start]
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
                }
                Stage::Loading(stage) => {
                    stage
                        .screen
                        .advance(progress.finished && stage.media_ready() && stage.race_ready());
                }
                Stage::Frontend(stage) => {
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
                        if let oag_game::state_machine::Event::Enter(name) = event {
                            stage.install_movie(name, &mut self.audio);
                        }
                    }
                    report(&events, stage.trace);
                    for note in stage.frontend.take_notes() {
                        info!("{note}");
                    }
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
                            &boot::default_audio_cache_dir(),
                        );
                    }
                }
                Stage::Menu(stage) => {
                    stage.tick(dt);
                    // Snapshotted *before* the input is consumed, because the
                    // page being left stops existing the moment the model
                    // moves. Compared by page id rather than by stack depth:
                    // `back` and `open` both change the page, and a jump
                    // between two pages at the same depth is still a change.
                    let before = stage.menu.page().id.clone();
                    let leaving = menu::draw_list(
                        &stage.menu,
                        &stage.skin,
                        &keys::bound_keys,
                        &|text| font::measure(&stage.text_atlas, text),
                        None,
                        &stage.frame,
                    );
                    let events = stage.menu.update(self.controls.buttons_mut());
                    if stage.menu.page().id != before {
                        stage.begin_change(leaving);
                    }
                    for event in events {
                        self.handle_menu(&event);
                    }
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
                    // **The race is over, so nothing is stepped.** The world is
                    // left exactly as the finishing tick left it and the frame
                    // loop goes on drawing it under the results table, which is
                    // what makes "the race is complete" a state rather than a
                    // banner over a race that is still going.
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
                    self.audio.race_tick(&mut stage.race);
                }
                Stage::Race(stage) if self.paused => {
                    // Same shape as the finished arm above and for the same
                    // reason: the audio is not simulation state, so ticking
                    // it is what keeps the engine note honest while `World`
                    // itself sits frozen at whatever tick `Start` caught it on.
                    self.audio.race_tick(&mut stage.race);
                }
                Stage::Race(stage) => {
                    // Outside `Race::tick`, so this cannot reach the hash: it
                    // tops the slot up the way a pad grant would, and only when
                    // the slot is already empty, so firing still spends it.
                    if let Some(weapon) = self.give {
                        let ship = &mut stage.race.world.ships[0];
                        if ship.pickup.weapon.is_none() {
                            ship.pickup.weapon = Some(weapon);
                        }
                    }
                    stage.race.tick(&snapshot);
                    // Immediately after the tick and inside this loop, so a cue
                    // lands on the tick that raised it whether the frame
                    // stepped once, twice or not at all. See
                    // `audio::Audio::race_tick`.
                    self.audio.race_tick(&mut stage.race);
                    if self.log_every > 0 && stage.race.world.tick % u64::from(self.log_every) == 0
                    {
                        println!("{}", race::describe(&stage.race.telemetry()));
                    }
                }
            }
        }

        let frame = match self.gpu.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame)
            | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                self.gpu
                    .surface
                    .configure(&self.gpu.device, &self.gpu.config);
                return Ok(());
            }
            other => {
                debug!("skipping frame: {other:?}");
                return Ok(());
            }
        };

        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("frame"),
            });
        // Where the game goes on the surface, and how many pixels it is drawn
        // with. One rectangle for every stage, so a change moves the whole game
        // rather than only what happens to be on screen.
        let rect = display::viewport(self.gpu.size(), self.settings.display.aspect);
        // The presentation target follows the *surface*, not the render scale -
        // it is where the resolved frame and the UI meet, so it is sized in the
        // pixels a player actually has. Cheap when nothing moved; a window
        // resize is the only thing that reallocates it.
        self.framebuffer
            .resize_output(&self.gpu.device, self.gpu.size());
        let wanted = upscale::target_size(
            rect,
            self.render_profile().render_scale,
            self.gpu.device.limits().max_texture_dimension_2d,
        );
        if self.framebuffer.resize(&self.gpu.device, wanted) {
            // A depth attachment whose size does not match the colour one is a
            // validation error, so the race's has to follow - and now that is
            // true of a scene sitting in `LoadingStage::built_race` as well:
            // `Session::advance_race_build` builds it against this size while
            // the loading screen is still up, and it can wait out a whole fade
            // there before `finish_loading` ever swaps it in. A resize that
            // lands during that wait has to reach it in place, or the swap
            // hands over a scene sized for a framebuffer that no longer exists.
            match &mut self.stage {
                Stage::Race(stage) => {
                    stage.scene.resize(
                        &self.gpu.device,
                        self.gpu.config.format,
                        self.framebuffer.allocation(),
                    );
                }
                Stage::Loading(stage) => {
                    if let Some(Ok(built)) = stage.built_race.as_mut() {
                        built.scene.resize(
                            &self.gpu.device,
                            self.gpu.config.format,
                            self.framebuffer.allocation(),
                        );
                    }
                }
                _ => {}
            }
            // A parked race is not `self.stage` - see `Session::suspended_race` -
            // so the match above never reaches it, and the menus over it are
            // free to change window mode, size or render scale while it waits.
            // Same validation-error reasoning as the two arms above: its
            // depth attachment has to track the framebuffer too, or the resize
            // that greeted the menus leaves `resume_race`'s scene sized for a
            // framebuffer that no longer exists.
            if let Some(stage) = self.suspended_race.as_mut() {
                stage.scene.resize(
                    &self.gpu.device,
                    self.gpu.config.format,
                    self.framebuffer.allocation(),
                );
            }
        }

        // **The controller's own rectangle, re-applied every frame and not
        // only when it moves.** `Framebuffer::resize` above resets the extent
        // to the allocation whenever it reallocates, so a scale applied once
        // would silently return to full size on the next window resize - and
        // it has to land *after* that block and *before* `extent()` is read
        // below, or the projection and ADR-0039's jitter follow a stale
        // rectangle. Both failure modes are silent.
        //
        // Off, and on every capture path, `drs::Controller::extent` is the
        // allocation and this is inert.
        let drs_limits = self.drs_limits();
        self.framebuffer.set_extent(self.drs.extent(drs_limits));

        // **Two targets, and which one a stage draws into is whether it has a
        // scene**, per [ADR-0038](../../../../../docs/architecture/adr/0038-a-stage-with-no-scene-draws-at-presentation-resolution.md).
        //
        // A race draws into the *scene* target, at the render scale, and fills
        // it - the target is the game's rectangle, and the bars are the surface
        // the blit does not cover.
        //
        // Every other stage is UI all the way down: the launcher, the loading
        // screen, the front end and the menus draw into the *presentation*
        // target instead, at the aspect rectangle, and `resolve_scene` is
        // skipped for them entirely. `Renderer::render` clears, so that clear
        // draws the bars exactly where the blit's own would have. Their
        // backdrop movie comes along at presentation size, which is one
        // resample rather than two.
        // **The extent, not the allocation** - `inside` is a viewport, and
        // since [ADR-0037](../../../../../docs/architecture/adr/0037-dynamic-resolution-varies-a-viewport-not-an-allocation.md)
        // the scene texture can be larger than the rectangle drawn into it.
        // Everything that normalises against this inherits the extent for
        // free, the camera jitter of ADR-0039 included.
        let size = self.framebuffer.extent();
        let inside = (0.0, 0.0, size.0 as f32, size.1 as f32);
        let scene_target = self.framebuffer.view();
        let ui_target = self.framebuffer.output();
        // Whether this frame has anything for the upscaler to carry. Read
        // before the match, which borrows `self.stage` mutably.
        let has_scene = matches!(self.stage, Stage::Race(_));
        // Claim a timestamp pair for the scene pass, on the frames that have
        // one. Here rather than inside the match for the same reason
        // `pvs_culling` is read here: the match borrows `self.stage`. Paired
        // with exactly one `resolve` below, which is why the claim is gated on
        // the same `has_scene` the pass itself is.
        if has_scene && let Some(timer) = self.pass_timer.as_mut() {
            timer.begin(self.frame_index);
        }
        // And one for the motion-blur chain, on the same terms. **Claimed
        // before it is known whether the chain will encode anything**, because
        // the claim has to precede the pass and only `MotionBlur::render`
        // knows whether its scratch targets are built yet - `stats.blur_encoded`
        // is the answer, and the `abandon` below is what a `false` costs.
        if has_scene && let Some(timer) = self.blur_timer.as_mut() {
            timer.begin(self.frame_index);
        }
        // And one for the HD/Fury bloom chain, on the same terms as the blur
        // claim above. **Claimed before it is known whether this scene even
        // has one**: `race::scene::frame::Scene::hd` is `None` on Pulse,
        // Pure and any HD circuit with no `HDR and Bloom` block, and there is
        // no cheap way to ask that here, above the match that borrows
        // `self.stage`. `stats.hd_bloom_encoded` is the answer and the
        // `abandon` below is what a `false` costs - see `SceneStats::hd_bloom_encoded`.
        if has_scene && let Some(timer) = self.hd_bloom_timer.as_mut() {
            timer.begin(self.frame_index);
        }
        // Read before the match, which borrows `self.stage` mutably.
        let pvs_culling = self.pvs_culling();
        // Read before `self.stage` is borrowed mutably below, for the same
        // reason `pvs_culling` is.
        let render_profile = self.render_profile();
        // Diagnostic only, and read before the match for the same reason
        // `pvs_culling` is: whether this is the frame `race_ready_at` is still
        // waiting on. Splits `Session::race_ready_at`'s single "since the scene
        // was ready" number into where inside this one frame it actually goes -
        // see the timers below and in `Session::finish_race_loading`.
        let timing_first_race_frame =
            self.race_ready_at.is_some() && matches!(self.stage, Stage::Race(_));
        let (scene_stats, video_label) = match &mut self.stage {
            Stage::Launcher(stage) => {
                stage.render(&self.gpu, &mut encoder, ui_target, rect);
                (None, None)
            }
            Stage::Loading(stage) => {
                stage.render(&self.gpu, &mut encoder, ui_target, rect, phase, &progress);
                (None, None)
            }
            Stage::Frontend(stage) => {
                // The same feed the menus will borrow, and it is alive from the
                // session opening rather than from the menus opening - so the
                // backdrop under `Show Logo` is the one already looping, not a
                // second decoder.
                stage.render(
                    &self.gpu,
                    &mut encoder,
                    ui_target,
                    rect,
                    self.backdrop.as_mut(),
                )?;
                (None, stage.video_label())
            }
            Stage::Menu(stage) => {
                stage.render(
                    &self.gpu,
                    &mut encoder,
                    ui_target,
                    rect,
                    self.backdrop.as_mut(),
                )?;
                (None, self.backdrop.as_ref().map(movie::Feed::decoder_label))
            }
            Stage::Race(stage) => {
                let start = timing_first_race_frame.then(std::time::Instant::now);
                // The Zone visualiser's own input - see
                // `oag_audio::spectrum` and `race::scene::frame::Scene::render`'s
                // own doc comment on this parameter. Through the recovered
                // peak-hold on the way, which is the original's own
                // per-frame ballistics and belongs on this side of the seam
                // for the same reason its state does: it is one hold for the
                // frame, not one per drawable that reads it.
                let spectrum = self.audio.output().spectrum().levels();
                let zone_spectrum = self.zone_hold.advance(&spectrum).to_vec();
                let stats = stage.render(
                    &self.gpu,
                    &mut encoder,
                    scene_target,
                    inside,
                    self.settings.graphics.fov,
                    self.settings.graphics.frustum_culling,
                    pvs_culling,
                    self.anim_seconds,
                    render_profile.motion_blur,
                    // **Derived from the upscaler, not read from the flag**,
                    // and computed here because this is the one place that
                    // holds both sizes the count depends on: `size` is the
                    // extent the scene is drawn at and `rect` the rectangle it
                    // resolves into. See `upscale::jitter_phases`.
                    upscale::jitter_phases(
                        self.camera_jitter,
                        render_profile.reconstruction,
                        self.gpu.temporal,
                        size,
                        (rect.2 as u32, rect.3 as u32),
                    ),
                    &zone_spectrum,
                    self.pass_timer
                        .as_ref()
                        .and_then(oag_render::timing::PassTimer::writes),
                    // The two halves of one pair, for the chain's first and
                    // last pass - see `PassTimer::half_writes`.
                    self.blur_timer.as_ref().and_then(|timer| {
                        Some(oag_render::post::motion_blur::ChainTimestamps {
                            begin: timer.half_writes(oag_render::timing::Half::Begin)?,
                            end: timer.half_writes(oag_render::timing::Half::End)?,
                        })
                    }),
                    // The same split, for the HD/Fury bloom chain.
                    self.hd_bloom_timer.as_ref().and_then(|timer| {
                        Some(oag_render::post::hd_bloom::ChainTimestamps {
                            begin: timer.half_writes(oag_render::timing::Half::Begin)?,
                            end: timer.half_writes(oag_render::timing::Half::End)?,
                        })
                    }),
                );
                if let Some(start) = start {
                    info!("first race frame: encoded in {:?}", start.elapsed());
                }
                (Some(stats), None)
            }
        };

        // **A claim the chain did not write into is given back here**, not
        // left to resolve: `MotionBlur::render` encodes nothing at `off` or
        // before its scratch targets exist, and a slot whose closing timestamp
        // was never written reads back an unspecified value rather than a zero.
        // Four unresolved claims end measurement for the run, so this is the
        // difference between "motion blur is off" and "nothing is timed any
        // more".
        if !scene_stats.is_some_and(|stats| stats.blur_encoded)
            && let Some(timer) = self.blur_timer.as_mut()
        {
            timer.abandon();
        }
        // The same rule for the HD/Fury bloom chain: `false` on Pulse, Pure,
        // or an HD circuit with no `HDR and Bloom` block, all of which claim a
        // slot above with nothing to write into it. `Chain::run` has no early
        // return once it starts, so this is the only way the claim goes
        // unwritten - see `SceneStats::hd_bloom_encoded`.
        if !scene_stats.is_some_and(|stats| stats.hd_bloom_encoded)
            && let Some(timer) = self.hd_bloom_timer.as_mut()
        {
            timer.abandon();
        }

        // Only sampled under `Dev`, which is the one tier that shows it - a
        // reader that costs nothing at 4 Hz would still be a syscall a frame
        // at the four-figure rates `Vsync::Off` can reach, paid on every
        // stage for a line nobody but `Dev` draws. See `perf::memory::Probe`.
        let memory = match self.settings.graphics.perf_overlay {
            perf::Overlay::Dev => self.memory.sample(now),
            _ => None,
        };

        // **Only when there was a scene to carry.** A UI-only stage has already
        // drawn straight into the presentation target above, and its own clear
        // put the bars there; running the resolve now would blit a stale scene
        // over the top of it. See
        // [ADR-0038](../../../../../docs/architecture/adr/0038-a-stage-with-no-scene-draws-at-presentation-resolution.md).
        //
        // For a race this is the upscaler and the blit, into the presentation
        // target rather than onto the surface - so that this and a `--presented`
        // capture cannot drift apart, and so the HUD below lands on a frame that
        // is already at presentation size. Ungraded on purpose; `composite`
        // grades.
        if has_scene {
            let presentation = {
                let render_profile = self.render_profile();
                upscale::Presentation {
                    reconstruction: render_profile.reconstruction,
                    sharpness: render_profile.upscale_sharpness.stops(),
                    brightness: self.settings.display.brightness,
                    gamma: self.settings.display.gamma,
                }
            };
            // **Only a race answers, and only on an adapter that can run it.**
            // Every other stage has no scene to reconstruct from (ADR-0038) and
            // hands `None`, which falls the fallback ladder one rung exactly as
            // an unsupported adapter does. Read *after* the stage rendered
            // above, which is what makes it this frame's camera rather than the
            // previous one's.
            let temporal = match &self.stage {
                Stage::Race(stage) if self.gpu.temporal => stage.temporal(),
                _ => None,
            };

            // **The claim is here, not up beside the scene pass's, and it is
            // gated on the same three things `resolve_scene` decides with.**
            // A timestamp pair that no pass writes does not resolve to zero -
            // its value is unspecified, and the query set is not cleared
            // between frames, so a slot reused four frames later can hand back
            // the pair a *previous* frame wrote. That reads as a plausible
            // number attributed to the wrong frame, which is worse on an
            // overlay than a blank. So a slot is claimed only when the chain
            // will actually run.
            //
            // `resolve` below stays unconditional: `PassTimer::begin`'s own
            // documentation is that a claimed-and-unresolved slot never comes
            // back, and four of those end measurement for the run. Claim less,
            // always resolve.
            let will_upscale_temporally = temporal.is_some()
                && presentation.reconstruction == crate::display::Reconstruction::Fsr3
                && self.framebuffer.temporal_upscaler_viable();
            if will_upscale_temporally && let Some(timer) = self.upscale_timer.as_mut() {
                timer.begin(self.frame_index);
            }

            self.framebuffer.resolve_scene(
                &self.gpu.device,
                &self.gpu.queue,
                &mut encoder,
                rect,
                &presentation,
                temporal,
                // One pair around FSR 3.1's whole compute pass, on the frames
                // the claim above took a slot for.
                self.upscale_timer
                    .as_ref()
                    .and_then(oag_render::timing::PassTimer::compute_writes),
            );
        }

        // The HUD, after the resolve and at presentation size, laid out against
        // the aspect rectangle rather than the offscreen extent - so it stays
        // sharp at any render scale and an upscaler never sees it. It is still
        // *inside* the grade, which is the whole reason the presentation target
        // exists rather than the HUD simply going onto the surface the way the
        // performance overlay does. See
        // [ADR-0036](../../../../../docs/architecture/adr/0036-ui-composites-at-presentation-resolution.md).
        //
        // Only the race has one; every other stage *is* its own UI and drew
        // itself into this target already.
        if let Stage::Race(stage) = &mut self.stage {
            stage.draw_hud(&self.gpu, &mut encoder, self.framebuffer.output(), rect);
        }

        // Last, and the only pass that writes the surface: everything a player
        // sees is in the presentation target by now, so this is where
        // brightness and gamma can reach all of it.
        self.framebuffer.composite(
            &self.gpu.queue,
            &mut encoder,
            &view,
            self.settings.display.brightness,
            self.settings.display.gamma,
        );

        // **After** the resolve, and onto the surface rather than the offscreen
        // target: the overlay composites at presentation resolution, per
        // [ADR-0036](../../../../../docs/architecture/adr/0036-ui-composites-at-presentation-resolution.md).
        // `rect` rather than `inside` for the same reason - the 480x272 grid
        // this lays out in is mapped onto the aspect rectangle of the *surface*
        // now, which is the same rectangle on screen and a different set of
        // pixels to rasterise into.
        //
        // This overturns `perf.rs`'s older "the overlay should cost what the
        // game costs". That is right for a render scale a player sets once and
        // wrong for one that moves: the overlay is the row somebody reads to
        // judge what a resolution controller is doing, and an overlay
        // resampling along with the scene cannot be read while it moves. It
        // also takes the overlay out of FXAA, SMAA and FSR 1, whose treatment
        // of coverage-atlas glyphs [ADR-0013](../../../../../docs/architecture/adr/0013-anti-aliasing-architecture.md)
        // flagged as inherited doubt rather than intent.
        //
        // The cost of this is that the overlay is no longer graded, since the
        // grade rides in the resolve above. Deliberate and permanent for this
        // one element: it is an instrument, and a frame-time graph a brightness
        // of 20 % makes unreadable is a worse instrument. Every *other* piece of
        // UI keeps the grade when it moves, which is what the presentation
        // target in ADR-0036's decision is for.
        //
        // `Renderer::overlay` loads rather than clears, so the blit and the
        // aspect bars underneath it survive. One pass, skipped entirely when the
        // setting is off.
        let list = perf::draw_list(
            &self.meter,
            self.settings.graphics.perf_overlay,
            self.presentation_hz(),
            scene_stats,
            video_label,
            memory,
            // Only where there was a scene: every other stage draws straight
            // into the presentation target, so the scaled target's size would
            // name a texture nothing on screen came from. Read after the draw
            // rather than before, so the row is what this frame *was* drawn
            // at - the resize above is the only thing that moves either size,
            // and it happens before the stage renders.
            has_scene.then(|| perf::RenderSize {
                extent: self.framebuffer.extent(),
                allocation: self.framebuffer.allocation(),
            }),
            // All four GPU readings, whatever the stage: a reading is a
            // frame or more old, so gating this on `has_scene` would blank the
            // row on the menu frame that is finally reporting the last race
            // frame's cost. Each is `None` until its own first reading lands.
            perf::GpuCost {
                scene: self.scene_cost.stats().map(|s| s.mean_ms / 1000.0),
                blur: self.blur_cost.stats().map(|s| s.mean_ms / 1000.0),
                bloom: self.hd_bloom_cost.stats().map(|s| s.mean_ms / 1000.0),
                upscale: self.upscale_cost.stats().map(|s| s.mean_ms / 1000.0),
            },
        );
        if !list.is_empty() {
            self.overlay.overlay(
                &self.gpu.device,
                &self.gpu.queue,
                &mut encoder,
                &view,
                &list,
                rect,
            );
        }

        // Last thing into the encoder, and only when this frame claimed a pair:
        // the copy has to follow the pass that wrote the timestamps and
        // precede the submit that runs both.
        if let Some(timer) = self.pass_timer.as_mut() {
            timer.resolve(&mut encoder);
        }
        if let Some(timer) = self.upscale_timer.as_mut() {
            timer.resolve(&mut encoder);
        }
        if let Some(timer) = self.blur_timer.as_mut() {
            timer.resolve(&mut encoder);
        }
        if let Some(timer) = self.hd_bloom_timer.as_mut() {
            timer.resolve(&mut encoder);
        }

        let submit_start = timing_first_race_frame.then(std::time::Instant::now);
        self.gpu.queue.submit(Some(encoder.finish()));
        // **After the submit**, which is what makes it safe to start a map on
        // the buffer this frame just copied into. What comes back is a frame or
        // more old and says which frame it was - so a reading from a frame that
        // carried a load is dropped rather than recorded as a 300 ms scene.
        self.read_timing_and_feed_drs(&render_profile, drs_limits);
        if let Some(start) = submit_start {
            info!("first race frame: submitted in {:?}", start.elapsed());
        }
        let present_start = timing_first_race_frame.then(std::time::Instant::now);
        self.gpu.queue.present(frame);
        if let Some(start) = present_start {
            info!("first race frame: presented in {:?}", start.elapsed());
        }
        // The third and last diagnostic timestamp - see `Session::race_ready_at`.
        // Taken rather than read, so this fires once: the first frame drawn
        // with the new scene, not every frame after it.
        if matches!(self.stage, Stage::Race(_))
            && let Some(ready_at) = self.race_ready_at.take()
        {
            info!(
                "first race frame presented: {:?} since the scene was ready",
                ready_at.elapsed()
            );
        }
        Ok(())
    }
}
