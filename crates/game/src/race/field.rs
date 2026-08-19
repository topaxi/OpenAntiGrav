//! The field: the racing line the drivers follow, what each craft can see of
//! the others, the standings that come out of it, and the two things an
//! opponent does with a pickup.
//!
//! Split out of `race.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change. Its tests are
//! `race/tests/field.rs`.

use super::*;

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
        self.ai_tuning = tuning;
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
        self.autopilot = on;
        if !on || self.racing_line.is_empty() {
            return;
        }
        let position = self.world.ships[0].physics.body.position;
        let index = self
            .racing_line
            .nearest(position, 0, self.racing_line.len());
        self.world.ships[0].driver.index = u32::try_from(index).unwrap_or(0);
    }

    /// Whether the player's craft is being driven for them.
    #[must_use]
    pub fn autopilot(&self) -> bool {
        self.autopilot
    }

    /// What the player's own driver wants this tick.
    ///
    /// The opponents' path exactly - the same line, the same tuning, the same
    /// pilot slot and the same view of the field - through slot 0's own
    /// [`oag_ai::Driver`], which is on the ship like everyone else's and so
    /// inside the world snapshot. A wrecked craft is released rather than
    /// driven, for the reason [`Self::step_opponents`] gives.
    pub(super) fn autopilot_controls(&mut self) -> oag_physics::ShipControls {
        let places = self.places();
        let field = self.field_for(0, &places);
        let pilot = self.ai_pilots[0];
        let ship = &mut self.world.ships[0];
        if ship.physics.craft_state != oag_physics::CraftState::Racing {
            return oag_physics::ShipControls::default();
        }
        ship.driver.drive(
            &ship.physics,
            &oag_ai::Context {
                line: &self.racing_line,
                tuning: &self.ai_tuning,
                pilot: &pilot,
                field: &field,
            },
        )
    }

    /// The line the opponents' drivers follow, index-parallel to
    /// [`Self::ai_order`] rather than to [`Self::spline`].
    ///
    /// Read-only, and it exists so a test can ask the question that matters
    /// when an opponent misbehaves: *is the craft where its driver believes it
    /// is?* Comparing a ship's position against `point(driver.index)` separates
    /// a driver that has lost its place on the line from one that is simply
    /// driving badly, and those two have nothing in common as bugs.
    #[must_use]
    pub fn racing_line(&self) -> &oag_ai::Line {
        &self.racing_line
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
            .and_then(|index| self.spline.sample(index))
    }

    /// The same lookup, as an index into [`Self::spline`].
    ///
    /// For the one caller that needs the number rather than the sample:
    /// [`Self::respawn`] takes a *sample* index, because the player reaches it
    /// from `Spline::nearest` and an opponent from its driver, and a single
    /// parameter carrying two index spaces depending on the caller is the bug
    /// this pair of accessors exists to make impossible to write.
    #[must_use]
    pub(super) fn sample_index_of(&self, ai_index: usize) -> Option<usize> {
        if self.ai_order.is_empty() {
            return None;
        }
        Some(self.ai_order[ai_index % self.ai_order.len()] as usize)
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
    /// (`oag_ai::Driver::wants_to_fire`), and two ways of being put back - the
    /// authored reset volumes and, below, the two dwell counters for a craft that
    /// has left the circuit or stopped on it.
    ///
    /// **What it still does not get**: reaction latency, and adaptation between
    /// races - which was blocked on per-craft lap times and is not any more.
    pub(super) fn step_opponents(&mut self) {
        let damage_rules = oag_gameplay::damage_rules(self.world.race.mode);
        // Once for the whole grid: every craft's view needs the same ordering.
        let places = self.places();
        for slot in 1..self.world.ship_count as usize {
            if !self.world.ships[slot].active {
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
            // What happens next is the gap: nothing reads an opponent's
            // `Eliminated`, because the explosion, the respawn and the
            // elimination bookkeeping are all unbuilt. So it coasts, settles and
            // is passed. See `oag_physics::damage::CraftState`.
            let pilot = self.ai_pilots[slot];
            let field = self.field_for(slot, &places);
            let ship = &mut self.world.ships[slot];
            let handling = ship.handling;
            let position = ship.physics.body.position;
            let controls = if ship.physics.craft_state == oag_physics::CraftState::Racing {
                ship.driver.drive(
                    &ship.physics,
                    &oag_ai::Context {
                        line: &self.racing_line,
                        tuning: &self.ai_tuning,
                        pilot: &pilot,
                        field: &field,
                    },
                )
            } else {
                oag_physics::ShipControls::default()
            };

            // The pads, measured from where the craft starts the tick, exactly as
            // slot 0's are. Both classes: a speed pad boosts an opponent and a
            // weapon pad hands it a pickup.
            let (moved, sweep) = self.pad_sweep(slot, position);
            let pad_hit = self.test_speedup_pads(slot, position, moved, &sweep);
            self.test_weapon_pads(slot, position, moved, &sweep);
            self.spend_opponent_pickup(slot, &controls, &field);

            // The driver just located itself, and the line is index-parallel to
            // `ai_order`, so this costs a lookup rather than a search.
            let index = self.world.ships[slot].driver.index as usize;
            let track_sample = self.ai_sample(index).map(Spline::track_sample);
            let track_sample_next = self
                .ai_sample(index + 1)
                .map(Spline::track_sample)
                .or(track_sample);
            let env = Environment {
                track_sample,
                track_sample_next,
                // **The same pads the player crosses.** Measured from the
                // position the tick started at and swept to where the craft is
                // now, exactly as slot 0's are - see `Race::pad_sweep`. Without
                // this an opponent is the only thing on the circuit that a speed
                // pad does not touch, and the field falls away from a player who
                // uses them.
                pad_hit,
                class_gravity_scale: self.class_gravity_scale,
                damage_rules,
                ..Environment::default()
            };
            let evaluated = oag_physics::step(
                &mut self.world.ships[slot].physics,
                &controls,
                &handling,
                &env,
                &self.collision,
                self.dt,
            );
            // The same shell bulge slot 0 gets in `tick`. Nothing hands an
            // opponent a Shield yet - the AI has no fire decision for one - so
            // this is unreachable today and is here because the alternative is a
            // rival shield that silently has no visual the day it arrives.
            if evaluated.shield.absorbed {
                self.shield[slot].hit();
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
            self.respawn_cooldown[slot] = self.respawn_cooldown[slot].saturating_sub(1);
            // Unconditionally, and before the `||` could skip it: **both** dwell
            // counters have to see every tick or a craft banks time it never
            // spent away. Two `let`s rather than two terms of the `if`, because
            // `||` short-circuits and the second counter would then only advance
            // on ticks the first one happened not to fire.
            let lost = self.lost_off_the_circuit(slot);
            let stalled = self.stalled(slot);
            if lost || stalled || self.reset_zone_touched(slot, &env, position) {
                self.respawn(slot, last_good);
            } else if self.respawn_cooldown[slot] == 0 {
                self.respawns_in_a_row[slot] = 0;
            }
        }
    }

    /// Advances every active craft's [`oag_race::Standing`].
    ///
    /// Slot order, and slot order only: this feeds the finishing order, which is
    /// simulation state. See `docs/architecture/determinism.md`.
    ///
    /// **`laps_target` comes from the player's race**, because it is the mode's
    /// number rather than a craft's - all eight are running the same event.
    pub(super) fn update_standings(&mut self) {
        // A free function so the `course` and `world` borrows stay disjoint -
        // cloning a `Course` once a tick to satisfy the borrow checker would be
        // a `Vec` copy per frame for nothing.
        if let Some(course) = &self.course {
            advance_standings(&mut self.world, course);
        }
    }

    /// Every craft's race position, `1`-based, in slot order.
    ///
    /// Inactive slots are placed too - they are at the start line and last,
    /// which is where a slot with no craft in it belongs. A caller that cares
    /// reads [`Self::ship_count`] first.
    #[must_use]
    pub fn places(&self) -> [u8; MAX_SHIPS] {
        let Some(course) = &self.course else {
            // No closed ring, so no positions. Everyone is first, which is the
            // honest answer for a track that cannot count a lap at all.
            return [1; MAX_SHIPS];
        };
        let standings: [oag_race::Standing; MAX_SHIPS] =
            std::array::from_fn(|slot| self.world.ships[slot].standing);
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
        let Some(course) = &self.course else {
            return oag_ai::Field::EMPTY;
        };
        let mine = &self.world.ships[slot];
        if !mine.active {
            return oag_ai::Field::EMPTY;
        }
        let body = &mine.physics.body;
        let (forward, right) = (body.forward(), body.right());
        let my_distance = mine.standing.distance(course);
        let my_speed = mine.physics.body.linear_velocity.dot(forward);

        let mut field = oag_ai::Field {
            place: places[slot],
            ..oag_ai::Field::EMPTY
        };
        let (mut best_ahead, mut best_behind, mut best_alongside) =
            (f32::INFINITY, f32::INFINITY, f32::INFINITY);

        for other in 0..self.world.ship_count as usize {
            if other == slot || !self.world.ships[other].active {
                continue;
            }
            let them = &self.world.ships[other];
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
        self.course.as_ref()
    }

    /// The player's own race position, `1`-based.
    #[must_use]
    pub fn player_place(&self) -> u8 {
        self.places()[0]
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
        for a in 0..self.world.ship_count as usize {
            for b in (a + 1)..self.world.ship_count as usize {
                if !self.world.ships[a].active || !self.world.ships[b].active {
                    continue;
                }
                let (first, second) = self.world.ships.split_at_mut(b);
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

    /// Fires an opponent's Rocket at the craft ahead, if its driver wants to.
    ///
    /// Returns whether anything left the rails, so the caller knows whether to
    /// spend the pickup.
    ///
    /// **The owner is the firing slot, and that is the line to get right.** The
    /// player's path passes `0` because the player *is* slot 0; an opponent
    /// passing `0` would put rockets in the air owned by the player, which
    /// `projectile::step` would then fly straight through the player and
    /// detonate on whoever actually fired them.
    /// `an_opponents_rocket_is_owned_by_the_slot_that_fired_it` pins it.
    /// The same, for a Missile.
    ///
    /// **Two gates rather than one**, and they answer different questions.
    /// `wants_to_fire` is the *driver's*: is there somebody ahead, is the road
    /// straight enough, has the trigger rolled this tick. `Race::fire_missile`
    /// then runs the *weapon's* - `Ship_AcquireLock`'s recovered window, cone and
    /// along-track screen - and declines if nothing is lockable.
    ///
    /// The driver's chosen slot is deliberately **not** used as the target. It is
    /// its reason to press the button, not the missile's lock; letting it pick
    /// would make a missile's target depend on who fired it, and the player's
    /// path has no `wants_to_fire` to consult at all.
    pub(super) fn fire_opponent_missile(&mut self, slot: usize, field: &oag_ai::Field) -> bool {
        let context = oag_ai::Context {
            line: &self.racing_line,
            tuning: &self.ai_tuning,
            pilot: &self.ai_pilots[slot],
            field,
        };
        if self.world.ships[slot]
            .driver
            .wants_to_fire(&context)
            .is_none()
        {
            return false;
        }
        let Some(stats) = self
            .weapons
            .as_ref()
            .and_then(oag_formats::weapons::WeaponStats::missile)
        else {
            return false;
        };
        self.fire_missile(slot, &stats)
    }

    pub(super) fn fire_opponent_rocket(&mut self, slot: usize, field: &oag_ai::Field) -> bool {
        let context = oag_ai::Context {
            line: &self.racing_line,
            tuning: &self.ai_tuning,
            pilot: &self.ai_pilots[slot],
            field,
        };
        if self.world.ships[slot]
            .driver
            .wants_to_fire(&context)
            .is_none()
        {
            return false;
        }
        let Some(stats) = self
            .weapons
            .as_ref()
            .and_then(oag_formats::weapons::WeaponStats::rocket)
        else {
            return false;
        };
        let ship = &self.world.ships[slot];
        let shots = oag_gameplay::projectile::launch(
            &ship.physics,
            &ship.handling.dimensions,
            &stats,
            to_format_class(self.class),
        );
        let mut fired = 0;
        for (position, velocity) in shots {
            if self.world.projectiles.spawn(
                oag_formats::weapons::Weapon::Rocket,
                position,
                velocity,
                slot as u8,
            ) {
                fired += 1;
            }
        }
        fired > 0
    }

    /// What an opponent does with a pickup it is holding.
    ///
    /// # This is a policy, and it is the crudest one that is not "nothing"
    ///
    /// **Nothing about it is recovered**, and the file this comment used to point
    /// at turns out not to be the answer. `WeaponAIstats.xml`'s loader was found
    /// on 2026-08-17 (`WeaponAiStats_Load`, `0x08851d88`, called one line after
    /// `AiStats_LoadAll` in the same race-setup function) and its schema is three
    /// floats a weapon: `useAgainstPlayer`, `useAgainstAI`, `absorb`. The shipped
    /// values are nearly uniform - `1.0` absorb throughout, `1.2`/`1.1` for
    /// everything but Plasma and Quake - so there is no weapon the file marks as
    /// absorb-only or fire-only. **That is because they are probabilities, not
    /// flags**: `FUN_088518b4` reads the record every frame and compares each
    /// value against a normalised random draw, gated on an along-track range
    /// test. **The whole decision is recovered as of 2026-08-17** -
    /// `WeaponAi_Update` (`0x08851550`) reconsiders four times a second and
    /// `WeaponAi_DecideFireOrAbsorb` (`0x088518b4`) rolls the authored odds
    /// against a five-entry difficulty table (`0`, `0.0005`, `0.002`, `0.008`,
    /// `0.05`, and a second table for Eliminator), with a `2.0` multiplier in
    /// one mode and `5.0` in Eliminator. **It is not ported**, and porting it
    /// needs the two skill indices and the `0..1` scalar the branches gate on,
    /// none of which is read yet. See
    /// `docs/ghidra/functions/psp-pulse-usa/weapon-ai.md`, and
    /// `docs/ghidra/functions/psp-pulse-usa/ai-stats.md`. The decision below is
    /// therefore still invention, and still kept small enough to be obviously
    /// provisional:
    ///
    /// - **Turbo is fired at once**, but only on a stretch the driver is not
    ///   braking for. A turbo spent into a corner is a turbo spent into a wall.
    ///   It arms that craft's own boost plume, the same reuse the player's Turbo
    ///   makes of the speed pad's visual.
    /// - **A Rocket is aimed**, as of 2026-08-11, at whatever
    ///   `oag_ai::Driver::wants_to_fire` picks - a craft ahead, in range, inside
    ///   a cone, with straight enough road between. It is kept rather than spent
    ///   when there is no target, no authored rocket or no free slot.
    /// - **Everything else is absorbed**, which pays energy into the pool and is
    ///   a real effect rather than a discard. It is also what a cautious human
    ///   does with a weapon they cannot aim, and the remaining nine cannot be
    ///   aimed: they need a lock, a beam or a mechanic nothing has read.
    pub(super) fn spend_opponent_pickup(
        &mut self,
        slot: usize,
        controls: &oag_physics::ShipControls,
        field: &oag_ai::Field,
    ) {
        let Some(weapon) = self.world.ships[slot].pickup.weapon else {
            return;
        };
        // Looked up and copied out before the branches, so the borrow of
        // `self.weapons` ends here: the Rocket arm needs `&mut self` to put
        // anything in the air, and every one of these returns an owned value.
        let Some((simple, absorb)) = self
            .weapons
            .as_ref()
            .map(|weapons| (weapons.simple(weapon), weapons.absorb(weapon)))
        else {
            return;
        };

        if weapon == oag_formats::weapons::Weapon::Turbo {
            // Full throttle means the speed target is not asking it to slow for
            // anything it can see, which is the only "is this a straight?" this
            // engine has. **The controls the driver chose this tick**, not
            // `ShipState::thrust`: that is written by `controls::update` inside
            // the step, which has not run yet, so it still holds last tick's
            // value here.
            // **`> 0.0`, not `>= 1.0`.** `Driver::caution` scales the throttle
            // down by an arbitrary fraction when a craft is closing on one
            // ahead, so an exact comparison against full throttle would have a
            // craft silently stop using Turbo the moment anybody was within
            // forty-five units of its nose. What this asks is the question it
            // always meant: is the speed target letting it accelerate, or is it
            // lifting for a corner.
            let on_a_straight = controls.thrust > 0.0;
            let Some(simple) = simple else {
                return;
            };
            if !on_a_straight {
                return;
            }
            // **And clear over the distance the boost itself covers**, which is
            // not the distance the driver was looking at. Its braking horizon is
            // built from the speed it is doing now, so a craft that fires a
            // Turbo at 126 arrives at the next corner doing 270 having checked
            // 160 units ahead. Measured on `16_Track`: one craft in a field of
            // seven did exactly that and left the circuit, and it never touched
            // a wall on the way - see `docs/gameplay/ai.md`.
            // **And not into the back of somebody.** A separate condition from
            // the throttle above, deliberately: coupling the two through
            // `Driver::caution`'s fractional lift would make this depend on how
            // hard the craft happened to be lifting rather than on whether
            // there is anyone there.
            if let Some(ahead) = field.ahead
                && ahead.gap < TURBO_CLEARANCE
            {
                return;
            }
            let ship = &self.world.ships[slot];
            let speed = ship
                .physics
                .body
                .linear_velocity
                .dot(ship.physics.body.forward())
                .max(0.0);
            let boosted = speed * TURBO_SPEED_RATIO;
            let context = oag_ai::Context {
                line: &self.racing_line,
                tuning: &self.ai_tuning,
                pilot: &self.ai_pilots[slot],
                field,
            };
            if !ship
                .driver
                .allows_speed(&context, boosted, boosted * simple.time)
            {
                return;
            }
            self.world.ships[slot].physics.turbo_timer = simple.time;
            // And its own plume, the same reuse the player's Turbo makes of it -
            // an opponent's boost has to be visible from behind, or the field
            // gains speed with nothing on screen saying why.
            self.exhaust[slot].boost(exhaust::BOOST_SECONDS);
        } else if weapon == oag_formats::weapons::Weapon::Rocket
            && self.fire_opponent_rocket(slot, field)
        {
            // Fired. `fire_opponent_rocket` reports false when there was no
            // target, no authored rocket or no free slot, and then the pickup is
            // kept rather than spent - the same rule the player's path follows.
        } else if weapon == oag_formats::weapons::Weapon::Missile
            && self.fire_opponent_missile(slot, field)
        {
            // Likewise. Note this chain's shape: **anything not named here falls
            // through to the absorb branch**, silently, which is how a weapon
            // added to `pickup::IMPLEMENTED` without an arm reaches a player as a
            // pickup that quietly turns into energy. `oag_gameplay`'s
            // `IMPLEMENTED` and this chain have to grow together.
        } else {
            let Some(amount) = absorb else {
                return;
            };
            let dimensions = self.world.ships[slot].handling.dimensions;
            oag_physics::damage::add(&mut self.world.ships[slot].physics, &dimensions, amount);
        }
        self.world.ships[slot].pickup.weapon = None;
    }
}

/// Advances every active craft's standing, and syncs the player's lap from it.
///
/// Slot order, and slot order only: this feeds the finishing order, which is
/// simulation state. See `docs/architecture/determinism.md`.
///
/// `laps_target` comes from the player's race because it is the *mode's* number
/// rather than a craft's - all eight are running the same event.
pub(super) fn advance_standings(world: &mut World, course: &Course) {
    let tick = world.tick;
    let target = world.race.laps_target;
    for slot in 0..world.ship_count as usize {
        let ship = &mut world.ships[slot];
        if !ship.active {
            continue;
        }
        let position = ship.physics.body.position;
        ship.standing.update(course, position, tick, target);
    }
    // One lap rule, not two: the player's displayed lap is the standing's.
    // `RaceState` keeps the clock, the best lap, the Zone counters and the finish
    // condition, all of which are the player's alone.
    world.race.lap = world.ships[0].standing.lap;
}
