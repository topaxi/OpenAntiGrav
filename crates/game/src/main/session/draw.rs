//! The draw half of [`super::Session::frame`]: everything from acquiring the
//! surface texture to presenting it.
//!
//! Split out of `frame.rs` on 2026-09-05, which the pause overlay's own scene
//! render (`Stage::Menu` gaining a scene of its own, off
//! `Session::suspended_race`) pushed past the 1,000-line ceiling
//! `scripts/check-file-size.py` holds every file to. The seam was already
//! named: `frame.rs`'s own module doc calls out "the fixed timestep, the
//! stage update, and the draw" as three things, and this is the third.
//!
//! Three values cross the cut: `now` and `frame_seconds`, `frame()`'s own
//! readings of the wall clock and the timestep, and `drs_limits`, moved down
//! bodily with its one caller. Everything else either lives only on this
//! side or is cheap and read-only enough to recompute
//! (`Session::loading_progress`) rather than thread through as a parameter.

use anyhow::Result;
use log::{debug, info};

use oag_game::{display, drs, movie, perf, upscale};

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

    /// Acquires the surface, draws whatever stage is on screen, and presents.
    ///
    /// `now` and `frame_seconds` are [`Session::frame`]'s own readings, taken
    /// once at the top of the fixed timestep rather than re-read here: `now`
    /// feeds the memory probe's own timeline, and `frame_seconds` is `None`
    /// on a frame that carried a load, which is not a frame time - see
    /// [`Session::read_timing_and_feed_drs`].
    pub(super) fn draw(
        &mut self,
        now: std::time::Instant,
        frame_seconds: Option<f32>,
    ) -> Result<()> {
        // Recomputed rather than threaded through from `Session::frame`:
        // `loading_progress` is a plain read of settled state, nothing
        // between the two call sites changes its answer, and passing it
        // instead would be a parameter earning its keep only on the one
        // stage (`Stage::Loading`) that reads it.
        let (phase, progress) = self.loading_progress();
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
        //
        // **A parked race counts too.** `Session::suspended_race` renders into
        // `scene_target` from inside the `Stage::Menu` arm below, exactly as
        // `Stage::Race` does directly - so there is a scene for `resolve_scene`
        // to carry into the presentation target whenever one is parked, and
        // ADR-0038 ("a stage with no scene draws at presentation resolution")
        // does not apply: the menus over a parked race are a stage *with* one.
        let has_scene = matches!(self.stage, Stage::Race(_))
            || (matches!(self.stage, Stage::Menu(_)) && self.suspended_race.is_some());
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
            // The menu's own rows draw later, past the resolve below - see the
            // block that follows `draw_hud`. What happens here is only the
            // parked race's scene, when there is one, into `scene_target`:
            // the same call `Stage::Race` makes below, so the picture behind
            // the menus is the real thing rather than a stand-in for it.
            Stage::Menu(_) => {
                let stats = self.suspended_race.as_mut().map(|parked| {
                    let spectrum = self.audio.output().spectrum().levels();
                    let zone_spectrum = self.zone_hold.advance(&spectrum).to_vec();
                    parked.render(
                        &self.gpu,
                        &mut encoder,
                        scene_target,
                        inside,
                        self.settings.graphics.fov,
                        self.settings.graphics.frustum_culling,
                        pvs_culling,
                        self.anim_seconds,
                        render_profile.motion_blur,
                        render_profile.shadows,
                        // No jitter: `temporal` below stays `None` for this
                        // stage, so there is no reconstruction pass to feed a
                        // phase to, and a frozen picture has no motion for one
                        // to reconstruct from anyway.
                        None,
                        &zone_spectrum,
                        self.pass_timer
                            .as_ref()
                            .and_then(oag_render::timing::PassTimer::writes),
                        self.blur_timer.as_ref().and_then(|timer| {
                            Some(oag_render::post::motion_blur::ChainTimestamps {
                                begin: timer.half_writes(oag_render::timing::Half::Begin)?,
                                end: timer.half_writes(oag_render::timing::Half::End)?,
                            })
                        }),
                        self.hd_bloom_timer.as_ref().and_then(|timer| {
                            Some(oag_render::post::hd_bloom::ChainTimestamps {
                                begin: timer.half_writes(oag_render::timing::Half::Begin)?,
                                end: timer.half_writes(oag_render::timing::Half::End)?,
                            })
                        }),
                    )
                });
                (
                    stats,
                    self.backdrop.as_ref().map(movie::Feed::decoder_label),
                )
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
                    render_profile.shadows,
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
            if will_upscale_temporally {
                super::timing::claim_upscale(
                    self.upscale_timer.as_mut(),
                    self.upscale_presented_timer.as_mut(),
                    self.frame_index,
                );
            }

            let upscale_encoded = self.framebuffer.resolve_scene(
                &self.gpu.device,
                &self.gpu.queue,
                &mut encoder,
                rect,
                &presentation,
                temporal,
                super::timing::upscale_timestamps(
                    self.upscale_timer.as_ref(),
                    self.upscale_presented_timer.as_ref(),
                ),
            );
            // **The same rule the blur and HD-bloom claims already follow**:
            // a claim `will_upscale_temporally` made and `resolve_scene` did
            // not write into is given back here, not left to resolve to an
            // unspecified value. Without this, a shader that fails to build
            // on one adapter and not another (RADV and NVIDIA's proprietary
            // driver disagree on more than one WGSL corner) leaks the one
            // slot claimed on the failing frame - and, because
            // `Session::render_profile().reconstruction.is_temporal()` reads
            // the *setting* rather than whether FSR 3.1 is actually running,
            // every frame after that reads `Cost::fixed` as `None` forever
            // and `Controller::record` is never called again: the render
            // scale freezes wherever it happened to be the instant the build
            // failed, with no way back. See
            // `Session::read_timing_and_feed_drs`'s own `has_hd_bloom`-style
            // fix for the read-back half of the same bug.
            if will_upscale_temporally && !upscale_encoded {
                super::timing::abandon_upscale(
                    self.upscale_timer.as_mut(),
                    self.upscale_presented_timer.as_mut(),
                );
            }
        }

        // The HUD, after the resolve and at presentation size, laid out against
        // the aspect rectangle rather than the offscreen extent - so it stays
        // sharp at any render scale and an upscaler never sees it. It is still
        // *inside* the grade, which is the whole reason the presentation target
        // exists rather than the HUD simply going onto the surface the way the
        // performance overlay does. See
        // [ADR-0036](../../../../../docs/architecture/adr/0036-ui-composites-at-presentation-resolution.md).
        //
        // Only the race has one; every other stage that draws straight into
        // this target already did, above - **except the menus**, which draw
        // here instead. See the block below.
        if let Stage::Race(stage) = &mut self.stage {
            // `self.gpu.size()`, matching `Framebuffer::output`'s own size:
            // `resize_output` above already made sure of that this frame, and
            // `RaceStage::draw_hud`'s `target_size` has to be `view`'s real
            // size rather than assumed from `gpu.config` internally - see its
            // own doc.
            stage.draw_hud(
                &self.gpu,
                &mut encoder,
                self.framebuffer.output(),
                rect,
                self.gpu.size(),
            );
        }

        // **The menus draw here, after the resolve, rather than in the match
        // above.** Every other stage owns the presentation target outright and
        // draws into it first; the menus cannot, because a race parked behind
        // them (`Session::suspended_race`) rendered its own scene into
        // `scene_target` in that same match, and `resolve_scene` above has not
        // run yet at that point in the function - drawing the menu there and
        // resolving the scene here would have the resolve overwrite the rows
        // the menu had just drawn. Waiting until here, in the same slot the
        // HUD already uses, means the menu paints onto whatever the frame
        // already carries: the parked race's resolved picture when one is
        // parked, or nothing when the presentation target was never touched
        // this frame (`has_scene` is `false`) - either way `MenuStage::render`
        // itself decides whether to clear or load, off `suspended_race`.
        if let Stage::Menu(stage) = &mut self.stage {
            stage.render(
                &self.gpu,
                &mut encoder,
                self.framebuffer.output(),
                rect,
                self.backdrop.as_mut(),
                &|button| self.controls.bound_keys(button),
                self.suspended_race.is_some(),
            )?;
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
            // All five GPU readings, whatever the stage: a reading is a
            // frame or more old, so gating this on `has_scene` would blank the
            // row on the menu frame that is finally reporting the last race
            // frame's cost. Each is `None` until its own first reading lands.
            perf::GpuCost {
                scene: self.scene_cost.stats().map(|s| s.mean_ms / 1000.0),
                blur: self.blur_cost.stats().map(|s| s.mean_ms / 1000.0),
                bloom: self.hd_bloom_cost.stats().map(|s| s.mean_ms / 1000.0),
                upscale: self.upscale_cost.stats().map(|s| s.mean_ms / 1000.0),
                upscale_presented: self
                    .upscale_presented_cost
                    .stats()
                    .map(|s| s.mean_ms / 1000.0),
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
        super::timing::resolve_upscale(
            self.upscale_timer.as_mut(),
            self.upscale_presented_timer.as_mut(),
            &mut encoder,
        );
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
        self.read_timing_and_feed_drs(&render_profile, frame_seconds, drs_limits);
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
