//! [`Race::tick`]: one 60 Hz step of the simulation, from an input snapshot to
//! the state the renderer and the HUD read.
//!
//! Split out of `race.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change.

use super::*;

impl Race {
    /// Advances the simulation one fixed tick, and the camera with it.
    ///
    /// The snapshot is mapped through [`oag_gameplay::ship_controls`], which is the
    /// one place "cross is thrust" is written down.
    pub fn tick(&mut self, snapshot: &InputSnapshot) -> Evaluated {
        // Under autopilot the snapshot is not consulted at all - see
        // [`Race::set_autopilot`], which is a verification aid and not a mode.
        // Before `spend_pickup`, which still reads the snapshot: an autopiloted
        // craft is steered for and fires nothing, because nothing picks a target
        // for anybody yet.
        let controls = if self.autopilot {
            self.autopilot_controls()
        } else {
            ship_controls(snapshot, self.scheme)
        };

        // Before the force law, so a Turbo fired this tick boosts this tick.
        self.spend_pickup(snapshot);

        // The two spline samples the magstrip hold reads. In the original these are
        // `AiTrack_LocatePosition`'s two output records on the ship entity; here
        // they are the nearest table entry and the one after it, which is the same
        // "where am I and what is next" pair at this crate's resolution. Off a
        // magstrip nothing consumes them, so a track without one is unaffected.
        let nearest = self
            .spline
            .nearest(self.world.ships[0].physics.body.position);
        let index = nearest.map(|(index, _, _)| index);
        // The same scan, read a third way: how far off the sample table the craft
        // is, which is the player's own off-track trigger below. Taken from here
        // rather than scanned again after the step - the table is ~3,400 samples
        // and a second fold would double the tick's hottest loop to move a
        // measurement one tick, under a dwell of [`PLAYER_RESCUE_TICKS`].
        let spline_distance = nearest.map(|(_, _, distance)| distance);
        let track_sample = nearest.map(|(_, sample, _)| Spline::track_sample(sample));
        // The neighbour is the next entry in table order, which at the end of a
        // path is the *next path's* first sample rather than the geometric
        // successor through a junction. The original follows the junction graph;
        // this is one sample out of 34,000 per lap and is recorded rather than
        // pretended away.
        //
        // Still table order, deliberately: the player's locator works in sample
        // space, where every path the track authors is reachable. An opponent's
        // equivalent goes through [`Self::ai_sample`] instead, because a driver
        // is asking a lap question. See [`ai_order`].
        let track_sample_next = index
            .and_then(|index| self.spline.sample(index + 1))
            .map(Spline::track_sample);

        // **Latched on the distance itself**, not on the counters below, so where
        // the craft is put back cannot depend on the order the two are updated
        // in. See [`Race::last_on_track`].
        if let (Some(index), Some(distance)) = (index, spline_distance)
            && distance <= self.player_rescue_distance
        {
            self.last_on_track = u32::try_from(index).unwrap_or(u32::MAX);
        }

        if let Some(index) = index {
            // An index into this module's own sample table, which is *not* what
            // `Ship::segment` documents: that field means a per-path control-point
            // segment. Nothing reads it today, and it is written because a locator
            // that keeps no note of where it was is the thing that has to be replaced
            // when the brute-force scan does. Converting the one into the other needs
            // a lap-counting convention nobody has recovered.
            self.world.ships[0].segment = u16::try_from(index).unwrap_or(u16::MAX);
        }

        // Zone's auto-speed, from the zone the run has reached. Read before the
        // step, from the zone the last tick left behind, because that is the
        // order the original runs in: `Zone_Update` assigns the counter into the
        // craft and `Ship_UpdateEngine` reads it on the following craft update.
        let auto_speed = self
            .zone
            .map(|zone| oag_race::zone::thrust(zone.start, zone.increment, self.world.race.zone));
        let before = self.world.ships[0].physics.body.position;
        // Step 15's input, measured before the step because that is when the
        // original measures it: `Ship_ApplySpeedupPad` runs inside the same craft
        // update as the other fourteen terms, all of them against the position the
        // tick started at, and the integrator moves the body afterwards.
        let (moved, sweep) = self.pad_sweep(0, before);
        let pad_hit = self.test_speedup_pads(0, before, moved, &sweep);
        // After the speed pad, sharing its sweep. The two are independent - a
        // track can author a weapon pad on top of a speed pad and both fire -
        // and this one returns nothing, because a pickup is an event rather than
        // a per-tick force.
        self.test_weapon_pads(0, before, moved, &sweep);
        let ship = &mut self.world.ships[0];
        let env = Environment {
            track_sample,
            track_sample_next,
            auto_speed,
            pad_hit,
            class_gravity_scale: self.class_gravity_scale,
            // The mode's own `Weapons`/`Damage` defaults, which decide both what
            // a wall costs the energy pool and whether it recovers - a time trial
            // and a speed lap run with both off, so their pool floors at 20 and
            // regenerates, exactly as the disc's manual text describes. See
            // `oag_gameplay::damage_rules`.
            damage_rules: oag_gameplay::damage_rules(self.world.race.mode),
            ..Environment::default()
        };
        let evaluated = oag_physics::step(
            &mut ship.physics,
            &controls,
            &ship.handling,
            &env,
            &self.collision,
            self.dt,
        );

        // **After the craft moved and before the race rules.** A rocket fired
        // this tick was spawned from the pose the tick *started* at, in
        // `spend_pickup`, so flying it here gives it a full tick of travel from
        // where the muzzle was rather than half a tick from wherever the step
        // ended up - which is the same argument that puts the Turbo's boost on
        // the tick it was fired. Before the rules, so a craft blown up by a
        // blast this tick is blown up before the lap counter reads it.
        // The whole table rather than the Rocket's block: a blast is looked up by
        // the weapon that made it now that more than one weapon can make one.
        let damage_rules = oag_gameplay::damage_rules(self.world.race.mode);
        let impacts = oag_gameplay::projectile::step(
            &mut self.world,
            self.dt,
            &self.collision,
            self.weapons.as_ref(),
            super::to_format_class(self.class),
            damage_rules,
        );
        // After `projectile::step`, so a flare rides where its rocket
        // actually ended the tick rather than a tick behind it.
        self.advance_projectile_flares();
        for impact in impacts.iter().flatten() {
            self.ignite_blast(impact.point, impact.struck.map(usize::from));
        }
        // After the craft have moved, so a flare sits on this tick's nozzle
        // rather than the last one's.
        self.advance_engine_flares();
        self.stage.advance(self.dt, &mut self.stage_rng);

        self.respawn_cooldown[0] = self.respawn_cooldown[0].saturating_sub(1);
        // Unconditionally and before the `||`, so the dwell sees every tick -
        // the same argument `step_opponents` makes for its two counters.
        let off_the_track = self.lost_off_the_track(spline_distance);
        if off_the_track || self.reset_zone_touched(0, &env, before) {
            // The last sample the craft was *on the track* at, which for a reset
            // contact is where it was a tick or two ago and for the off-track
            // trigger is where it left. `index` - the nearest sample to wherever
            // the craft is now - would be that same place for the first and a
            // sample on some other part of the circuit for the second.
            self.respawn(0, Some(self.last_on_track as usize));
        } else if self.respawn_cooldown[0] == 0 {
            // Clear of the trigger with the cooldown expired: whatever run of
            // back-to-back respawns was happening is over.
            self.respawns_in_a_row[0] = 0;
        }

        self.step_opponents();
        self.resolve_craft_pairs();

        self.world.tick += 1;

        // The race rules run last of the simulation, on the position the step
        // produced and the tick it produced it on. Running them before the step
        // would test last tick's position against this tick's clock, which is a
        // whole tick of error on a quantity whose job is to be exact at one
        // instant. A track with no closed ring simply has no lap counter; the
        // load report already said so.
        // The craft finished blowing up this tick, so the race is over. Checked
        // before the rules run rather than after, because a wrecked craft's
        // position should not go on counting laps - and checked every tick
        // because `RaceState::eliminate` is the thing that makes it idempotent.
        //
        // Only Zone can reach this: a time trial and a speed lap race with the
        // original's `Damage` option off, which floors their pool at 20. See
        // `oag_gameplay::damage_rules`.
        if self.world.ships[0].physics.craft_state == oag_physics::CraftState::Eliminated
            && self.world.race.eliminate()
        {
            // The original plays `_BLOWUP`, hides the HUD and swings the camera
            // into its mode 5 on the way here. **None of that is built** - there
            // is no explosion, no HUD hide and no camera mode - so the race
            // simply stops. See `oag_physics::damage::CraftState`.
        }

        if let Some(course) = &self.course {
            let position = self.world.ships[0].physics.body.position;
            // The same flag the collision sparks fire on, so "the HUD says that
            // zone was not clean" and "sparks came off the hull" cannot disagree.
            let contact = evaluated.wall.impact;
            let outcome =
                self.world
                    .race
                    .update(course, position, self.world.tick, self.dt, contact);

            // A zone survived without touching anything pays shield back, clamped
            // to the ship's own pool. `Ship_SetShield` (`0x0883e6f4`) does the
            // same clamp against the stat block's maximum, so a full ship gains
            // nothing and the bar cannot overfill.
            if outcome.perfect_zone
                && let Some(zone) = self.zone
            {
                let ship = &mut self.world.ships[0];
                let max = ship.handling.dimensions.shield;
                ship.physics.shield = (ship.physics.shield + zone.recharge).min(max);
            }

            if outcome.lap_completed {
                self.grant_free_turbo();
            }
        }

        // Every craft's standing, the player's included, from the position it
        // holds now. **After the player's `RaceState`**, so the two see the same
        // tick, and slot 0's lap is then taken from the standing rather than
        // counted a second time - see `Ship::standing`. Outside the borrow above
        // rather than inside it because this walks the whole ship array.
        self.update_standings();

        // **After the standings**, so the last crossing is in the table this
        // reads, and a no-op on every tick but the one the race ends on. It
        // takes a snapshot and touches no simulation state, which is what lets
        // it sit inside the tick without moving a hash - see
        // [`Race::capture_results`].
        self.capture_results();

        let target = target_of(&self.world.ships[0]);
        self.camera.advance(target, &self.chase_params, self.dt);

        // Advanced here, on the fixed tick, and not in the frame loop. That is
        // what makes the headless `capture` path - which calls only `tick` -
        // produce the same flare at the same tick count as the window does, and
        // it is the same reason the chase camera is advanced from here.
        self.advance_exhausts();

        // Open while the boost's own timer runs, close once it has expired, both
        // as an exponential approach so neither edge is a step. Driven from
        // `pad_timer` rather than from the exhaust, because the exhaust's boost
        // is a `max` that other things may one day also arm and this must follow
        // the pad specifically.
        if self.boost_fov_kick != crate::display::BoostFovKick::OFF {
            let boosting = self.world.ships[0].physics.pad_timer > 0.0;
            let (target, rate) = if boosting {
                (1.0, BOOST_FOV_OPEN_RATE)
            } else {
                (0.0, BOOST_FOV_CLOSE_RATE)
            };
            self.boost_kick += (target - self.boost_kick) * rate * self.dt;
            // Snapped, so a closed kick is *exactly* zero and `projection` takes
            // its bit-identical path rather than an `atan(tan(x))` round trip
            // that never quite settles.
            if !boosting && self.boost_kick < 1.0e-3 {
                self.boost_kick = 0.0;
            }
        }

        // The flaps, on the fixed tick beside the camera and the exhaust, and
        // for the same reason: a headless capture calls only `tick`, so an
        // animation advanced in the frame loop would be at a different angle in
        // a screenshot than in a window at the same tick count.
        //
        // Two rates, not one. `up_speed` and `down_speed` are authored beside
        // each other in `<AirbrakeGraphics>` and are *not* the force law's
        // `gain`/`falloff`, so a flap deploys and returns at its own pace while
        // the airbrake it depicts ramps at another. The state it chases is the
        // ramped `airbrake_left`/`airbrake_right` on `0..=100` rather than the
        // raw input, so a flap follows the airbrake the ship actually has.
        //
        // **The rates are on the airbrake's own `0..=100` scale**, per second,
        // which is why `self.flaps` holds a level rather than an angle. Not
        // read out of the binary - no consumer of `<AirbrakeGraphics>` was
        // located - but not a coin flip either: every team authors
        // `up_speed = 500` against a `down_speed` of 80 to 120, and on this
        // reading that is a flap that snaps out in 0.2 s and folds back over
        // about a second, which is what an airbrake does. Read as *fractions*
        // of full deflection per second instead, 500 would be full travel in
        // two milliseconds and the parameter would not be worth authoring.
        // It is also the convention `<Airbrake gain/falloff>` already uses on
        // the same scale, one element away.
        let ship = &self.world.ships[0].physics;
        for (flap, level) in self
            .flaps
            .iter_mut()
            .zip([ship.airbrake_left, ship.airbrake_right])
        {
            let target = level.clamp(0.0, 100.0);
            let (rate, rising) = if target > *flap {
                (self.flap_graphics.up_speed, true)
            } else {
                (self.flap_graphics.down_speed, false)
            };
            let step = rate * self.dt;
            *flap = if rising {
                (*flap + step).min(target)
            } else {
                (*flap - step).max(target)
            };
        }

        // Cooldown-gated, not edge-triggered - see `Self::sparks_cooldown`'s
        // doc comment for why a sustained scrape must re-fire periodically
        // rather than spawn once and go silent.
        self.sparks_cooldown = (self.sparks_cooldown - self.dt).max(0.0);
        // The trigger rule runs whether or not the disc's own effect
        // loaded - see `Setup::effects`. Only the pool needs it.
        let effect = self.effects.get(sparks::DAMAGE_EFFECT).cloned();
        let can_fire = evaluated.wall.impact && self.sparks_cooldown <= 0.0;
        let model_matrix = self.ship_model_matrix();
        if can_fire && let Some(contact) = evaluated.wall.resolved {
            // `contact.point` is deliberately the *penetrating* hull sample
            // point, not the wall surface - see its doc comment on
            // `oag_physics::wall::WallContact`, which matches the original's
            // own `Collision_AddContact` exactly. Correct for physics, wrong
            // for a visual: that point sits up to `depth` units inside the
            // opaque wall, so a spark quad centred there is depth-tested away
            // by the wall's own geometry almost every time and reads as
            // nothing spawning. `point + normal * depth` is the same
            // correction `resolve_contact`'s `escape` vector already applies
            // to push the hull back out to the surface.
            let surface = contact.point + contact.normal * contact.depth;
            // The original triggers the `Ship Collision Fx` locator nearest
            // the contact and the burst emits from that node from then on
            // (`Ship_DispatchCollisionFx`); with no locators authored, the
            // surface point itself is mapped into model space so it still
            // rides the hull with rotation.
            let anchor_model = self
                .collision_fx
                .iter()
                .copied()
                .min_by(|a, b| {
                    let da = (model_matrix.transform_point3(*a) - surface).length_squared();
                    let db = (model_matrix.transform_point3(*b) - surface).length_squared();
                    da.total_cmp(&db)
                })
                .unwrap_or_else(|| model_matrix.inverse().transform_point3(surface));
            self.sparks_anchor = Some(anchor_model);
            if let Some(effect) = effect.as_deref() {
                self.sparks.ignite(
                    effect,
                    surface,
                    sparks::severity(evaluated.wall.impact_speed),
                );
            }
            self.sparks_ignitions += 1;
            self.sparks_cooldown = sparks::COLLISION_COOLDOWN;
        }
        // Unconditional, like the exhaust: the emitters keep trickling and
        // already-live particles keep ageing even on a tick with no fresh
        // impact. The anchor is the chosen hull locator under the ship's
        // *current* transform, the way the original's scene-graph node rides
        // the craft.
        let anchor = self
            .sparks_anchor
            .map_or(self.ship().physics.body.position, |local| {
                model_matrix.transform_point3(local)
            });
        if let Some(effect) = effect.as_deref() {
            self.sparks
                .advance(effect, self.dt, anchor, &mut self.sparks_rng);
        }

        evaluated
    }
}
