//! Ending a race: the finish condition, and the table it leaves behind.
//!
//! The rule itself is `oag_race`'s - [`oag_race::RaceState::finished`] is set by
//! the lap counter when the player crosses for the last time, and
//! [`oag_race::places`] orders the field. Nothing here re-derives either. What
//! this adds is the *moment*: the tick that flag flips is when the board is
//! taken, and from then on the race has a result.
//!
//! **The simulation is not stopped here.** [`Race::tick`] goes on doing exactly
//! what it did before, byte for byte, so nothing that ticks a race for a fixed
//! count - the trace harness, a headless capture, the disc-backed AI tests -
//! sees a different world than it did. Standing still once the race is over is
//! the *composition root's* call, on the same argument that keeps the camera
//! cycle outside the tick: see `Session::frame`, which stops calling `tick` once
//! [`Race::finished`] answers yes - **unless the player crossed the line**
//! ([`Race::runs_on_after_the_line`]), where the original keeps the race running
//! behind the panels and so does this.

use super::*;

use crate::scoreboard::{self, Board, Craft};

/// What a finished run reports about itself that the world does not keep, for
/// `EndRace Results`: the Zone table's two tallies and the lap table's third column.
///
/// **Presentation-side state**, on [`RaceView`]: no field reaches
/// [`RaceSim::state_hash`], so keeping a tally here cannot move a hash or a
/// replay. It is fed from events the simulation already emits and reads nothing
/// back into it.
///
/// Only what this build can honestly count. Zone's `Laps cleared` and `Perfect laps`
/// (`Zone_Update`'s `+0x1a14`/`+0x1a16`) are the rest of that block and what steps them
/// is not recovered, so they stay off - see
/// `docs/ghidra/functions/psp-pulse-usa/endrace-screens.md`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RunStats {
    /// Zones that ended without a wall contact - the `outcome.perfect_zone` edge,
    /// `Zone_Update`'s `+0x1a12`.
    pub perfect_zones: u32,
    /// Speedup pads the player entered on each lap, the `boostimg` column.
    ///
    /// `Ship_ApplySpeedupPad` (`0x08848f9c`) adds one to `craft + 0x900 + lap * 0x10 +
    /// 0x94` each time a human-flown craft **enters a new pad** - the same edge that
    /// arms the exhaust flare and, in Zone, scores 100 - and `Race_BuildEndRaceResult`
    /// copies the four-or-five laps it reports into the third column. Read off a live
    /// write watchpoint (2026-09-30): the field's only writer is that instruction.
    /// Indexed by lap - 1, like [`oag_race::RaceState::lap_splits`] and bounded by the
    /// same [`oag_race::MAX_RECORDED_LAPS`].
    pub boosts_by_lap: [u32; oag_race::MAX_RECORDED_LAPS],
    /// The fastest the player's craft has been going, in hundredths of a unit a
    /// second, as `Zone_Update` keeps it: a `u16` of `speed * 100`, replaced when a
    /// larger one comes along.
    top_speed_centi: u16,
}

impl RunStats {
    /// Folds one tick's speed in. `speed` is the player's `|dot(velocity, forward)|`,
    /// which is `craft+0x2ec` (`docs/ghidra/functions/psp-pulse-usa/engine.md`) - the
    /// figure `Zone_Update` reads - though not sampled on the original's frame
    /// boundary, so the maximum can differ by a tick's worth of acceleration.
    pub(super) fn observe(&mut self, speed: f32) {
        // `(uint)(speed * 100.0)` masked to sixteen bits, compared as unsigned.
        let centi = ((speed * 100.0) as u32 & 0xffff) as u16;
        self.top_speed_centi = self.top_speed_centi.max(centi);
    }

    /// One speedup pad entered on `lap` (1-based, `RaceState::lap`'s own meaning). A lap
    /// past the recorded few is dropped, as its split is.
    pub(super) fn count_boost(&mut self, lap: u32) {
        let slot = usize::try_from(lap.saturating_sub(1)).unwrap_or(usize::MAX);
        if let Some(count) = self.boosts_by_lap.get_mut(slot) {
            *count += 1;
        }
    }

    /// `ER_TOP_SPEED`'s number: `top * 0xe10 / 100000` in integers, which is the same
    /// 3.6 the speedometer multiplies by.
    #[must_use]
    pub fn top_speed_kmh(self) -> u32 {
        u32::from(self.top_speed_centi) * 0xe10 / 100_000
    }
}

impl Race {
    /// Whether the race has reached its finish condition.
    ///
    /// **The player's own condition**, which is what ends the event: the lap
    /// counter sets it when they cross the line for the last time. An opponent
    /// crossing ahead of them finishes *that craft* - it takes a
    /// [`oag_race::Standing::finish_tick`] and stops being ordered by distance -
    /// and does not end the race.
    ///
    /// A speed lap and a Zone run have no lap target, so this stays `false` for
    /// them however long they run. See [`oag_race::Mode::laps_target`].
    #[must_use]
    pub fn finished(&self) -> bool {
        self.sim.world.primary_race().finished
    }

    /// Whether the world goes on being stepped under the end-race screens: the
    /// player crossed the line for the last time, so the craft is the driver's
    /// (see [`Race::flown_for_the_player`]) and the rest of the field is still
    /// lapping.
    ///
    /// **Only the line.** A race that ended any other way - the player's craft
    /// destroyed in a single race, an Eliminator's kill target, a Zone run - is
    /// left standing as before: the original's behaviour there was not measured
    /// (Time Trial and Single Race on Talon's Junction, 2026-10-01), and a wreck
    /// that respawns under the panels, or a Zone session that restarts itself,
    /// would be an invention. See `docs/gameplay/after-the-finish.md`.
    #[must_use]
    pub fn runs_on_after_the_line(&self) -> bool {
        self.finished() && self.sim.world.ships[self.player_slot()].standing.finished()
    }

    /// Whether the player's wreck in a **Single Race** is what ended the race: the
    /// one other ending whose world also goes on under the end-race screens.
    ///
    /// **The original never leaves `InGame` after that wreck** - measured
    /// 2026-10-02, 17,800+ frames with the field racing on, the HUD hidden and the
    /// spectator camera cutting, no legend and no panel
    /// (`docs/gameplay/after-the-finish.md`). Ours runs the world on in the same
    /// way and then shows `Race End Photo` and the panels, because a wrecked
    /// player needs a way out: **the maintainer's decision of 2026-10-02**, "hold,
    /// then results". That the legend shows at all, and when, is chosen, not
    /// measured. Zone is not included: it ends the same way but was not looked at.
    #[must_use]
    pub fn runs_on_after_the_wreck(&self) -> bool {
        self.finished()
            && self.sim.world.mode() == Mode::SingleRace
            && self.view.wreck_ended_tick.is_some()
    }

    /// Whether the world goes on being stepped under the end-race screens, for
    /// either ending that does it: the player's crossing
    /// ([`Self::runs_on_after_the_line`]) or a Single Race wreck
    /// ([`Self::runs_on_after_the_wreck`]).
    #[must_use]
    pub fn runs_on_after_the_end(&self) -> bool {
        self.runs_on_after_the_line() || self.runs_on_after_the_wreck()
    }

    /// One tick of a race that ended another way than the player's crossing:
    /// the world stands still, and only what a player can *see* keeps moving.
    ///
    /// A wreck that ends a Zone run is state 5 (a Single Race's runs the whole
    /// world, [`Race::runs_on_after_the_wreck`]), and the original goes on running
    /// behind the panels: the state-5 shake, the
    /// wreck's fire, the big explosion 1.5 s on and its shockwave ring, the
    /// screen flashes and the destroy camera's ease. They are all presentation
    /// state on [`RaceView`] (the particle stage, the shake, the flash, the blast
    /// pool, the camera's easing) and none of them reaches
    /// [`RaceSim::state_hash`]; this writes nothing under `self.sim`, `world.tick`
    /// included, so the standings and the board stay frozen exactly as
    /// [`Race::runs_on_after_the_line`]'s own rows say. In [`Race::tick`]'s order.
    ///
    /// **Chosen, not measured:** only the player's wreck was looked at, so every
    /// other thing the original's running world does under the panels (the
    /// opponents flying on, shots in flight, the engines' trails) is left
    /// standing as before.
    pub fn tick_cosmetics(&mut self) {
        let dt = self.sim.dt;
        self.view.shake.advance(dt);
        self.advance_bomb_blast_models(dt);
        self.advance_wreck_fx();
        self.view.stage.advance(dt, &mut self.view.stage_rng);
        self.advance_craft_flashes();
        self.advance_destroy_camera();
        let eye = self.camera_position();
        if let Some(flash) = &mut self.view.screen_flash {
            flash.advance(dt, eye);
        }
    }

    /// One tick of a finished race, for whoever drives the frame loop: the
    /// whole world after the player's last crossing ([`Race::runs_on_after_the_line`],
    /// neutral input because the panels consume the pad), only the cosmetics
    /// ([`Race::tick_cosmetics`]) after any other ending.
    pub fn tick_finished(&mut self) {
        if self.runs_on_after_the_end() {
            self.tick(&oag_gameplay::PlayerInputs::none());
        } else {
            self.tick_cosmetics();
        }
    }

    /// The results screens' tallies. See [`RunStats`].
    #[must_use]
    pub fn run_stats(&self) -> RunStats {
        self.view.run_stats
    }

    /// The results, once there are any.
    ///
    /// `None` for the whole race and `Some` from the finishing tick on. The
    /// table is taken at that instant and never revised - see [`RaceView::results`]'s
    /// field for why that matters.
    #[must_use]
    pub fn results(&self) -> Option<&Board> {
        self.view.results.as_ref()
    }

    /// Takes the board, on the tick the race finishes and only then.
    ///
    /// Called at the end of [`Race::tick`], after the standings have been
    /// advanced, so the last crossing is already in the table it reads.
    pub(super) fn capture_results(&mut self) {
        if !self.finished() || self.view.results.is_some() {
            return;
        }
        let places = self.places();
        // Every slot the grid has, whether or not the mode fielded a full one:
        // `ship_count` is 8 for a single race and 1 for the three single-ship
        // modes, and a slot beyond it holds no craft at all.
        //
        // The places are used exactly as `oag_race::places` assigned them,
        // gaps included, rather than renumbered over the rows that survive the
        // cut. That table is what the HUD's own position readout comes from,
        // and a scoreboard that renumbered would contradict the number the
        // player was just looking at.
        let crafts: Vec<Craft> = (0..usize::from(self.sim.world.ship_count))
            .map(|slot| {
                let ship = &self.sim.world.ships[slot];
                Craft {
                    slot: u8::try_from(slot).unwrap_or(u8::MAX),
                    place: places[slot],
                    lap: ship.standing.lap,
                    // The racing clock: the board prints this as the craft's
                    // race time, and the original's own clock starts on the
                    // release, not on the grid.
                    finish_tick: ship.standing.finish_tick.map(oag_race::race_clock_ticks),
                    // The craft's own clock rather than `RaceState`'s, which
                    // only times slot 0 - the two agree there, which is what
                    // `crates/game/tests/lap_times_ground_truth.rs` asserts.
                    best_lap_ticks: ship.standing.best_lap_ticks,
                }
            })
            .collect();
        self.view.results = Some(scoreboard::build(
            &crafts,
            self.sim.world.laps_target(),
            oag_race::race_clock_ticks(self.sim.world.tick),
        ));
    }
}

#[cfg(test)]
mod tests;
