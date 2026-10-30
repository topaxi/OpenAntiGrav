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
    /// This took a single [`InputSnapshot`] until 2026-09-16, which made "the
    /// player" and "slot 0" the same statement and left nowhere to put a second
    /// person's controls. It now takes [`PlayerInputs`] - one snapshot per slot,
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
    /// skipped by it. Until 2026-10-13 only the primary slot was stepped and a
    /// second human's snapshot reached nothing (ADR-0052 recorded that as a
    /// consequence of its own decision).
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
        let queued = self.sim.cues.len();
        let evaluations = self.tick_body(inputs);
        #[cfg(feature = "view")]
        self.note_cues(queued);
        evaluations
    }

    /// [`Self::tick`] without the journal around it.
    fn tick_body(&mut self, inputs: &oag_gameplay::PlayerInputs) -> Evaluations {
        self.ensure_speed_plans();
        // Before anything moves: the poses a draw blends from. See `between_ticks`.
        #[cfg(feature = "view")]
        if self.draws() {
            self.capture_poses();
        }
        // Decays every tick regardless of contact, like `Camera_ArmShake`'s own
        // `shake_timer`, and **first**: the original arms the shake while it
        // updates the craft and reads the full `0.6` s timer in the same
        // frame's `Camera_SubmitScene`, decrementing after. Advancing here, before
        // anything this tick can arm it, gives the same order - the frame an
        // impact lands in shows the shake at progress 0 and the next one at
        // one tick - where advancing at the end of the tick would show the
        // impact frame already one tick in. Measured 2026-09-30, see
        // `oag_render::camera::shake`.
        #[cfg(feature = "view")]
        if self.draws() {
            self.advance_now_playing();
            self.view.contact.shake.advance(self.sim.dt);
            self.advance_seat_shakes(self.sim.dt);
            let mut shown = false;
            for slot in 0..MAX_SHIPS {
                if self.sim.world.person_flown(slot) {
                    let board = &mut self.view.huds[slot].messages;
                    board.advance();
                    shown |= board.just_shown();
                }
            }
            if shown {
                self.sim.cues.push(oag_gameplay::cue::CueEvent::new(
                    oag_gameplay::cue::Cue::Message,
                    0,
                ));
            }
        }
        // Before anything steps, so the perfect start's edge is a change
        // across this tick - see `race::perfect_start`.
        #[cfg(feature = "view")]
        let launch_grades = self.launch_grades();
        #[cfg(feature = "view")]
        let roll_payouts = self.roll_payout_timers();
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
        self.set_wrong_way_kill();
        self.set_hover_caps();
        // Every craft a person is flying this tick. Slot 0 alone under
        // `World::SINGLE_PLAYER`, which is every session this engine starts
        // today; a second `Local` or `Remote` slot is stepped the same way and
        // an AI slot never is. Controls first, all of them, so a slot's
        // autopilot reads the field as the tick found it.
        let mut controls = [oag_physics::ShipControls::default(); MAX_PLAYERS];
        for (slot, slot_controls) in controls.iter_mut().enumerate() {
            if self.sim.world.person_flown(slot) {
                *slot_controls = self.player_controls(slot, inputs.get(slot));
            }
        }
        #[cfg(test)]
        {
            self.last_controls = controls;
            self.pair_contacts.clear();
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
        // `tick_autopilot` after it, so a pickup armed this tick gets its whole
        // duration rather than a tick less, and so the cancel-on-absorb branch
        // inside `spend_pickup` is not immediately undone by a decrement.
        for slot in 0..MAX_PLAYERS {
            if self.sim.world.person_flown(slot) {
                self.spend_pickup(slot, inputs.get(slot));
                self.tick_autopilot(slot);
            } else if self.sim.opponent_autopilot {
                self.tick_autopilot(slot);
            }
        }
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
        // Phase C: each simulation mod's one call, its commands into the steps below.
        self.tick_extensions(inputs);

        // Each person's craft, in slot order, against the same unmoved
        // projectile pool and the same unspent field-wide effects: the force law
        // sees this tick's pads, the walls and its own controls, and nothing a
        // neighbour did to it until the pair pass after. See
        // `Race::step_player`.
        let mut steps: [Option<PlayerStep>; MAX_PLAYERS] = std::array::from_fn(|_| None);
        for slot in 0..MAX_PLAYERS {
            if self.sim.world.person_flown(slot) {
                steps[slot] = Some(self.step_player(slot, &controls[slot]));
            }
        }
        let primary = self.sim.world.primary_slot();
        assert!(
            steps[primary].is_some(),
            "the primary slot is always stepped as a player"
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
        let damage_rules = self.sim.damage_rules();
        // One report per slot: a hit a shield swallowed (the *only* thing that
        // makes a shell visibly react - see `oag_render::shield::ShipShield::hit`)
        // or one that got through (the hull's own sparks). Both have to come
        // back out of the step rather than being invisible on both sides.
        let mut hits = [oag_weapons::projectile::WeaponHit::default(); MAX_SHIPS];
        // Read before the step, for `ignite_missile_bounces` below: a bounce
        // never stops a projectile, so it never reaches `impacts` and the
        // only way to see one is to compare this counter before and after.
        #[cfg(feature = "view")]
        let bounces_before: [u8; oag_weapons::projectile::MAX_PROJECTILES] =
            std::array::from_fn(|slot| self.sim.world.projectiles.slots[slot].bounces);
        // Read before the step too, for `ignite_blast`'s Bomb arm below: a
        // detonated slot resets to `Projectile::default()` inside `step`
        // (`Quat::IDENTITY`), so the frozen pose a laid Bomb detonated with
        // has to be read before that happens - the same before/after shape
        // `bounces_before` already takes, and `impacts` is slot-indexed the
        // same way. Read for every slot rather than gated on `kind ==
        // Bomb`, since nothing here is on the determinism-hashed path.
        #[cfg(feature = "view")]
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
        self.sim.extensions.weapon_hits(&hits);
        for (slot, hit) in hits.iter().enumerate() {
            if hit.absorbed {
                self.shield_hit(slot);
            }
            if hit.landed {
                self.note_weapon_hit(slot);
            }
        }
        // A hit that got through throws the struck hull's own sparks - the
        // victim's `Ship_Damage`, not the weapon. See `race::hit_sparks`.
        #[cfg(feature = "view")]
        if self.draws() {
            self.throw_hit_sparks(&hits, false);
        }
        // After `projectile::step`, so a flare rides where its rocket
        // actually ended the tick rather than a tick behind it.
        #[cfg(feature = "view")]
        if self.draws() {
            self.advance_projectile_flares();
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
                self.sim.last_damager[struck as usize] = Some(impact.owner);
            }
            self.credit_blast(impact, &hits);
            #[cfg(feature = "view")]
            self.note_blast(
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
                self.sim
                    .cues
                    .push(oag_gameplay::cue::CueEvent::at_point(cue, impact.point));
            }
            // HD's Cannon throws its own spark on the craft it strikes, from
            // the weapon's side - see `Race::throw_weapon_spark`. A no-op
            // unless the title built weapon anchors (HD only).
            #[cfg(feature = "view")]
            if let (Weapon::Cannon, Some(struck)) = (impact.kind, impact.struck)
                && self.draws()
            {
                self.throw_weapon_spark(usize::from(struck), impact.point);
            }
            if let Some((wall, ship)) = hit_cues {
                let cue = if impact.struck.is_some() { ship } else { wall };
                self.sim
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
            self.advance_plasma_blast_models(self.sim.dt);
            // Same one-tick order, for the Bomb's own hemisphere/shockwave pool.
            self.advance_bomb_blast_models(self.sim.dt);
            self.advance_mag_floor_fx();
            self.advance_magstrip_wake();
            self.ignite_missile_bounces(&bounces_before);
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
                    self.sim.cues.push(oag_gameplay::cue::CueEvent::at_point(
                        oag_gameplay::cue::Cue::MissileHitWall,
                        projectile.position,
                    ));
                }
                // On the blade's own emitter - see `Cue::ShurikenHit`.
                Some(oag_tables::weapons::Weapon::Shuriken) => {
                    self.sim.cues.push(oag_gameplay::cue::CueEvent::at_point(
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
            self.advance_engine_flares();
            self.advance_absorb_bursts();
            self.advance_hit_sparks();
            self.draw_extension_effects();
            self.advance_damage_smoke();
            self.advance_wreck_fx();
            self.view
                .stage
                .advance(self.sim.dt, &mut self.view.stage_rng);
            self.advance_scenery_fx(self.sim.dt);
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
        #[cfg(feature = "view")]
        if self.draws() {
            self.advance_craft_flashes();
            self.advance_destroy_camera();
        }

        self.sim.world.tick += 1;

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
            if self.eliminate_if_wrecked(slot) && slot == self.player_slot() {
                self.view.wreck_ended_tick = Some(self.sim.world.tick);
                // The original plays `~BLOWUP`, hides the HUD and swings the camera
                // into its mode 5 on the way here. **The sound is built** and is
                // held on `Race::craft_is_exploding` rather than raised here,
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
        self.update_standings();
        #[cfg(feature = "view")]
        if self.draws() {
            self.tick_split_gap();
        }

        // **After the standings**, so the wave's own proximity test reads
        // every craft's freshest `Standing::progress` rather than last
        // tick's. See `Race::advance_quake`'s own doc comment.
        self.advance_quake();
        #[cfg(feature = "view")]
        if self.draws() {
            self.advance_quake_visual();
        }

        // Beside the Quake and for the same reason: both are single-instance
        // weapons that read every craft's freshest state rather than last
        // tick's. See `Race::advance_leach_beam`.
        self.advance_leach_beam();
        #[cfg(feature = "view")]
        if self.draws() {
            self.advance_leach_beam_visual();
            self.advance_leach_beam_ribbon();
            self.advance_leach_strip();
            self.advance_rocket_smoke();
        }

        // Beside them for the same reason: a Repulser's wave sweep reads every
        // craft's ring index from this tick. See `Race::advance_repulsers`.
        self.advance_repulsers();
        #[cfg(feature = "view")]
        if self.draws() {
            self.advance_repulser_visual();
        }

        // **After the standings**, so the last crossing is in the table this
        // reads, and a no-op on every tick but the one the race ends on. It
        // takes a snapshot and touches no simulation state, which is what lets
        // it sit inside the tick without moving a hash - see
        // [`Race::capture_results`].
        self.capture_results();

        #[cfg(feature = "view")]
        if self.draws() {
            let target = self.camera_target();
            self.view
                .camera
                .advance(target, &self.view.chase_params, self.sim.dt);
            // The split-screen views' own cameras, view-side: a no-op with one view.
            self.advance_viewpoints();
            // After the chase camera and before the eye is read: behind the end-race panels the
            // spectator director's pose replaces it. A no-op until the player has finished.
            self.advance_finish_camera();
            // After the camera, so the flash's falloff measures from this tick's eye.
            let eye = self.camera_position();
            if let Some(flash) = &mut self.view.screen_flash {
                flash.advance(self.sim.dt, eye);
            }
            self.advance_cockpit_hit();

            // Advanced here, on the fixed tick, and not in the frame loop. That is
            // what makes the headless `capture` path - which calls only `tick` -
            // produce the same flare at the same tick count as the window does, and
            // it is the same reason the chase camera is advanced from here.
            // The perfect start arms the flare the way a pad does, so before the
            // exhausts take this tick's step, as the pad's arming is.
            self.fire_perfect_starts(&launch_grades);
            self.fire_roll_payouts(&roll_payouts);
            self.advance_hull_shakes();
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
                let boosting = self.sim.world.ships[self.player_slot()].physics.pad_timer > 0.0;
                self.view.boost_kick = step_boost_kick(self.view.boost_kick, boosting, self.sim.dt);
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
            for slot in 0..MAX_SHIPS {
                // A craft nobody flies folds its flaps back, so a seat handed to the AI
                // does not keep its last deflection.
                let flown = self.sim.world.person_flown(slot);
                let ship = &self.sim.world.ships[slot].physics;
                for (flap, level) in self.view.flaps[slot]
                    .iter_mut()
                    .zip([ship.airbrake_left, ship.airbrake_right])
                {
                    let level = if flown { level } else { 0.0 };
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
            }
        }
        // The lock-on, simulation state advanced from the world alone
        // ([`crate::lock`]), also in a sim-only tick: the next tick's shot reads
        // it. Then the reticle the player sees, on the fixed tick beside the
        // camera and the exhaust and for the same reason: a headless capture
        // calls only `tick`, so a reticle advanced in the frame loop would be at
        // a different extent in a screenshot than in a window at the same tick
        // count. **After the camera**, because it projects through it, and only
        // when the tick is drawn: it follows the lock and decides nothing.
        self.advance_locks();
        #[cfg(feature = "view")]
        if self.draws() {
            self.update_sight();
        }

        // After every writer of the shield pool this tick has run - the wall
        // contact above, the weapon damage inside `projectile::step`, and the
        // perfect-zone recharge - so a hit from any of the three flashes on
        // the tick it lands rather than one late; and after
        // `advance_absorb_bursts`, which the blink reads. Every bar of every
        // viewed slot: see `Self::advance_slot_huds`.
        #[cfg(feature = "view")]
        if self.draws() {
            self.advance_slot_huds();
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
            let shielded = self.sim.world.ships[slot].physics.shield_pickup_timer > 0.0;
            self.raise_contact_cue(slot, evaluated.wall.impact, shielded);
        }
        // The sparks, the shake and the shield edge are the seat's own: the
        // craft this machine's person flies, which is `primary_slot` except on a
        // network client seated elsewhere. Every other local person has their own
        // view (a split or a window) and their own sparks and shake, stepped from
        // their craft's wall contact the same way. Lean (no `view` feature):
        // `draws()` is false and there is nothing to draw.
        #[cfg(feature = "view")]
        if self.draws() {
            let seat = self.player_slot();
            let seat = if steps[seat].is_some() { seat } else { primary };
            if let Some(step) = &steps[seat] {
                let evaluated = step.evaluated;
                self.advance_contact_fx(seat, &evaluated);
            }
            self.advance_seat_contact_fx(&steps, seat);
        }

        // Last, after every write this tick makes: the recording stores the
        // inputs this tick was handed and the state it arrived at, and reads
        // the player's lap edge the rules above just produced. A no-op unless
        // `start_recording` was called - see `race::replay`.
        #[cfg(feature = "view")]
        if self.draws() {
            self.record_tick(inputs);
        }
        Evaluations::new(&steps, primary)
    }

    #[cfg(feature = "view")]
    /// The seat's collision sparks, the shake a hit arms and the shield edge, for its wall
    /// contact this tick (`slot`, `evaluated`). The two contact sounds are raised for every
    /// flown slot in [`Race::tick`].
    ///
    /// Everything [`Race::tick`] draws on a wall contact, in one place so
    /// [`Race::tick_sim_only`] can skip it whole. The one piece of
    /// simulation state in it is `contact_cue_cooldown`, which the sim-only
    /// path advances itself.
    fn advance_contact_fx(&mut self, slot: usize, evaluated: &Evaluated) {
        let shielded = self.sim.world.ships[slot].physics.shield_pickup_timer > 0.0;
        // The announcer, on the shield coming up and not on it being up.
        // `Shield_Activate` plays `"shieldactive"` one line above the
        // `Sound_PlayLooping` that opens `~SHIELD`, so the two share an instant
        // and differ in shape: a one-shot on this edge, a held voice on the
        // level the edge starts. The level half is `Race::shield_is_up`, read
        // by the audio layer; the edge is here, with every other cue edge.
        if shielded && !self.view.shield_was_up {
            self.sim.cues.push(oag_gameplay::cue::CueEvent::new(
                oag_gameplay::cue::Cue::ShieldActive,
                slot,
            ));
        }
        self.view.shield_was_up = shielded;
        let model_matrix = self.ship_model_matrix_of(slot);
        self.view.contact.advance(
            &self.view.handles,
            &self.view.collision_fx,
            &self.sim.world.ships[slot],
            model_matrix,
            self.sim.dt,
            evaluated,
        );
    }

    #[cfg(feature = "view")]
    /// Every other local person's sparks and shake, from their own craft's wall contact.
    /// A split screen's or a window's extra seat has a [`ContactFx`](super::contact_fx::ContactFx)
    /// beside its camera; the seat `own` is the one [`Self::advance_contact_fx`] ran.
    fn advance_seat_contact_fx(&mut self, steps: &[Option<PlayerStep>], own: usize) {
        for (slot, step) in steps.iter().enumerate() {
            let Some(step) = step else { continue };
            if slot == own || self.view.viewpoints.contact[slot].is_none() {
                continue;
            }
            let model_matrix = self.ship_model_matrix_of(slot);
            if let Some(fx) = self.view.viewpoints.contact[slot].as_mut() {
                fx.advance(
                    &self.view.handles,
                    &self.view.collision_fx,
                    &self.sim.world.ships[slot],
                    model_matrix,
                    self.sim.dt,
                    &step.evaluated,
                );
            }
        }
    }
}

/// One tick of the boost's field-of-view kick: open while the boost's own timer runs, close once
/// it has expired, both as an exponential approach so neither edge is a step. The player's
/// ([`RaceView::boost_kick`](super::view::RaceView::boost_kick)) and every other local seat's
/// (`Cam::boost_kick`) are this one rule.
#[cfg(feature = "view")]
pub(super) fn step_boost_kick(kick: f32, boosting: bool, dt: f32) -> f32 {
    let (target, rate) = if boosting {
        (1.0, BOOST_FOV_OPEN_RATE)
    } else {
        (0.0, BOOST_FOV_CLOSE_RATE)
    };
    let kick = kick + (target - kick) * rate * dt;
    // Snapped, so a closed kick is *exactly* zero and `projection` takes
    // its bit-identical path rather than an `atan(tan(x))` round trip
    // that never quite settles.
    if !boosting && kick < 1.0e-3 {
        0.0
    } else {
        kick
    }
}
