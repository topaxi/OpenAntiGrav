//! [`Race::tick`]: one 60 Hz step of the simulation, from an input snapshot to
//! the state the renderer and the HUD read.
//!
//! Split out of `race.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change.

use super::*;
use crate::player_step::{Evaluations, PlayerStep};
use oag_gameplay::MAX_PLAYERS;

impl Race {
    /// Advances the simulation one fixed tick, and the camera with it.
    ///
    /// Each slot's snapshot is mapped through [`oag_gameplay::ship_controls`],
    /// which is the one place "cross is thrust" is written down.
    ///
    /// # One snapshot per grid slot, and every person's craft stepped from its own
    ///
    /// It takes [`PlayerInputs`] - one snapshot per slot,
    /// indexed the same way [`oag_gameplay::World::race`] and
    /// `World::controllers` are - so split screen, a second window and a remote
    /// client differ only in where a snapshot comes from.
    ///
    /// **The player step runs once per person-flown slot**
    /// ([`oag_gameplay::World::person_flown`]): `Local` and `Remote` controllers
    /// with a craft on the grid, and the primary slot whatever its controller
    /// says. Each gets its own controls, autopilot, pickup spend, pads, force
    /// law, off-track rescue and race rules (`crate::player_step`); an AI slot
    /// is still flown by [`Self::step_opponents`] and a person's slot is
    /// skipped by it.
    ///
    /// The return value is one [`Evaluations`] entry per stepped slot; the
    /// camera and the full-screen HUD read the primary's through
    /// [`Evaluations::primary`]. What stays on the primary slot is the *view*
    /// (camera, shake, messages, run statistics), because a viewport per player
    /// is split screen's work, not this tick's.
    ///
    /// The pieces that run over the whole grid - `lay_mines`,
    /// [`Self::advance_cannons`], `slowdown::drain`, `disruption::advance`,
    /// [`Self::step_opponents`] - ask `World::controllers` which slots take a
    /// snapshot.
    ///
    /// [`PlayerInputs`]: oag_gameplay::PlayerInputs
    pub fn tick(&mut self, inputs: &oag_gameplay::PlayerInputs) -> Evaluations {
        #[cfg(feature = "view")]
        let queued = self.core.sim.cues.len();
        if let Some(xp) = self.core.sim.xp.as_mut() {
            xp.advance_window();
        }
        let evaluations = self.tick_body(inputs);
        self.core.observe_xp();
        #[cfg(feature = "view")]
        crate::stage::stage!(self).note_cues(&self.core, queued);
        evaluations
    }

    /// [`Self::tick`] without the journal around it.
    fn tick_body(&mut self, inputs: &oag_gameplay::PlayerInputs) -> Evaluations {
        self.core.ensure_speed_plans();
        // Before anything moves: the poses a draw blends from. See `between_ticks`.
        #[cfg(feature = "view")]
        if self.draws() {
            self.view.capture_poses(&self.core);
        }
        // Decays every tick regardless of contact, like `Camera_ArmShake`'s own
        // `shake_timer`, and **first**: the original arms the shake while it
        // updates the craft and reads the full `0.6` s timer in the same
        // frame's `Camera_SubmitScene`, decrementing after. Advancing here, before
        // anything this tick can arm it, gives the same order - the frame an
        // impact lands in shows the shake at progress 0 and the next one at
        // one tick - where advancing at the end of the tick would show the
        // impact frame already one tick in. Measured, see
        // `oag_render::camera::shake`.
        #[cfg(feature = "view")]
        if self.draws() {
            self.view.advance_shakes(&self.core);
            if let Some(cue) = self.view.message_cue(&self.core) {
                self.core.sim.cues.push(cue);
            }
        }
        // Before anything steps, so the perfect start's edge is a change
        // across this tick - see `race::perfect_start`.
        #[cfg(feature = "view")]
        let launch_grades = self.core.sim.launch_grades();
        #[cfg(feature = "view")]
        let roll_payouts = self.core.sim.roll_payout_timers();
        // Every craft is in the original's grid state until the green light:
        // `Race_PlaceGrid` puts the field in state 0 and `Race_StartRacing`
        // moves it to state 1. Derived from the one countdown clock rather than
        // stored, so a respawn after the start (state 3) can never re-enter it
        // and a restart, which rebuilds the race, always begins in it. Physics
        // reads the flag; see `oag_physics::ShipState::on_grid`.
        //
        // **Zone is in it too** (read live on a Zone engine): flags bit 1 is
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
        let on_grid = RaceState::thrust_gated(self.core.sim.world.tick + 1);
        // The launch boost's clock starts at the same tick in every mode, Zone
        // included (it has a countdown; its auto-speed is not multiplied yet, see
        // `oag_physics::engine::engine`).
        let released = !RaceState::thrust_gated(self.core.sim.world.tick + 1);
        // Zone's hover is the four-corner variant for every craft in the field.
        let four_corner = self.core.sim.world.mode() == Mode::Zone;
        for ship in &mut self.core.sim.world.ships {
            ship.physics.on_grid = on_grid;
            ship.physics.released = released;
            ship.physics.four_corner = four_corner;
        }
        self.core.sim.set_wrong_way_kill();
        self.core.sim.set_hover_caps();
        // A transit whose count ran out puts its craft further round before anything
        // reads where it is: its controls, its autopilot's lane snap, the modules' call
        // (whose `transit-end` this is) and its step all see it out.
        let transited = self.advance_transits();
        // Every craft a person is flying this tick. Slot 0 alone under
        // `World::SINGLE_PLAYER`, which is every session this engine starts
        // today; a second `Local` or `Remote` slot is stepped the same way and
        // an AI slot never is. Controls first, all of them, so a slot's
        // autopilot reads the field as the tick found it.
        let mut controls = [oag_physics::ShipControls::default(); MAX_PLAYERS];
        for (slot, slot_controls) in controls.iter_mut().enumerate() {
            if self.core.sim.world.person_flown(slot) {
                *slot_controls = self.core.sim.player_controls(slot, inputs.get(slot));
            }
        }
        self.core.sim.tick_flips(inputs, &mut controls);
        #[cfg(test)]
        {
            self.last_controls = controls;
            self.pair_contacts.clear();
            self.pair_impulses.clear();
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
            &mut self.core.sim.world.ships[..self.core.sim.world.ship_count as usize],
            self.core
                .sim
                .weapons
                .as_deref()
                .map(|table| table.slowdown_limit),
        );

        self.core.sim.cancel_pickup_use_of_downed_crafts();
        // Before the force law, so a Turbo fired this tick boosts this tick.
        // `tick_autopilot` after it, so a pickup armed this tick gets its whole
        // duration rather than a tick less, and so the cancel-on-absorb branch
        // inside `spend_pickup` is not immediately undone by a decrement.
        for slot in 0..MAX_PLAYERS {
            // A press in a transit is not heard: the craft is out of the world.
            if self.core.sim.world.person_flown(slot)
                && self.core.sim.world.ships[slot].transit.is_some()
            {
                self.core.tick_autopilot(slot);
            } else if self.core.sim.world.person_flown(slot) {
                self.spend_pickup(slot, inputs.get(slot));
                self.core.tick_autopilot(slot);
            } else if self.core.sim.opponent_autopilot {
                self.core.tick_autopilot(slot);
            }
        }
        // The start-of-race voice, on the ticks the original plays it.
        self.core.tick_countdown_voice();
        // Immediately after both, so the first mine of a cluster is laid on the
        // tick the button was pressed and from where the craft was when it was
        // pressed. A mine never moves again, so this is the only tick that can
        // place it correctly.
        self.core.sim.lay_mines();
        // A Quake press made under a live wave, now that the wave may have ended.
        self.core.sim.fire_armed_quakes();
        // Right after, for the same reason: a Cannon's own countdown has to
        // see every tick the button is down, and a round fired this tick has
        // to leave from where the craft started it. It reads the snapshot for
        // the *held* state of fire rather than the press edge `spend_pickup`
        // above consumes - see `Race::advance_cannons`'s own doc comment.
        self.advance_cannons(inputs);
        // Phase C: each simulation mod's one call, its commands into the steps below.
        self.tick_extensions(inputs, transited);

        // Each person's craft, in slot order, against the same unmoved
        // projectile pool and the same unspent field-wide effects: the force law
        // sees this tick's pads, the walls and its own controls, and nothing a
        // neighbour did to it until the pair pass after. See
        // `Race::step_player`.
        let mut steps: [Option<PlayerStep>; MAX_PLAYERS] = std::array::from_fn(|_| None);
        for slot in 0..MAX_PLAYERS {
            if self.core.sim.world.person_flown(slot) {
                steps[slot] = Some(self.step_player(slot, &controls[slot]));
            }
        }
        let primary = self.core.sim.world.primary_slot();
        assert!(
            steps[primary].is_some()
                || self.core.sim.world.controllers[primary] == oag_gameplay::Controller::Away,
            "the primary slot is stepped as a player unless its person is away"
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
        let damage_rules = self.core.sim.damage_rules();
        // One report per slot: a hit a shield swallowed (the *only* thing that
        // makes a shell visibly react - see `oag_render::shield::ShipShield::hit`)
        // or one that got through (the hull's own sparks). Both have to come
        // back out of the step rather than being invisible on both sides.
        let mut hits = [oag_weapons::projectile::WeaponHit::default(); MAX_SHIPS];
        // Read before the step: what the drawn bounce and blast below compare the step's result
        // against. See `RaceView::before_projectile_step`.
        #[cfg(feature = "view")]
        let before = self.view.before_projectile_step(&self.core.sim.world);
        let laws = self.core.sim.flight_laws();
        let own_craft = self.core.sim.own_craft_probe();
        let plasma_release = self.core.sim.plasma_release();
        let impacts = oag_weapons::projectile::step(
            &mut self.core.sim.world.projectiles,
            &mut self.core.sim.world.ships[..self.core.sim.world.ship_count as usize],
            self.core.sim.dt,
            &*self.core.sim.collision,
            self.core
                .sim
                .spline
                .cage()
                .map(|cage| cage as &dyn oag_physics::cage::Containment),
            self.core.sim.weapons.as_deref(),
            &self.core.sim.class,
            laws,
            own_craft,
            plasma_release,
            match self.core.sim.blast_push {
                oag_title::weapons::BlastPush::Posted => oag_weapons::projectile::BlastPush::Posted,
                oag_title::weapons::BlastPush::Immediate => {
                    oag_weapons::projectile::BlastPush::Immediate
                }
            },
            damage_rules,
            &mut hits,
        );
        self.core.sim.extensions.weapon_hits(&hits);
        for (slot, hit) in hits.iter().enumerate() {
            #[cfg(feature = "view")]
            if hit.absorbed {
                crate::stage::stage!(self).shield_hit(&self.core, slot);
            }
            if hit.landed {
                self.core.sim.note_weapon_hit(slot);
            }
        }
        self.core.sim.credit_impact_points(&impacts, &hits);
        let by = std::array::from_fn(|slot| {
            self.core
                .sim
                .impact_reaching(&impacts, slot)
                .map(|impact| (impact.owner, impact.kind))
        });
        self.core.xp_weapon_hits(&hits, &by);
        // A hit that got through throws the struck hull's own sparks - the
        // victim's `Ship_Damage`, not the weapon. See `race::hit_sparks`.
        #[cfg(feature = "view")]
        if self.draws() {
            self.core.lending_cues(|core, cues| {
                crate::stage::stage!(self).throw_hit_sparks(core, cues, &hits, false)
            });
        }
        // After `projectile::step`, so a flare rides where its rocket
        // actually ended the tick rather than a tick behind it.
        #[cfg(feature = "view")]
        if self.draws() {
            self.view.advance_projectile_flares(&self.core);
            self.view.advance_plasma_launch(&self.core);
        }
        // `slot` is read only by the drawn blast below.
        #[cfg_attr(not(feature = "view"), allow(unused_variables))]
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
                self.core.sim.last_damager[struck as usize] = Some(impact.owner);
            }
            self.core.sim.credit_blast(impact, &hits);
            #[cfg(feature = "view")]
            crate::stage::stage!(self).note_blast(
                &self.core,
                impact.kind,
                impact.point,
                impact.struck.map(usize::from),
                before.orientations[slot],
            );
            // A Pulse Plasma craft hit detonates twice in the same tick: the
            // bolt's own blast above, and one at the struck craft.
            #[cfg(feature = "view")]
            if let Some(at) = self.view.plasma_craft_hit_blast_point(
                &self.core,
                impact.kind,
                impact.struck.map(usize::from),
            ) {
                crate::stage::stage!(self).note_blast(
                    &self.core,
                    impact.kind,
                    at,
                    impact.struck.map(usize::from),
                    before.orientations[slot],
                );
            }
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
            // is the third ending" section and `oag_gameplay::cue::Cue::PlasmaHitShip`'s
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
                    oag_gameplay::cue::Cue::PlasmaHitWall,
                    oag_gameplay::cue::Cue::PlasmaHitShip,
                )),
                Weapon::Rocket => Some((
                    oag_gameplay::cue::Cue::RocketHitWall,
                    oag_gameplay::cue::Cue::RocketHitShip,
                )),
                Weapon::Cannon => Some((
                    oag_gameplay::cue::Cue::CannonHitWall,
                    oag_gameplay::cue::Cue::CannonHitShip,
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
                self.core
                    .sim
                    .cues
                    .push(oag_gameplay::cue::CueEvent::at_point(cue, impact.point));
            }
            // HD's Cannon throws its own spark on the craft it strikes, from
            // the weapon's side - see `RaceView::throw_weapon_spark`. A no-op
            // unless the title built weapon anchors (HD only).
            #[cfg(feature = "view")]
            if let (Weapon::Cannon, Some(struck)) = (impact.kind, impact.struck)
                && self.draws()
            {
                self.view
                    .throw_weapon_spark(&self.core, usize::from(struck), impact.point);
            }
            if let Some((wall, ship)) = hit_cues {
                let cue = if impact.struck.is_some() { ship } else { wall };
                self.core
                    .sim
                    .cues
                    .push(oag_gameplay::cue::CueEvent::at_point(cue, impact.point));
            }
        }
        // After the loop above, so a Plasma detonated this tick is already
        // in the pool at `age == 0.0` when this ages every slot - the same
        // one-tick order `advance_projectile_flares` above takes relative to
        // a freshly-placed flare. See `blast_models` module doc comment.
        #[cfg(feature = "view")]
        if self.draws() {
            self.view
                .advance_plasma_blast_models(&self.core, self.core.sim.dt);
            // Same one-tick order, for the Bomb's own hemisphere/shockwave pool.
            self.view.advance_bomb_blast_models(self.core.sim.dt);
            self.view.advance_mag_floor_fx(&self.core);
            self.core
                .lending_cues(|core, cues| self.view.advance_magstrip_wake(core, cues));
            self.view
                .ignite_missile_bounces(&self.core, &before.bounces);
        }
        // The same `bounces_before`/after edge the visual bounce above reads,
        // for the two weapons' own bounce cues - `Cue::MissileHitWall`
        // (`MISSILEEXPWALL`, on every bounce, not the projectile's final
        // ending - see that variant's own doc comment) and `Cue::ShurikenHit`
        // (`SHURIKENHIT`, `Shuriken_Bounce`, confidence 88).
        // `weapons::bounced_this_tick` already gates on the weapon, so a
        // stray nonzero `bounces` on some other kind never fires either cue.
        // Lean: a cue is dropped on a tick that is not drawn, so none is raised.
        #[cfg(feature = "view")]
        for (slot, projectile) in self.core.sim.world.projectiles.slots.iter().enumerate() {
            if !weapons::bounced_this_tick(
                projectile.kind,
                before.bounces[slot],
                projectile.bounces,
            ) {
                continue;
            }
            match projectile.kind {
                Some(oag_tables::weapons::Weapon::Missile) => {
                    self.core
                        .sim
                        .cues
                        .push(oag_gameplay::cue::CueEvent::at_point(
                            oag_gameplay::cue::Cue::MissileHitWall,
                            projectile.position,
                        ));
                }
                // On the blade's own emitter - see `Cue::ShurikenHit`.
                Some(oag_tables::weapons::Weapon::Shuriken) => {
                    self.core
                        .sim
                        .cues
                        .push(oag_gameplay::cue::CueEvent::at_point(
                            oag_gameplay::cue::Cue::ShurikenHit,
                            projectile.position,
                        ));
                }
                _ => {}
            }
        }
        // After the craft have moved, so a flare sits on this tick's nozzle
        // rather than the last one's.
        #[cfg(feature = "view")]
        if self.draws() {
            self.view.advance_engine_flares(&self.core);
            self.view.advance_absorb_bursts(&self.core);
            self.view.advance_hit_sparks(&self.core);
            self.draw_extension_effects();
            self.view.advance_damage_smoke(&self.core);
            self.view.advance_wreck_fx(&self.core);
            self.view.advance_floor_sparks(&self.core);
            self.view.advance_particles(&self.core);
            self.view.advance_scenery_fx(&self.core, self.core.sim.dt);
        }

        for (slot, step) in steps.iter().enumerate() {
            if let Some(step) = step {
                self.rescue_player(slot, step);
            }
        }

        self.step_opponents();
        self.resolve_craft_pairs();
        // Every craft's disruption counts down *after* the tick's controls
        // were filtered with it - `Ship_UpdateWeapons`'s own order, so an
        // effect of one tick is one tick. Over the whole field at once, as
        // `slowdown::drain` is, so the countdown cannot depend on which
        // branch stepped the craft.
        oag_weapons::disruption::advance(
            &mut self.core.sim.world.ships[..self.core.sim.world.ship_count as usize],
            self.core.sim.dt,
        );

        // The destroyed-craft pass, over the whole field: Eliminator's
        // respawn-and-bookkeeping, and a single race's opponent respawn.
        // Before the ending check just below and before the standings, so a
        // craft respawned this tick is placed where it is put rather than
        // left mid-explosion for the lap counter to read. See
        // `crate::eliminator`.
        self.tick_destroyed_craft();
        #[cfg(feature = "view")]
        if self.draws() {
            self.view.advance_craft_flashes(&self.core);
            self.view.advance_destroy_camera(&self.core);
        }

        self.core.sim.world.tick += 1;

        // Each person-flown slot's own elimination and race rules. The ending
        // below reaches `RaceState::eliminate` for the craft that finished
        // blowing up; its view half (the wreck timer, the HUD hide) is the
        // primary slot's alone.
        for (slot, step) in steps.iter().enumerate() {
            let Some(step) = step else {
                continue;
            };
            // The craft finished blowing up this tick, so its race is over.
            // Checked before the rules run rather than after, because a wrecked
            // craft's position should not go on counting laps - and checked every
            // tick because `RaceState::eliminate` is the thing that makes it
            // idempotent.
            //
            // Zone *and a single race* reach this - a time trial and a speed lap
            // do not, racing with the original's `Damage` option off, which
            // floors their pool at 20. See `oag_gameplay::damage_rules`, and
            // `RaceState::eliminate` for the disc text that settles the single
            // race's own answer as the same one.
            //
            // **Eliminator does not reach this branch at all.**
            // `tick_destroyed_craft` above has already turned this tick's
            // `Eliminated` craft back into a `Racing` one via a respawn, before
            // this read - see `crate::eliminator`'s own doc comment for why
            // the mode's own text rules out `RaceState::eliminate` as its ending.
            // An *opponent* in a single race never reaches it, because the branch
            // reads a person-flown craft's state alone; an opponent's wreck stays
            // down (the original's state 6, measured live) and ends nothing.
            if self.core.sim.eliminate_if_wrecked(slot) && slot == self.core.player_slot() {
                self.core.outcome.wreck_ended_tick = Some(self.core.sim.world.tick);
                // The original plays `~BLOWUP`, hides the HUD and swings the camera
                // into its mode 5 on the way here. **The sound is built** and is
                // held on `SimRace::craft_is_exploding` rather than raised here,
                // because case 4 opens a handle rather than firing a one-shot. The
                // HUD hide and the camera mode are still absent - case 4's own
                // addresses for both are on zone-mode.md. See
                // `oag_physics::damage::CraftState`.
            }
            // The race rules run last of the simulation, on the position the step
            // produced and the tick it produced it on. Running them before the step
            // would test last tick's position against this tick's clock, which is a
            // whole tick of error on a quantity whose job is to be exact at one
            // instant. A track with no closed ring simply has no lap counter; the
            // load report already said so.
            let evaluated = step.evaluated;
            self.advance_player_rules(slot, &evaluated);
        }

        // Every craft's standing, the player's included, from the position it
        // holds now. **After the player's `RaceState`**, so the two see the same
        // tick, and slot 0's lap is then taken from the standing rather than
        // counted a second time - see `Ship::standing`. Outside the borrow above
        // rather than inside it because this walks the whole ship array.
        self.core.sim.update_standings();
        // After the lap counter, so a stage steps on the tick its crossing counts.
        self.core.sim.advance_detonator(inputs);
        #[cfg(feature = "view")]
        if self.draws() {
            self.view.play_detonator_events(&self.core);
            self.view.advance_emp_visual(&self.core);
        }
        #[cfg(feature = "view")]
        if self.draws() {
            self.view.tick_split_gap(&self.core);
        }

        // **After the standings**, so the wave's own proximity test reads
        // every craft's freshest `Standing::progress` rather than last
        // tick's. See `Race::advance_quake`'s own doc comment.
        self.advance_quake();
        #[cfg(feature = "view")]
        if self.draws() {
            self.view.advance_quake_visual(&self.core);
        }

        // Beside the Quake and for the same reason: both are single-instance
        // weapons that read every craft's freshest state rather than last
        // tick's. See `Race::advance_leach_beam`.
        self.advance_leach_beam();
        #[cfg(feature = "view")]
        if self.draws() {
            self.view.advance_leach_beam_visual(&self.core);
            self.core
                .lending_cues(|core, cues| self.view.advance_leach_beam_ribbon(core, cues));
            self.view.advance_leach_strip(&self.core);
            self.view.advance_rocket_smoke(&self.core);
            self.view.advance_missile_smoke(&self.core);
            self.view.advance_wingtip(&self.core);
        }

        // Beside them for the same reason: a Repulser's wave sweep reads every
        // craft's ring index from this tick. See `Race::advance_repulsers`.
        self.advance_repulsers();
        #[cfg(feature = "view")]
        if self.draws() {
            self.view.advance_repulser_visual(&self.core);
        }

        // **After the standings**, so the last crossing is in the table this
        // reads, and a no-op on every tick but the one the race ends on. It
        // takes a snapshot and touches no simulation state, which is what lets
        // it sit inside the tick without moving a hash - see
        // [`SimRace::capture_results`].
        self.core.capture_results();

        #[cfg(feature = "view")]
        if self.draws() {
            self.view.advance_chase_camera(&self.core);

            // Advanced here, on the fixed tick, and not in the frame loop. That is
            // what makes the headless `capture` path - which calls only `tick` -
            // produce the same flare at the same tick count as the window does, and
            // it is the same reason the chase camera is advanced from here.
            // The perfect start arms the flare the way a pad does, so before the
            // exhausts take this tick's step, as the pad's arming is.
            self.core.lending_cues(|core, cues| {
                crate::stage::stage!(self).fire_perfect_starts(core, cues, &launch_grades)
            });
            self.core.lending_cues(|core, cues| {
                crate::stage::stage!(self).fire_roll_payouts(core, cues, &roll_payouts)
            });
            self.view.advance_craft_looks(&self.core);
        }
        // The lock-on, simulation state advanced from the world alone
        // ([`crate::lock`]), also in a sim-only tick: the next tick's shot reads
        // it. Then the reticle the player sees, on the fixed tick beside the
        // camera and the exhaust and for the same reason: a headless capture
        // calls only `tick`, so a reticle advanced in the frame loop would be at
        // a different extent in a screenshot than in a window at the same tick
        // count. **After the camera**, because it projects through it, and only
        // when the tick is drawn: it follows the lock and decides nothing.
        self.core.sim.advance_locks();
        #[cfg(feature = "view")]
        if self.draws() {
            self.view.update_sight(&self.core);
        }

        // After every writer of the shield pool this tick has run - the wall
        // contact above, the weapon damage inside `projectile::step`, and the
        // perfect-zone recharge - so a hit from any of the three flashes on
        // the tick it lands rather than one late; and after
        // `advance_absorb_bursts`, which the blink reads. Every bar of every
        // viewed slot: see `Self::advance_slot_huds`.
        #[cfg(feature = "view")]
        if self.draws() {
            self.view.advance_slot_huds(&self.core);
        } else {
            self.view.forget_incoming_raises();
        }

        // Every person-flown craft's wall-contact cue, in slot order, drawn tick
        // or not: the cooldown (`contact_cue_cooldown`) is sim-side bookkeeping
        // that a replayed tick has to keep running, and which craft this
        // machine's person flies must not decide which cooldowns tick, or a
        // client's cue cadence would drift after every correction. A one-person
        // race flies slot 0 alone, so it runs exactly as before.
        for (slot, evaluated) in steps
            .iter()
            .enumerate()
            .filter_map(|(slot, step)| step.as_ref().map(|step| (slot, step.evaluated)))
        {
            let shielded = self.core.sim.world.ships[slot].physics.shield_pickup_timer > 0.0;
            self.core
                .sim
                .raise_wall_cue(slot, &evaluated.wall, shielded);
            self.core.sim.note_wall_contact(slot, &evaluated.wall);
        }
        // After every contact of the tick is noted: the opponents' walls
        // (`step_opponents`), the craft pairs (`resolve_craft_pairs`) and the walls above.
        self.core.sim.raise_impulse_cues();
        // The sparks, the shake and the shield edge are the seat's own: the
        // craft this machine's person flies, which is `primary_slot` except on a
        // network client seated elsewhere. Every other local person has their own
        // view (a split or a window) and their own sparks and shake, stepped from
        // their craft's wall contact the same way. Lean (no `view` feature):
        // `draws()` is false and there is nothing to draw.
        #[cfg(feature = "view")]
        if self.draws() {
            let seat = self.core.player_slot();
            let seat = if steps[seat].is_some() { seat } else { primary };
            if let Some(step) = &steps[seat] {
                let evaluated = step.evaluated;
                if let Some(cue) = self.view.shield_up_cue(&self.core, seat) {
                    self.core.sim.cues.push(cue);
                }
                self.view.advance_contact_fx(&self.core, seat, &evaluated);
            }
            self.view.advance_seat_contact_fx(&self.core, &steps, seat);
            self.view.advance_field_sparks(&self.core);
        }

        // Last, after every write this tick makes: the recording stores the
        // inputs this tick was handed and the state it arrived at, and reads
        // the player's lap edge the rules above just produced. A no-op unless
        // `start_recording` was called - see `race::replay`.
        #[cfg(feature = "view")]
        if self.draws() {
            crate::stage::stage!(self).record_tick(&self.core, inputs);
        }
        Evaluations::new(&steps, primary)
    }
}
