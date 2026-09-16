//! What happens to a destroyed craft: Eliminator's respawn and the kill/death
//! bookkeeping that makes it a mode about kills rather than position, and a
//! single race's own opponent respawn, which the original runs through the
//! same craft states (5 then 6) in every mode and only the player's own
//! ending cuts short.
//!
//! Split out of `tick.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`, the same reason every other `race/*.rs`
//! module split out of `race.rs` in the first place.
//!
//! # What is measured here, and what is chosen
//!
//! **Measured** (`docs/ghidra/functions/psp-pulse-usa/shield.md`): a
//! completed lap refills [`LAP_REFILL_FRACTION`] of the maximum pool through
//! `Ship_AddShield`, with the absorb feedback -
//! [`Race::eliminator_lap_health_refill`].
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

/// How much of the skill-indexed maximum a completed lap gives back:
/// `Ship_RefillLapShield` (`0x0883de30`) is
/// `Ship_AddShield(stats_base[0x84 + skill * 4] * 0.2, entity)`, the same
/// table cell `Ship_ResetShield` fills the pool from. Confidence 85.
pub const LAP_REFILL_FRACTION: f32 = 0.2;

/// Seconds a craft spends in state 5, out of the race, before the respawn
/// timer even starts: `Ship_SetState`'s case 5 writes `1.5` into
/// `entity+0x874` and `Ship_UpdateDestroyed` (`0x08847650`) counts it down.
/// After `DESTROYED_DURATION`'s own half-second explosion. Confidence 85.
pub(super) const DESTROYED_DWELL: f32 = 1.5;

/// Seconds an AI craft then waits in state 6 before `Ship_UpdateRespawn`
/// (`0x08847914`) puts it back: `Ship_SetState`'s case 6 writes `0.8` for a
/// craft whose `entity+0x368` is set and `2.0` for the local player, who
/// never reaches it in a single race. Confidence 85.
pub(super) const AI_RESPAWN_WAIT: f32 = 0.8;

/// Seconds an Eliminator craft spends fully `Eliminated` before it returns.
///
/// [`DESTROYED_DWELL`] alone. The Eliminator's own state 8 sets a second
/// timer - `1.0` s for the local player, `2.0` s otherwise, the opposite
/// ratio to state 6 - and what counts it down was not found, so this build
/// does not add it; when it is read, this is the constant that grows.
pub(super) const ELIMINATOR_RESPAWN_DELAY: f32 = DESTROYED_DWELL;

impl Race {
    /// Brings a destroyed craft back, on the terms its mode sets.
    ///
    /// **Two modes bring one back, and the third does not.** In an Eliminator
    /// every craft returns after [`ELIMINATOR_RESPAWN_DELAY`], with the
    /// death and kill bookkeeping the mode is about. In a race with opponents,
    /// a single race, an *opponent* returns after
    /// [`DESTROYED_DWELL`] plus [`AI_RESPAWN_WAIT`], which is the original's
    /// own state 5 then state 6 (`Ship_UpdateDestroyed`, `Ship_UpdateRespawn`,
    /// `docs/ghidra/functions/psp-pulse-usa/shield.md`), and the player does
    /// not: the Arcade race's own update (`FUN_0882c5c4`) ends the race on
    /// the player's destroyed bit before state 6 can run, which is
    /// [`RaceState::eliminate`]'s job here. Zone, a time trial and a speed
    /// lap field nobody else, so only the player's own ending applies.
    ///
    /// Call once a tick, after every craft has been stepped
    /// ([`Race::step_opponents`] included) and before
    /// [`Race::update_standings`] reads a lap off any of them, so a craft
    /// that respawns this tick is placed where it is *put*, not where the
    /// explosion left it.
    pub(super) fn tick_destroyed_craft(&mut self) {
        let mode = self.sim.world.mode();
        let player = self.sim.world.primary_slot();

        for slot in 0..self.sim.world.ship_count as usize {
            if !self.sim.world.ships[slot].active {
                continue;
            }
            if self.sim.world.ships[slot].physics.craft_state != oag_physics::CraftState::Eliminated
            {
                // Not currently down - the countdown has nothing to run and
                // must not carry over into the *next* death, which would
                // shorten it.
                self.sim.respawn_delay[slot] = 0.0;
                continue;
            }
            let delay = if mode == Mode::Eliminator {
                ELIMINATOR_RESPAWN_DELAY
            } else if slot != player && mode.has_opponents() {
                DESTROYED_DWELL + AI_RESPAWN_WAIT
            } else {
                // The player's own ending, or a solo mode: `RaceState::eliminate`.
                continue;
            };

            if self.sim.respawn_delay[slot] <= 0.0 {
                self.sim.respawn_delay[slot] = delay;
            }
            self.sim.respawn_delay[slot] -= self.sim.dt;
            if self.sim.respawn_delay[slot] > 0.0 {
                // Still down.
                continue;
            }

            if mode == Mode::Eliminator {
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
            }

            // **A full pool, and for state 6 that is measured**:
            // `Ship_UpdateRespawn` calls `Ship_ResetShield` on its way back to
            // state 1. For the Eliminator's state 8 the consumer of its own
            // timer is unread, so the same refill there is the reading carried
            // over rather than one of its own.
            let dimensions = self.sim.world.ships[slot].handling.dimensions;
            oag_physics::damage::reset(&mut self.sim.world.ships[slot].physics, &dimensions);

            // The player's own last-known-good sample is latched separately
            // ([`RaceSim::last_on_track`]); an opponent's is read off its own
            // driver, the same lookup `Race::step_opponents` uses for its own
            // recovery. **The pose is the racing line's**, where
            // `Ship_UpdateRespawn` puts the craft at the AI corridor's
            // midpoint five units up, facing forty units down the tangent -
            // a stated departure, since `Race::respawn` is the one recovery
            // every path here shares.
            let sample_index = if slot == player {
                Some(self.sim.last_on_track as usize)
            } else {
                let index = self.sim.world.ships[slot].driver.index as usize;
                self.sample_index_of(index)
            };
            self.respawn(slot, sample_index);
        }
        if mode != Mode::Eliminator {
            return;
        }

        let target = self.sim.eliminator_kill_target;
        let kills = self.sim.world.ships[..self.sim.world.ship_count as usize]
            .iter()
            .filter(|ship| ship.active)
            .map(|ship| ship.standing.kills)
            .max()
            .unwrap_or(0);
        self.sim
            .world
            .primary_race_mut()
            .eliminator_finished(target, kills);
    }

    /// Refills a fifth of a craft's shield on a completed lap, Eliminator only.
    ///
    /// **Measured as of 2026-09-16, and this used to be a full refill chosen
    /// off `MSC_EVENT_ELIM`'s "you regain health after each lap".**
    /// `Eliminator_UpdateKillTarget` (`0x0882ce18`) and the multiplayer
    /// mode's racing-state update both call `Ship_RefillLapShield`
    /// (`0x0883de30`) on the player's craft when its crossing count
    /// (`craft+0xac8`) has gone up and `Ship_State` is `1`: that adds
    /// [`LAP_REFILL_FRACTION`] of the skill-indexed maximum through
    /// `Ship_AddShield`'s clamp, then plays the absorb feedback -
    /// `Ship_PlayAbsorbFeedback` (`0x08840640`), the `ABSORB` cue and the
    /// staggered `WO_WEAPON_ABSORB` bursts. The cue is raised here; the
    /// bursts are not yet drawn. See
    /// `docs/ghidra/functions/psp-pulse-usa/shield.md`.
    ///
    /// The original's `last_crossings != 0` gate - the grid-exit crossing
    /// refills nothing - is `RaceState::lap_gate`'s job here: `lap_completed`
    /// needs the near half and then the far half driven first, and the
    /// spawn-to-line crossing has driven neither.
    ///
    /// **The player's craft only in the original** - the mode object reads
    /// its own `+0x2c0`. Called for every slot's lap here, because an
    /// opponent that never healed would be an opponent the field wears down
    /// on its own; stated rather than hidden, and `slot` is what to narrow
    /// if that reading is wrong.
    pub(super) fn eliminator_lap_health_refill(&mut self, slot: usize) {
        if self.sim.world.mode() != Mode::Eliminator {
            return;
        }
        let ship = &mut self.sim.world.ships[slot];
        if ship.physics.craft_state != oag_physics::damage::CraftState::Racing {
            return;
        }
        let dimensions = ship.handling.dimensions;
        oag_physics::damage::add(
            &mut ship.physics,
            &dimensions,
            dimensions.shield * LAP_REFILL_FRACTION,
        );
        self.sim.cues.push(crate::audio::sfx::CueEvent::new(
            crate::audio::sfx::Cue::Absorb,
            slot,
        ));
    }
}
