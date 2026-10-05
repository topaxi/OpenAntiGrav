//! The two things an opponent does with a pickup, and the one thing it does
//! about somebody else's.
//!
//! Split out of `field.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py` on the day the Plasma landed and took that
//! file to 1,017; a move, with no behaviour change. The seam is the one the
//! parent's own module doc already named - "the two things an opponent does
//! with a pickup" - and it is real: everything here asks *what should this
//! craft do with the weapon it is holding*, where everything left in the
//! parent is the line, the standings and who can see whom. Its tests stay in
//! `race/tests/field.rs` alongside the rest of the opponent coverage.
//!
//! **One pre-existing bug was fixed on the way**, and it is the second of its
//! kind in this subsystem: `fire_opponent_rocket`'s doc comment was stranded
//! above `fire_opponent_missile`, so the Rocket's paragraphs described the
//! Missile and the Rocket itself carried nothing. `Driver::drift` had the
//! identical fault, found the same way - by moving the code. Each comment is
//! back on its own function.

use super::*;

impl Race {
    /// Fires an opponent's Missile, once `Race::opponent_fires` has decided to.
    ///
    /// Returns whether anything left the rails, so the caller knows whether to
    /// spend the pickup.
    ///
    /// **Two gates rather than one**, and they answer different questions.
    /// `Race::opponent_fires` is the *driver's*: has the roll come up, with
    /// something in the shot's path. `Race::fire_missile` then runs the
    /// *weapon's* - `Ship_AcquireLock`'s recovered window, cone and along-track
    /// screen - and declines if nothing is lockable.
    pub(crate) fn fire_opponent_missile(&mut self, slot: usize) -> bool {
        let Some(stats) = self
            .sim
            .weapons
            .as_ref()
            .and_then(oag_tables::weapons::WeaponStats::missile)
        else {
            return false;
        };
        self.fire_missile(slot, &stats)
    }

    /// Fires an opponent's Rocket, once `Race::opponent_fires` has decided to.
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
    pub(crate) fn fire_opponent_rocket(&mut self, slot: usize) -> bool {
        let Some(stats) = self
            .sim
            .weapons
            .as_ref()
            .and_then(oag_tables::weapons::WeaponStats::rocket)
        else {
            return false;
        };
        let ship = &self.sim.world.ships[slot];
        // `None` where the weapon table authors a speed per class and this
        // race's rung is outside them - the shot does not happen rather than
        // flying at a rung it was not tuned for.
        let Some(fired) = oag_weapons::projectile::fire_rocket(
            &mut self.sim.world.projectiles,
            &ship.physics,
            &stats,
            &self.sim.class,
            slot as u8,
        ) else {
            return false;
        };
        fired > 0
    }

    /// Launches an opponent's travelling wave, if its driver wants a shot now
    /// and none is already in flight.
    ///
    /// Returns whether the wave left, so the caller can leave the pickup in
    /// the craft's hands rather than cashing it in - the same `&&`-chain
    /// reason every other armed weapon here returns a `bool`.
    ///
    /// **The busy check comes first**, matching `Weapon_FireQuake`'s own -
    /// only one Quake can be in flight in the whole race, so a driver that
    /// wants to fire into an already-travelling wave simply keeps the
    /// pickup, the same as a full projectile pool would. Whether to fire is
    /// `Race::opponent_fires`'s, as for every forward weapon; under the
    /// original's law the Quake is the one that needs nothing in its path.
    fn fire_opponent_quake(&mut self, slot: usize) -> bool {
        if self.sim.world.quake.is_some() {
            return false;
        }
        let Some(stats) = self
            .sim
            .weapons
            .as_ref()
            .and_then(oag_tables::weapons::WeaponStats::quake)
        else {
            return false;
        };
        let ship = &self.sim.world.ships[slot];
        let Some(progress) = ship.standing.progress else {
            return false;
        };
        // The direction sign, the same dot product `Race::spend_pickup`'s own
        // Quake arm reads - see that arm's doc comment for the reasoning and
        // the `unwrap_or(1.0)` default.
        let forward_dot_tangent = ship
            .standing
            .course_index
            .and_then(|index| self.sim.course.as_ref()?.tangent(index as usize))
            .map(|tangent| tangent.dot(ship.physics.body.forward()))
            .unwrap_or(1.0);
        self.sim.world.quake = Some(oag_weapons::projectile::quake::Wave::launch(
            slot as u8,
            progress,
            forward_dot_tangent,
            &stats,
        ));
        true
    }

    /// Fastens an opponent's beam onto whoever is in front of it, if its driver
    /// wants a shot now and nobody else's beam is up.
    ///
    /// Returns whether the beam went out, on the same `&&`-chain contract every
    /// other armed weapon here follows.
    ///
    /// **The busy check comes first**, matching `Weapon_FireLeachBeam`'s own
    /// world-wide pool cursor: one beam in the whole race, so a driver that
    /// wants to fire into somebody else's link keeps the pickup instead.
    ///
    /// # An opponent only fires this one with a lock, and that is chosen
    ///
    /// The player's own arm fires with or without a lock, because the original
    /// does - `Weapon_FireLeachBeam` claims the pool slot before it branches,
    /// and an unlocked beam is a 0.75-second no-op. **An opponent declines
    /// instead**, keeping the pickup until it has somebody to fasten onto.
    ///
    /// **Chosen, not measured**, and tuned for a better race rather than for
    /// fidelity - the licence `HANDOVER.md` records for opponent behaviour. The
    /// reasoning: the LeachBeam's pool is a *global* cursor, so an opponent
    /// burning it on nothing locks the weapon out of the whole race for three
    /// seconds, the player included. That is the one weapon here where a wasted
    /// AI shot costs somebody else their turn, and letting an opponent hold on
    /// to it until it can actually use it makes it a threat instead of a
    /// nuisance. No confidence score, because nothing was measured.
    fn fire_opponent_leach_beam(&mut self, slot: usize) -> bool {
        if self.sim.world.leach_beam.is_some() {
            return false;
        }
        let Some(stats) = self
            .sim
            .weapons
            .as_ref()
            .and_then(oag_tables::weapons::WeaponStats::leach_beam)
        else {
            return false;
        };
        let count = self.sim.world.ship_count as usize;
        let ship = &self.sim.world.ships[slot];
        // The same window `Race::sight_target` runs for the player, from the
        // craft's own position - the LeachBeam has no recovered launch offset to
        // measure from, so neither path invents one.
        let Some(target) = oag_weapons::projectile::missile::lock_window(
            &self.sim.world.ships[..count],
            slot as u8,
            ship.physics.body.position,
            ship.physics.body.forward(),
            stats.lock_min_dist,
            stats.lock_max_dist,
            self.sim.course.as_ref().map(oag_race::Course::length),
        ) else {
            return false;
        };
        // `LEACH`, the same locked-arm-only cue `Race::spend_pickup`'s own
        // LeachBeam case pushes - an opponent only ever fires this weapon
        // with a lock (this function's own gate, above), so there is no
        // unlocked arm to reach here at all.
        self.sim.cues.push(oag_sound::sfx::CueEvent::new(
            oag_sound::sfx::Cue::Leach,
            slot,
        ));
        self.sim.world.leach_beam = Some(oag_weapons::projectile::leach_beam::Beam::locked(
            slot as u8, target, &stats,
        ));
        true
    }

    /// Puts one plasma bolt in the air for an opponent, if its driver wants a
    /// shot now.
    ///
    /// Returns whether anything left, so the caller can leave the pickup in the
    /// craft's hands rather than cashing it in - the `&&`-chain bug
    /// `an_opponent_that_declines_a_shot_does_not_absorb_the_pickup` covers.
    ///
    /// Whether to fire is `Race::opponent_fires`'s. Under the original's law
    /// the Plasma differs from the Rocket in two read places only: its own
    /// `WeaponAIstats.xml` row and its own shot speed in the path test.
    fn fire_opponent_plasma(&mut self, slot: usize) -> bool {
        let Some(stats) = self
            .sim
            .weapons
            .as_ref()
            .and_then(oag_tables::weapons::WeaponStats::plasma)
        else {
            return false;
        };
        let ship = &self.sim.world.ships[slot];
        let Some((position, velocity)) = oag_weapons::projectile::plasma::launch(
            &ship.physics,
            &ship.handling.dimensions,
            &stats,
            &self.sim.class,
        ) else {
            return false;
        };
        // The same wind-up the player's bolt takes - see
        // `Race::spend_pickup`'s Plasma arm and
        // `oag_weapons::projectile::plasma::CHARGE_SECONDS`. An opponent's
        // charging bolt occupies the array the same way, so the
        // "none already in flight" gate above sees it.
        self.sim.world.projectiles.charge_up(
            position,
            velocity,
            slot as u8,
            oag_weapons::projectile::plasma::CHARGE_SECONDS,
        )
    }

    /// Throws an opponent's blade, if its driver wants a shot now.
    ///
    /// Returns whether anything left, for [`Race::fire_opponent_plasma`]'s
    /// reason; whether to throw is `Race::opponent_fires`'s.
    ///
    /// **The coin is drawn from the world's generator here exactly as it is on
    /// the player's path**, and only once there is room in the array - see
    /// `Race::spend_pickup`'s Shuriken arm, whose ordering this mirrors line for
    /// line. A driver that declines the shot draws nothing at all, so the
    /// generator stream does not depend on how many opponents happened to be
    /// holding a Shuriken this tick.
    fn throw_opponent_shuriken(&mut self, slot: usize) -> bool {
        let Some(stats) = self
            .sim
            .weapons
            .as_ref()
            .and_then(oag_tables::weapons::WeaponStats::shuriken)
        else {
            return false;
        };
        if self.sim.world.projectiles.live() >= oag_weapons::projectile::MAX_PROJECTILES {
            return false;
        }
        let physics = self.sim.world.ships[slot].physics;
        let dimensions = self.sim.world.ships[slot].handling.dimensions;
        let Some((position, velocity)) = oag_weapons::projectile::shuriken::launch(
            &physics,
            &dimensions,
            &stats,
            &self.sim.class,
            &mut self.sim.world.rng,
        ) else {
            return false;
        };
        let thrown = self
            .sim
            .world
            .projectiles
            .throw(position, velocity, slot as u8, stats.fuse);
        if thrown {
            self.sim.cues.push(oag_sound::sfx::CueEvent::new(
                oag_sound::sfx::Cue::ShurikenLaunch,
                slot,
            ));
        }
        thrown
    }

    /// What an opponent does with a pickup it is holding.
    ///
    /// # When a forward weapon fires is the original's; the rest is policy
    ///
    /// **The fire half of `WeaponAi_DecideFireOrAbsorb` (`0x088518b4`) is
    /// ported** (2026-10-03): `Race::opponent_fires` asks `oag_ai::weapon_ai`
    /// for the Rocket, Missile, Plasma, Shuriken, LeachBeam and Quake, on the
    /// odds out of the title's `WeaponAIstats.xml`, and falls back to
    /// `oag_ai::Driver::wants_to_fire` where that file was not read. See
    /// `docs/ghidra/functions/psp-pulse-usa/weapon-ai.md`. Its absorb half is
    /// not ported, and everything below that is not a forward weapon is still
    /// this project's own:
    ///
    /// - **Turbo is fired at once**, but only on a stretch the driver is not
    ///   braking for. A turbo spent into a corner is a turbo spent into a wall.
    ///   It arms that craft's own boost plume, the same reuse the player's Turbo
    ///   makes of the speed pad's visual.
    /// - **A forward weapon is fired when `Race::opponent_fires` says so**, and
    ///   kept rather than spent when it does not, or when the weapon itself
    ///   declines (no lock, no authored stats, no free slot).
    /// - **An Autopilot is absorbed**, and for once that is not a policy but
    ///   the only thing it could be: the pickup hands a craft to its driver, and
    ///   an opponent is already flown by one. There is nothing for it to do. It
    ///   is named here rather than left to fall through the default below,
    ///   because "no effect by design" and "no arm written yet" must not look
    ///   the same in this file.
    /// - **A Mine or a Bomb is dropped when somebody is close behind**, on
    ///   `oag_ai::Driver::wants_to_drop`, and the pickup is **not** spent on the
    ///   tick it is taken: a cluster comes out over half a second and the craft
    ///   holds the weapon until the last one is laid. This is the one arm that
    ///   returns rather than falling to the bottom of the function.
    /// - **A declined shot keeps the pickup**, on every armed weapon. It did not
    ///   until 2026-08-26, and the bug was invisible because the comments here
    ///   described the intended behaviour rather than the written one: each arm
    ///   was `weapon == X && self.fire_x(..)`, so a helper returning `false` -
    ///   the ordinary "not this tick" answer, nineteen ticks in twenty - made the
    ///   *condition* false and fell into the absorb branch. An opponent cashed
    ///   every weapon in for energy on the tick it collected it and essentially
    ///   never fired one.
    /// - **Everything else is absorbed**, which pays energy into the pool and is
    ///   a real effect rather than a discard. It is also what a cautious human
    ///   does with a weapon they cannot aim, and the remaining six cannot be
    ///   aimed: they need a lock, a beam or a mechanic nothing has read.
    pub(crate) fn spend_opponent_pickup(
        &mut self,
        slot: usize,
        controls: &oag_physics::ShipControls,
        field: &oag_ai::Field,
    ) {
        // Before anything returns: it advances this craft's weapon-AI clocks.
        let fires = self.opponent_fires(slot, field);
        let Some(weapon) = self.sim.world.ships[slot].pickup.weapon else {
            return;
        };
        // Looked up and copied out before the branches, so the borrow of
        // `self.sim.weapons` ends here: the Rocket arm needs `&mut self` to put
        // anything in the air, and every one of these returns an owned value.
        let Some((simple, absorb)) = self
            .sim
            .weapons
            .as_ref()
            .map(|weapons| (weapons.simple(weapon), weapons.absorb(weapon)))
        else {
            return;
        };

        if weapon == oag_tables::weapons::Weapon::Turbo {
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
            let ship = &self.sim.world.ships[slot];
            let speed = ship
                .physics
                .body
                .linear_velocity
                .dot(ship.physics.body.forward())
                .max(0.0);
            let boosted = speed * TURBO_SPEED_RATIO;
            let context = oag_ai::Context {
                line: self.line_of(slot),
                tuning: &self.sim.ai_tuning,
                pilot: &self.sim.ai_pilots[slot],
                field,
                yaw_ceiling: None,
                plan: None,
            };
            if !ship
                .driver
                .allows_speed(&context, boosted, boosted * simple.time)
            {
                return;
            }
            self.sim.world.ships[slot].physics.turbo_timer = simple.time;
            // And its own plume, the same reuse the player's Turbo makes of it -
            // an opponent's boost has to be visible from behind, or the field
            // gains speed with nothing on screen saying why.
            self.view.exhaust[slot].boost(exhaust::BOOST_SECONDS);
        } else if weapon == oag_tables::weapons::Weapon::Rocket {
            // **The `if` is inside the arm, not `&&`ed onto its condition**, and
            // that is the whole of finding an opponent that never fired. As an
            // `&&` a declined shot made the *condition* false and fell through to
            // the absorb branch below, so the craft cashed the Rocket in for
            // energy on the first tick it chose not to shoot - which, at a
            // `TRIGGER_RATE` of `0.05`, is nineteen ticks in twenty and so
            // effectively the tick it collected it. The comment that used to sit
            // here claimed the opposite in as many words. See
            // `an_opponent_that_declines_a_shot_does_not_absorb_the_pickup`.
            if !fires || !self.fire_opponent_rocket(slot) {
                // No target, no authored rocket or no free slot. Keep it - the
                // same rule the player's path follows, which is a `match` with
                // early returns and always did.
                return;
            }
        } else if weapon == oag_tables::weapons::Weapon::Missile {
            if !fires || !self.fire_opponent_missile(slot) {
                return;
            }
        } else if weapon == oag_tables::weapons::Weapon::Plasma {
            if !fires || !self.fire_opponent_plasma(slot) {
                return;
            }
        } else if weapon == oag_tables::weapons::Weapon::Shuriken {
            if !fires || !self.throw_opponent_shuriken(slot) {
                return;
            }
        } else if weapon == oag_tables::weapons::Weapon::Quake {
            if !fires || !self.fire_opponent_quake(slot) {
                return;
            }
        } else if weapon == oag_tables::weapons::Weapon::Repulser {
            if !fires || !self.fire_repulser(slot) {
                return;
            }
        } else if weapon == oag_tables::weapons::Weapon::LeachBeam {
            if !fires || !self.fire_opponent_leach_beam(slot) {
                return;
            }
        } else if weapon == oag_tables::weapons::Weapon::Disruptor {
            if !self.fire_opponent_disruptor(slot, field) {
                return;
            }
        } else if matches!(
            weapon,
            oag_tables::weapons::Weapon::Mine | oag_tables::weapons::Weapon::Bomb
        ) {
            // **Neither branch spends the pickup here.** A drop that starts keeps
            // the weapon in the slot until the last charge is laid, which is
            // recovered; a drop that is declined keeps it for the reason above.
            // So this arm returns either way.
            self.drop_opponent_mines(weapon, slot, field);
            return;
        } else if weapon == oag_tables::weapons::Weapon::Cannon {
            // **Named here rather than left to fall through to absorb, and
            // for a different reason than the Autopilot's.** The Cannon is the
            // one weapon that does not fire off the press edge at all - not for
            // the player either. `Weapon_RequestFire`'s bit `0x2000` is
            // dispatched by nothing; `Cannon_UpdateReload` (`0x0883f424`) reads
            // the *held* half of the same button and arms bit `0x4000`, which is
            // what `Weapons_DispatchFire` actually dispatches. So this arm has
            // nothing to do on the tick the pickup lands, and
            // `Race::advance_cannons` - which runs every slot, not just the
            // player's - does the firing on every tick after it. See
            // `docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md`.
            //
            // Keeping the pickup rather than absorbing it is therefore not
            // conservatism any more: the craft has to go on holding the weapon
            // for `advance_cannons` to spend it, exactly as the Mine's arm above
            // holds one for the length of its cluster.
            return;
        } else {
            // **Anything not named above falls through to absorb**, which is how
            // a weapon added to `pickup::IMPLEMENTED` without an arm reaches a
            // craft as a pickup that quietly turns into energy. `oag_gameplay`'s
            // `IMPLEMENTED` and this chain have to grow together, and
            // `every_implemented_weapon_has_a_fire_arm_on_both_paths` is what
            // makes that fail loudly.
            // **In the Eliminator an opponent keeps the weapon.** An absorb
            // there pays no energy and spends the weapon on a one-second Shield
            // (`Race::eliminator_absorb`, the player's path), and the original's
            // AI takes that branch at about `0.001` a quarter-second decision
            // (`WeaponAi_DecideFireOrAbsorb` forces `+0x34` to `0` in mode 8;
            // `docs/ghidra/functions/psp-pulse-usa/weapon-ai.md`). Not ported:
            // it measured as no change in the time to five kills.
            if !self.sim.world.mode().pickups_absorb() {
                return;
            }
            let Some(amount) = absorb else {
                return;
            };
            let dimensions = self.sim.world.ships[slot].handling.dimensions;
            oag_physics::damage::add(&mut self.sim.world.ships[slot].physics, &dimensions, amount);
        }
        // `Held::take` for the same reason `Race::spend_pickup` uses it: the
        // Mine/Bomb arm above always returns before here, so this is
        // defensive rather than reachable today, but a direct field write
        // would silently stop being safe the day that stops being true.
        self.sim.world.ships[slot].pickup.take();
    }
}

impl Race {
    /// Starts an opponent's mine drop, if its driver wants one now.
    ///
    /// Returns whether a drop began, so the caller can leave the pickup in the
    /// craft's hands - unlike every other opponent arm, this one is not finished
    /// on the tick it is taken. `Race::lay_mines` runs the rest of it and clears
    /// the slot when the cluster is out.
    ///
    /// # An opponent uses the *behind* gate, and the player uses none
    ///
    /// The asymmetry is the same one the Rocket and the Missile already have and
    /// it is not a policy about the weapon: a human decides when to drop by
    /// pressing the button, and `oag_ai::Driver::wants_to_drop` is what stands in
    /// for that decision. What it is *not* is a rule about mines that the player
    /// is being let off - see that function, whose gates are the forward
    /// weapon's with the aiming taken out.
    fn drop_opponent_mines(
        &mut self,
        weapon: oag_tables::weapons::Weapon,
        slot: usize,
        field: &oag_ai::Field,
    ) -> bool {
        let Some(weapons) = self.sim.weapons.as_ref() else {
            return false;
        };
        let Some(drop) = oag_weapons::projectile::mine::Drop::for_weapon(weapon, weapons) else {
            // The table authors no block for this weapon, so nothing to lay.
            // Falls through to absorb, which is the right answer for a pickup
            // that cannot be spent.
            return false;
        };
        let ship = &self.sim.world.ships[slot];
        let context = oag_ai::Context {
            line: self.line_of(slot),
            tuning: &self.sim.ai_tuning,
            pilot: &self.sim.ai_pilots[slot],
            field,
            yaw_ceiling: None,
            plan: None,
        };
        if ship.driver.wants_to_drop(&context).is_none() {
            // Nobody behind worth laying them for, or the trigger did not roll.
            // **Keep the pickup rather than absorb it** - the same rule the
            // Rocket and the Missile follow when they decline a shot, and the
            // reason this returns `false` into an `&&` whose right-hand side is
            // the whole action.
            return false;
        }
        self.sim.world.ships[slot].pickup.begin_drop(drop.count);
        true
    }
}

impl Race {
    /// The nearest laid charge `slot` should steer around, if any.
    ///
    /// **This is the half that knows what a weapon is**, which is why it is here
    /// and not in `oag-ai`: that crate is handed plain numbers and dodges what it
    /// is told about. See [`oag_ai::Hazard`].
    ///
    /// # What counts as a threat, and why it is the trigger radius
    ///
    /// A charge is reported when all four hold:
    ///
    /// 1. **It is a laid charge**, not something in flight. A rocket cannot be
    ///    steered around - by the time a driver could react it has arrived - and
    ///    pretending otherwise would be a dodge that never works.
    /// 2. **It is not this craft's own.** `projectile::mine::triggered_by`
    ///    excludes the owner outright, so a craft's own cluster is harmless to
    ///    it and swerving round one would be a driver frightened of nothing.
    /// 3. **It is ahead**, along this craft's own forward axis, inside
    ///    `oag_ai::avoidance::LOOKAHEAD`.
    /// 4. **It is close enough across to be tripped**: within the charge's own
    ///    authored `trigger_radius` plus half a hull. **Not `blastradius`** - a
    ///    mine's blast is wider than a Pulse lane, so a driver told to leave it
    ///    would swerve into the scenery and take the hit anyway. Missing the
    ///    trigger means the charge never goes off at all, and that is a dodge a
    ///    craft can actually make.
    ///
    /// # Nearest wins, and ties break by slot
    ///
    /// The array is walked in index order and a strictly-nearer charge replaces
    /// the incumbent, so two charges at exactly equal distance resolve to the
    /// lower slot. That is the same rule every other search in this file
    /// follows and it is what keeps the result out of the hands of float noise -
    /// see `docs/architecture/determinism.md`.
    pub(crate) fn hazard_for(
        &self,
        slot: usize,
        forward: oag_core::math::Vec3,
        right: oag_core::math::Vec3,
    ) -> Option<oag_ai::Hazard> {
        let radii = oag_weapons::projectile::TriggerRadii::from_table(self.sim.weapons.as_ref());
        let origin = self.sim.world.ships[slot].physics.body.position;
        // Half the hull's width, so a charge the craft would clip with a wingtip
        // counts as much as one it would drive over.
        let half_width = self.sim.world.ships[slot].handling.dimensions.width * 0.5;

        let mut best: Option<oag_ai::Hazard> = None;
        for charge in &self.sim.world.projectiles.slots {
            let Some(kind) = charge.kind else { continue };
            if charge.owner as usize == slot {
                continue;
            }
            let Some(trigger_radius) = radii.get(kind) else {
                // Not a laid charge - or one whose weapon the table does not
                // author, which cannot be tripped and so is not a threat.
                continue;
            };
            let to_it = charge.position - origin;
            let distance = to_it.dot(forward);
            if distance <= 0.0 || distance >= oag_ai::AVOIDANCE_LOOKAHEAD {
                continue;
            }
            let offset = to_it.dot(right);
            if offset.abs() > trigger_radius + half_width {
                continue;
            }
            if best.is_none_or(|best| distance < best.distance) {
                best = Some(oag_ai::Hazard { distance, offset });
            }
        }
        best
    }
}
