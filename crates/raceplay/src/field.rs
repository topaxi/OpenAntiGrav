//! The field: the racing line the drivers follow, what each craft can see of
//! the others, the standings that come out of it, and the two things an
//! opponent does with a pickup.
//!
//! Split out of `race.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change. Its tests are
//! `race/tests/field.rs`.

use super::*;

mod fire_law;
mod opponent_weapons;
mod pad_seek;

pub use fire_law::FireLaw;
pub use pad_seek::PadSeeking;
pub(super) use pad_seek::line_positions as pad_seek_positions;

/// How much of an Autopilot pickup is left when the announcer warns.
///
/// `Autopilot_Update`'s test is `now < 1.0 && previous > 1.0`, a literal in the
/// handler rather than anything the disc authors.
const AUTOPILOT_WARNING_SECONDS: f32 = 1.0;

impl Race {
    /// Replaces what the opponents are flown with, mid-race.
    ///
    /// **For measurement, and it is the only way to sweep a knob across a
    /// circuit.** The tuning is otherwise decided once at
    /// [`Self::start`] from the difficulty, which is right for a race and
    /// useless for finding out what a number is worth: a sweep that had to
    /// recompile between points could not be a test, and a tuning fitted on the
    /// one circuit that happens to be the default is how this page got a grip
    /// figure that was wrong on the other eleven.
    ///
    /// It is not a difficulty setting and nothing in the game calls it.
    pub fn set_ai_tuning(&mut self, tuning: oag_ai::Tuning) {
        self.sim.ai_tuning = tuning;
    }

    /// Replaces the character one slot is flying, mid-race.
    ///
    /// **For measurement, exactly as [`Self::set_ai_tuning`] is**, and it exists
    /// because a lone-craft benchmark measures whichever pilot
    /// `oag_ai::pilot_for_slot` happened to hand slot 1 - the same one on every
    /// circuit, since the choice is a function of the race seed and the slot.
    /// A sweep that wanted to see what a *shy* pilot costs against an
    /// *aggressive* one therefore had no way to ask, and would have read one
    /// character's numbers as the field's.
    ///
    /// The pilot is taken as given: [`Self::start`] has already run the level's
    /// `Difficulty::temper` over the roster, so a caller sweeping tiers has to
    /// temper it too. It is not a difficulty setting and nothing in the game
    /// calls it.
    pub fn set_ai_pilot(&mut self, slot: usize, pilot: oag_ai::Pilot) {
        if let Some(entry) = self.sim.ai_pilots.get_mut(slot) {
            *entry = pilot;
        }
    }

    /// Tempers `pilot` by `skill` and assigns it to the player's slot, for
    /// `--autopilot-pilot`.
    ///
    /// The same temper [`Self::start`] runs over an opponent's roster entry,
    /// so naming `aggressive` here is the same character a grid slot could
    /// actually draw at that skill - not a stronger or weaker claim than the
    /// roster makes. `skill` is `--autopilot-skill`'s choice, or the race's
    /// own AI difficulty when that flag was not given; see
    /// [`Self::set_autopilot_tuning`] for the other half of that flag.
    pub fn set_autopilot_pilot(&mut self, pilot: oag_ai::Pilot, skill: oag_ai::Difficulty) {
        self.set_ai_pilot(0, skill.temper(&pilot));
    }

    /// Overrides the tuning [`Self::autopilot_controls`] flies with, for
    /// `--autopilot-skill` - independent of [`RaceSim::ai_tuning`], which stays
    /// the opponents' own.
    ///
    /// **Why this cannot reuse [`Self::set_ai_tuning`]:** that setter replaces
    /// the tuning for the *whole field*, so a race with opponents would have
    /// them match the flag too. `--autopilot-skill` exists to feel one skill's
    /// numbers against the field a real race deals, not to replace the
    /// field's own setting mid-race.
    pub fn set_autopilot_tuning(&mut self, tuning: oag_ai::Tuning) {
        self.sim.autopilot_tuning = Some(tuning);
    }

    /// Hands the player's craft to an opponent's driver, or takes it back.
    ///
    /// **A verification aid, and the same kind of thing `Options::opponents`
    /// is** - a switch that exists so something can be *checked* without a human
    /// at the keyboard, not a mode the original has. A race ends when the player
    /// crosses the line for the last time, and until this existed the only way
    /// to reach that state at all was to drive three laps by hand: no test could
    /// assert on a finished race, and no `--screenshot` could show one.
    ///
    /// It replaces the mapped input snapshot outright rather than blending with
    /// it, so a run under autopilot is a run nobody is steering. It is flown with
    /// the *player's* recovery - [`Self::lost_off_the_track`], not the opponents'
    /// [`Self::lost_off_the_circuit`] - because it is still slot 0 going through
    /// [`Self::tick`]. That is the same net a human gets, which is what makes a
    /// measurement taken under it mean anything about a hand-driven lap.
    ///
    /// **Not the Autopilot pickup**, and the distinction is worth keeping.
    /// `Ai_Construct` names the local player's input source the literal
    /// `"autopilot input"`, which says the original hands the player's craft to
    /// a driver the same way this does - but *what* the pickup then does, how
    /// long it lasts and how it ends are all unread, and nothing here claims
    /// them. This is a switch a test and a `--screenshot` can flip, not a
    /// weapon. See `HANDOVER.md`.
    ///
    /// # Turning it on seeds slot 0's driver, and has to
    ///
    /// `start.rs` seeds a `Driver` inside the *opponent* loop, so slot 0's is
    /// left at [`oag_ai::Driver::default`] with its line index at zero - and
    /// `Driver::drive` searches a 48-sample window around the last index, on
    /// purpose, so a driver that begins at sample zero when the grid is at
    /// sample 2,500 never finds itself and steers at the piece of circuit it
    /// thinks it is on. That is the same trap `start.rs` records paying for on
    /// eight of the disc's twelve circuits, and it is why this is a locate
    /// rather than a flag assignment.
    ///
    /// **Here rather than in `start.rs`**, deliberately: `driver.index` is
    /// inside the world snapshot and is hashed, so seeding slot 0 at spawn would
    /// move every ordinary race's determinism hash for the sake of a field
    /// nothing reads unless this flag is on. Written where the flag is, it moves
    /// nothing that is not already under it.
    ///
    /// The personality is left neutral - `Driver::default`'s seed is zero and
    /// `Personality::from_seed(0)` is explicitly the shared tuning - so the
    /// player's craft is flown by the baseline driver rather than by a character
    /// drawn for a slot the roster never dealt.
    pub fn set_autopilot(&mut self, on: bool) {
        self.sim.autopilot = on;
        if on {
            self.locate_player_driver();
        }
    }

    /// Puts slot 0's driver where slot 0 actually is.
    ///
    /// Shared by the operator's flag above and by the **Autopilot pickup**,
    /// which arrives at any point on the circuit and so needs it more: a driver
    /// left at index zero steers at the piece of line it thinks it is on, for
    /// the windowed-search reason [`Self::set_autopilot`] spells out.
    pub(super) fn locate_player_driver(&mut self) {
        if self.sim.racing_line.is_empty() {
            return;
        }
        let position = self.sim.world.ships[0].physics.body.position;
        let index = self
            .sim
            .racing_line
            .nearest(position, 0, self.sim.racing_line.len());
        let driver = &mut self.sim.world.ships[0].driver;
        driver.index = u32::try_from(index).unwrap_or(0);
        // Located on the ring, so it is on the ring. See `oag_ai::branch::Branching::on_ring`.
        driver.branching = driver.branching.on_ring();
    }

    /// Whether slot 0 is being flown for the player this tick, by any of four
    /// routes.
    ///
    /// The operator's `--autopilot` and the pickup are deliberately separate
    /// facts joined here rather than one flag: the first is a verification aid
    /// that runs a whole race, the second is a weapon with a timer, and a test
    /// that sets one must not silently spend the other. The third is a
    /// Disruptor's two Autopilot effects, which hand the craft to its driver
    /// for their `time` at a scaled thrust - see
    /// `oag_weapons::disruption::Disruption::autopilot_thrust_scale`.
    ///
    /// **The fourth is the finish line.** Once the player has crossed it for the
    /// last time the original stops reading the pad and flies the craft itself
    /// for as long as the race stays up behind the end-race panels - measured
    /// 2026-10-01 on PPSSPP, three runs, with no input held at all: the craft kept
    /// lapping the circuit for the 35 s logged. See
    /// `docs/gameplay/after-the-finish.md`. The same driver as the other three
    /// routes, which is the maintainer's standing rule (the AI obeys player physics),
    /// with its thrust capped at the original's own post-finish law - see
    /// `race::finished_thrust`.
    #[must_use]
    pub fn flown_for_the_player(&self, slot: usize) -> bool {
        self.sim.autopilot
            || self.sim.world.ships[slot].standing.finished()
            || self.sim.world.ships[slot].autopilot_timer > 0.0
            || self.sim.world.ships[slot]
                .disruption
                .autopilot_thrust_scale()
                .is_some()
    }

    /// Counts the Autopilot pickup down and lets go of the craft at zero.
    ///
    /// `Autopilot_Update` (`0x08861404`), ported: subtract `dt`, raise the
    /// `disengaging` announcer on the tick the remainder crosses one second,
    /// and stop at zero. See
    /// `docs/ghidra/functions/psp-pulse-usa/autopilot.md`.
    ///
    /// **The original keeps last tick's value in `craft+0x144` and this does
    /// not need to.** That field exists only to make the warning an edge, and
    /// with a fixed timestep the previous value *is* `timer + dt` exactly -
    /// ADR-0007. One consequence is recorded rather than hidden: a pickup armed
    /// with less than a second on it would fire the warning on the original's
    /// first tick and not here, because there is no stale previous value to
    /// cross. No authored `<Weapon type="Autopilot"><Stats time>` is anywhere
    /// near that short.
    pub(super) fn tick_autopilot(&mut self, slot: usize) {
        let timer = self.sim.world.ships[slot].autopilot_timer;
        if timer <= 0.0 {
            return;
        }
        let now = timer - self.sim.dt;
        self.sim.world.ships[slot].autopilot_timer = now.max(0.0);
        if now < AUTOPILOT_WARNING_SECONDS && timer > AUTOPILOT_WARNING_SECONDS {
            self.sim.cues.push(oag_sound::sfx::CueEvent::new(
                oag_sound::sfx::Cue::Disengaging,
                slot,
            ));
        }
    }

    /// Whether the player's craft is being driven for them.
    #[must_use]
    pub fn autopilot(&self) -> bool {
        self.sim.autopilot
    }

    /// What the player's own driver wants this tick.
    ///
    /// The opponents' path exactly - the same line, the same pilot slot and
    /// the same view of the field - through slot 0's own [`oag_ai::Driver`],
    /// which is on the ship like everyone else's and so inside the world
    /// snapshot. A wrecked craft is released rather than driven, for the
    /// reason [`Self::step_opponents`] gives.
    ///
    /// **The tuning is the one exception.** [`RaceSim::autopilot_tuning`], not
    /// [`RaceSim::ai_tuning`], when `--autopilot-skill` set it - see
    /// [`Self::set_autopilot_tuning`] for why the two stay apart.
    pub(super) fn autopilot_controls(&mut self, slot: usize) -> oag_physics::ShipControls {
        self.steer_branching(slot);
        let places = self.places();
        let field = self.field_for(slot, &places);
        let pilot = self.sim.ai_pilots[slot];
        let tuning = self.sim.autopilot_tuning.unwrap_or(self.sim.ai_tuning);
        let ship = &mut self.sim.world.ships[slot];
        if ship.physics.craft_state != oag_physics::CraftState::Racing {
            return oag_physics::ShipControls::default();
        }
        // The player's own hull, the same way an opponent gets its own. See
        // `oag_ai::pace::hull_yaw_ceiling`.
        let yaw_ceiling = oag_ai::hull_yaw_ceiling(&ship.handling);
        let route = ship.driver.branching.route;
        ship.driver.drive(
            &ship.physics,
            &oag_ai::Context {
                line: super::routes::line_for(&self.sim.routes, &self.sim.racing_line, route),
                tuning: &tuning,
                pilot: &pilot,
                field: &field,
                yaw_ceiling: Some(yaw_ceiling),
                plan: super::routes::plan_for(
                    &self.sim.routes,
                    self.sim.speed_plan.as_ref(),
                    route,
                ),
            },
        )
    }

    /// The line the opponents' drivers follow, index-parallel to
    /// [`RaceSim::ai_order`] rather than to [`RaceSim::spline`].
    ///
    /// Read-only, and it exists so a test can ask the question that matters
    /// when an opponent misbehaves: *is the craft where its driver believes it
    /// is?* Comparing a ship's position against `point(driver.index)` separates
    /// a driver that has lost its place on the line from one that is simply
    /// driving badly, and those two have nothing in common as bugs.
    #[must_use]
    pub fn racing_line(&self) -> &oag_ai::Line {
        &self.sim.racing_line
    }

    /// The spline sample a driver's index stands on.
    ///
    /// **`driver.index` is an index into the racing line, not into the sample
    /// table**, and on the three circuits where those differ using one as the
    /// other reads a sample from the wrong side of the track. Wraps, because the
    /// line is a closed ring and the caller that wants the *next* sample is
    /// asking a lap question rather than a table question - `ai_order[i + 1]`,
    /// never `ai_order[i] + 1`, which at the end of a path is a different place
    /// entirely.
    #[must_use]
    pub fn ai_sample(&self, ai_index: usize) -> Option<&Sample> {
        self.sample_index_of(ai_index)
            .and_then(|index| self.sim.spline.sample(index))
    }

    /// The same lookup, as an index into [`RaceSim::spline`].
    ///
    /// For the one caller that needs the number rather than the sample:
    /// [`Self::respawn`] takes a *sample* index, because the player reaches it
    /// from `Spline::nearest` and an opponent from its driver, and a single
    /// parameter carrying two index spaces depending on the caller is the bug
    /// this pair of accessors exists to make impossible to write.
    ///
    /// [`Self::ai_sample_index`] is the public half, for a diagnostic that
    /// needs the number rather than the sample.
    pub(super) fn sample_index_of(&self, ai_index: usize) -> Option<usize> {
        if self.sim.ai_order.is_empty() {
            return None;
        }
        Some(self.sim.ai_order[ai_index % self.sim.ai_order.len()] as usize)
    }

    /// Which spline sample a racing-line index maps to.
    ///
    /// Public because `ai_order` being a *permutation* rather than the identity
    /// is itself a failure mode - the path-order splice that put craft off the
    /// track on three circuits - and a diagnostic that cannot read it cannot
    /// tell a spliced racing line from a merely unsupported one.
    #[must_use]
    pub fn ai_sample_index(&self, ai_index: usize) -> Option<usize> {
        self.sample_index_of(ai_index)
    }

    /// Drives and steps every craft that is not the player's.
    ///
    /// Slot order, and only slot order: iteration here feeds simulation state, so
    /// it must be something that cannot vary between runs. See
    /// `docs/architecture/determinism.md`.
    ///
    /// Each craft gets **its own** `Environment`, built from the sample its own
    /// driver is standing on. That is what the old "they do not move" comment
    /// meant by an opponent stepped with the player's environment reading the
    /// player's track sample: the hover probes and the magstrip hold both take
    /// their surface from it, so sharing one would fly seven craft against the
    /// player's piece of track.
    ///
    /// **What an opponent gets**, since this list was four items shorter than the
    /// truth for long enough to mislead: pads of both classes, a standing that
    /// counts *and times* its laps, an `Exhaust`, a livery, a pickup it can spend
    /// ([`Self::spend_opponent_pickup`]), a target for what it fires
    /// (`Race::opponent_fires`), and two ways of being put back - the
    /// authored reset volumes and, below, the two dwell counters for a craft that
    /// has left the circuit or stopped on it.
    ///
    /// **What it still does not get**: reaction latency, and adaptation between
    /// races - which was blocked on per-craft lap times and is not any more.
    pub(super) fn step_opponents(&mut self) {
        let damage_rules = self.sim.damage_rules();
        // Once for the whole grid: every craft's view needs the same ordering.
        let places = self.places();
        for slot in 1..self.sim.world.ship_count as usize {
            if !self.sim.world.ships[slot].active {
                continue;
            }

            // **A wrecked opponent stops driving, and is still stepped.** A single
            // race runs with `Damage` on - see `oag_gameplay::damage_rules` - so
            // an opponent grinding a wall empties its pool exactly as the player
            // does, and `oag_physics::step` will keep integrating whatever it is
            // handed. Left driving, it would be a wreck holding full throttle
            // round the circuit, which is worse than the parked hulls this
            // replaced. Released rather than skipped, because
            // `damage::advance_state` runs *inside* the step and the destroyed
            // sequence has to finish.
            //
            // What happens next is `Race::tick_destroyed_craft`'s: in the
            // Eliminator the craft is put back on the line with a full pool
            // after state 8's wait; anywhere else it stays a wreck, as the
            // original's state 6 does (measured live). See
            // `oag_physics::damage::CraftState` and `crate::eliminator`.
            let pilot = self.sim.ai_pilots[slot];
            let field = self.field_for(slot, &places);
            self.steer_branching(slot);
            let ship = &mut self.sim.world.ships[slot];
            let route = ship.driver.branching.route;
            let handling = ship.handling;
            let position = ship.physics.body.position;
            // Captured before `oag_physics::step`, because `damage::advance_state`
            // runs *inside* it: the wall counters below must not charge a craft
            // for the tick it was wrecked on, nor for the thousands after.
            let was_racing = ship.physics.craft_state == oag_physics::CraftState::Racing;
            let mut controls = if was_racing {
                ship.driver.drive(
                    &ship.physics,
                    &oag_ai::Context {
                        line: super::routes::line_for(
                            &self.sim.routes,
                            &self.sim.racing_line,
                            route,
                        ),
                        tuning: &self.sim.ai_tuning,
                        pilot: &pilot,
                        field: &field,
                        // **This craft's own hull, not the field's average.**
                        // `<Turning amount>` is authored per team and spans
                        // 1.30 to 1.80 over the disc's eight, so a grid of
                        // eight teams is eight different yaw ceilings and one
                        // global belief was wrong for all of them. See
                        // `oag_ai::hull_yaw_ceiling`.
                        yaw_ceiling: Some(oag_ai::hull_yaw_ceiling(&handling)),
                        plan: super::routes::plan_for(
                            &self.sim.routes,
                            self.sim.speed_plan.as_ref(),
                            route,
                        ),
                    },
                )
            } else {
                oag_physics::ShipControls::default()
            };
            // The same start-line hold the player gets in `Race::tick` - see
            // `RaceState::thrust_gated`. Without this an opponent storms off the
            // line the instant the race loads while the player is still held for
            // the "3, 2, 1, go" countdown, which is not a driving skill gap, just
            // an ungated AI path.
            if RaceState::thrust_gated(self.sim.world.tick) {
                controls.thrust = 0.0;
            }
            // An Eliminator leader waits for the pack; only ever a reduction.
            controls.thrust *= self.eliminator_pack_scale(slot);
            // An opponent under a Disruptor: always AI-driven, so the Stall
            // passes it by and the rest apply. See `Race::disrupted_controls`.
            controls = self.disrupted_controls(slot, controls, true);

            // The pads, measured from where the craft starts the tick, exactly as
            // slot 0's are. Both classes: a speed pad boosts an opponent and a
            // weapon pad hands it a pickup.
            let (moved, sweep) = self.pad_sweep(slot, position);
            let pad_hit = self.test_speedup_pads(slot, position, moved, &sweep);
            self.test_weapon_pads(slot, position, moved, &sweep);
            self.spend_opponent_pickup(slot, &controls, &field);

            // The driver just located itself, and the line is index-parallel to
            // `ai_order`, so this costs a lookup rather than a search.
            let index = self.sim.world.ships[slot].driver.index as usize;
            let track_sample = self.ai_sample_for(slot, index).map(Spline::track_sample);
            let track_sample_next = self
                .ai_sample_for(slot, index + 1)
                .map(Spline::track_sample)
                .or(track_sample);
            // The beam's one-shot throttle, consumed here exactly as slot
            // 0's is above - a beam locks onto an opponent as readily as onto
            // the player.
            let thrust_scale =
                std::mem::replace(&mut self.sim.world.ships[slot].pending_thrust_scale, 1.0);
            let env = Environment {
                track_sample,
                track_sample_next,
                thrust_scale,
                // **The same pads the player crosses.** Measured from the
                // position the tick started at and swept to where the craft is
                // now, exactly as slot 0's are - see `Race::pad_sweep`. Without
                // this an opponent is the only thing on the circuit that a speed
                // pad does not touch, and the field falls away from a player who
                // uses them.
                pad_hit,
                class_gravity_scale: self.sim.class_gravity_scale,
                start_boost: self.sim.start_boost,
                damage_rules,
                ..Environment::default()
            };
            let evaluated = oag_physics::step(
                &mut self.sim.world.ships[slot].physics,
                &controls,
                &handling,
                &env,
                &self.sim.collision,
                self.sim.dt,
            );
            // **An opponent's hull sounds too**, on its own 0.8-second re-arm
            // and off its own emitter. `shielded` is `false` because nothing
            // hands an opponent a Shield yet - the same gap the shell bulge
            // below records - so the silent branch is unreachable here rather
            // than suppressed.
            self.raise_contact_cue(slot, evaluated.wall.impact, false);
            // The wall counters, on the same footing as the roll ones below:
            // bookkeeping, outside `state_hash`, and the only thing in this
            // project that can see the standard an Ace is held to - lapping
            // without touching a wall. `contacts` rather than
            // `ShipState::wall_contact_prev` because the latter is the
            // *inbound* test and a grind is not inbound; `impulse_sum` rather
            // than a shield delta because the pool also moves for rolls and
            // pads. See `RaceSim::wall_contact_ticks` for both arguments.
            //
            // **Gated on the craft still racing, and that gate is the whole
            // difference between a measurement and a fiction.** A wrecked
            // opponent is released rather than skipped - see the comment at the
            // top of this loop - so `oag_physics::step` keeps integrating it and
            // a hull settled against a wall accumulates contact ticks and
            // charged damage for the rest of the run, at a frozen
            // `driver.index`. Ungated, `10_Track` at PHANTOM read 7,157 contact
            // ticks against 328 before a tuning change that *halved* what the
            // walls charged it, and `07_Track` charged 122.05 against a pool of
            // 95.00. Both are the wreck, not the driving.
            if was_racing {
                self.sim.wall_racing_ticks[slot] += 1;
                if evaluated.wall.contacts > evaluated.wall.floor_contacts {
                    self.sim.wall_contact_ticks[slot] += 1;
                    if evaluated.wall.impact {
                        self.sim.wall_inbound_ticks[slot] += 1;
                    }
                    self.sim.wall_damage[slot] += oag_physics::damage::contact_damage(
                        evaluated.wall.impulse_sum,
                        damage_rules,
                    );
                }
            }
            // Bookkeeping for the deviation, not state the race reads back: our
            // opponents barrel-roll and the original's never do, so what an
            // opponent spends on them has to be countable on the disc's own
            // circuits. `arm` charges `roll_cost` percent of the pool, which is
            // what makes the cost derivable rather than measured off a shield
            // that a wall may also have moved this tick.
            if evaluated.roll_armed {
                self.sim.rolls_armed[slot] += 1;
                self.sim.rolls_spent[slot] +=
                    handling.roll_cost * 0.01 * handling.dimensions.shield;
            }
            // The same shell bulge slot 0 gets in `tick`. Nothing hands an
            // opponent a Shield yet - the AI has no fire decision for one - so
            // this is unreachable today and is here because the alternative is a
            // rival shield that silently has no visual the day it arrives.
            if evaluated.shield.absorbed {
                self.view.shield[slot].hit();
            }
            // **The same recovery the player gets**, and it is not a nicety.
            // Without it an opponent that leaves the geometry keeps going: the
            // solo benchmark measured craft receding from the track at racing
            // speed, 1,500 units per ten seconds, for the rest of the race.
            // Seven of the disc's twelve circuits never saw a completed lap for
            // this reason, and it read as a driving fault rather than as a
            // missing mechanism.
            //
            // Same `env` and same pre-step position the force law just ran with,
            // for the reason `reset_zone_touched` gives, and `index` is the
            // sample the craft was on before the step - the last place it is
            // known to have been on the track. Mapped out of the driver's index
            // space, because [`Self::respawn`] speaks the sample table's.
            let last_good = self.sample_index_of(index);
            self.sim.respawn_cooldown[slot] = self.sim.respawn_cooldown[slot].saturating_sub(1);
            // Unconditionally, and before the `||` could skip it: **both** dwell
            // counters have to see every tick or a craft banks time it never
            // spent away. Two `let`s rather than two terms of the `if`, because
            // `||` short-circuits and the second counter would then only advance
            // on ticks the first one happened not to fire.
            let lost = self.lost_off_the_circuit(slot);
            let stalled = self.stalled(slot);
            // **Not a wreck.** A destroyed craft is coasting by definition, so
            // the stall dwell would fire on it and `Race::respawn` - whose
            // `place_at` rebuilds the physics state - would put it back on
            // the line as a *racing* craft with an empty pool, three
            // seconds before `Race::tick_destroyed_craft` brings it back
            // properly. That is what used to happen, and it is why the
            // clean-lap gate never saw those deaths. The dwells still count,
            // so a craft that comes back stalled is rescued on time.
            let alive =
                self.sim.world.ships[slot].physics.craft_state == oag_physics::CraftState::Racing;
            let reset = self.reset_zone_touched(slot, &env, position);
            let airborne = self.airborne_too_long(slot);
            if alive && (lost || stalled || airborne || reset) {
                self.sim.last_respawn_cause[slot] = Some(if lost && self.beneath_the_line(slot) {
                    respawn::RespawnCause::Beneath
                } else if lost {
                    respawn::RespawnCause::LostCircuit
                } else if stalled {
                    respawn::RespawnCause::Stalled
                } else if airborne {
                    respawn::RespawnCause::Airborne
                } else {
                    respawn::RespawnCause::ResetZone
                });
                self.respawn(slot, last_good);
            } else if self.sim.respawn_cooldown[slot] == 0 {
                self.sim.respawns_in_a_row[slot] = 0;
            }
        }
    }

    /// Advances every active craft's [`oag_race::Standing`].
    ///
    /// Slot order, and slot order only: this feeds the finishing order, which is
    /// simulation state. See `docs/architecture/determinism.md`.
    ///
    /// **`laps_target` comes from the player's race**, because it is the
    /// *event's* number rather than a craft's - all eight are running the same
    /// mode in the same speed class. That reasoning used to say "the mode's
    /// number", which stopped being exact when the lap count became per speed
    /// class (`oag_race::Mode::SINGLE_RACE_LAPS_BY_CLASS`); the conclusion is
    /// unchanged, because a race is run in one class and every craft on the
    /// grid is in it.
    pub(super) fn update_standings(&mut self) {
        // A free function so the `course` and `world` borrows stay disjoint -
        // cloning a `Course` once a tick to satisfy the borrow checker would be
        // a `Vec` copy per frame for nothing.
        let player = self.player_slot();
        let finished_before = self.sim.world.ships[player].standing.finished();
        if let Some(course) = &self.sim.course {
            advance_standings(&mut self.sim.world, course);
        }
        // The tick the player crosses the line for the last time is the tick
        // their craft is handed to the driver, which has never run for them:
        // put it on the piece of line the craft is actually on, for the
        // windowed-search reason `Self::set_autopilot` spells out.
        if !finished_before && self.sim.world.ships[player].standing.finished() {
            self.locate_player_driver();
        }
    }

    /// Every craft's race position, `1`-based, in slot order.
    ///
    /// Inactive slots are placed too - they are at the start line and last,
    /// which is where a slot with no craft in it belongs. A caller that cares
    /// reads [`Self::ship_count`] first.
    #[must_use]
    pub fn places(&self) -> [u8; MAX_SHIPS] {
        let Some(course) = &self.sim.course else {
            // No closed ring, so no positions. Everyone is first, which is the
            // honest answer for a track that cannot count a lap at all.
            return [1; MAX_SHIPS];
        };
        let standings: [oag_race::Standing; MAX_SHIPS] =
            std::array::from_fn(|slot| self.sim.world.ships[slot].standing);
        oag_race::places(&standings, course)
    }

    /// What slot `slot` can see of the rest of the grid.
    ///
    /// **`Field::EMPTY` when the track has no closed ring.** `Standing::distance`
    /// needs a `Course` to mean anything, and a guessed along-track gap is worse
    /// than none - it would put a craft half a lap away in the mirror and have a
    /// driver defend against it. The same answer [`Race::places`] gives, for the
    /// same reason.
    ///
    /// Slot order throughout and `total_cmp` for every comparison, so nothing
    /// here can feed the simulation an order that depends on a hasher.
    ///
    /// Takes the places rather than calling [`Race::places`] itself, because the
    /// caller asks this once per craft and that would sort the standings eight
    /// times a tick to get the same answer.
    #[must_use]
    pub(super) fn field_for(&self, slot: usize, places: &[u8; MAX_SHIPS]) -> oag_ai::Field {
        let Some(course) = &self.sim.course else {
            return oag_ai::Field::EMPTY;
        };
        let mine = &self.sim.world.ships[slot];
        if !mine.active {
            return oag_ai::Field::EMPTY;
        }
        let body = &mine.physics.body;
        let (forward, right) = (body.forward(), body.right());
        let my_distance = mine.standing.distance(course);
        let my_speed = mine.physics.body.linear_velocity.dot(forward);

        let mut field = oag_ai::Field {
            place: places[slot],
            hazard: self.hazard_for(slot, forward, right),
            pad: self.pad_for(slot),
            ..oag_ai::Field::EMPTY
        };
        let (mut best_ahead, mut best_behind, mut best_alongside) =
            (f32::INFINITY, f32::INFINITY, f32::INFINITY);

        for other in 0..self.sim.world.ship_count as usize {
            if other == slot || !self.sim.world.ships[other].active {
                continue;
            }
            let them = &self.sim.world.ships[other];
            let gap = them.standing.distance(course) - my_distance;
            if gap.abs() >= oag_ai::AWARENESS_RANGE {
                continue;
            }
            let to_them = them.physics.body.position - body.position;
            let range = to_them.length();
            // **The rate `|gap|` shrinks**, so it means the same thing for a
            // craft ahead and one behind - see `oag_ai::Rival::closing`.
            let their_speed = them.physics.body.linear_velocity.dot(forward);
            let approach = their_speed - my_speed;
            let closing = if gap >= 0.0 { -approach } else { approach };
            let rival = oag_ai::Rival {
                slot: other as u8,
                gap,
                offset: to_them.dot(right),
                closing,
                range,
                cos_bearing: if range > f32::EPSILON {
                    to_them.dot(forward) / range
                } else {
                    1.0
                },
            };

            // Alongside first, and exclusively: a craft level with this one is
            // not something to defend against or lift for, it is something to
            // avoid touching.
            if gap.abs() < ALONGSIDE_GAP && rival.offset.abs() < ALONGSIDE_WIDTH {
                if rival.offset.abs() < best_alongside {
                    best_alongside = rival.offset.abs();
                    field.alongside = Some(rival);
                }
            } else if gap >= 0.0 {
                if gap < best_ahead {
                    best_ahead = gap;
                    field.ahead = Some(rival);
                }
            } else if -gap < best_behind {
                best_behind = -gap;
                field.behind = Some(rival);
            }
        }
        field
    }

    /// The lap counter's ring, for a caller that wants to measure against it.
    #[must_use]
    pub fn course(&self) -> Option<&Course> {
        self.sim.course.as_ref()
    }

    /// The player's own race position, `1`-based.
    #[must_use]
    pub fn player_place(&self) -> u8 {
        self.places()[self.player_slot()]
    }

    /// Craft against craft, every pair, once a tick.
    ///
    /// `oag_physics::pair::resolve` is `Body_ResolveContactPair` (`0x0884ef30`),
    /// read 2026-08-11 - the function this engine was missing, and the reason
    /// craft used to pass through each other. The response is recovered; the
    /// *shape* is not, and `pair::overlap` says so.
    ///
    /// **After every craft has been stepped**, not during. Stepping and
    /// resolving interleaved would let a craft that has already moved this tick
    /// collide with one that has not, which makes the outcome depend on slot
    /// order - and the original resolves contacts in its own pass, after
    /// integration, which
    /// `docs/ghidra/functions/ps2-pulse-eu/craft-update.md` records for the PS2
    /// twin.
    ///
    /// Pairs are visited in slot order, low index first, and each unordered pair
    /// exactly once. `pair::resolve` is symmetric in its two arguments - pinned
    /// by its own test - so the order cannot change the world; visiting in a
    /// fixed order anyway is what
    /// `docs/architecture/determinism.md` asks for, because a *sequence* of
    /// resolutions is order-dependent even when each one is not.
    pub(super) fn resolve_craft_pairs(&mut self) {
        for a in 0..self.sim.world.ship_count as usize {
            for b in (a + 1)..self.sim.world.ship_count as usize {
                if !self.sim.world.ships[a].active || !self.sim.world.ships[b].active {
                    continue;
                }
                let (first, second) = self.sim.world.ships.split_at_mut(b);
                let ship_a = &mut first[a];
                let ship_b = &mut second[0];
                let a_size = ship_a.handling.dimensions;
                let b_size = ship_b.handling.dimensions;
                oag_physics::pair::resolve(
                    &mut ship_a.physics,
                    &a_size,
                    &mut ship_b.physics,
                    &b_size,
                );
            }
        }
    }
}

/// Advances every active craft's standing, and syncs the player's lap from it.
///
/// Slot order, and slot order only: this feeds the finishing order, which is
/// simulation state. See `docs/architecture/determinism.md`.
///
/// `laps_target` comes from the player's race because it is the *event's*
/// number rather than a craft's - all eight are running the same mode in the
/// same speed class. See [`Field::update_standings`] for why that is still the
/// right reading now that the lap count varies with the class.
pub(super) fn advance_standings(world: &mut World, course: &Course) {
    let tick = world.tick;
    let target = world.laps_target();
    for slot in 0..world.ship_count as usize {
        let ship = &mut world.ships[slot];
        if !ship.active {
            continue;
        }
        let position = ship.physics.body.position;
        ship.standing.update(course, position, tick, target);
    }
    // One lap rule, not two: a displayed lap is the standing's. `RaceState`
    // keeps the clock, the best lap, the Zone counters and the finish
    // condition; `Standing` keeps the lap and the place, and this is the one
    // place the first is told about the second.
    //
    // **Over the human slots rather than slot 0**, which is the same single
    // write while slot 0 is the only human - see `World::human_slots`. An AI
    // slot's `race[i].lap` is deliberately left alone: nothing reads it, and
    // syncing it would put a lap counter into the hash for seven craft that
    // never had one, which is a state change dressed as a refactor.
    //
    // Spelled out rather than `world.human_slots()`, which borrows the world
    // the loop then writes to: collecting it first would put a heap allocation
    // in the 60 Hz step to dodge a borrow. Same slot order, same single write.
    for slot in 0..oag_gameplay::MAX_PLAYERS {
        if world.controllers[slot].is_human() {
            world.race[slot].lap = world.ships[slot].standing.lap;
        }
    }
}
