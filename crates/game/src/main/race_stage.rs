//! The race stage: a loaded track, and the scene that draws it.

use log::warn;
use oag_display::display;
use oag_game::{race, upscale};

use crate::gpu::Gpu;

/// A race, and everything only it needs.
pub(crate) struct RaceStage {
    pub(crate) scene: race::Scene,
    pub(crate) race: race::Race,
    /// The HUD, or `None` when the disc's layout could not be read.
    pub(crate) hud: Option<oag_game::hud::Overlay>,
    /// The results table, drawn once the race has one.
    ///
    /// Not an `Option`, unlike the HUD: that one needs a layout off the disc and
    /// there may be none, where this needs only a font and
    /// `oag_game::font::Atlas::build` always answers with the built-in 5x7 set.
    pub(crate) scoreboard: oag_game::scoreboard::Overlay,
    /// The start-line countdown's own `<Mode3D>` model, or `None` when this
    /// mode's layout carries no `Cockpit321Go` widget to place it by - see
    /// `oag_game::hud::countdown`.
    pub(crate) countdown: Option<oag_game::hud::Countdown>,
    /// Which circuit/mode/class row [`oag_game::records`] persists this
    /// race's outcome under - resolved once, at load, from
    /// [`oag_game::race::Loaded::title`] and the [`oag_game::race::Options`]
    /// that produced it, because neither is reachable from a finished or an
    /// escaped race any other way. See `Session::frame`'s finish-transition
    /// arm and `Session::escape`, the only two readers.
    pub(crate) result_key: oag_game::records::Key,
    /// This race's own personal-best comparison, computed once at the finish
    /// transition and drawn on the results table alongside
    /// [`oag_game::scoreboard::Board`] - see [`RaceStage::draw_hud`].
    ///
    /// `None` until the finish arm sets it, and forever `None` on a race that
    /// is escaped rather than finished: [`oag_game::race::Race::results`] is
    /// `None` on that path too, so there is no table for it to be drawn on.
    /// See `Session::frame`'s finish-transition arm, the only writer.
    pub(crate) personal_best: Option<oag_game::records::PersonalBest>,
    /// Whether this race's result has already been folded into
    /// `Session::records` and saved **since this stage last became live** -
    /// not "ever", which is why `Session::resume_race` clears it back to
    /// `false` on the way out of a park.
    ///
    /// **Read and set at both capture sites, for two different reasons.**
    /// `Session::frame`'s finish-transition arm re-enters every tick for as
    /// long as the results table sits on screen, so without this check a
    /// finished race would resave the identical result every frame; without
    /// setting it there, `Session::escape` leaving that same race a moment
    /// later would save it a second time for nothing new to say. Reading it
    /// in `escape` too covers the mirror case: a race that finished and was
    /// saved is then escaped from its own results table without the finish
    /// arm's every-tick check ever seeing it happen again.
    ///
    /// **Clearing it on resume is not optional.** A race `escape` parks
    /// (`Session::suspended_race`, an unfinished race only) keeps this same
    /// `RaceStage`, flag included, across the trip through the menus. Left
    /// set, a Speed Lap improved after resuming would never be recorded -
    /// `escape`'s own guard would see it as already saved - and a Time
    /// Trial escaped mid-race and then resumed to a real finish would have
    /// that finish silently swallowed by `Session::frame`'s guard instead of
    /// reaching `Store::record` at all. The race genuinely was captured once
    /// on the way into the park; clearing the flag on the way back out does
    /// not lose that write; it only stops it from blocking the next one.
    pub(crate) result_saved: bool,
}

impl RaceStage {
    /// `&mut self` rather than `&self`, unlike the other stages: the HUD's
    /// renderers own a growable instance buffer, the same reason
    /// [`Renderer::overlay`] takes `&mut self`. The scene stays immutable behind
    /// its own `RefCell`.
    // The two culling flags are independent settings read from the same place,
    // and a struct to carry them past one call site would name the grouping
    // without clarifying it - the same call this file already makes for
    // `Scene::new`.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn render(
        &mut self,
        gpu: &Gpu,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        viewport: (f32, f32, f32, f32),
        fov: display::Fov,
        cull: bool,
        pvs_cull: bool,
        anim_seconds: Option<f32>,
        motion_blur: display::MotionBlur,
        shadows: display::Shadows,
        camera_jitter: Option<u32>,
        zone_spectrum: &[f32],
        timestamps: Option<wgpu::RenderPassTimestampWrites<'_>>,
        blur_timestamps: Option<oag_render::post::motion_blur::ChainTimestamps<'_>>,
        hd_bloom_timestamps: Option<oag_render::post::hd_bloom::ChainTimestamps<'_>>,
    ) -> race::SceneStats {
        // The Zone stage grade, pointed at the zone the race has reached before
        // the frame is built - the same per-frame order `Zone_UpdateStage` runs
        // in on 2048, where it is called from the render update rather than the
        // simulation. See `race::Scene::sync_zone_grade`.
        self.scene.sync_zone_grade(&self.race);
        // The scene and nothing else. The HUD used to follow it here, into the
        // same target; it composites at presentation resolution now - see
        // [`RaceStage::draw_hud`].
        self.scene.render(
            &gpu.device,
            &gpu.queue,
            encoder,
            view,
            &self.race,
            viewport,
            fov,
            cull,
            pvs_cull,
            anim_seconds,
            motion_blur,
            shadows,
            camera_jitter,
            zone_spectrum,
            timestamps,
            blur_timestamps,
            hd_bloom_timestamps,
        )
    }

    /// What a temporal upscaler needs from this frame, or `None` when nothing
    /// has been drawn yet or jitter is off.
    ///
    /// **Read after [`RaceStage::render`], never before**: it reports the frame
    /// that was drawn, and a caller that asked first would hand the upscaler the
    /// *previous* frame's camera and offset - which produces a picture rather
    /// than an error.
    pub(crate) fn temporal(&self) -> Option<upscale::Temporal<'_>> {
        self.scene
            .temporal()
            .map(|(depth, velocity, frame)| upscale::Temporal {
                depth,
                velocity,
                sample_count: self.scene.sample_count(),
                camera: frame.camera,
                jitter: frame.jitter,
                phase_count: frame.phase_count,
                reset: frame.reset,
            })
    }

    /// Reads the player's own outcome so far, straight off [`race::Race`]'s
    /// already-public state - never from inside a tick.
    ///
    /// The one place both capture sites (`Session::frame`'s finish-transition
    /// arm and `Session::escape`) build an
    /// [`oag_game::records::Observation`], so the field list only has to
    /// agree with `oag_game::records` in one place. See that module's own
    /// doc for why it takes primitives rather than a `&Race`.
    pub(crate) fn observation(&self) -> oag_game::records::Observation {
        let standing = &self.race.sim.world.ships[0].standing;
        oag_game::records::Observation {
            finished: self.race.finished(),
            place: Some(self.race.places()[0]),
            laps_completed: oag_game::records::laps_completed(
                standing.lap,
                self.race.finished(),
                self.race.sim.world.race.laps_target,
            ),
            tick: standing.finish_tick.unwrap_or(self.race.sim.world.tick),
            best_lap_ticks: standing.best_lap_ticks,
            // No campaign cell is selected for any race yet - that is
            // frontend wiring (`Cell Selection`/`Grid Selection`), out of
            // scope here. `oag_tables::race_campaign::Cell::evaluate_medal`
            // and `oag_game::records::Medal` are ready for whoever wires it:
            // see `Observation::campaign_medal`'s own doc and the `campaign`
            // handover thread.
            campaign_medal: None,
        }
    }

    /// Draws the HUD, or the results table once the race has one.
    ///
    /// **Separate from [`RaceStage::render`], and drawn into a different target
    /// than the scene.** It used to go over the scene inside the offscreen
    /// target, which meant it was rasterised at the render scale and then
    /// resampled - and, with `[graphics] upscaler` set, sharpened by FSR 1 as
    /// though a coverage-atlas glyph were scene content. It now composites into
    /// the presentation target after the resolve, so it is drawn in the pixels
    /// the player has whatever the render scale is, and no upscaler ever sees
    /// it. See
    /// [ADR-0036](../../../docs/architecture/adr/0036-ui-composites-at-presentation-resolution.md);
    /// `viewport` is the aspect rectangle on the surface, not the offscreen
    /// extent.
    ///
    /// **The HUD goes when the race does.** Once the flag is out the speed bar,
    /// the lap counter and the clock are all reporting a simulation nobody is
    /// stepping any more, and the original hides the HUD at its own race end
    /// too - see the `_BLOWUP` note in `race::tick`.
    ///
    /// `target_size` is `view`'s own full pixel dimensions - **not** derived
    /// from `gpu.config` here, because it used to be and that is exactly what
    /// broke: the ordinary per-frame call draws into `Framebuffer::output`,
    /// which does track the surface, but [`RaceStage::warm_up`] draws this
    /// same pass into the scene's own (possibly letterboxed, smaller) target
    /// instead - see its own doc. `Countdown::draw` builds its private depth
    /// attachment from this value and has to match `view`'s real size or the
    /// pass is a validation error, so each caller passes the size of the
    /// texture it actually handed in as `view` rather than one this function
    /// would have to guess at.
    pub(crate) fn draw_hud(
        &mut self,
        gpu: &Gpu,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        viewport: (f32, f32, f32, f32),
        target_size: (u32, u32),
    ) {
        match self.race.results() {
            Some(board) => self.scoreboard.draw(
                &gpu.device,
                &gpu.queue,
                encoder,
                view,
                board,
                self.personal_best.as_ref(),
                viewport,
            ),
            None => {
                if let Some(hud) = &mut self.hud {
                    let mut readout = self.race.readout();
                    // The rung the grade is showing, which is what names the
                    // speed class - see `Scene::zone_stage`.
                    readout.zone_stage = self.scene.zone_stage().unwrap_or(0);
                    readout.zone_next_in = self
                        .scene
                        .zones_to_next_stage(u16::try_from(readout.zone).unwrap_or(u16::MAX));
                    hud.draw(&gpu.device, &gpu.queue, encoder, view, &readout, viewport);
                    // The countdown, for exactly the measured start-line gate's
                    // span and no other window - see `oag_game::hud::countdown`
                    // and `oag_race::RaceState::thrust_gated`, whose own doc
                    // names the three live captures behind the 272-tick figure.
                    // Not gated further by mode: the capture that measured it
                    // was Time Trial alone, and `thrust_gated` already applies
                    // it to every mode on the same reasoning
                    // `RaceState::eliminate` uses, so this follows it rather
                    // than adding a second, narrower guess on top.
                    //
                    // **Not while the gantry is showing it.** The countdown
                    // belongs on the start gantry - the maintainer's account
                    // of the original, and what `race::gantry` now draws -
                    // so this overlay is the fallback for a circuit that
                    // stands no gantry, not a second countdown beside one.
                    if let Some(countdown) = &mut self.countdown
                        && !self.scene.draws_gantry()
                        && oag_race::RaceState::thrust_gated(readout.race_ticks)
                    {
                        countdown.draw(
                            &gpu.device,
                            &gpu.queue,
                            encoder,
                            view,
                            readout.race_ticks as f32 / 60.0,
                            viewport,
                            target_size,
                        );
                    }
                }
            }
        }
    }

    /// Forces the driver to actually compile every pipeline this scene
    /// reaches, before anyone is shown a frame that needs one.
    ///
    /// **Building a `wgpu::RenderPipeline` object is not the same as the
    /// backend having compiled it.** Several drivers defer that to the first
    /// real draw call that uses it, and building the scene eagerly - see
    /// `Session::advance_race_build` - does not touch that: it moves the CPU
    /// side of the build off the loading screen's fade, but the compile still
    /// waits for the first frame `Stage::Race` actually draws, wherever that
    /// lands. Measured at up to three seconds of a held, black frame - the
    /// loading screen's own last one, at zero opacity - between the hand-off
    /// and the first thing presented.
    ///
    /// So this draws the grid pose once, into `target`, and blocks until the
    /// GPU has actually finished it - `queue.submit` alone only queues the
    /// work, and returning before it lands would just move the stall back to
    /// whichever frame the driver gets around to it on. `target` is
    /// deliberately the caller's own framebuffer rather than a fresh scratch
    /// texture: it is the exact size and format the real first frame draws
    /// into, so the exact pipeline variants get warmed, and it is safe to
    /// overwrite because nothing is reading it: since
    /// [ADR-0038](../../../docs/architecture/adr/0038-a-stage-with-no-scene-draws-at-presentation-resolution.md)
    /// the loading screen does not draw into the scene target at all - it goes
    /// straight to the presentation target - so this target sits idle from the
    /// moment a race is being built until the first frame that draws one.
    ///
    /// `target_size` is `target`'s own full pixel dimensions - the caller's
    /// `Framebuffer::allocation`, not `Gpu::size`. The two agree only when the
    /// window is undecorated and exactly the configured aspect; anywhere else
    /// the scene target is letterboxed smaller than the window, and passing
    /// the window's own size on to [`RaceStage::draw_hud`] is the mismatch
    /// that surfaced as a `depth attachment`/`color attachment` size
    /// validation error - see `oag_game::hud::Countdown::draw`'s doc for why
    /// that pass in particular is where it shows.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn warm_up(
        &mut self,
        gpu: &Gpu,
        target: &wgpu::TextureView,
        target_size: (u32, u32),
        viewport: (f32, f32, f32, f32),
        fov: display::Fov,
        cull: bool,
        pvs_cull: bool,
        anim_seconds: Option<f32>,
    ) {
        let mut encoder = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("race warmup"),
            });
        self.render(
            gpu,
            &mut encoder,
            target,
            viewport,
            fov,
            cull,
            pvs_cull,
            anim_seconds,
            // Off: a warmup draws one frame of the grid pose, so there is no
            // previous camera and nothing the blur pipelines add to warm -
            // they are fullscreen passes with no per-scene variants.
            display::MotionBlur::Off,
            // **On, unlike the blur beside it**, and for the reason that
            // decides every entry in this list: a warmup exists to pay the
            // pipeline compiles, and the blob tier has a pipeline of its own.
            // It costs one quad per craft on a frame that is discarded.
            display::Shadows::Blob,
            // Off for the same reason, and one more: jitter is a matrix, not a
            // pipeline variant, so there is nothing here for it to warm. It
            // still costs this frame a phase - `Scene::frame_index` advances on
            // every render - so with jitter on, the first frame a player sees is
            // phase one rather than phase zero. Deterministic, and noted in
            // ADR-0039 so a shifted capture is not a mystery.
            None,
            // Empty: this frame is discarded, and the Zone visualiser
            // pipeline variant it would warm is the same one every other
            // Zone draw uses regardless of what the lookup holds.
            &[],
            // Untimed: a warmup frame is the pipeline compiles this call
            // exists to pay, so its cost is the one measurement that would
            // mislead a controller most. All three pairs, and the blur chain
            // would encode nothing here anyway at `MotionBlur::Off`.
            None,
            None,
            None,
        );
        // The HUD's own pipelines still need warming, and still into `target`
        // even though the real frame now draws them into the presentation
        // target instead. What a driver defers is a compile, and a pipeline is
        // keyed on its format - which is the same for both targets - not on
        // the size of the attachment it is first used against. `target_size`
        // still has to be `target`'s own size and not the presentation
        // target's, though: it is what the countdown's own depth attachment
        // is built from, and that one *is* sized against the attachment
        // rather than only its format. See this function's own doc.
        self.draw_hud(gpu, &mut encoder, target, viewport, target_size);
        gpu.queue.submit(Some(encoder.finish()));
        if let Err(e) = gpu.device.poll(wgpu::PollType::wait_indefinitely()) {
            // Not fatal: the worst outcome is the compile this call exists to
            // avoid happening on the first real frame instead, exactly as it
            // did before this existed.
            warn!("could not wait for the race scene's warmup submit: {e}");
        }
    }
}
