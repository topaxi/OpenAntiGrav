//! The race stage: a loaded track, and the scene that draws it.

use crate::medal_watch::ticks_to_centiseconds;
use oag_display::display;
use oag_present::upscale;
use oag_raceplay as race;

use crate::app::gpu::GpuContext;

pub(crate) mod auto_replay;
pub(crate) mod endrace;
pub(crate) mod endrace_touch;
pub(crate) mod ghost;
pub(crate) mod hd_loyalty;
mod medal_message;

/// A race, and everything only it needs.
pub(crate) struct RaceStage {
    pub(crate) scene: Box<race::Scene>,
    pub(crate) race: Box<race::Race>,
    /// The race's keyframes since its first tick, for photo mode's rewind; `None` where photo
    /// mode is not offered (`crate::photo::WRITES`).
    pub(crate) keeper: Option<Box<race::timeline::Keeper>>,
    /// The HUD, or `None` when the disc's layout could not be read.
    pub(crate) hud: Option<Box<crate::hud_overlay::Overlay>>,
    /// The results table, drawn once the race has one.
    ///
    /// Not an `Option`, unlike the HUD: that one needs a layout off the disc and
    /// there may be none, where this needs only a font and
    /// `oag_ui::font::Atlas::build` always answers with the built-in 5x7 set.
    pub(crate) scoreboard: Box<crate::scoreboard::Overlay>,
    /// The start-line countdown's own `<Mode3D>` model, or `None` when this
    /// mode's layout carries no `Cockpit321Go` widget to place it by - see
    /// `crate::hud_countdown`.
    pub(crate) countdown: Option<Box<crate::hud_countdown::Countdown>>,
    /// Detonator's three medal models, or `None` outside a Detonator race on a title that shows
    /// them - see `crate::hud_medal`.
    pub(crate) medals: Option<Box<crate::hud_medal::Medals>>,
    /// The countdown art's own clock in seconds while a resume counts down (and
    /// `GO` lingers after it), set by the session each frame; `None` otherwise.
    /// The art is drawn on this clock through the same model the start uses, and
    /// over the gantry's circuits too, since a gantry stands at the start line and
    /// a resume happens wherever the race was paused. See `session::pause`.
    pub(crate) resume_art: Option<f32>,
    /// The language's `GO`, while [`Self::resume_art`] is up on a title that draws the
    /// count as text; set by the session when a countdown begins, empty otherwise.
    pub(crate) resume_go: String,
    /// The track-description panel over the pre-race flyby, or `None` off Pulse's PSP disc and
    /// when any part of it would not read - see `crate::track_panel`.
    pub(crate) track_panel: Option<Box<crate::track_panel::Overlay>>,
    /// The screen an attract demo draws over its race, when the title authors one and it read.
    pub(crate) demo_overlay: Option<Box<crate::demo_overlay::Overlay>>,
    /// Which circuit/mode/class row [`crate::records`] persists this
    /// race's outcome under - resolved once, at load, from
    /// [`oag_raceplay::Loaded::title`] and the [`oag_raceplay::Options`]
    /// that produced it, because neither is reachable from a finished or an
    /// escaped race any other way. See `Session::frame`'s finish-transition
    /// arm and `Session::escape`, the only two readers.
    pub(crate) result_key: crate::records::Key,
    /// The archive entry of the circuit the race loaded, as the loader names it
    /// (`oag_raceplay::Loaded::circuit`).
    pub(crate) circuit: String,
    /// What the HUD's `RECORD` readout counts down to: the stored best under
    /// [`Self::result_key`] as it stood at load, and the track's authored
    /// time. `None` for a mode with no such readout and whenever the track's
    /// `stats.xml` did not load. See `oag_hud::RecordTarget`.
    pub(crate) record_target: Option<oag_hud::RecordTarget>,
    /// This race's own personal-best comparison, computed once at the finish
    /// transition and drawn on the results table alongside
    /// [`oag_raceplay::scoreboard::Board`] - see [`RaceStage::draw_hud`].
    ///
    /// `None` until the finish arm sets it, and forever `None` on a race that
    /// is escaped rather than finished: [`oag_raceplay::SimRace::results`] is
    /// `None` on that path too, so there is no table for it to be drawn on.
    /// See `Session::frame`'s finish-transition arm, the only writer.
    pub(crate) personal_best: Option<crate::records::PersonalBest>,
    /// A Detonator run's record notice, judged where the race is saved (before it is folded in).
    pub(crate) score_record: Option<oag_ui_screens::endrace::ScoreRecord>,
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
    /// The run this race belongs to (ADR-0099), moved in from `Session::run` the moment this
    /// stage is built (`crate::app::session::load`'s `advance_race_build`): its campaign cell,
    /// rung, skill scale and the options it launched with. RACE AGAIN, NEXT RACE and RESTART
    /// relaunch from a clone of it. Default (no cell) for an ordinary RACE-page or `--race`
    /// launch.
    pub(crate) run: crate::app::run_context::RunContext,
    /// The title this race runs, for the law its campaign cell's medal is read with
    /// ([`oag_title::Campaign::counting_modes`]).
    pub(crate) title: &'static oag_title::Title,
    /// The final standings rank this leg earns against
    /// [`Session::tournament`], or `None` on every leg but a Tournament
    /// cell's own last one - see that field's own doc.
    ///
    /// **Read by [`RaceStage::campaign_medal`] alone**, and drained from
    /// `Session::tournament` at the same finish-transition
    /// (`Session::frame`'s `result_saved` guard) that
    /// `RaceStage::observation` is next built from - the mirror of
    /// the run's campaign cell's "moved once, read once" shape, except
    /// this is written *after* the stage already exists rather than at
    /// build time, because the rank is not known until the leg itself
    /// finishes.
    ///
    /// [`Session::tournament`]: crate::app::session::Session::tournament
    pub(crate) tournament_final_rank: Option<u8>,
    /// Whether a window of this frame has already drawn, and so did the frame's
    /// camera-independent work (`Scene::render_window`'s `shared`). Cleared once a frame by
    /// [`RaceStage::begin_frame`]; set by the first window that draws, so a frame does that work
    /// once however many windows it has - and a window 1 that draws nothing leaves it to window 2.
    pub(crate) shared_frame_work: bool,
    /// The Wipeout 2048 campaign event this race was launched against, or
    /// `None` for every other title and for an ordinary `--race`/menu
    /// launch on 2048 itself. Resolved by `race::load_event` onto
    /// [`race::Loaded::campaign_2048_event`] and carried straight through -
    /// see that field's own doc for why this rides on `Loaded` rather than
    /// through `Session` the way the run's campaign cell does.
    pub(crate) campaign_2048_event: Option<race::Campaign2048Progress>,
    /// What the finish banked into the profile's XP total, for the EndRace page: `None` until the
    /// finish banks, and for a race that armed no tally.
    pub(crate) xp_banked: Option<crate::endrace::touch::XpBanked>,
    /// The EndRace flow (`Results` -> `Rewards`/`Menu`) once the race has
    /// finished and this title carries `EndRace_Definition.xml` - `None`
    /// until then, and forever `None` on a title that does not (or on an
    /// open source [`endrace::EndRaceRuntime::new`] could not read), which
    /// falls this stage back to [`RaceStage::scoreboard`] exactly as before
    /// this existed. Built once, by `Session::frame`'s finish-transition
    /// arm - see `crate::app::session::endrace`. [`Self::endrace_unavailable`]
    /// is what makes that "forever" true rather than "retried every frame".
    pub(crate) endrace: Option<endrace_touch::EndRace>,
    /// Set once [`Session::build_endrace`] has tried to build
    /// [`Self::endrace`] and failed, so it is never tried again for this
    /// race.
    ///
    /// **Without it the failure path is a per-frame loop, not a log line.**
    /// `Session::frame` calls `build_endrace` every frame the race sits
    /// finished, and the `endrace.is_some()` guard only stops the *success*
    /// case from repeating: a source whose screens cannot be read reopened
    /// its disc image and all seven of HD's archives 60 times a second, and
    /// said so on every one of them, for as long as the results table was on
    /// screen. Set at each of the three exits past that point - the source
    /// not opening, the screens not reading, the renderer not building - and
    /// at none of the guards before it, which mean "not yet" rather than
    /// "never".
    ///
    /// Per race rather than per process: a second race on a source that has
    /// no screens says so once more, which is a fact about that race.
    ///
    /// [`Session::build_endrace`]: crate::app::session::Session::build_endrace
    pub(crate) endrace_unavailable: bool,
    /// The worker building [`Self::endrace`]'s screens while the race runs, so the finishing frame
    /// does no disc read or GPU build. See `crate::app::session::endrace::prefetch`.
    pub(crate) endrace_prefetch: Option<crate::app::session::endrace::prefetch::Prefetch>,
    /// Whether that worker has been offered a start, so it is started once per race.
    pub(crate) endrace_prefetch_started: bool,
    /// Raised when this stage is dropped so the worker stops at its next phase; see the `Drop`.
    pub(crate) endrace_cancel: std::sync::Arc<std::sync::atomic::AtomicBool>,
    /// The best medal [`Self::tick_messages`] has already announced, so a medal
    /// is raised once however many ticks it stays earned.
    pub(super) earned_medal: Option<oag_tables::race_campaign::Medal>,
    /// The header a recording of this race is filed under: the records key, the team, the seed
    /// and the launch settings. Kept from the build so the session can start the recording once
    /// it has finished writing to the race.
    pub(crate) replay_header: oag_replay::Header,
    /// Whether this race is recorded for the REPLAYS page and not yet written
    /// ([`Self::arm_auto_replay`]).
    pub(crate) auto_replay: bool,
    /// Whether this race is a recording being watched: it is never parked behind the menus, and
    /// nothing it does is a record, a ghost or a replay.
    pub(crate) watching: bool,
    /// Set when this race is an attract demo: every craft is flown by the AI, nothing it does is
    /// a result, a ghost or a recording, and it is never parked behind the menus.
    pub(crate) demo: Option<crate::demo::Run>,
    /// Set when this race is the in-game benchmark: its frames are timed, nothing it does is a
    /// result, a ghost or a recording, and its end raises the stats screen
    /// (`session::benchmark`).
    pub(crate) benchmark: Option<Box<crate::benchmark::Live>>,
}

impl Drop for RaceStage {
    /// Cancels and joins the EndRace worker before this stage's GPU resources are released: a
    /// worker still submitting while the main thread tears a race down crashes the driver (a
    /// SIGSEGV on leaving a race with the prefetch unfinished). Waits only for the phase the
    /// worker is in; a stage dropped off the frame thread (`drop_off_thread`) waits there.
    fn drop(&mut self) {
        self.write_auto_replay();
        self.endrace_cancel
            .store(true, std::sync::atomic::Ordering::Relaxed);
        if let Some(mut task) = self.endrace_prefetch.take() {
            drop(task.join());
        }
    }
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
    ///
    /// `alpha` is how far between the race's previous tick and its current
    /// one the poses are drawn, `1.0` being the current tick exactly - see
    /// [`race::between_ticks`]. It stays set for the HUD that follows, so the
    /// reticle sits on the craft as drawn, and the caller puts `1.0` back once
    /// the frame is drawn.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn render(
        &mut self,
        gpu: &impl GpuContext,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        alpha: f32,
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
        blur_timestamps: Option<oag_post::motion_blur::ChainTimestamps<'_>>,
        hd_bloom_timestamps: Option<oag_post::hd_bloom::ChainTimestamps<'_>>,
    ) -> oag_present::perf::SceneStats {
        // The Zone stage grade, pointed at the zone the race has reached before
        // the frame is built - the same per-frame order `Zone_UpdateStage` runs
        // in on 2048, where it is called from the render update rather than the
        // simulation. See `race::Scene::sync_zone_grade`.
        //
        // A step rebuilds bind group 2 on the drawables that read the Zone
        // effect for real, so the stage's own textures follow the transition
        // sphere the same frame its colours do - see
        // `race::Scene::rebind_zone_art`. Gated on the edge `sync_zone_grade`
        // reports, not run every frame, the same "log the edge, not the
        // state" rule its own info! call follows.
        if self.scene.sync_zone_grade(&self.race) {
            self.scene.rebind_zone_art(gpu.device(), gpu.queue());
        }
        self.race.set_draw_alpha(alpha);
        // A multi-window race deals its views across the windows: this is window 1's. See
        // `Self::render_window`, which the other windows call too.
        if self.race.windowed() {
            return self.render_window(
                gpu,
                encoder,
                view,
                0,
                viewport,
                fov,
                cull,
                pvs_cull,
                anim_seconds,
                shadows,
                zone_spectrum,
                race::WindowEffects {
                    motion_blur,
                    camera_jitter,
                    timestamps,
                    blur_timestamps,
                    hd_bloom_timestamps,
                },
            );
        }
        // A split screen draws one scene per view, in command buffers of its own, so
        // `encoder` keeps only what the caller records after it: see
        // `Scene::render_viewports`. The blur runs once per view after the last, its
        // timestamp pair bracketing every chain; no scene timer. Each view jitters on its
        // seat's own sequence, and the upscaler resolves each view with a chain of its own.
        if self.race.view_state().viewport_count() > 1 {
            return self.scene.render_viewports(
                gpu.device(),
                gpu.queue(),
                view,
                &self.race,
                viewport,
                fov,
                cull,
                pvs_cull,
                anim_seconds,
                shadows,
                zone_spectrum,
                race::WindowEffects {
                    motion_blur,
                    camera_jitter,
                    blur_timestamps,
                    ..Default::default()
                },
                &mut [],
            );
        }
        // The scene and nothing else. The HUD used to follow it here, into the
        // same target; it composites at presentation resolution now - see
        // [`RaceStage::draw_hud`].
        self.scene.render(
            gpu.device(),
            gpu.queue(),
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

    /// Window `window`'s views of a multi-window race drawn into `view`: the one seat it holds
    /// full-frame (`Scene::render_view`, recorded into `encoder`), or several cut up inside it
    /// as a split screen cuts its views (`Scene::render_window_views`, which submits its own
    /// command buffers first). The caller submits `encoder` before the next window draws, and
    /// has set the draw alpha. `effects` is the window's blur, jitter and timers, for a window of
    /// one seat (a window of several is a split and draws the blur only). A window with no seats draws
    /// nothing.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn render_window(
        &mut self,
        gpu: &impl GpuContext,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        window: usize,
        viewport: (f32, f32, f32, f32),
        fov: display::Fov,
        cull: bool,
        pvs_cull: bool,
        anim_seconds: Option<f32>,
        shadows: display::Shadows,
        zone_spectrum: &[f32],
        effects: race::WindowEffects<'_>,
    ) -> oag_present::perf::SceneStats {
        // Pointed at the zone the race has reached, as `Self::render` does; a second window
        // of the same frame finds it already there and the rebuild is not repeated.
        if self.scene.sync_zone_grade(&self.race) {
            self.scene.rebind_zone_art(gpu.device(), gpu.queue());
        }
        let drawn = !self.race.window_drawn(window).is_empty();
        let stats = self.scene.render_window(
            gpu.device(),
            gpu.queue(),
            encoder,
            view,
            &self.race,
            window,
            viewport,
            fov,
            cull,
            pvs_cull,
            anim_seconds,
            shadows,
            zone_spectrum,
            self.shared_frame_work,
            effects,
            &mut [],
        );
        self.shared_frame_work |= drawn;
        stats
    }

    /// A new frame begins: no window has drawn yet, so the first to draw does the
    /// camera-independent work and the rest read it (`Self::shared_frame_work`).
    pub(crate) fn begin_frame(&mut self) {
        self.shared_frame_work = false;
    }

    /// What a temporal upscaler needs from this frame, or `None` when nothing
    /// has been drawn yet or jitter is off.
    ///
    /// **Read after [`RaceStage::render`], never before**: it reports the frame
    /// that was drawn, and a caller that asked first would hand the upscaler the
    /// *previous* frame's camera and offset - which produces a picture rather
    /// than an error.
    ///
    /// `window` is whose frame: 0 for a race that is not dealt across windows, and for window 1.
    pub(crate) fn temporal(&self, window: usize) -> Option<upscale::Temporal<'_>> {
        self.scene.temporal(window)
    }

    /// Hands the end screens the race's board when it was retaken since they were built:
    /// [`oag_raceplay::SimRace::refresh_results`]'s craft that crossed after the person, now
    /// with a time. Touches no race state; the screen keeps its page and its cursor.
    pub(crate) fn refresh_endrace_board(&mut self) {
        if let (Some(endrace_touch::EndRace::Disc(runtime)), Some(board)) =
            (&mut self.endrace, self.race.core.results())
        {
            runtime.refresh_field(board, self.race.core.results_revision());
        }
    }

    /// Reads the player's own outcome so far, straight off [`race::Race`]'s
    /// already-public state - never from inside a tick.
    ///
    /// The one place both capture sites (`Session::frame`'s finish-transition
    /// arm and `Session::escape`) build an
    /// [`crate::records::Observation`], so the field list only has to
    /// agree with `crate::records` in one place. See that module's own
    /// doc for why it takes primitives rather than a `&Race`.
    pub(crate) fn observation(&self) -> crate::records::Observation {
        let standing = self.race.core.player_standing();
        let finished = self.race.core.sim.finished();
        // A scored race (an HD Eliminator) is placed on its points, the board's own order, which
        // the results table, the headline, the voice and the record all read.
        let place = self
            .race
            .core
            .results()
            .filter(|board| board.scored())
            .and_then(|board| board.player())
            .map_or_else(|| self.race.core.player_place(), |row| row.place);
        crate::records::Observation {
            finished,
            place: Some(place),
            laps_completed: crate::records::laps_completed(
                standing.lap,
                finished,
                self.race.core.sim.world.laps_target(),
            ),
            // The racing clock, so a finish time - and the medal and record
            // it feeds - excludes the start-line countdown.
            tick: oag_race::race_clock_ticks(
                standing
                    .finish_tick
                    .unwrap_or(self.race.core.sim.world.tick),
            ),
            best_lap_ticks: standing.best_lap_ticks,
            campaign_medal: self
                .campaign_medal(finished, standing.best_lap_ticks)
                .or_else(|| self.campaign_2048_medal(finished)),
            campaign_difficulty: self
                .run
                .campaign_difficulty
                .map(Self::to_campaign_difficulty),
        }
    }

    /// [`Self::campaign_medal`]'s own counterpart for
    /// [`Self::campaign_2048_event`] - Wipeout 2048's own two-tier
    /// pass/elite law (`oag_2048::campaign::evaluate_tier`) rather than
    /// Pulse/HD's three-tier `Cell_EvaluateMedal`.
    ///
    /// **`Tier::Elite` maps to `Medal::Gold`, `Tier::Pass` to
    /// `Medal::Bronze` - chosen, not measured, no confidence score.** 2048's
    /// own objective law never authors a middle tier (measured - see
    /// `oag_2048::campaign::objective_type`'s own doc comment), so
    /// `Medal::Silver` is never produced by this function; floor maps to
    /// floor and top to top rather than squashing the two into adjacent
    /// rungs. See `docs/architecture/persistence.md`'s own 2048 paragraph.
    fn campaign_2048_medal(&self, finished: bool) -> Option<crate::records::Medal> {
        let progress = self.campaign_2048_event.as_ref()?;
        let objectives = progress.objectives.as_ref()?;
        let standing = self.race.core.player_standing();
        let outcome = oag_2048::campaign::EventOutcome {
            finished,
            place: self.race.core.player_place(),
            finish_centiseconds: finished.then(|| {
                ticks_to_centiseconds(oag_race::race_clock_ticks(
                    standing
                        .finish_tick
                        .unwrap_or(self.race.core.sim.world.tick),
                ))
            }),
            zone: self.race.core.sim.world.primary_race().zone,
            kills: standing.kills,
            score: standing.score as u32,
        };
        match oag_2048::campaign::evaluate_tier(objectives, &outcome)? {
            oag_2048::campaign::Tier::Elite => Some(crate::records::Medal::Gold),
            oag_2048::campaign::Tier::Pass => Some(crate::records::Medal::Bronze),
        }
    }

    /// The medal this race earns against the run's campaign cell, or `None`
    /// with no cell in play.
    ///
    /// **The value evaluated per mode, and why two of the five gate on
    /// `finished`.** Confirmed against `Data\Plugins\grids\grid_00.xml`'s
    /// own authored targets, not merely inferred from the field list:
    ///
    /// - `Race` - the finishing place, only once `finished` - a running
    ///   position mid-race is not a result.
    /// - `Time Trial` - the finish tick, converted to centiseconds, only
    ///   once `finished`. Its own gold targets (`10000`/`11500` on two
    ///   `16_Track`/`18_Track` cells) sit in the 100-120 s range a **3-lap**
    ///   Venom race actually takes (`race-modes.md` measures one ending
    ///   "around tick 7,500", 125 s) - a *total* race time, not a lap.
    /// - `Speed Lap` - the best single lap, converted the same way,
    ///   whenever one exists, gated on nothing: the mode never finishes, so
    ///   the alternative (the tick at whatever moment the race was left) is
    ///   not "how fast", it is "how long the player happened to stay" -
    ///   confirmed the same way: a `03_Track` Speed Lap cell's gold is
    ///   `4000` (40 s) against `laps="7"`, a fifth of a 7-lap total at the
    ///   same pace and squarely one lap's own length.
    /// - `Zone` - the zone counter reached so far (`RaceState::zone`, which
    ///   counts *completed* 10-second steps - `advance_zone` increments it
    ///   only after each one finishes, so no off-by-one), gated on nothing:
    ///   it only ever grows, so a value read at any moment is a real
    ///   "zones survived", the same "never ends, escape leaves" shape Speed
    ///   Lap has. `grid0_4_2`'s own `20`/`17`/`15` targets are exactly the
    ///   [`oag_tables::race_campaign::Cell::evaluate_medal`] doc's own
    ///   worked example.
    /// - `Elimination` - the player's own kill count, gated on nothing, for
    ///   the same "only ever grows" reason as `Zone`.
    ///
    /// - `Tournament` - [`Self::tournament_final_rank`], which is `Some`
    ///   only on the cell's own last leg, once that leg has finished -
    ///   exactly `tournament.md`'s "the medal-eligible value is the final
    ///   standings rank... only on the tournament's last leg" law. `None` on
    ///   every earlier leg, which this method reads as "no medal yet" the
    ///   same way an unfinished `Race` does.
    /// - `Head2Head` - the finishing place, exactly like `Race`, gated on
    ///   `finished` the same way. Confirmed by the census, not merely
    ///   inferred from the shared value shape: all 23 authored cells carry
    ///   `Gold Target="1"`/`Silver Target="0"`/`Bronze Target="0"` - win or
    ///   nothing - and `evaluate_medal`'s own three-way compare against a
    ///   finishing position (always `>= 1`) can never satisfy a `0` target
    ///   by accident. See `docs/ghidra/functions/psp-pulse-usa/head2head.md`.
    ///
    /// `Detonator` - the cannon's score (`detonator_score`), once the race is finished, compared
    /// `>=` against the cell's targets (a counting mode; the title's `counting_modes`).
    ///
    /// Everything else - `Custom Grid`, `AI Race`, `NitroBattle` - is `None`:
    /// the run's campaign cell never carries one of those, since
    /// `crate::campaign::race_mode_for_cell` refuses to map them onto a
    /// launch in the first place.
    fn campaign_medal(
        &self,
        finished: bool,
        best_lap_ticks: Option<u32>,
    ) -> Option<crate::records::Medal> {
        let tier = self.campaign_tier(finished, best_lap_ticks)?;
        Some(crate::unlock::to_records_medal(
            self.campaign_tiers().class(tier),
        ))
    }

    /// The medal tiers this race's campaign awards: a mod campaign's own list, else the title's.
    pub(crate) fn campaign_tiers(&self) -> &oag_tables::race_campaign::MedalTiers {
        self.run.campaign_custom.as_ref().map_or(
            oag_tables::race_campaign::MedalTiers::standard(),
            |launch| &launch.tiers,
        )
    }

    /// The name of the tier this race has earned on its campaign cell, as the campaign's own
    /// list calls it; recorded under a mod campaign's title by
    /// `Launch::record_result`.
    pub(crate) fn campaign_tier_name(&self) -> Option<String> {
        let standing = self.race.core.player_standing();
        let tier = self.campaign_tier(self.race.core.sim.finished(), standing.best_lap_ticks)?;
        Some(self.campaign_tiers().get(tier)?.name.clone())
    }

    /// The tier this race earns against the run's campaign cell: [`Self::campaign_medal`]'s law,
    /// as a position in [`Self::campaign_tiers`].
    fn campaign_tier(
        &self,
        finished: bool,
        best_lap_ticks: Option<u32>,
    ) -> Option<oag_tables::race_campaign::Medal> {
        use oag_tables::race_campaign::Mode as CampaignMode;
        let cell = self.run.campaign_cell.as_ref()?;
        let value = match cell.mode {
            CampaignMode::Race | CampaignMode::Head2Head if finished => {
                Some(i64::from(self.race.core.player_place()))
            }
            CampaignMode::TimeTrial if finished => {
                let tick = self
                    .race
                    .core
                    .player_standing()
                    .finish_tick
                    .unwrap_or(self.race.core.sim.world.tick);
                Some(ticks_to_centiseconds(oag_race::race_clock_ticks(tick)))
            }
            CampaignMode::Race | CampaignMode::Head2Head | CampaignMode::TimeTrial => None,
            CampaignMode::SpeedLap => {
                best_lap_ticks.map(|ticks| ticks_to_centiseconds(u64::from(ticks)))
            }
            CampaignMode::Zone => crate::medal_watch::live_value(&cell.mode, &self.race),
            // Kills on a title that flips Elimination, the place on the end-of-race board
            // elsewhere (HD, Omega): `crate::medal_watch::elimination_value`.
            CampaignMode::Elimination => crate::medal_watch::elimination_value(
                self.title.campaign.counting_modes,
                self.run.campaign_custom.is_some(),
                finished,
                self.race.core.player_standing().kills,
                self.race.core.eliminator_place(),
            ),
            CampaignMode::Tournament => self.tournament_final_rank.map(i64::from),
            // A Detonator cell's medal is the cannon's score
            // (`crate::medal_watch::detonator_value`).
            CampaignMode::Other(ref name) if name == crate::campaign::DETONATOR_CELL_MODE => {
                crate::medal_watch::detonator_value(finished, self.race.core.sim.detonator_score())
            }
            CampaignMode::CustomGrid | CampaignMode::AiRace | CampaignMode::Other(_) => None,
        }?;
        // Evaluated against whichever rung the cell was actually launched
        // at (`Session::launch_campaign_cell`'s own doc), falling back to
        // `Medium` - the same rung `Cell::evaluate_medal` itself always
        // used - for the cells the run's rung is `None` on:
        // Pulse, and any flat-schema HD cell, both of which
        // `Cell::targets_for_difficulty` already answers identically for
        // every rung.
        let difficulty = self
            .run
            .campaign_difficulty
            .unwrap_or(oag_tables::race_campaign::Difficulty::Medium);
        match &self.run.campaign_custom {
            Some(launch) => launch.tier_targets.evaluate(&cell.mode, value, difficulty),
            None => cell.evaluate_medal_for_difficulty(
                value,
                difficulty,
                self.title.campaign.counting_modes,
            ),
        }
    }

    /// [`oag_tables::race_campaign::Difficulty`] -> `crate::records::Difficulty`,
    /// the same shape the inline `Medal` match just above takes, restated
    /// as its own function only because it has two call sites
    /// ([`Self::observation`] and `Self::campaign_medal`'s own fallback
    /// would make a third inline copy read oddly next to each other).
    fn to_campaign_difficulty(
        difficulty: oag_tables::race_campaign::Difficulty,
    ) -> crate::records::Difficulty {
        match difficulty {
            oag_tables::race_campaign::Difficulty::Easy => crate::records::Difficulty::Easy,
            oag_tables::race_campaign::Difficulty::Medium => crate::records::Difficulty::Medium,
            oag_tables::race_campaign::Difficulty::Hard => crate::records::Difficulty::Hard,
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
    /// from `gpu.output.config` here, because it used to be and that is exactly what
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
        gpu: &impl GpuContext,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        viewport: (f32, f32, f32, f32),
        target_size: (u32, u32),
    ) {
        oag_gpu::skip::arm_writes(gpu.device());
        if oag_gpu::skip::skipped("hud") {
            return;
        }
        if self.draw_end_screens(gpu, encoder, view, viewport, target_size) {
            return;
        }
        self.draw_track_panel(
            gpu,
            encoder,
            view,
            viewport,
            self.race.views_drawn(),
            self.race.cut(),
        );
        self.draw_window_hud(gpu, encoder, view, viewport, target_size, 0);
    }

    /// The attract demo's screen over `view`, `seconds` into the demo. Nothing off a demo, in a
    /// wallpaper (which has no one to tell to press a button) or with no screen to draw.
    pub(crate) fn draw_demo_screen(
        &mut self,
        gpu: &impl GpuContext,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        viewport: (f32, f32, f32, f32),
    ) {
        let Some(run) = self.demo.as_ref().filter(|run| !run.wallpaper()) else {
            return;
        };
        let seconds = run.ticks() as f32 / 60.0;
        if let Some(screen) = &mut self.demo_overlay {
            screen.draw(gpu.device(), gpu.queue(), encoder, view, seconds, viewport);
        }
    }

    /// The race's HUD alone over `view`: photo mode with its HUD row on, and a parked race under
    /// the pause screen's tint. No end screens and no track panel, which are not part of the view.
    pub(crate) fn draw_view_hud(
        &mut self,
        gpu: &impl GpuContext,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        viewport: (f32, f32, f32, f32),
        target_size: (u32, u32),
    ) {
        self.draw_window_hud(gpu, encoder, view, viewport, target_size, 0);
    }

    /// What the race shows once it is over, over `view`, and whether it drew anything: the
    /// disc's own EndRace screens when this title carries `EndRace_Definition.xml` and it read
    /// (see `RaceStage::endrace`'s own doc), else the built-in results table. `false` while the
    /// race runs, where the caller draws the panel and the HUD instead. **Every window of a
    /// multi-window race calls this** and draws the one shared flow at its own size and aspect
    /// (`viewport` is the window's aspect rectangle); the flow is one state, so a press in any
    /// window moves the one menu. **Chosen, not measured.**
    pub(crate) fn draw_end_screens(
        &mut self,
        gpu: &impl GpuContext,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        viewport: (f32, f32, f32, f32),
        target_size: (u32, u32),
    ) -> bool {
        if let Some(done) = self.benchmark.as_ref().and_then(|live| live.done.as_ref()) {
            let say =
                |id: &str, english: &str| done.strings.get_or_fallback(id, english).to_string();
            let list = crate::benchmark::screen::draw_list(
                &done.report,
                self.scoreboard.face(),
                done.focus,
                done.saved.as_deref(),
                &say,
                done.view,
            );
            self.scoreboard
                .draw_list(gpu.device(), gpu.queue(), encoder, view, &list, viewport);
            return true;
        }
        if let Some(endrace) = &mut self.endrace {
            endrace.draw(
                gpu.device(),
                gpu.queue(),
                encoder,
                view,
                viewport,
                target_size,
            );
            return true;
        }
        let Some(board) = self.race.core.results() else {
            return false;
        };
        self.scoreboard.draw(
            gpu.device(),
            gpu.queue(),
            encoder,
            view,
            board,
            self.personal_best.as_ref(),
            viewport,
        );
        true
    }

    /// The track panel over `view`, laid out against the first of `count` views cut `cut` in
    /// `viewport`: the circuit's name and blurb while the flyby plays, then fading behind the
    /// chase view. Nothing when there is no panel to draw or it is not on screen.
    pub(crate) fn draw_track_panel(
        &mut self,
        gpu: &impl GpuContext,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        viewport: (f32, f32, f32, f32),
        count: usize,
        cut: oag_render::viewport::Cut,
    ) {
        if let (Some(panel), Some(progress)) =
            (&mut self.track_panel, self.race.track_panel_progress())
        {
            panel.draw(
                gpu.device(),
                gpu.queue(),
                encoder,
                view,
                progress,
                crate::hud_views::panel_viewport(count, cut, viewport),
            );
        }
    }

    /// The track panel on window `window` of a multi-window race, past window 1 (which
    /// [`Self::draw_hud`] draws): laid out for that window's own views, so it fits that window's
    /// size and aspect. A window with no seat to show draws none.
    pub(crate) fn draw_window_track_panel(
        &mut self,
        gpu: &impl GpuContext,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        viewport: (f32, f32, f32, f32),
        window: usize,
    ) {
        let count = self.race.window_drawn(window).len();
        if count == 0 {
            return;
        }
        let cut = self.race.window_cut(window);
        self.draw_track_panel(gpu, encoder, view, viewport, count, cut);
    }

    /// The race HUD of window `window` over `view`, and the start-line countdown: everything
    /// [`Self::draw_hud`] draws while the race runs, for the seats a window shows. A multi-window
    /// race's further windows call this beside [`Self::draw_window_track_panel`], after
    /// [`Self::draw_end_screens`] has said the race is still running. `window` is not read when
    /// the race is not windowed.
    pub(crate) fn draw_window_hud(
        &mut self,
        gpu: &impl GpuContext,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        viewport: (f32, f32, f32, f32),
        target_size: (u32, u32),
        window: usize,
    ) {
        if let Some(hud) = &mut self.hud
            && self.race.hud_shown()
        {
            // A title with no countdown art says the count in its own HUD face
            // (`oag_title::hud::ResumeCountdown::Text`), inside the HUD's own
            // frame so it is not overwritten by it.
            let word = self
                .resume_art
                .filter(|_| hud.resume_countdown() == oag_title::hud::ResumeCountdown::Text)
                .map(|seconds| {
                    crate::pause::Count::at(seconds)
                        .text(&self.resume_go)
                        .to_string()
                });
            crate::hud_views::draw(
                hud,
                gpu.device(),
                gpu.queue(),
                encoder,
                view,
                &self.race,
                &self.scene,
                window,
                self.run.campaign_cell.as_ref(),
                self.record_target.as_ref(),
                word.as_deref(),
                viewport,
            );
            // The countdown, for exactly the measured start-line gate's
            // span and no other window - see `crate::hud_countdown`
            // and `oag_race::RaceState::thrust_gated`, whose own doc
            // names the three live captures behind the 272-tick figure.
            // Not gated further by mode: the capture that measured it
            // was Time Trial alone, and `thrust_gated` already applies
            // it to every mode on the same reasoning
            // `RaceState::eliminate` uses, so this follows it rather
            // than adding a second, narrower guess on top.
            //
            // **Not while the gantry is showing it.** The countdown
            // belongs on the start gantry - the original's, and what `race::gantry` now draws -
            // so this overlay is the fallback for a circuit that
            // stands no gantry, not a second countdown beside one.
            //
            // **A resume countdown draws the same art on its own clock,
            // wherever on the track the race was paused.**
            let start_line = (!self.scene.draws_gantry()
                && oag_race::RaceState::thrust_gated(self.race.core.sim.world.tick))
            .then_some(self.race.core.sim.world.tick as f32 / 60.0);
            // The start's is one picture over the whole screen; a resume's is
            // one per view of a split, each centred as it would be alone
            // (chosen: a split's seam is no place for a digit). **In a multi-window
            // race a window's views are its own**: one picture per view the window
            // cuts, so a window of one seat draws one over the whole of it.
            let (views, cut) = if self.race.windowed() {
                (
                    self.race.window_drawn(window).len().max(1),
                    self.race.window_cut(window),
                )
            } else {
                (self.race.view_state().viewport_count(), self.race.cut())
            };
            let rects = if self.resume_art.is_some() {
                oag_render::viewport::layout(views, viewport, cut)
            } else {
                vec![viewport]
            };
            // Only a resume is placed here; the start's is the authored one.
            let anchor =
                (self.resume_art.is_some() && views > 1).then(|| crate::hud_countdown::Anchor {
                    across: 0.5,
                    down: crate::pause::anchor_y(views),
                    scale: crate::pause::SPLIT_ART_SCALE,
                });
            // Detonator's medal: the best the score has reached, in its widget's rectangle, where
            // the HUD's safe area puts it. One view only: a split screen has no Detonator.
            if let Some(medals) = &mut self.medals {
                let medal = self.race.detonator_medal();
                let seconds = self.race.core.sim.world.tick as f32 / 60.0;
                medals.draw(
                    gpu.device(),
                    gpu.queue(),
                    encoder,
                    view,
                    medal,
                    seconds,
                    hud.space(),
                    &|rect| hud.place_rect(rect),
                    viewport,
                    target_size,
                );
            }
            if let Some(countdown) = &mut self.countdown
                && let Some(seconds) = self.resume_art.or(start_line)
            {
                for rect in rects {
                    countdown.draw(
                        gpu.device(),
                        gpu.queue(),
                        encoder,
                        view,
                        seconds,
                        anchor,
                        rect,
                        target_size,
                    );
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
    /// validation error - see `crate::hud_countdown::Countdown::draw`'s doc for why
    /// that pass in particular is where it shows.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn warm_up(
        &mut self,
        gpu: &impl GpuContext,
        target: &wgpu::TextureView,
        target_size: (u32, u32),
        viewport: (f32, f32, f32, f32),
        fov: display::Fov,
        cull: bool,
        pvs_cull: bool,
        anim_seconds: Option<f32>,
    ) {
        let mut encoder = gpu
            .device()
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("race warmup"),
            });
        self.render(
            gpu,
            &mut encoder,
            target,
            // The tick exactly: a warmup draws the grid pose once, to pay the
            // pipeline compiles, and there is no previous tick to blend from.
            1.0,
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
        gpu.queue().submit(Some(encoder.finish()));
        // WebGPU cannot block on a submit (`PollType::Wait` is always `CantWait` there), so the
        // browser's driver compiles in the background as it always did.
        #[cfg(not(target_arch = "wasm32"))]
        if let Err(e) = gpu.device().poll(wgpu::PollType::wait_indefinitely()) {
            // Not fatal: the worst outcome is the compile this call exists to
            // avoid happening on the first real frame instead, exactly as it
            // did before this existed.
            log::warn!("could not wait for the race scene's warmup submit: {e}");
        }
    }
}
