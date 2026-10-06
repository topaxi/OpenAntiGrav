//! [`Race::tick`]: one 60 Hz step of the simulation, from an input snapshot to
//! the state the renderer and the HUD read.
//!
//! Split out of `race.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change.

use super::*;

impl Race {
    /// Advances the simulation one fixed tick, and the camera with it.
    ///
    /// Each slot's snapshot is mapped through [`oag_gameplay::ship_controls`],
    /// which is the one place "cross is thrust" is written down.
    ///
    /// # One snapshot per grid slot, and one craft stepped from one of them
    ///
    /// This took a single [`InputSnapshot`] until 2026-09-16, which made "the
    /// player" and "slot 0" the same statement and left nowhere to put a second
    /// person's controls. It now takes [`PlayerInputs`] - one snapshot per slot,
    /// indexed the same way [`oag_gameplay::World::race`] and
    /// `World::controllers` are - so split screen, a second window and a remote
    /// client differ only in where a snapshot comes from.
    ///
    /// **What this change does *not* do is step two human craft.** The player
    /// half of this function still runs once, for
    /// [`oag_gameplay::World::primary_slot`], and returns that craft's
    /// [`Evaluated`] for the camera and the view to read. Making the step
    /// itself per-slot means N cameras, N `Evaluated`s and N HUDs, which is
    /// split screen's own work rather than this prerequisite's - see the
    /// handover thread that planned it. The slot is read rather than written as
    /// `0` so that work is a change of loop bound and not a hunt through the
    /// file.
    ///
    /// The pieces that *do* already run over the whole grid - `lay_mines`,
    /// [`Self::advance_cannons`], `slowdown::drain`, `disruption::advance`,
    /// [`Self::step_opponents`] - ask `World::controllers` which slots take a
    /// snapshot, so they are per-player today.
    ///
    /// [`PlayerInputs`]: oag_gameplay::PlayerInputs
    pub fn tick(&mut self, inputs: &oag_gameplay::PlayerInputs) -> Evaluated {
        // Decays every tick regardless of contact, like `Camera_ArmShake`'s own
        // `shake_timer`, and **first**: the original arms the shake while it
        // updates the craft and reads the full `0.6` s timer in the same
        // frame's `Camera_SubmitScene`, decrementing after. Advancing here, before
        // anything this tick can arm it, gives the same order - the frame an
        // impact lands in shows the shake at progress 0 and the next one at
        // one tick - where advancing at the end of the tick would show the
        // impact frame already one tick in. Measured 2026-09-30, see
        // `oag_render::camera::shake`.
        self.view.shake.advance(self.sim.dt);
        // Before anything steps, so the perfect start's edge is a change
        // across this tick - see `race::perfect_start`.
        let launch_grades = self.launch_grades();
        // Every craft is in the original's grid state until the green light:
        // `Race_PlaceGrid` puts the field in state 0 and `Race_StartRacing`
        // moves it to state 1. Derived from the one countdown clock rather than
        // stored, so a respawn after the start (state 3) can never re-enter it
        // and a restart, which rebuilds the race, always begins in it. Physics
        // reads the flag; see `oag_physics::ShipState::on_grid`.
        //
        // **Zone is in it too** (2026-10-02, read live on a Zone engine): flags bit 1 is
        // the grid state and reads `0x3` through Zone's countdown with the craft still,
        // and Zone's auto-speed is gated on it (`Ship_UpdateEngine`'s `(flags & 1) &&
        // !(flags & 2)`), as the four-corner epilogue's `craft+0x2a4` guard is. See
        // `docs/physics/grid-state.md`.
        //
        // **One tick before thrust is released**, not the same one: the craft
        // enters state 1 on the frame *before* the throttle word steps
        // (`flags_1c0` and `state_2a4` read `0x1`/`1` there on four live runs,
        // `docs/physics/grid-state.md`), so the coupling and the launch boost's
        // clock both start a tick ahead of the first thrust. Measured on Pulse PSP.
        let on_grid = RaceState::thrust_gated(self.sim.world.tick + 1);
        // The launch boost's clock starts at the same tick in every mode, Zone
        // included (it has a countdown; its auto-speed is not multiplied yet, see
        // `oag_physics::engine::engine`).
        let released = !RaceState::thrust_gated(self.sim.world.tick + 1);
        // Zone's hover is the four-corner variant for every craft in the field.
        let four_corner = self.sim.world.mode() == Mode::Zone;
        for ship in &mut self.sim.world.ships {
            ship.physics.on_grid = on_grid;
            ship.physics.released = released;
            ship.physics.four_corner = four_corner;
        }
        // The one craft a person is flying this tick. `0` under
        // `World::SINGLE_PLAYER`, which is every session this engine starts, so
        // reading it changes nothing and hard-coding it would have cost the
        // next reader the search.
        let player = self.sim.world.primary_slot();
        let snapshot = inputs.get(player);
        // Flown for the player by either route - the operator's `--autopilot`
        // or the pickup - and then the snapshot is not consulted for steering
        // at all. See [`Race::flown_for_the_player`].
        //
        // Before `spend_pickup`, which still reads the snapshot: an autopiloted
        // craft is steered for and fires nothing, because nothing picks a target
        // for anybody yet. It also means a pickup armed *this* tick takes hold
        // on the next one, which is a tick of latency this port has and has not
        // measured against the original.
        let flown = self.flown_for_the_player(player);
        let mut controls = if flown {
            self.autopilot_controls(player)
        } else {
            ship_controls(snapshot, self.sim.scheme)
        };
        // Past the line the original's own law sets the pace - see
        // `race::finished_thrust`.
        if let Some(cap) = self.finished_thrust_cap(player) {
            controls.thrust = controls.thrust.min(cap);
        }
        // A Disruptor hit filters whatever the pilot - human or driver -
        // asked for: no thrust, no airbrakes, a mirrored yaw, or an
        // autopilot's thrust scale. Before the start-line gate below, so a
        // stalled craft on the line is still gated and not doubly so. See
        // `oag_weapons::disruption`.
        controls = self.disrupted_controls(player, controls, flown);
        // The start-line countdown: measured, not authored - see
        // `RaceState::thrust_gated` for the live capture this reproduces. Only
        // thrust was held and recorded, so only thrust is gated; steering stays
        // live through the countdown, which is unmeasured either way but costs
        // nothing to leave alone. Autopilot is gated too - the capture measured
        // the engine's own gate, not a distinction between input sources.
        if RaceState::thrust_gated(self.sim.world.tick) {
            controls.thrust = 0.0;
        }

        // **Before every craft is stepped, and over the whole field at once.**
        // The weapon slowdown a blast credited last tick becomes a running timer
        // here, so this tick's engine, hover and grip all see it - the original
        // drains the same slot from the craft's entity update, ahead of
        // `Ship_UpdateCraft`. One call for all eight slots rather than one
        // beside each step, so the drain cannot depend on whether a craft is
        // flown by slot 0's branch or the field's. See
        // `oag_weapons::slowdown::drain`.
        oag_weapons::slowdown::drain(
            &mut self.sim.world.ships[..self.sim.world.ship_count as usize],
            self.sim.weapons.as_ref().map(|table| table.slowdown_limit),
        );

        // Before the force law, so a Turbo fired this tick boosts this tick.
        self.spend_pickup(player, snapshot);
        // After it, so a pickup armed this tick gets its whole duration rather
        // than a tick less, and so the cancel-on-fire branch inside
        // `spend_pickup` is not immediately undone by a decrement.
        self.tick_autopilot(player);
        // The start-of-race voice, on the ticks the original plays it.
        self.tick_countdown_voice();
        // Immediately after both, so the first mine of a cluster is laid on the
        // tick the button was pressed and from where the craft was when it was
        // pressed. A mine never moves again, so this is the only tick that can
        // place it correctly.
        self.lay_mines();
        // Right after, for the same reason: a Cannon's own countdown has to
        // see every tick the button is down, and a round fired this tick has
        // to leave from where the craft started it. It reads the snapshot for
        // the *held* state of fire rather than the press edge `spend_pickup`
        // above consumes - see `Race::advance_cannons`'s own doc comment.
        self.advance_cannons(inputs);

        // The two spline samples the magstrip hold reads. In the original these are
        // `AiTrack_LocatePosition`'s two output records on the ship entity; here
        // they are the nearest table entry and the one after it, which is the same
        // "where am I and what is next" pair at this crate's resolution. Off a
        // magstrip nothing consumes them, so a track without one is unaffected.
        let nearest = self
            .sim
            .spline
            .nearest(self.sim.world.ships[player].physics.body.position);
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
            .and_then(|index| self.sim.spline.sample(index + 1))
            .map(Spline::track_sample);

        // **Latched on the distance itself**, not on the counters below, so where
        // the craft is put back cannot depend on the order the two are updated
        // in. See [`RaceSim::last_on_track`].
        if let (Some(index), Some(distance)) = (index, spline_distance)
            && distance <= self.sim.player_rescue_distance
        {
            // Zero on an index that will not fit, matching `Race::start` - the
            // arm is unreachable on any real table, and the value it writes is
            // read back as a sample index, so it has to be a *valid* one. A
            // saturating `u32::MAX` here would make `Race::respawn` find no
            // sample and return having recovered nothing, silently.
            self.sim.last_on_track = u32::try_from(index).unwrap_or(0);
        }

        if let Some(index) = index {
            // An index into this module's own sample table, which is *not* what
            // `Ship::segment` documents: that field means a per-path control-point
            // segment. Nothing reads it today, and it is written because a locator
            // that keeps no note of where it was is the thing that has to be replaced
            // when the brute-force scan does. Converting the one into the other needs
            // a lap-counting convention nobody has recovered.
            self.sim.world.ships[player].segment = u16::try_from(index).unwrap_or(u16::MAX);
        }

        // Zone's auto-speed, from the zone the run has reached. Read before the
        // step, from the zone the last tick left behind, because that is the
        // order the original runs in: `Zone_Update` assigns the counter into the
        // craft and `Ship_UpdateEngine` reads it on the following craft update.
        let auto_speed = self.sim.zone.map(|zone| {
            oag_race::zone::thrust(
                zone.start,
                zone.increment,
                self.sim.world.primary_race().zone,
            )
        });
        let before = self.sim.world.ships[player].physics.body.position;
        // Step 15's input, measured before the step because that is when the
        // original measures it: `Ship_ApplySpeedupPad` runs inside the same craft
        // update as the other fourteen terms, all of them against the position the
        // tick started at, and the integrator moves the body afterwards.
        let (moved, sweep) = self.pad_sweep(player, before);
        let pad_hit = self.test_speedup_pads(player, before, moved, &sweep);
        // After the speed pad, sharing its sweep. The two are independent - a
        // track can author a weapon pad on top of a speed pad and both fire -
        // and this one returns nothing, because a pickup is an event rather than
        // a per-tick force.
        self.test_weapon_pads(player, before, moved, &sweep);
        // The mode's rules, read before the craft is borrowed: `World::mode`
        // asks the whole world and the borrow checker will not have both.
        let env_damage_rules = self.sim.damage_rules();
        let ship = &mut self.sim.world.ships[player];
        // `Ship_UpdateEngine`'s read-and-reset of `craft+0x31c`: the beam's
        // throttle armed last tick is consumed by this step and nothing after
        // it. See `Ship::pending_thrust_scale`. One asymmetry: the original's
        // engine early-returns *above* that line for a stunned or slowed
        // craft and so keeps the armed value for a later tick, where this
        // discards it either way. A beam re-arms it every tick the link
        // holds, so the two differ only on the tick a stun ends after the
        // link has already broken - one tick of throttle, not chased.
        let thrust_scale = std::mem::replace(&mut ship.pending_thrust_scale, 1.0);
        let env = Environment {
            track_sample,
            track_sample_next,
            auto_speed,
            pad_hit,
            thrust_scale,
            class_gravity_scale: self.sim.class_gravity_scale,
            start_boost: self.sim.start_boost,
            // The mode's own `Weapons`/`Damage` defaults, which decide both what
            // a wall costs the energy pool and whether it recovers - a time trial
            // and a speed lap run with both off, so their pool floors at 20 and
            // regenerates, exactly as the disc's manual text describes. See
            // `oag_gameplay::damage_rules`.
            damage_rules: env_damage_rules,
            ..Environment::default()
        };
        let evaluated = oag_physics::step(
            &mut ship.physics,
            &controls,
            &ship.handling,
            &env,
            &self.sim.collision,
            self.sim.dt,
        );
        // The contact half of the same edge as the blast one below, and the one
        // a player meets first: scraping a wall behind a shield costs nothing
        // and bulges the shell. The original's contact loop takes exactly this
        // branch instead of its hull-damage call.
        if evaluated.shield.absorbed {
            self.view.shield[player].hit();
        }
        // **After the craft moved and before the race rules.** A rocket fired
        // this tick was spawned from the pose the tick *started* at, in
        // `spend_pickup`, so flying it here gives it a full tick of travel from
        // where the muzzle was rather than half a tick from wherever the step
        // ended up - which is the same argument that puts the Turbo's boost on
        // the tick it was fired. Before the rules, so a craft blown up by a
        // blast this tick is blown up before the lap counter reads it.
        // The whole table rather than the Rocket's block: a blast is looked up by
        // the weapon that made it now that more than one weapon can make one.
        let damage_rules = self.sim.damage_rules();
        // One report per slot: a hit a shield swallowed (the *only* thing that
        // makes a shell visibly react - see `oag_render::shield::ShipShield::hit`)
        // or one that got through (the hull's own sparks). Both have to come
        // back out of the step rather than being invisible on both sides.
        let mut hits = [oag_weapons::projectile::WeaponHit::default(); MAX_SHIPS];
        // Read before the step, for `ignite_missile_bounces` below: a bounce
        // never stops a projectile, so it never reaches `impacts` and the
        // only way to see one is to compare this counter before and after.
        let bounces_before: [u8; oag_weapons::projectile::MAX_PROJECTILES] =
            std::array::from_fn(|slot| self.sim.world.projectiles.slots[slot].bounces);
        // Read before the step too, for `ignite_blast`'s Bomb arm below: a
        // detonated slot resets to `Projectile::default()` inside `step`
        // (`Quat::IDENTITY`), so the frozen pose a laid Bomb detonated with
        // has to be read before that happens - the same before/after shape
        // `bounces_before` already takes, and `impacts` is slot-indexed the
        // same way. Read for every slot rather than gated on `kind ==
        // Bomb`, since nothing here is on the determinism-hashed path.
        let orientations_before: [Quat; oag_weapons::projectile::MAX_PROJECTILES] =
            std::array::from_fn(|slot| self.sim.world.projectiles.slots[slot].orientation);
        let impacts = oag_weapons::projectile::step(
            &mut self.sim.world.projectiles,
            &mut self.sim.world.ships[..self.sim.world.ship_count as usize],
            self.sim.dt,
            &self.sim.collision,
            self.sim.weapons.as_ref(),
            &self.sim.class,
            damage_rules,
            &mut hits,
        );
        for (slot, hit) in hits.iter().enumerate() {
            if hit.absorbed {
                self.view.shield[slot].hit();
            }
            if hit.landed {
                self.note_weapon_hit(slot);
            }
        }
        // A hit that got through throws the struck hull's own sparks - the
        // victim's `Ship_Damage`, not the weapon. See `race::hit_sparks`.
        self.throw_hit_sparks(&hits, false);
        // After `projectile::step`, so a flare rides where its rocket
        // actually ended the tick rather than a tick behind it.
        self.advance_projectile_flares();
        for (slot, impact) in impacts
            .iter()
            .enumerate()
            .filter_map(|(slot, impact)| impact.as_ref().map(|impact| (slot, impact)))
        {
            // Eliminator's own kill attribution - see `crate::eliminator`.
            // A direct hit only: a blast's own splash damage against a craft
            // it did not directly strike is not credited, which this project
            // labels a chosen simplification rather than a recovery.
            if let Some(struck) = impact.struck {
                self.sim.last_damager[struck as usize] = Some(impact.owner);
            }
            self.credit_blast(impact, &hits);
            self.ignite_blast(
                impact.kind,
                impact.point,
                impact.struck.map(usize::from),
                orientations_before[slot],
            );
            // `Plasma_SweepCraftHit` (`0x0886afb8`) plays `PLASMAHITSHIP` on a
            // craft hit and clears the bolt's own emitter before
            // `Plasmas_Update`'s pass-two teardown would otherwise play
            // `PLASMAHITWALL` unconditionally - so the original plays exactly
            // one of the two per ending, never both. `impact.struck.is_some()`
            // is this port's own equivalent of that emitter-cleared branch:
            // a craft hit (the shared sweep-segment test in
            // `oag_weapons::projectile::flight`) plays `PlasmaHitShip`, and a
            // wall hit or the 10 s timeout (`struck: None` either way) plays
            // `PlasmaHitWall` as before. See
            // `docs/ghidra/functions/psp-pulse-usa/plasma.md`'s "a craft hit
            // is the third ending" section and `oag_sound::sfx::Cue::PlasmaHitShip`'s
            // own doc comment.
            //
            // The Rocket and the Cannon share the same wall/craft split, each
            // off its own pair of cues - see `Cue::RocketHitWall`/
            // `Cue::RocketHitShip` and `Cue::CannonHitWall`/`Cue::CannonHitShip`
            // for the evidence and, for both, why a `struck: None` reaching
            // here can never be their own weapon's flight-time reap: neither
            // one's timeout path in `oag_weapons::projectile::flight` writes
            // an `Impact` at all, so every entry this loop sees for either
            // kind is a real wall or craft hit.
            use oag_tables::weapons::Weapon;
            let hit_cues = match impact.kind {
                Weapon::Plasma => Some((
                    oag_sound::sfx::Cue::PlasmaHitWall,
                    oag_sound::sfx::Cue::PlasmaHitShip,
                )),
                Weapon::Rocket => Some((
                    oag_sound::sfx::Cue::RocketHitWall,
                    oag_sound::sfx::Cue::RocketHitShip,
                )),
                Weapon::Cannon => Some((
                    oag_sound::sfx::Cue::CannonHitWall,
                    oag_sound::sfx::Cue::CannonHitShip,
                )),
                _ => None,
            };
            // A Missile's own endings: a craft hit plays `MISSILEEXPSHIP`, and
            // one that outlived its fuse plays `SHURIKENEXPL` - the fuse is the
            // one impact that struck nothing and spends no blast
            // (`blast: false`). A missile that spends its bounce budget on a
            // wall also strikes nothing but does blast, and plays neither: the
            // pool plays `MISSILEEXPWALL` off a bit this port has not read the
            // setter of. See `Cue::MissileHitShip` and `Cue::MissileExpire`.
            if let Some(cue) = weapons::missile_ending_cue(impact) {
                self.sim
                    .cues
                    .push(oag_sound::sfx::CueEvent::at_point(cue, impact.point));
            }
            // HD's Cannon throws its own spark on the craft it strikes, from
            // the weapon's side - see `Race::throw_weapon_spark`. A no-op
            // unless the title built weapon anchors (HD only).
            if let (Weapon::Cannon, Some(struck)) = (impact.kind, impact.struck) {
                self.throw_weapon_spark(usize::from(struck), impact.point);
            }
            if let Some((wall, ship)) = hit_cues {
                let cue = if impact.struck.is_some() { ship } else { wall };
                self.sim
                    .cues
                    .push(oag_sound::sfx::CueEvent::at_point(cue, impact.point));
            }
        }
        // After the loop above, so a Plasma detonated this tick is already
        // in the pool at `age == 0.0` when this ages every slot - the same
        // one-tick order `advance_projectile_flares` above takes relative to
        // a freshly-placed flare. See `blast_models` module doc comment.
        self.advance_plasma_blast_models(self.sim.dt);
        // Same one-tick order, for the Bomb's own hemisphere/shockwave pool.
        self.advance_bomb_blast_models(self.sim.dt);
        self.advance_mag_floor_fx();
        self.advance_magstrip_wake();
        self.ignite_missile_bounces(&bounces_before);
        // The same `bounces_before`/after edge the visual bounce above reads,
        // for the two weapons' own bounce cues - `Cue::MissileHitWall`
        // (`MISSILEEXPWALL`, on every bounce, not the projectile's final
        // ending - see that variant's own doc comment) and `Cue::ShurikenHit`
        // (`SHURIKENHIT`, `Shuriken_Bounce`, confidence 88).
        // `weapons::bounced_this_tick` already gates on the weapon, so a
        // stray nonzero `bounces` on some other kind never fires either cue.
        for (slot, projectile) in self.sim.world.projectiles.slots.iter().enumerate() {
            if !weapons::bounced_this_tick(
                projectile.kind,
                bounces_before[slot],
                projectile.bounces,
            ) {
                continue;
            }
            match projectile.kind {
                Some(oag_tables::weapons::Weapon::Missile) => {
                    self.sim.cues.push(oag_sound::sfx::CueEvent::at_point(
                        oag_sound::sfx::Cue::MissileHitWall,
                        projectile.position,
                    ));
                }
                // On the blade's own emitter - see `Cue::ShurikenHit`.
                Some(oag_tables::weapons::Weapon::Shuriken) => {
                    self.sim.cues.push(oag_sound::sfx::CueEvent::at_point(
                        oag_sound::sfx::Cue::ShurikenHit,
                        projectile.position,
                    ));
                }
                _ => {}
            }
        }
        // After the craft have moved, so a flare sits on this tick's nozzle
        // rather than the last one's.
        self.advance_engine_flares();
        self.advance_absorb_bursts();
        self.advance_hit_sparks();
        self.advance_wreck_fx();
        self.view
            .stage
            .advance(self.sim.dt, &mut self.view.stage_rng);
        self.advance_scenery_fx(self.sim.dt);

        self.sim.respawn_cooldown[player] = self.sim.respawn_cooldown[player].saturating_sub(1);
        // Unconditionally and before the `||`, so the dwell sees every tick -
        // the same argument `step_opponents` makes for its two counters.
        let off_the_track = self.lost_off_the_track(player, spline_distance);
        // **Not a wreck**, the same gate `step_opponents` applies: in an
        // Eliminator the player sits `Eliminated` for the respawn dwell, and
        // a rescue firing inside it would rebuild the craft as a racing one
        // with an empty pool. In a single race the ending has already fired.
        let alive =
            self.sim.world.ships[player].physics.craft_state == oag_physics::CraftState::Racing;
        let reset = self.reset_zone_touched(player, &env, before);
        let airborne = self.airborne_too_long(player);
        if alive && (off_the_track || airborne || reset) {
            self.sim.last_respawn_cause[player] = Some(if off_the_track {
                respawn::RespawnCause::OffTrack
            } else if airborne {
                respawn::RespawnCause::Airborne
            } else {
                respawn::RespawnCause::ResetZone
            });
            // The last sample the craft was *on the track* at, which for a reset
            // contact is where it was a tick or two ago and for the off-track
            // trigger is where it left. `index` - the nearest sample to wherever
            // the craft is now - would be that same place for the first and a
            // sample on some other part of the circuit for the second.
            self.respawn(player, Some(self.sim.last_on_track as usize));
        } else if self.sim.respawn_cooldown[player] == 0 {
            // Clear of the trigger with the cooldown expired: whatever run of
            // back-to-back respawns was happening is over.
            self.sim.respawns_in_a_row[player] = 0;
        }

        self.step_opponents();
        self.resolve_craft_pairs();
        // Every craft's disruption counts down *after* the tick's controls
        // were filtered with it - `Ship_UpdateWeapons`'s own order, so an
        // effect of one tick is one tick. Over the whole field at once, as
        // `slowdown::drain` is, so the countdown cannot depend on which
        // branch stepped the craft.
        oag_weapons::disruption::advance(
            &mut self.sim.world.ships[..self.sim.world.ship_count as usize],
            self.sim.dt,
        );

        // The destroyed-craft pass, over the whole field: Eliminator's
        // respawn-and-bookkeeping, and a single race's opponent respawn.
        // Before the ending check just below and before the standings, so a
        // craft respawned this tick is placed where it is put rather than
        // left mid-explosion for the lap counter to read. See
        // `crate::eliminator`.
        self.tick_destroyed_craft();
        self.advance_craft_flashes();
        self.advance_destroy_camera();

        self.sim.world.tick += 1;

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
        // Zone *and a single race* reach this - a time trial and a speed lap do
        // not, racing with the original's `Damage` option off, which floors
        // their pool at 20. See `oag_gameplay::damage_rules`, and
        // `RaceState::eliminate` for the disc text that settles the single
        // race's own answer as the same one.
        //
        // **Eliminator does not reach this branch at all.**
        // `tick_destroyed_craft` above has already turned this tick's
        // `Eliminated` craft back into a `Racing` one via a respawn, before
        // this read - see `crate::eliminator`'s own doc comment for why
        // the mode's own text rules out `RaceState::eliminate` as its ending.
        // An *opponent* in a single race never reaches it, because the branch
        // reads the player's own state alone; an opponent's wreck stays down
        // (the original's state 6, measured live) and ends nothing.
        if self.sim.world.mode() != Mode::Eliminator
            && self.sim.world.ships[player].physics.craft_state
                == oag_physics::CraftState::Eliminated
            && self.sim.world.primary_race_mut().eliminate()
        {
            self.view.wreck_ended_tick = Some(self.sim.world.tick);
            // The original plays `~BLOWUP`, hides the HUD and swings the camera
            // into its mode 5 on the way here. **The sound is built** and is
            // held on `Race::craft_is_exploding` rather than raised here,
            // because case 4 opens a handle rather than firing a one-shot. The
            // HUD hide and the camera mode are still absent - case 4's own
            // addresses for both are on zone-mode.md. See
            // `oag_physics::damage::CraftState`.
        }

        {
            let body = &self.sim.world.ships[player].physics.body;
            self.view
                .run_stats
                .observe(body.linear_velocity.dot(body.forward()).abs());
        }

        if let Some(course) = &self.sim.course {
            let position = self.sim.world.ships[player].physics.body.position;
            // The same flag the collision sparks fire on, so "the HUD says that
            // zone was not clean" and "sparks came off the hull" cannot disagree.
            let contact = evaluated.wall.impact;
            let tick = self.sim.world.tick;
            let outcome = self.sim.world.primary_race_mut().update(
                course,
                position,
                tick,
                self.sim.dt,
                contact,
            );

            // A zone survived without touching anything pays shield back, clamped
            // to the ship's own pool. `Ship_SetShield` (`0x0883e6f4`) does the
            // same clamp against the stat block's maximum, so a full ship gains
            // nothing and the bar cannot overfill.
            if outcome.perfect_zone {
                self.view.run_stats.perfect_zones += 1;
            }
            if outcome.perfect_zone
                && let Some(zone) = self.sim.zone
            {
                let ship = &mut self.sim.world.ships[player];
                let max = ship.handling.dimensions.shield;
                ship.physics.shield = (ship.physics.shield + zone.recharge).min(max);
            }

            // The announcer's own trigger: raised on every zone step, whether
            // or not this title's ladder names that particular number - the
            // same "raise the edge, let the audio layer decide if it has
            // anything to say" split `raise_contact_cue` and `Banks::pick`
            // already keep. See `oag_title::ZoneAnnouncer`.
            if outcome.zone_advanced {
                self.push_announcement(self.sim.world.primary_race().zone);
            }

            // The speed-class announcer's own trigger, on a title whose
            // zone-to-stage ladder is recovered: the zone counter just
            // stepped by exactly one (`RaceState::advance_zone`'s own
            // `saturating_add(1)`), so the zone before this tick is the new
            // value minus one. Fires only when that step also crosses a
            // stage boundary - most zone steps do not, since the ladder's
            // bands are wider than one zone - which is what keeps this from
            // announcing a class on every zone survived. See
            // `oag_title::ZoneStages` and `oag_title::ZoneClassAnnouncer`.
            if outcome.zone_advanced
                && let Some(stages) = self.sim.zone_stages
            {
                let after = self.sim.world.primary_race().zone;
                let before = after.saturating_sub(1);
                if let Some(stage_after) = stages.stage_for(after)
                    && Some(stage_after) != stages.stage_for(before)
                {
                    self.push_class_announcement(stage_after);
                }
            }

            // The free Time Trial/Speed Lap Turbo, on every tick the player
            // crosses the line forwards - the first crossing included - as the
            // original's state-2 handlers do (`0x0882ddd8`, `0x0882d578`,
            // `0x08823270`: `craft+0x911` set -> held = 4). Never at the
            // release: a craft that has not reached the line holds nothing.
            if outcome.lap_completed || outcome.first_crossing {
                self.grant_free_turbo(player);
            }
            if outcome.lap_completed {
                // Eliminator's own per-lap mechanic, a no-op on every other
                // mode - see `Race::eliminator_lap_health_refill`.
                self.eliminator_lap_health_refill(player);
            }
        }

        // Every craft's standing, the player's included, from the position it
        // holds now. **After the player's `RaceState`**, so the two see the same
        // tick, and slot 0's lap is then taken from the standing rather than
        // counted a second time - see `Ship::standing`. Outside the borrow above
        // rather than inside it because this walks the whole ship array.
        self.update_standings();

        // **After the standings**, so the wave's own proximity test reads
        // every craft's freshest `Standing::progress` rather than last
        // tick's. See `Race::advance_quake`'s own doc comment.
        self.advance_quake();
        self.advance_quake_visual();

        // Beside the Quake and for the same reason: both are single-instance
        // weapons that read every craft's freshest state rather than last
        // tick's. See `Race::advance_leach_beam`.
        self.advance_leach_beam();
        self.advance_leach_beam_visual();
        self.advance_leach_beam_ribbon();

        // Beside them for the same reason: a Repulser's wave sweep reads every
        // craft's ring index from this tick. See `Race::advance_repulsers`.
        self.advance_repulsers();
        self.advance_repulser_visual();

        // **After the standings**, so the last crossing is in the table this
        // reads, and a no-op on every tick but the one the race ends on. It
        // takes a snapshot and touches no simulation state, which is what lets
        // it sit inside the tick without moving a hash - see
        // [`Race::capture_results`].
        self.capture_results();

        let target = target_of(&self.sim.world.ships[0]);
        self.view
            .camera
            .advance(target, &self.view.chase_params, self.sim.dt);
        // After the chase camera and before the eye is read: behind the end-race panels the
        // spectator director's pose replaces it. A no-op until the player has finished.
        self.advance_finish_camera();
        // After the camera, so the flash's falloff measures from this tick's eye.
        let eye = self.camera_position();
        if let Some(flash) = &mut self.view.screen_flash {
            flash.advance(self.sim.dt, eye);
        }

        // Advanced here, on the fixed tick, and not in the frame loop. That is
        // what makes the headless `capture` path - which calls only `tick` -
        // produce the same flare at the same tick count as the window does, and
        // it is the same reason the chase camera is advanced from here.
        // The perfect start arms the flare the way a pad does, so before the
        // exhausts take this tick's step, as the pad's arming is.
        self.fire_perfect_starts(&launch_grades);
        self.advance_exhausts();
        // After the trails have taken this tick's sample, so a craft is tested
        // against the ribbon as it stands now rather than one tick stale.
        self.advance_trail_hits();
        // Same argument, and after `step`/`step_opponents` so a shell that lost
        // its last tick of protection this frame starts fading this frame.
        self.advance_shields();

        // Open while the boost's own timer runs, close once it has expired, both
        // as an exponential approach so neither edge is a step. Driven from
        // `pad_timer` rather than from the exhaust, because the exhaust's boost
        // is a `max` that other things may one day also arm and this must follow
        // the pad specifically.
        if self.view.boost_fov_kick != oag_display::display::BoostFovKick::OFF {
            let boosting = self.sim.world.ships[0].physics.pad_timer > 0.0;
            let (target, rate) = if boosting {
                (1.0, BOOST_FOV_OPEN_RATE)
            } else {
                (0.0, BOOST_FOV_CLOSE_RATE)
            };
            self.view.boost_kick += (target - self.view.boost_kick) * rate * self.sim.dt;
            // Snapped, so a closed kick is *exactly* zero and `projection` takes
            // its bit-identical path rather than an `atan(tan(x))` round trip
            // that never quite settles.
            if !boosting && self.view.boost_kick < 1.0e-3 {
                self.view.boost_kick = 0.0;
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
        // which is why `self.view.flaps` holds a level rather than an angle. Not
        // read out of the binary - no consumer of `<AirbrakeGraphics>` was
        // located - but not a coin flip either: every team authors
        // `up_speed = 500` against a `down_speed` of 80 to 120, and on this
        // reading that is a flap that snaps out in 0.2 s and folds back over
        // about a second, which is what an airbrake does. Read as *fractions*
        // of full deflection per second instead, 500 would be full travel in
        // two milliseconds and the parameter would not be worth authoring.
        // It is also the convention `<Airbrake gain/falloff>` already uses on
        // the same scale, one element away.
        let ship = &self.sim.world.ships[0].physics;
        for (flap, level) in self
            .view
            .flaps
            .iter_mut()
            .zip([ship.airbrake_left, ship.airbrake_right])
        {
            let target = level.clamp(0.0, 100.0);
            let (rate, rising) = if target > *flap {
                (self.view.flap_graphics.up_speed, true)
            } else {
                (self.view.flap_graphics.down_speed, false)
            };
            let step = rate * self.sim.dt;
            *flap = if rising {
                (*flap + step).min(target)
            } else {
                (*flap - step).max(target)
            };
        }

        // The lock-on reticle, on the fixed tick beside the camera and the
        // exhaust and for the same reason: a headless capture calls only
        // `tick`, so a reticle advanced in the frame loop would be at a
        // different extent in a screenshot than in a window at the same tick
        // count. **After the camera**, because it projects through it.
        self.update_sight();

        // After every writer of the shield pool this tick has run - the wall
        // contact above, the weapon damage inside `projectile::step`, and the
        // perfect-zone recharge - so a hit from any of the three flashes on
        // the tick it lands rather than one late. See
        // `Self::advance_shield_flash`.
        self.advance_shield_flash();
        // After it and after `advance_absorb_bursts` above - see
        // `Self::advance_shield_blink`'s own doc comment for why both must
        // already reflect this tick.
        self.advance_shield_blink();
        // HD's twin of the pair above, for the title whose readout arms on a
        // whole-percent drop - see `Self::advance_shield_flash_whole`.
        self.advance_shield_flash_whole();
        // 2048's own trail, independent of the two above - see
        // `Self::advance_energy_bar_delay`.
        self.advance_energy_bar_delay();
        // 2048's `ThrustBar` chase - see `Self::advance_thrust_chase`.
        self.advance_thrust_chase();

        // Cooldown-gated, not edge-triggered - see `Self::sparks_cooldown`'s
        // doc comment for why a sustained scrape must re-fire periodically
        // rather than spawn once and go silent.
        self.view.sparks_cooldown = (self.view.sparks_cooldown - self.sim.dt).max(0.0);
        // The trigger rule runs whether or not the disc's own effect
        // loaded - see `Setup::effects`. Only the pool needs it.
        let effect = self.view.effects.get(sparks::DAMAGE_EFFECT).cloned();
        // Two trigger modes, chosen by the effect's own flag and never by the
        // platform. A `LOOPING` emitter has no countdown, so an owner must let
        // go of it; a burst ends itself and the cooldown is only there to
        // space the next one. Pulse authors this effect as a burst and HD
        // authors it looping - see `Race::sparks_attached`.
        let attached_effect = effect
            .as_deref()
            .is_some_and(|effect| effect.roots().iter().any(|&r| effect.emitters[r].looping));
        // **A raised shield throws no hull sparks**, and that is recovered:
        // the contact loop's third `craft+0x1b8 & 0x10` gate (`0x0884255c`)
        // branches past `Ship_DispatchCollisionFx` (`0x0883de90`) entirely -
        // reaction #1 on `docs/.../contact-response.md`, the spark spawn. The
        // shell's own bulge is what a shielded contact shows instead, which is
        // the whole design: sparks are the hull being hurt and the hull is not
        // being hurt. See `docs/.../shield-pickup.md`.
        let shielded = self.sim.world.ships[0].physics.shield_pickup_timer > 0.0;
        // The announcer, on the shield coming up and not on it being up.
        // `Shield_Activate` plays `"shieldactive"` one line above the
        // `Sound_PlayLooping` that opens `~SHIELD`, so the two share an instant
        // and differ in shape: a one-shot on this edge, a held voice on the
        // level the edge starts. The level half is `Race::shield_is_up`, read
        // by the audio layer; the edge is here, with every other cue edge.
        if shielded && !self.view.shield_was_up {
            self.sim.cues.push(oag_sound::sfx::CueEvent::new(
                oag_sound::sfx::Cue::ShieldActive,
                0,
            ));
        }
        self.view.shield_was_up = shielded;
        let can_fire = !shielded
            && if attached_effect {
                // Ignite once per contact, not once per cooldown: re-arming
                // would leave a chattering scrape with no sparks for up to
                // `COLLISION_COOLDOWN` at a time, which a burst never suffers
                // because it has already finished by then.
                evaluated.wall.impact && !self.view.sparks_attached
            } else {
                evaluated.wall.impact && self.view.sparks_cooldown <= 0.0
            };
        // The two contact sounds; the reasoning is on the helper, which the
        // opponents' own step calls with their slots.
        self.raise_contact_cue(0, evaluated.wall.impact, shielded);
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
            // the contact and the burst emits from that node - and along its
            // authored `+Y` - from then on (`Ship_DispatchCollisionFx`); with
            // no locators authored, the surface point itself is mapped into
            // model space, aimed along world up, so it still rides the hull
            // with rotation.
            let (anchor_model, up_model) = self
                .view
                .collision_fx
                .iter()
                .min_by(|a, b| {
                    let da = (model_matrix.transform_point3(a.position) - surface).length_squared();
                    let db = (model_matrix.transform_point3(b.position) - surface).length_squared();
                    da.total_cmp(&db)
                })
                .map_or_else(
                    || (model_matrix.inverse().transform_point3(surface), Vec3::Y),
                    |anchor| (anchor.position, anchor.up),
                );
            self.view.sparks_anchor = Some(anchor_model);
            self.view.sparks_anchor_up = Some(up_model);
            if let Some(effect) = effect.as_deref() {
                self.view.sparks.ignite(
                    effect,
                    surface,
                    sparks::severity(evaluated.wall.impact_speed),
                );
            }
            // `Camera_ArmShake`'s own severity is the *raw* clamp
            // `Ship_DispatchCollisionFx` passes it, not the spark's
            // slope-and-floor-reshaped one - see
            // `oag_fx::sparks::clamped_intensity`. `mode = 3` (ahead) vs
            // `mode = 1` (elsewhere) is a dot product of the contact point
            // against the craft's forward, exactly as
            // `docs/ghidra/functions/ps2-pulse-eu/collision-shake.md`
            // records it.
            //
            // Re-armed on the same cooldown gate as the spark burst
            // (`can_fire`) rather than every tick a contact is live: the
            // original re-arms once per collision-function call on a fresh
            // contact, which this project's own contact evaluation does not
            // expose as a separate edge, and pairing the shake's cadence with
            // the visible spark burst is the natural substitute.
            let ship = self.ship();
            let ahead =
                (contact.point - ship.physics.body.position).dot(ship.physics.body.forward()) > 0.0;
            let side = if ahead {
                oag_render::camera::shake::Side::Ahead
            } else {
                oag_render::camera::shake::Side::Elsewhere
            };
            let shake_severity = sparks::clamped_intensity(evaluated.wall.impact_speed);
            self.view
                .shake
                .arm(shake_severity, side, &mut self.view.shake_rng);
            self.view.sparks_ignitions += 1;
            self.view.sparks_cooldown = sparks::COLLISION_COOLDOWN;
            self.view.sparks_attached = attached_effect;
        }
        // Releasing on "no contact at all" rather than on `impact` going false
        // keeps the fountain alive through a gentle slide after the hit -
        // `impact` is the *inbound* test and a scrape stops being inbound long
        // before it stops being a scrape. `stop` takes the emission and leaves
        // the live particles to finish their own lives, so the trail outlasts
        // the contact the way it does when a rocket detonates.
        //
        // Limit worth knowing: `stop` is all-or-nothing across a tree, so this
        // rule does not generalise to the four HD effects that mix `LOOPING`
        // with clear siblings - it would truncate the siblings. None of the
        // four is wired to this trigger.
        if self.view.sparks_attached && evaluated.wall.resolved.is_none() {
            self.view.sparks.stop();
            self.view.sparks_attached = false;
        }
        // Unconditional, like the exhaust: the emitters keep trickling and
        // already-live particles keep ageing even on a tick with no fresh
        // impact. The anchor is the chosen hull locator under the ship's
        // *current* transform, the way the original's scene-graph node rides
        // the craft - and so is the direction its authored `+Y` maps to: a
        // craft banking mid-scrape tilts the spray with it rather than
        // spraying along a fixed world up. `transform_vector3` rather than
        // `transform_point3`: it is a direction, not a point, so only the
        // matrix's rotation applies and not its translation.
        let anchor = self
            .view
            .sparks_anchor
            .map_or(self.ship().physics.body.position, |local| {
                model_matrix.transform_point3(local)
            });
        let up = self
            .view
            .sparks_anchor_up
            .map_or(Vec3::Y, |local| model_matrix.transform_vector3(local));
        if let Some(effect) = effect.as_deref() {
            self.view
                .sparks
                .advance(effect, self.sim.dt, anchor, up, &mut self.view.sparks_rng);
        }

        // Last, after every write this tick makes: the recording stores the
        // inputs this tick was handed and the state it arrived at, and reads
        // the player's lap edge the rules above just produced. A no-op unless
        // `start_recording` was called - see `race::replay`.
        self.record_tick(inputs);
        evaluated
    }
}
