//! The race stage: a loaded track, and the scene that draws it.

use log::warn;
use oag_display::display;
use oag_game::medal_watch::ticks_to_centiseconds;
use oag_present::upscale;
use oag_raceplay as race;

use crate::gpu::GpuContext;

#[path = "race_stage/endrace.rs"]
pub(crate) mod endrace;
#[path = "race_stage/ghost.rs"]
pub(crate) mod ghost;
#[path = "race_stage/hd_loyalty.rs"]
pub(crate) mod hd_loyalty;
#[path = "race_stage/medal_message.rs"]
mod medal_message;

/// A race, and everything only it needs.
pub(crate) struct RaceStage {
    pub(crate) scene: race::Scene,
    pub(crate) race: race::Race,
    /// The HUD, or `None` when the disc's layout could not be read.
    pub(crate) hud: Option<oag_game::hud_overlay::Overlay>,
    /// The results table, drawn once the race has one.
    ///
    /// Not an `Option`, unlike the HUD: that one needs a layout off the disc and
    /// there may be none, where this needs only a font and
    /// `oag_ui::font::Atlas::build` always answers with the built-in 5x7 set.
    pub(crate) scoreboard: oag_game::scoreboard::Overlay,
    /// The start-line countdown's own `<Mode3D>` model, or `None` when this
    /// mode's layout carries no `Cockpit321Go` widget to place it by - see
    /// `oag_game::hud_countdown`.
    pub(crate) countdown: Option<oag_game::hud_countdown::Countdown>,
    /// The track-description panel over the pre-race flyby, or `None` off Pulse's PSP disc and
    /// when any part of it would not read - see `oag_game::track_panel`.
    pub(crate) track_panel: Option<oag_game::track_panel::Overlay>,
    /// Which circuit/mode/class row [`oag_game::records`] persists this
    /// race's outcome under - resolved once, at load, from
    /// [`oag_raceplay::Loaded::title`] and the [`oag_raceplay::Options`]
    /// that produced it, because neither is reachable from a finished or an
    /// escaped race any other way. See `Session::frame`'s finish-transition
    /// arm and `Session::escape`, the only two readers.
    pub(crate) result_key: oag_game::records::Key,
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
    /// is escaped rather than finished: [`oag_raceplay::Race::results`] is
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
    /// The campaign cell this race was launched against, or `None` for an
    /// ordinary RACE-page/RACE REMIX/`--race` launch. Drained from
    /// `Session::campaign_cell` into here the moment this stage is built
    /// (`crate::main::session::load::finish_loading`) - the mirror of
    /// `DAT_08b30ffc`, per
    /// `docs/ghidra/functions/psp-pulse-usa/race-campaign.md`'s "how a
    /// campaign event launches". Read by [`RaceStage::observation`] alone.
    pub(crate) campaign_cell: Option<oag_tables::race_campaign::Cell>,
    /// [`Self::campaign_cell`]'s own difficulty rung - `None` on every
    /// non-campaign launch, same as that field, and also `None` on a
    /// campaign cell with no rung to run at, per
    /// `Session::launch_campaign_cell`'s own doc. Read by
    /// [`RaceStage::campaign_medal`] alone.
    pub(crate) campaign_difficulty: Option<oag_tables::race_campaign::Difficulty>,
    /// [`Self::campaign_cell`]'s own resolved AI skill scale, drained the
    /// same moment and applied to [`Self::race`]'s AI tuning right after -
    /// see `Session::campaign_ai_skill_scale`'s own doc and
    /// `crate::main::session::load::advance_race_build`, the one place this
    /// is read.
    pub(crate) campaign_ai_skill_scale: Option<f32>,
    /// The final standings rank this leg earns against
    /// [`Session::tournament`], or `None` on every leg but a Tournament
    /// cell's own last one - see that field's own doc.
    ///
    /// **Read by [`RaceStage::campaign_medal`] alone**, and drained from
    /// `Session::tournament` at the same finish-transition
    /// (`Session::frame`'s `result_saved` guard) that
    /// `RaceStage::observation` is next built from - the mirror of
    /// [`Self::campaign_cell`]'s own "drained once, read once" shape, except
    /// this is written *after* the stage already exists rather than at
    /// build time, because the rank is not known until the leg itself
    /// finishes.
    ///
    /// [`Session::tournament`]: crate::main::session::Session::tournament
    pub(crate) tournament_final_rank: Option<u8>,
    /// The Wipeout 2048 campaign event this race was launched against, or
    /// `None` for every other title and for an ordinary `--race`/menu
    /// launch on 2048 itself. Resolved by `race::load_event` onto
    /// [`race::Loaded::campaign_2048_event`] and carried straight through -
    /// see that field's own doc for why this rides on `Loaded` rather than
    /// through `Session` the way [`Self::campaign_cell`] does.
    pub(crate) campaign_2048_event: Option<race::Campaign2048Progress>,
    /// The EndRace flow (`Results` -> `Rewards`/`Menu`) once the race has
    /// finished and this title carries `EndRace_Definition.xml` - `None`
    /// until then, and forever `None` on a title that does not (or on an
    /// open source [`endrace::EndRaceRuntime::new`] could not read), which
    /// falls this stage back to [`RaceStage::scoreboard`] exactly as before
    /// this existed. Built once, by `Session::frame`'s finish-transition
    /// arm - see `crate::main::session::endrace`. [`Self::endrace_unavailable`]
    /// is what makes that "forever" true rather than "retried every frame".
    pub(crate) endrace: Option<endrace::EndRaceRuntime>,
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
    /// [`Session::build_endrace`]: crate::main::session::Session::build_endrace
    pub(crate) endrace_unavailable: bool,
    /// The best medal [`Self::tick_messages`] has already announced, so a medal
    /// is raised once however many ticks it stays earned.
    pub(super) earned_medal: Option<oag_tables::race_campaign::Medal>,
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
        gpu: &impl GpuContext,
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
        let standing = self.race.player_standing();
        let finished = self.race.finished();
        oag_game::records::Observation {
            finished,
            place: Some(self.race.player_place()),
            laps_completed: oag_game::records::laps_completed(
                standing.lap,
                finished,
                self.race.sim.world.laps_target(),
            ),
            // The racing clock, so a finish time - and the medal and record
            // it feeds - excludes the start-line countdown.
            tick: oag_race::race_clock_ticks(
                standing.finish_tick.unwrap_or(self.race.sim.world.tick),
            ),
            best_lap_ticks: standing.best_lap_ticks,
            campaign_medal: self
                .campaign_medal(finished, standing.best_lap_ticks)
                .or_else(|| self.campaign_2048_medal(finished)),
            campaign_difficulty: self.campaign_difficulty.map(Self::to_campaign_difficulty),
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
    fn campaign_2048_medal(&self, finished: bool) -> Option<oag_game::records::Medal> {
        let progress = self.campaign_2048_event.as_ref()?;
        let objectives = progress.objectives.as_ref()?;
        let standing = self.race.player_standing();
        let outcome = oag_2048::campaign::EventOutcome {
            finished,
            place: self.race.player_place(),
            finish_centiseconds: finished.then(|| {
                ticks_to_centiseconds(oag_race::race_clock_ticks(
                    standing.finish_tick.unwrap_or(self.race.sim.world.tick),
                ))
            }),
            zone: self.race.sim.world.primary_race().zone,
            kills: standing.kills,
        };
        match oag_2048::campaign::evaluate_tier(objectives, &outcome)? {
            oag_2048::campaign::Tier::Elite => Some(oag_game::records::Medal::Gold),
            oag_2048::campaign::Tier::Pass => Some(oag_game::records::Medal::Bronze),
        }
    }

    /// The medal this race earns against [`Self::campaign_cell`], or `None`
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
    /// Everything else - `Custom Grid`, `AI Race` - is `None`:
    /// `Self::campaign_cell` never carries one of those, since
    /// `oag_game::campaign::race_mode_for_cell` refuses to map them onto a
    /// launch in the first place.
    fn campaign_medal(
        &self,
        finished: bool,
        best_lap_ticks: Option<u32>,
    ) -> Option<oag_game::records::Medal> {
        use oag_tables::race_campaign::Mode as CampaignMode;
        let cell = self.campaign_cell.as_ref()?;
        let value = match cell.mode {
            CampaignMode::Race | CampaignMode::Head2Head if finished => {
                Some(i64::from(self.race.player_place()))
            }
            CampaignMode::TimeTrial if finished => {
                let tick = self
                    .race
                    .player_standing()
                    .finish_tick
                    .unwrap_or(self.race.sim.world.tick);
                Some(ticks_to_centiseconds(oag_race::race_clock_ticks(tick)))
            }
            CampaignMode::Race | CampaignMode::Head2Head | CampaignMode::TimeTrial => None,
            CampaignMode::SpeedLap => {
                best_lap_ticks.map(|ticks| ticks_to_centiseconds(u64::from(ticks)))
            }
            CampaignMode::Zone => oag_game::medal_watch::live_value(&cell.mode, &self.race),
            CampaignMode::Elimination => Some(i64::from(self.race.player_standing().kills)),
            CampaignMode::Tournament => self.tournament_final_rank.map(i64::from),
            CampaignMode::CustomGrid | CampaignMode::AiRace | CampaignMode::Other(_) => None,
        }?;
        // Evaluated against whichever rung the cell was actually launched
        // at (`Session::launch_campaign_cell`'s own doc), falling back to
        // `Medium` - the same rung `Cell::evaluate_medal` itself always
        // used - for the cells `Self::campaign_difficulty` is `None` on:
        // Pulse, and any flat-schema HD cell, both of which
        // `Cell::targets_for_difficulty` already answers identically for
        // every rung.
        let difficulty = self
            .campaign_difficulty
            .unwrap_or(oag_tables::race_campaign::Difficulty::Medium);
        let medal = cell.evaluate_medal_for_difficulty(value, difficulty)?;
        Some(match medal {
            oag_tables::race_campaign::Medal::Gold => oag_game::records::Medal::Gold,
            oag_tables::race_campaign::Medal::Silver => oag_game::records::Medal::Silver,
            oag_tables::race_campaign::Medal::Bronze => oag_game::records::Medal::Bronze,
        })
    }

    /// [`oag_tables::race_campaign::Difficulty`] -> `oag_game::records::Difficulty`,
    /// the same shape the inline `Medal` match just above takes, restated
    /// as its own function only because it has two call sites
    /// ([`Self::observation`] and `Self::campaign_medal`'s own fallback
    /// would make a third inline copy read oddly next to each other).
    fn to_campaign_difficulty(
        difficulty: oag_tables::race_campaign::Difficulty,
    ) -> oag_game::records::Difficulty {
        match difficulty {
            oag_tables::race_campaign::Difficulty::Easy => oag_game::records::Difficulty::Easy,
            oag_tables::race_campaign::Difficulty::Medium => oag_game::records::Difficulty::Medium,
            oag_tables::race_campaign::Difficulty::Hard => oag_game::records::Difficulty::Hard,
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
        gpu: &impl GpuContext,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        viewport: (f32, f32, f32, f32),
        target_size: (u32, u32),
    ) {
        // The disc's own three EndRace screens, when this title carries
        // `EndRace_Definition.xml` and it read - see `RaceStage::endrace`'s
        // own doc. `scoreboard` stays this stage's fallback: a title with no
        // such screen, or whose read failed, draws exactly what it always
        // did.
        if let Some(endrace) = &mut self.endrace {
            endrace.draw(
                gpu.device(),
                gpu.queue(),
                encoder,
                view,
                viewport,
                target_size,
            );
            return;
        }
        match self.race.results() {
            Some(board) => self.scoreboard.draw(
                gpu.device(),
                gpu.queue(),
                encoder,
                view,
                board,
                self.personal_best.as_ref(),
                viewport,
            ),
            None => {
                if let (Some(panel), Some(progress)) =
                    (&mut self.track_panel, self.race.track_panel_progress())
                {
                    panel.draw(gpu.device(), gpu.queue(), encoder, view, progress, viewport);
                }
                if let Some(hud) = &mut self.hud
                    && self.race.hud_shown()
                {
                    let mut readout = self.race.readout();
                    // The rung the grade is showing, which is what names the
                    // speed class - see `Scene::zone_stage`.
                    readout.zone_stage = self.scene.zone_stage().unwrap_or(0);
                    readout.zone_next_in = self
                        .scene
                        .zones_to_next_stage(u16::try_from(readout.zone).unwrap_or(u16::MAX));
                    // A campaign cell races its own ladder; any other Time
                    // Trial or Speed Lap races the record. See
                    // `oag_hud::Readout::time_trial_pace`. `campaign_cell`
                    // is title-blind (HD reuses the same
                    // `oag_tables::race_campaign::Cell`), so nothing here
                    // stops it computing on HD - what keeps this Pulse-only is
                    // `record_target` (`stats.xml` is read off Pulse's PSP disc
                    // alone) and `draw.rs`'s widget-name match: no shipped HD
                    // layout authors a widget named `TotalTime`/`TotalTimeTxt`.
                    readout.time_trial_pace = oag_hud::pace_for(
                        readout.mode,
                        readout.race_ticks,
                        readout.lap_ticks,
                        self.campaign_cell.as_ref(),
                        self.record_target.as_ref(),
                    );
                    hud.draw(gpu.device(), gpu.queue(), encoder, view, &readout, viewport);
                    // The countdown, for exactly the measured start-line gate's
                    // span and no other window - see `oag_game::hud_countdown`
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
                        && oag_race::RaceState::thrust_gated(self.race.sim.world.tick)
                    {
                        countdown.draw(
                            gpu.device(),
                            gpu.queue(),
                            encoder,
                            view,
                            self.race.sim.world.tick as f32 / 60.0,
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
    /// validation error - see `oag_game::hud_countdown::Countdown::draw`'s doc for why
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
        if let Err(e) = gpu.device().poll(wgpu::PollType::wait_indefinitely()) {
            // Not fatal: the worst outcome is the compile this call exists to
            // avoid happening on the first real frame instead, exactly as it
            // did before this existed.
            warn!("could not wait for the race scene's warmup submit: {e}");
        }
    }
}
