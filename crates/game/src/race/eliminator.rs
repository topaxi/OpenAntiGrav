//! Eliminator's own tick: respawn after death, and the kill/death bookkeeping
//! that makes it a mode about kills rather than position.
//!
//! Split out of `tick.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`, the same reason every other `race/*.rs`
//! module split out of `race.rs` in the first place.
//!
//! # What is measured here, and what is chosen
//!
//! **Measured** (`docs/ghidra/functions/psp-pulse-usa/race-campaign.md`):
//! `Eliminator_UpdateKillTarget` ends the race the instant any craft's own
//! kill count reaches the target, read off the current campaign cell or a
//! global default; `MSC_EVENT_ELIM` says a destroyed craft comes back rather
//! than sitting out, and `ER_DEATHS` is the row that counts how often.
//!
//! **Chosen, and said so where it happens**: which craft's weapon gets
//! credited for a kill. Nothing in the read executable was traced for a kill
//! *attribution* mechanism - only that the count exists at `entity+0x8d8` -
//! so [`RaceSim::last_damager`] is this build's own rule: the most recent craft
//! to land a direct weapon hit on the one that died, cleared the moment a
//! wall or another cause deals damage instead so a stale hit is never
//! credited for an unrelated death. A death with no recent weapon hit (wall
//! contact, or a shot too long ago) credits nobody, the same as a real
//! deathmatch usually treats a self-inflicted or environmental kill.
//!
//! Also chosen: the respawn pose. [`Race::respawn`] is the off-track/`Reset`
//! recovery this file reuses rather than a pipeline of its own - same
//! confidence 40 caveat that method's own doc comment already carries, and
//! sharing it means [`RaceSim::respawns`]'s count now mixes "put back after
//! leaving the circuit" with "put back after dying", which a future reader
//! should not read as two different figures agreeing by coincidence.

use super::*;

/// Seconds a craft spends fully `Eliminated` before it returns to the race.
///
/// **Measured, separately from the explosion itself.**
/// [`oag_physics::CraftState::Eliminated`]'s own doc comment records the
/// original moving on "after 1.5 s, into a respawn or the Eliminator's kill
/// bookkeeping" - this is that 1.5 s, counted from the tick the craft reaches
/// `Eliminated` (i.e. *after* `DESTROYED_DURATION`'s own half-second
/// explosion has already run).
pub(super) const ELIMINATOR_RESPAWN_DELAY: f32 = 1.5;

impl Race {
    /// Runs Eliminator's own destroyed-craft handling for every active craft
    /// this tick. A no-op on every other mode.
    ///
    /// **Replaces [`RaceState::eliminate`] for this mode rather than calling
    /// it.** `RaceState::eliminate` is the right ending for Zone and a single
    /// race - a destroyed craft there does not come back, see that method's
    /// own doc comment - but Eliminator's own text is explicit that it does,
    /// so [`Race::tick`] branches on the mode before reaching either path.
    ///
    /// Call once a tick, after every craft has been stepped
    /// ([`Race::step_opponents`] included) and before
    /// [`Race::update_standings`] reads a lap off any of them, so a craft
    /// that respawns this tick is placed where it is *put*, not where the
    /// explosion left it.
    pub(super) fn tick_eliminator(&mut self) {
        if self.sim.world.race.mode != Mode::Eliminator {
            return;
        }

        for slot in 0..self.sim.world.ship_count as usize {
            if !self.sim.world.ships[slot].active {
                continue;
            }
            if self.sim.world.ships[slot].physics.craft_state != oag_physics::CraftState::Eliminated
            {
                // Not currently down - the countdown has nothing to run and
                // must not carry over into the *next* death, which would
                // shorten it.
                self.sim.eliminator_respawn_timer[slot] = 0.0;
                continue;
            }

            if self.sim.eliminator_respawn_timer[slot] <= 0.0 {
                self.sim.eliminator_respawn_timer[slot] = ELIMINATOR_RESPAWN_DELAY;
            }
            self.sim.eliminator_respawn_timer[slot] -= self.sim.dt;
            if self.sim.eliminator_respawn_timer[slot] > 0.0 {
                // Still down.
                continue;
            }

            self.sim.world.ships[slot].standing.deaths += 1;
            // A kill only counts against a *different* craft with a recent
            // hit on record - see this module's own doc comment for why a
            // stale or absent damager credits nobody.
            if let Some(killer) = self.sim.last_damager[slot].take()
                && killer as usize != slot
                && self.sim.world.ships[killer as usize].active
            {
                self.sim.world.ships[killer as usize].standing.kills += 1;
            }

            // **A full refill, chosen rather than recovered.** The original
            // charges a respawn instead: `Ship_SetState`'s state-3 branch
            // computes `clamp(shield - 1, 0, 5)`, and what consumes that
            // figure is unread - see `oag_physics::damage`'s own module doc.
            // A full pool is the un-punitive reading, not a measurement.
            let dimensions = self.sim.world.ships[slot].handling.dimensions;
            oag_physics::damage::reset(&mut self.sim.world.ships[slot].physics, &dimensions);

            // The player's own last-known-good sample is latched separately
            // ([`RaceSim::last_on_track`]); an opponent's is read off its own
            // driver, the same lookup `Race::step_opponents` uses for its own
            // recovery.
            let sample_index = if slot == 0 {
                Some(self.sim.last_on_track as usize)
            } else {
                let index = self.sim.world.ships[slot].driver.index as usize;
                self.sample_index_of(index)
            };
            self.respawn(slot, sample_index);
        }

        let target = self.sim.eliminator_kill_target;
        let kills = self.sim.world.ships[..self.sim.world.ship_count as usize]
            .iter()
            .filter(|ship| ship.active)
            .map(|ship| ship.standing.kills)
            .max()
            .unwrap_or(0);
        self.sim.world.race.eliminator_finished(target, kills);
    }

    /// Refills a craft's shield to full on a completed lap, Eliminator only.
    ///
    /// **The disc's own text, applied.** `MSC_EVENT_ELIM`: *"you cannot
    /// absorb pickups, but you regain health after each lap"* - the amount
    /// is not stated, so a full refill is what this build reads a mode with
    /// no partial-heal vocabulary anywhere else as meaning; chosen, not
    /// measured, and easy to find the day a real figure turns up. See
    /// [`Mode::pickups_absorb`] for the sentence's other half.
    pub(super) fn eliminator_lap_health_refill(&mut self, slot: usize) {
        if self.sim.world.race.mode != Mode::Eliminator {
            return;
        }
        let dimensions = self.sim.world.ships[slot].handling.dimensions;
        self.sim.world.ships[slot].physics.shield = dimensions.shield;
    }
}
