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
//! **Measured** (`docs/ghidra/functions/psp-pulse-usa/eliminator-kill-target.md`,
//! `Ship_Damage`, confidence 85): a kill is credited on the blow that empties
//! the shield, when that blow is a weapon's, to the craft recorded as the
//! victim's last attacker - [`Race::credit_kill`]. A wall that finishes a craft
//! credits nobody, and an earlier wall scrape does not erase the attacker.
//!
//! **Chosen, and said so where it happens**: how the attacker id is derived.
//! [`RaceSim::last_damager`] is the craft whose weapon last hit the victim
//! directly, or whose blast last reached it ([`Race::credit_blast`]); which
//! blast a splash belongs to is re-derived from the impact whose radius the
//! victim was inside.
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
/// does not add it; when it is read, this is the constant that grows. **The
/// consequence runs the wrong way until then**: an Eliminator craft here
/// returns after `2.0` s from destruction against a single-race opponent's
/// `2.8`, where the original's Eliminator is the slower of the two.
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
                if mode == Mode::Eliminator {
                    self.credit_kill(slot);
                }
            }
            self.sim.respawn_delay[slot] -= self.sim.dt;
            if self.sim.respawn_delay[slot] > 0.0 {
                // Still down.
                continue;
            }

            // Counted in every mode a craft comes back in, not only the one
            // whose results screen has a row for it: a death is a fact about
            // the race, and `crates/game/tests/ai_clean_lap_gate.rs` reads it
            // to tell a craft that died and came back from one that never died.
            self.sim.world.ships[slot].standing.deaths += 1;
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
            self.sim.last_respawn_cause[slot] = Some(respawn::RespawnCause::Destroyed);
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
    /// staggered `WO_WEAPON_ABSORB` bursts, both through
    /// [`Race::play_absorb_feedback`]. See
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
        self.play_absorb_feedback(slot, false);
    }

    /// How much of its throttle an Eliminator opponent uses, so the field stays
    /// within reach of itself. `1.0` everywhere but the front of the pack.
    ///
    /// **Chosen, not measured; no confidence score.** The mode is won on kills,
    /// and a field of identical craft on one racing line strings itself out
    /// until nobody is inside shooting range of anybody. Measured on `16_Track`
    /// with the player parked, the spread between first and last opponent went
    /// from 118 units at the start to 3,900 by the six-minute mark. The
    /// original's own weapon AI fires readily only at a craft **inside 100
    /// units ahead** (`WeaponAi`'s skill score is `3` there and `0` otherwise,
    /// `docs/ghidra/functions/psp-pulse-usa/weapon-ai.md`), so it is a dense
    /// field the original's mode runs on. The original gets that density from
    /// an AI that is known to cheat; this gets some of it by **a leader that
    /// lets the pack catch up**: a craft with nobody ahead of it, and whose
    /// nearest opponent behind is more than [`PACK_REACH`] back, lifts off the
    /// throttle, down to [`PACK_MIN_THRUST`] once the pack is
    /// [`PACK_REACH`] + [`PACK_EASE_SPAN`] behind. The throttle is only ever
    /// *reduced*, so the AI obeys the player's physics and gets nothing the
    /// player's craft lacks.
    ///
    /// Only opponents count as the pack: the human's slot is left out, or a
    /// parked player would hold every opponent back. Craft that are down are
    /// ignored, and so is one more than [`PACK_LOST`] behind, which is a craft
    /// that has come off the circuit rather than one to wait for.
    pub(super) fn eliminator_pack_scale(&self, slot: usize) -> f32 {
        if self.sim.world.mode() != Mode::Eliminator {
            return 1.0;
        }
        let Some(course) = &self.sim.course else {
            return 1.0;
        };
        let racing = |other: usize| {
            other != 0
                && self.sim.world.ships[other].active
                && self.sim.world.ships[other].physics.craft_state
                    == oag_physics::CraftState::Racing
        };
        let mine = self.sim.world.ships[slot].standing.distance(course);
        let mut nearest_behind = f32::INFINITY;
        for other in (1..self.sim.world.ship_count as usize).filter(|&o| o != slot && racing(o)) {
            let gap = self.sim.world.ships[other].standing.distance(course) - mine;
            if gap >= 0.0 {
                return 1.0;
            }
            if -gap < PACK_LOST {
                nearest_behind = nearest_behind.min(-gap);
            }
        }
        if !nearest_behind.is_finite() {
            return 1.0;
        }
        let out_of_reach = ((nearest_behind - PACK_REACH) / PACK_EASE_SPAN).clamp(0.0, 1.0);
        1.0 - (1.0 - PACK_MIN_THRUST) * out_of_reach
    }

    /// Credits the craft that destroyed `victim`, the tick `victim` goes down.
    ///
    /// **Measured**, `Ship_Damage` (`0x088439ac`), confidence 85: on the blow
    /// that takes a shield to zero in game mode 8, and only when that blow's
    /// source is a weapon, the attacker recorded on the victim (`+0x4c`'s
    /// `+0x13c`, never cleared by anything read) has its kill counter
    /// (`+0x8d8`) raised, unless it is the victim itself. A wall that finishes
    /// a craft off credits nobody, and a wall scrape *earlier* does not erase
    /// the credit for a rocket that finishes it later - which this build used
    /// to, and which is why a third of Eliminator deaths credited nobody.
    ///
    /// **Ours**: the attacker id is [`RaceSim::last_damager`], set by a direct
    /// hit or by the nearest blast that reached the craft
    /// ([`Self::credit_blast`]); and "the fatal blow was a weapon" is a weapon
    /// hit within [`FATAL_BLOW_WINDOW_TICKS`] of the craft being seen down, since the
    /// destroyed sequence sits between the two.
    pub(super) fn credit_kill(&mut self, victim: usize) {
        let tick = self.sim.world.tick;
        let hit = self.sim.last_weapon_hit[victim];
        if hit == 0 || tick + 1 - hit > FATAL_BLOW_WINDOW_TICKS {
            return;
        }
        if let Some(killer) = self.sim.last_damager[victim]
            && killer as usize != victim
            && self.sim.world.ships[killer as usize].active
        {
            self.sim.world.ships[killer as usize].standing.kills += 1;
        }
    }

    /// Credits a blast's owner with every craft it hurt.
    ///
    /// The direct-hit half is in `Race::tick`; this is the splash. **Ours**: the
    /// original attributes the damage at the point it is applied, and this build
    /// applies a blast to every craft in its radius in one pass that does not
    /// say whose it was, so the owner is re-derived here as the impact whose
    /// radius the struck craft is inside.
    pub(super) fn credit_blast(
        &mut self,
        impact: &oag_gameplay::projectile::Impact,
        hits: &[oag_gameplay::projectile::WeaponHit; MAX_SHIPS],
    ) {
        if self.sim.world.mode() != Mode::Eliminator {
            return;
        }
        let Some(stats) =
            oag_gameplay::projectile::blast_stats(self.sim.weapons.as_ref(), impact.kind)
        else {
            return;
        };
        for (slot, hit) in hits
            .iter()
            .enumerate()
            .take(self.sim.world.ship_count as usize)
        {
            if slot == impact.owner as usize || !hit.landed {
                continue;
            }
            let ship = &self.sim.world.ships[slot];
            if !ship.active || (ship.physics.body.position - impact.point).length() > stats.radius {
                continue;
            }
            self.sim.last_damager[slot] = Some(impact.owner);
        }
    }
}

/// How many ticks before a craft is seen to be `Eliminated` a weapon hit may
/// have landed and still count as the blow that destroyed it.
///
/// **Measured on this build, not on the original**: a craft passes through its
/// half-second destroyed sequence (`DESTROYED_DURATION`, 30 ticks) between the
/// fatal blow and reaching `Eliminated`, and the probe read the gap at 28
/// ticks from the killing rocket, so forty covers it with a little to spare.
/// The original credits on the blow itself and needs no window.
const FATAL_BLOW_WINDOW_TICKS: u64 = 60;

/// How far behind the leader the nearest opponent may be before it eases off,
/// in units of track. Chosen, not measured.
const PACK_REACH: f32 = 60.0;

/// The extra gap over which the easing builds to its full effect. Chosen.
const PACK_EASE_SPAN: f32 = 120.0;

/// The fraction of its throttle a leader keeps at full easing. Chosen.
const PACK_MIN_THRUST: f32 = 0.3;

/// A craft further behind than this is not part of the pack. Chosen.
const PACK_LOST: f32 = 1500.0;
