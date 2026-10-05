//! What happens to a destroyed craft: Eliminator's respawn and the kill/death
//! bookkeeping that makes it a mode about kills rather than position, and the
//! absence of any respawn everywhere else - a craft destroyed in a single race
//! stays a wreck for good, the player and every opponent alike.
//!
//! Split out of `tick.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`, the same reason every other `race/*.rs`
//! module split out of `race.rs` in the first place.
//!
//! # A destroyed craft outside the Eliminator stays down
//!
//! **Measured live, 2026-10-02 (pulse-state6, PPSSPP v1.20.4, Pulse USA,
//! Single Race, two runs, two slots, two ways of destroying the craft)**: an AI
//! craft that reaches state 5 goes to state 6 after `1.5` s, its `entity+0x874`
//! timer (`0.8` s, `Ship_SetState`'s case 6) crosses zero about `0.8` s later,
//! and **the state stays 6 for the 13 seconds the capture watched, with the
//! destroyed bit `0x1000` set, the wreck as the live model, and the craft at
//! rest where it came down**. A write watchpoint on `entity+0x8C` saw two
//! writers in the whole run, both `Ship_SetState`, 4 to 5 and 5 to 6
//! (`ra` `0x08844598` and `0x088445e4`); nothing wrote it afterwards. That
//! agrees with the per-state jump table at `0x08a7bb88`, which sends state 6
//! to `FUN_08840500`, a bare timer countdown, and state 8 alone to
//! `Ship_UpdateRespawn` - and state 8 is the Eliminator's. So **a destroyed
//! craft in a single race does not come back**; this port used to return an
//! opponent after `1.5 + 0.8` s on a misreading of `shield.md`. Confidence 85:
//! two runs under injection (`Ship_SetState(4)` and `Ship_Damage`), 13 s each,
//! not a natural death by a weapon and not a full race to the flag. See
//! `docs/ghidra/functions/psp-pulse-usa/shield.md`.
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

/// Seconds the local player waits in the Eliminator's state 8: `Ship_SetState`'s
/// case 8 (`0x088446ec`) writes `1.0` into `entity+0x874` for the craft whose
/// `entity+0x368` is zero. State 8's own update is `Ship_UpdateRespawn`
/// (`0x08847914`): the per-state jump table at `0x08a7bb88` sends state `8` to
/// the call at `0x08841e44`, counts the timer down, and at zero relocates the
/// craft, refills the shield and goes to state 1. **Measured live 2026-10-02
/// (pulse-state6, run `e3`)**: the player's craft wrecked by `Ship_Damage` in an
/// Eliminator went 4 (0.5 s), 5 (1.5 s), 8 with `+0x874` reading `1.0`, then state 1
/// with a refilled shield 60 frames later. Confidence 90: one run, agreeing with the
/// disassembly (`lui 0x3F80` on the zero branch).
pub(super) const ELIMINATOR_PLAYER_WAIT: f32 = 1.0;

/// The same wait for every other craft: **`0.8`**, the same constant state 6's
/// case arms for a non-zero `entity+0x368`. **Measured live 2026-10-02
/// (pulse-state6, runs `e1` and `e2`, two boots of one race, slots 2 and 4)**: an
/// Eliminator opponent destroyed by `Ship_Damage` on PPSSPP went 4 (0.5 s), 5 (1.5 s),
/// 8 with `entity+0x874` reading `0.8`, then state 1 and a refilled shield `48` frames
/// later, both times. This used to be `2.0`, read off `Ship_SetState`'s case 8
/// with its branch-delay `lui 0x3F4C` (which executes on both paths) taken for
/// dead code. Confidence 92: two runs and the corrected disassembly agree.
pub(super) const ELIMINATOR_OPPONENT_WAIT: f32 = 0.8;

/// Seconds an Eliminator craft spends out before it returns: state 5's
/// [`DESTROYED_DWELL`] and then state 8's own wait, so `2.5` s for the local
/// player (whose destroy camera is still on the wreck for the second after the
/// big explosion) and `2.3` s for anyone else.
pub(super) fn eliminator_respawn_delay(is_player: bool) -> f32 {
    DESTROYED_DWELL
        + if is_player {
            ELIMINATOR_PLAYER_WAIT
        } else {
            ELIMINATOR_OPPONENT_WAIT
        }
}

impl Race {
    /// Brings a destroyed craft back, on the terms its mode sets.
    ///
    /// **Only the Eliminator brings one back.** Every craft there returns after
    /// [`eliminator_respawn_delay`] (the original's state 8,
    /// `Ship_UpdateRespawn`), with the death and kill bookkeeping the mode is
    /// about. In every other mode a destroyed craft stays down: the original
    /// sends it to state 6, a bare timer that no code revives (measured live,
    /// the module doc), so a single
    /// race's opponent is out for good like the player, whose own ending is
    /// the Arcade race's update (`FUN_0882c5c4`) reading the destroyed bit,
    /// [`RaceState::eliminate`]'s job here.
    ///
    /// Call once a tick, after every craft has been stepped
    /// ([`Race::step_opponents`] included) and before
    /// [`Race::update_standings`] reads a lap off any of them, so a craft
    /// that respawns this tick is placed where it is *put*, not where the
    /// explosion left it.
    pub(super) fn tick_destroyed_craft(&mut self) {
        self.tick_wreck_voice();
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
                eliminator_respawn_delay(slot == player)
            } else {
                // The player's own ending is `RaceState::eliminate`; an
                // opponent's wreck stays where it is (state 6, measured).
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
            // **A full pool, measured for the Eliminator's state 8**:
            // `Ship_UpdateRespawn` (state 8's update) calls `Ship_ResetShield`
            // on its way back to state 1.
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
                // **Where the wreck came to rest, not where the driver last
                // looked.** `Driver::drive` does not run while a craft is down,
                // so its index stops at the death and the wreck coasts on, up
                // to a hundred units. A respawn at the stale index teleports the
                // craft *backwards* over the start line if the wreck crossed it
                // meanwhile, which `Standing::update` reads as a backward wrap:
                // one lap lost and the gate reset, so the craft is one lap down
                // for the rest of the race, invisible to every other craft's
                // `Field` and out of reach of every pack rule. Measured on
                // `16_Track`, seed 5: the craft in slot 2 read one lap down
                // from tick 3025 to the end of the run. The original's `Ship_UpdateRespawn` places the
                // craft at the AI corridor's midpoint, which is a place near
                // the wreck, not a place near a stale index.
                // Searched in a window around the stale index and not the
                // whole ring: a circuit that passes over or under itself
                // within a wreck's reach must not put the craft back on the
                // other level. The window is the forward reach of a coast
                // (about 250 units, 100 samples at 2.5 units, measured on
                // `16_Track`) with room to spare.
                let index = self.line_of(slot).nearest(
                    self.sim.world.ships[slot].physics.body.position,
                    self.sim.world.ships[slot].driver.index as usize,
                    RESPAWN_SEARCH_WINDOW,
                );
                self.sample_index_for(slot, index)
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

    /// An absorb in the Eliminator: the held weapon is spent on a Shield, and
    /// no energy is paid.
    ///
    /// **Read statically, confidence 80** (instruction-level, not confirmed
    /// live). `Ship_AbsorbHeldPickup` (`0x08844ec4`), when `g_game_mode` is 8
    /// or `0x12` and the slot holds a weapon, calls `0x088612e8`, which writes
    /// `5` into both copies of the held id (`craft+0x1bc` and `+0x1c0`), then
    /// `Weapon_RequestFire` (`0x08862d9c`), whose case 5 sets fire bit `0x20` -
    /// `Shield_Fire` (`0x08861568`), the Shield's arm, by the stat offset it
    /// reads (`shield-pickup.md`). Every per-weapon arm of the absorb table then
    /// skips `Ship_AddShield` on the same flag, `0x08862bc0` clears the slot, and
    /// the tail skips `Ship_PlayAbsorbFeedback`. So the press arms the Shield
    /// pickup for `WeaponStats_Elimination.xml`'s Shield `time`, **1 s** (5 s in
    /// the race table), and the Shield is otherwise unobtainable here: its pad
    /// odds in that file are zero in every column. `MSC_EVENT_ELIM`'s "you
    /// cannot absorb pickups" is the energy half of this, and the mode used to
    /// refuse the press outright, keeping the weapon.
    ///
    /// A running Shield is not refreshed (`Shield_Fire`'s own guard, the same
    /// one the player's Shield arm follows), and the weapon is gone either way.
    /// Id 5 is the Cannon in `ai-stats.md`'s naming; what the press does is
    /// whatever bit `0x20` does, and that is the Shield. One edge is not ported:
    /// the original requests the Shield *before* its autopilot-cancel test, so
    /// an absorb under a running Autopilot also cancels it.
    pub(super) fn eliminator_absorb(&mut self, slot: usize) {
        let time = self
            .sim
            .weapons
            .as_ref()
            .and_then(|weapons| weapons.simple(oag_tables::weapons::Weapon::Shield))
            .map(|shield| shield.time);
        self.sim.world.ships[slot].pickup.take();
        if let Some(time) = time
            && self.sim.world.ships[slot].physics.shield_pickup_timer <= 0.0
        {
            self.sim.world.ships[slot].physics.shield_pickup_timer = time;
            self.view.shield[slot].activate();
        }
    }

    /// How much of its throttle an Eliminator opponent uses, so the field stays
    /// within reach of itself. `1.0` everywhere but the front of the pack.
    ///
    /// **Chosen, not measured; no confidence score.** The mode is won on kills,
    /// and a field of identical craft on one racing line strings itself out
    /// until nobody is inside shooting range of anybody. The original's weapon
    /// AI fires readily only at a craft **inside 100 units ahead** (`WeaponAi`'s
    /// skill score is `3` there and `0` otherwise,
    /// `docs/ghidra/functions/psp-pulse-usa/weapon-ai.md`), so it is a dense
    /// field the original's mode runs on. The original gets that density from
    /// an AI that is known to cheat; this gets it by **a leader that lets the
    /// pack catch up**: a craft with nobody ahead of it lifts off the throttle
    /// as its nearest follower falls back, down to [`PACK_MIN_THRUST`] once
    /// that follower is [`PACK_REACH`] + [`PACK_EASE_SPAN`] behind. The throttle
    /// is only ever *reduced*, so the AI obeys the player's physics and gets
    /// nothing the player's craft lacks. **Whether a throttle lift that exists
    /// only to bunch the field is acceptable is the maintainer's call**: it is
    /// catch-up by slowing the front. Measured with it off (`PACK_MIN_THRUST = 1.0`) on `16_Track`,
    /// player parked, eight fixed seeds, six game-minutes: 0 of 8 fields reach
    /// five kills, against 22 of 24 over three sets of eight with it on.
    ///
    /// **Why the earlier setting looked insensitive, 2026-10-02**: the scale was
    /// logged per tick per slot. At `PACK_REACH = 60` a leader with any follower
    /// inside 60 units ran full, which a tight pack is nearly always, so the
    /// easing acted on 73 of 1,680 samples; and every craft the respawn fault
    /// (see [`Race::tick_destroyed_craft`]) had put a lap down read thousands of
    /// units behind, past `PACK_LOST`, so it was not counted in the pack at all. The knob was
    /// sensitive all along - `PACK_REACH` 60 against 0 is 1 against 3 fields in
    /// eight finishing - the sample just never showed it.
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

    /// Raises `cont_elim` for a wrecked opponent, once per wreck.
    ///
    /// The original's only sound for an opponent's destruction: state 6's expiry
    /// plays it dry (`FUN_08840500`, `0x08840590`, confidence 90), `1.5 + 0.8 s`
    /// after the craft's state 5 begins - taken here as [`WRECK_VOICE_DELAY`]
    /// after the craft is first seen `Eliminated`. Every game mode except 2, 8 and
    /// 18; of those only 8 ("Elimination", [`Mode::Eliminator`]) exists in this
    /// build (`state-machine.md`'s `g_game_mode` table: 2 is Demo, 18 is
    /// Multiplayer Elimination), so every other mode raises it. The
    /// opponent's `EXPLSMALL`/`EXPLBIG` are not voiced. A title that does not
    /// voice its start has no speech bank loaded and raises nothing.
    fn tick_wreck_voice(&mut self) {
        if !self.sim.countdown_voice || self.sim.world.mode() == Mode::Eliminator {
            return;
        }
        let player = self.sim.world.primary_slot();
        for slot in 0..self.sim.world.ship_count as usize {
            let down = self.sim.world.ships[slot].active
                && self.sim.world.ships[slot].physics.craft_state
                    == oag_physics::CraftState::Eliminated;
            if slot == player || !down {
                self.sim.wreck_voice[slot] = 0.0;
                continue;
            }
            let timer = &mut self.sim.wreck_voice[slot];
            if *timer < 0.0 {
                continue;
            }
            if *timer == 0.0 {
                *timer = WRECK_VOICE_DELAY;
            }
            *timer -= self.sim.dt;
            if *timer <= 0.0 {
                *timer = -1.0;
                self.sim.cues.push(oag_sound::sfx::CueEvent::new(
                    oag_sound::sfx::Cue::ContElim,
                    player,
                ));
            }
        }
    }

    /// Notes that a weapon hit got through to `victim` this tick, for
    /// [`Self::credit_kill`].
    ///
    /// **Eliminator keeps only the blow that finished the craft.**
    /// `Ship_Damage` (`0x088439ac`) credits on the blow that empties the shield
    /// and only when its source is a weapon, so a hit that leaves the shield
    /// standing is not the killing blow, and a wall that finishes the craft
    /// afterwards must credit nobody. Reading the shield straight after the hit
    /// is that test; the time window in [`Self::credit_kill`] then only covers
    /// the destroyed sequence between the blow and the craft being seen down.
    /// Every other mode records every landed hit as before.
    pub(super) fn note_weapon_hit(&mut self, victim: usize) {
        if self.sim.world.mode() == Mode::Eliminator
            && self.sim.world.ships[victim].physics.shield > 0.0
        {
            return;
        }
        self.sim.last_weapon_hit[victim] = self.sim.world.tick + 1;
    }

    /// Records a hit by a weapon that is not a projectile - the LeachBeam's
    /// drain and the Quake's wave - as the original's pending-hit channel does:
    /// the shooter's index goes into the victim's `+0x13c` that `Ship_Damage`
    /// credits from (`LeachBeam_Drain` `0x08866804`; the Quake block of
    /// `FUN_088418e0`, `0x08841e60`), confidence 80. Eliminator only, where the
    /// original reads it; elsewhere nothing is written, so no other mode's
    /// state moves.
    pub(super) fn record_pending_hit(&mut self, victim: usize, attacker: u8) {
        if self.sim.world.mode() != Mode::Eliminator {
            return;
        }
        self.sim.last_damager[victim] = Some(attacker);
        self.note_weapon_hit(victim);
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
        impact: &oag_weapons::projectile::Impact,
        hits: &[oag_weapons::projectile::WeaponHit; MAX_SHIPS],
    ) {
        if self.sim.world.mode() != Mode::Eliminator {
            return;
        }
        let Some(stats) =
            oag_weapons::projectile::blast_stats(self.sim.weapons.as_ref(), impact.kind)
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

/// Seconds from a wrecked opponent's `Eliminated` to its `cont_elim`: state 5's
/// 1.5 s and state 6's 0.8 s, measured live (frames 167-168 from the injection,
/// less the 0.5 s of state 4 this build's `Eliminated` has already spent).
const WRECK_VOICE_DELAY: f32 = 2.3;

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
/// in units of track. Chosen, not measured. Zero: the leader never gets ahead
/// of its nearest follower by more than the span below.
const PACK_REACH: f32 = 0.0;

/// The extra gap over which the easing builds to its full effect. Chosen.
const PACK_EASE_SPAN: f32 = 30.0;

/// The fraction of its throttle a leader keeps at full easing. Chosen.
const PACK_MIN_THRUST: f32 = 0.1;

/// A craft further behind than this is not part of the pack. Chosen. Unchanged:
/// 300 and 1,500 both finish 22 of 24 seeds once a respawned craft keeps its lap.
const PACK_LOST: f32 = 1500.0;

/// How many racing-line samples forward of its stale index a destroyed craft's
/// respawn looks for the wreck. `RacingLine::nearest` also looks a quarter of
/// that back. Chosen, covering the 100 samples a wreck was measured to coast.
const RESPAWN_SEARCH_WINDOW: usize = 160;
