//! Pickups and what they fire: the free turbo, the one-slot inventory, the
//! blasts a rocket ignites and the sprites they are drawn as.
//!
//! Split out of `race.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change. Its tests are
//! `race/tests/weapons.rs`.

use super::*;

impl Race {
    /// Hands a time trial or a speed lap its free Turbo.
    ///
    /// **Two shipped records say this happens, and they were found
    /// independently.** The disc's own event text is explicit -
    /// `MSC_EVENT_TT` and `MSC_EVENT_SL` each read *"You will be given a free
    /// turbo pickup once per lap"* - and `TimeTrial_HUD.xml` authors a
    /// `PickupBackground` and **exactly one** weapon icon, `TurboIcon`, where
    /// `Arcade_HUD.xml` authors all thirteen and `Zone_HUD.xml` authors none.
    /// A layout carrying one pickup widget for a mode whose `Weapon Pad`s are
    /// hidden is otherwise inexplicable. Confidence **85** on the rule; the
    /// second record was found by `every_weapon_has_an_icon_widget_named_after_it`
    /// failing against the assumption that these layouts carried none.
    ///
    /// **Called from two places for one rule.** `oag_race::Outcome::lap_completed`
    /// fires this on every later lap's edge, and [`Race::start`] calls it once
    /// more, directly, before that edge exists for lap 1 - "once per lap"
    /// otherwise held for laps 2..N and silently dropped the first, which is
    /// the edge a player crosses at the start line before ever seeing a lap
    /// end. Reported 2026-08-19 as missing on lap 1; both calls share every
    /// gate below, so lap 1 gets exactly the same grant lap 2 does, just at
    /// its own start rather than its own end - the same instant, one lap
    /// earlier. **Still not recovered**: whether the original also grants it
    /// at the start line rather than crossing into lap 2, or somewhere else
    /// within the lap - "once per lap" constrains the count, not the moment,
    /// and nothing read pins the moment for laps 2..N either.
    ///
    /// Zone is excluded: it authors no pickup widgets at all, and its event text
    /// promises nothing.
    pub(super) fn grant_free_turbo(&mut self) {
        if !matches!(self.world.race.mode, Mode::TimeTrial | Mode::SpeedLap) {
            return;
        }
        // The same "only into an empty slot" rule a pad follows, so a player who
        // has not spent last lap's turbo does not silently lose this one - they
        // keep the one they have.
        if !self.world.ships[0].pickup.is_empty() {
            return;
        }
        // Gated on the table, so a disc whose weapon file did not load hands
        // out nothing rather than a Turbo with no duration to fire it for.
        if self
            .weapons
            .as_ref()
            .and_then(|w| w.simple(oag_formats::weapons::Weapon::Turbo))
            .is_none()
        {
            return;
        }
        self.world.ships[0].pickup.weapon = Some(oag_formats::weapons::Weapon::Turbo);
    }

    /// Fires or absorbs whatever the craft is holding.
    ///
    /// **The buttons are recovered and the actions are not.**
    /// `Options_LoadDefaultControlMapping` (`0x0883672c`) maps action 1, *fire*,
    /// to `SQUARE` and action 2, *absorb*, to `CIRCLE`, at confidence 90 - see
    /// `docs/ghidra/functions/psp-pulse-usa/input-bindings.md`, whose table is
    /// self-checking on the `accelerate`/`CROSS` row this project already knew.
    /// What each does with the pickup is this engine's, for the reason
    /// `oag_gameplay::pickup` gives at length: no grant, fire or absorb call
    /// site has been found.
    ///
    /// Absorb *pays* the recovered `<Weapon><Stats absorb>` into the pool
    /// through the recovered clamp, so of the two it is the better evidenced.
    ///
    /// Edge-triggered on both, so holding a button spends one pickup rather than
    /// one a tick. Nothing happens with an empty slot, including no sound - the
    /// original's "nothing to fire" cue is not implemented.
    pub(super) fn spend_pickup(&mut self, snapshot: &InputSnapshot) {
        let fire = snapshot
            .buttons
            .is_pressed(oag_gameplay::input::Button::Square);
        let absorb = snapshot
            .buttons
            .is_pressed(oag_gameplay::input::Button::Circle);
        if !fire && !absorb {
            return;
        }
        let Some(weapon) = self.world.ships[0].pickup.weapon else {
            return;
        };
        let Some(weapons) = self.weapons.as_ref() else {
            return;
        };

        // Fire wins a same-tick tie. Arbitrary, and stated rather than left to
        // the order of two `if`s: a pad hands out one thing and both buttons
        // spend it, so the two can only race on a tick where the player pressed
        // both.
        if fire {
            // **Firing while the autopilot is running cancels it and fires
            // nothing**, and that is recovered: `FUN_08844ec4` tests
            // `craft->0x1b8 & 0x800` before its held-weapon jump table and,
            // when it is set, stores `0.0` into the timer and branches past the
            // table entirely. The pickup is untouched on that path - the
            // original clears `craft+0x1bc` in the grant and in each handler,
            // never here - so the player keeps whatever they were holding. See
            // `docs/ghidra/functions/psp-pulse-usa/autopilot.md`.
            if self.world.ships[0].autopilot_timer > 0.0 {
                self.world.ships[0].autopilot_timer = 0.0;
                return;
            }
            match weapon {
                oag_formats::weapons::Weapon::Turbo => {
                    let Some(simple) = weapons.simple(weapon) else {
                        // The file authors no Turbo. Nothing to fire *with*, so
                        // the pickup is kept rather than spent on nothing.
                        return;
                    };
                    self.world.ships[0].physics.turbo_timer = simple.time;
                    // The same visual a speed pad arms, on the same argument:
                    // the plume is what a boost looks like, and there is one
                    // boost. Not recovered for this path - no capture of a fired
                    // Turbo exists - so it is the plume being reused rather than
                    // a reading of what the original shows.
                    self.exhaust[0].boost(exhaust::BOOST_SECONDS);
                }
                oag_formats::weapons::Weapon::Shield => {
                    let Some(simple) = weapons.simple(weapon) else {
                        // As above: nothing to raise a shield *for*, so the
                        // pickup is kept rather than spent on nothing.
                        return;
                    };
                    // **A running shield cannot be refreshed**, and that is
                    // recovered: `Shield_Fire` (`0x08861568`) does its whole
                    // body inside `if ((craft->0x1b8 & 0x10) == 0)` and its
                    // `else` arm only drops the fire request. The pickup is
                    // still gone either way - the original clears `craft+0x1bc`
                    // in the grant, not in the handler - so firing a Shield into
                    // a running one wastes it, which is what happens here.
                    //
                    // Deliberately *not* the "keep the pickup" shape the arms
                    // around it use for a missing table: that shape is for
                    // nothing having happened, and here something did.
                    if self.world.ships[0].physics.shield_pickup_timer <= 0.0 {
                        self.world.ships[0].physics.shield_pickup_timer = simple.time;
                        self.shield[0].activate();
                    }
                }
                oag_formats::weapons::Weapon::Autopilot => {
                    let Some(simple) = weapons.simple(weapon) else {
                        // As the two arms above: nothing to hand the craft over
                        // *for*, so the pickup is kept rather than spent.
                        return;
                    };
                    // **Re-arming is allowed here and is not for the Shield**,
                    // and the difference is read rather than chosen:
                    // `Shield_Fire` wraps its whole body in
                    // `if ((craft->0x1b8 & 0x10) == 0)` and `Autopilot_Fire`
                    // has no such guard - it assigns the timer unconditionally.
                    // It cannot fire into a running one anyway, because the
                    // branch above cancels instead.
                    self.world.ships[0].autopilot_timer = simple.time;
                    // The driver has to be told where the craft is before it
                    // flies it, for the windowed-search reason
                    // `Race::set_autopilot` records. A pickup is collected
                    // anywhere on the circuit, so this matters more here than
                    // it does for the operator's flag.
                    self.locate_player_driver();
                }
                oag_formats::weapons::Weapon::Rocket => {
                    let Some(stats) = weapons.rocket() else {
                        // No authored rocket, so nothing to put in the air.
                        return;
                    };
                    let ship = &self.world.ships[0];
                    // **Three, together, fanned by `<Rocket spread>`** - see
                    // `oag_gameplay::projectile::launch` and
                    // `docs/ghidra/functions/psp-pulse-usa/weapon-fire.md`. The
                    // order is the original's, and it matters: it decides which
                    // slot each rocket lands in, and the slot is hashed state.
                    let shots = oag_gameplay::projectile::launch(
                        &ship.physics,
                        &ship.handling.dimensions,
                        &stats,
                        to_format_class(self.class),
                    );
                    // Spent on the *first* shot getting away. A partial volley
                    // is better than a pickup that survives having fired two of
                    // three, and the array cannot fill from one press in a race
                    // this engine can currently run.
                    let mut fired = 0;
                    for (position, velocity) in shots {
                        if self.world.projectiles.spawn(
                            oag_formats::weapons::Weapon::Rocket,
                            position,
                            velocity,
                            0,
                        ) {
                            fired += 1;
                        }
                    }
                    if fired == 0 {
                        // Every slot was taken. Keep the pickup rather than
                        // spend it on a volley that never left - the same rule
                        // the two arms above follow for a missing table.
                        return;
                    }
                }
                oag_formats::weapons::Weapon::Missile => {
                    let Some(stats) = weapons.missile() else {
                        // As the Rocket: nothing to put in the air.
                        return;
                    };
                    if !self.fire_missile(0, &stats) {
                        // No slot, or nothing worth locking. Keep the pickup -
                        // firing a missile at nobody is spending it on nothing,
                        // and unlike a rocket it cannot usefully be thrown
                        // downrange on the off chance.
                        return;
                    }
                }
                // The other nine have no effect to run. Deliberately *not* spent:
                // a pickup that vanishes when fired and does nothing is worse
                // than one the player can still absorb.
                //
                // **This arm is a runtime no-op, not a compile error** - an
                // earlier version of this comment claimed adding to
                // `pickup::IMPLEMENTED` was "a compile-visible choice", and it is
                // not: a weapon added to that list with no arm here compiles
                // clean and hands the player a pickup that does nothing when
                // fired. What actually guards it is
                // `every_implemented_weapon_has_an_arm_here` in
                // `crate::race::tests::weapons`.
                _ => return,
            }
        } else {
            let Some(amount) = weapons.absorb(weapon) else {
                return;
            };
            let handling = self.world.ships[0].handling;
            oag_physics::damage::add(
                &mut self.world.ships[0].physics,
                &handling.dimensions,
                amount,
            );
        }
        self.world.ships[0].pickup.weapon = None;
    }

    /// Locks a target and puts one missile in the air for `slot`.
    ///
    /// Returns whether anything left the rail, so a caller can decline to spend
    /// the pickup - `false` means either that nothing was worth locking or that
    /// the array was full.
    ///
    /// # The player and an opponent use the same rule, on purpose
    ///
    /// `oag_ai::Driver::wants_to_fire` decides whether an *opponent* pulls the
    /// trigger, and its five gates are about that decision - is there somebody
    /// ahead, is the road straight enough, has the trigger rolled. They are not
    /// the weapon's rule. `Ship_AcquireLock` (`0x08844784`) is, and the original
    /// runs it for the player's craft; running it here for both means a missile's
    /// target does not depend on who fired it.
    ///
    /// So an opponent's shot is gated twice - by its own willingness *and* by the
    /// lock - which is the original's arrangement rather than an extra of ours.
    ///
    /// `pub` rather than `pub(super)` so `crates/game/tests/missile_ground_truth.rs`
    /// can reach it: an integration test links the *library*, so nothing narrower
    /// is visible to one. `oag-game` is the composition root and
    /// `scripts/check-dependency-rules.py` forbids anything depending on it, so
    /// `pub` here reaches the tests and the binary and nothing else.
    pub fn fire_missile(
        &mut self,
        slot: usize,
        stats: &oag_formats::weapons::MissileStats,
    ) -> bool {
        let count = self.world.ship_count as usize;
        let ship = &self.world.ships[slot];
        let (position, velocity, launch_kmh) = oag_gameplay::projectile::missile::launch(
            &ship.physics,
            &ship.handling.dimensions,
            stats,
            to_format_class(self.class),
        );
        // The lock is taken from the **nose**, where the missile actually starts,
        // rather than from the craft's centre: the near bound of the authored
        // window is ten units and a hull is four long, so measuring from the
        // wrong end moves the boundary by most of a craft.
        let target = oag_gameplay::projectile::missile::lock(
            &self.world.ships[..count],
            slot as u8,
            position,
            ship.physics.body.forward(),
            stats,
            self.course.as_ref().map(oag_race::Course::length),
        );
        // **No lock, no launch.** A missile with no target flies straight and is
        // simply a worse rocket, and the original does not fire one: its handler
        // takes the target from the craft and `Ship_FireHeldWeapon` passes a null
        // when the lock flag is clear. Keeping the pickup is ours - what the
        // original does with an unlocked press has not been read.
        let Some(target) = target else {
            return false;
        };
        self.world.projectiles.spawn_guided(
            oag_formats::weapons::Weapon::Missile,
            position,
            velocity,
            slot as u8,
            Some(target),
            launch_kmh,
        )
    }

    /// Plays the explosion a rocket that just went off authored.
    ///
    /// **Recovered end to end.** Which of the two files plays:
    /// [`TRACK_BLAST_EFFECT`] from `Rocket_Update`'s (`0x0885d2a8`) two
    /// collision branches, [`CRAFT_BLAST_EFFECT`] from
    /// `Rocket_SpawnCraftExplosion_q` (`0x0886ed34`) on the craft-hit path.
    /// Where it plays: a craft hit is drawn at the *struck craft's* own
    /// position dropped by [`CRAFT_BLAST_DROP`], not at the rocket's impact
    /// point. What it looks like: everything, out of the file, through
    /// [`psys::Stage`]. A craft that has since gone inactive falls back to
    /// the impact point rather than reading a stale pose.
    ///
    /// Nothing is drawn when the effect did not load. That is deliberate and
    /// it is the rule for every effect here: an authored particle system is
    /// not something to approximate with billboards, because a stand-in that
    /// reads as *plausible* is how a wrong picture survives review. Until
    /// 2026-08-12 this method drew three expanding additive puffs and a
    /// separate eight-billboard smoke trail, all invented.
    ///
    /// [`Race::sparks`] is deliberately not reused for it: that system
    /// re-anchors to the hull every tick, so a burst ignited at a rocket's
    /// impact would emit from the craft instead. It is one hull-mounted
    /// emitter, and this is why the stage exists alongside it.
    pub(super) fn ignite_blast(&mut self, point: Vec3, struck: Option<usize>) {
        let (name, at) = self.blast_for(point, struck);
        let Some(effect) = self.effects.get(name).cloned() else {
            return;
        };
        // Neutral severity: the field the collision sparks derive from an
        // impulse is the *hull's*, and nothing on the rocket path has been
        // read as feeding it. See `oag_render::sparks::severity`.
        self.stage.play(&effect, at, 1.0);
    }

    /// Which explosion a hit plays and where, split out from
    /// [`Race::ignite_blast`] so the recovered part can be asserted without a
    /// disc to load the effect from.
    pub(super) fn blast_for(&self, point: Vec3, struck: Option<usize>) -> (&'static str, Vec3) {
        match struck {
            Some(slot) if self.world.ships[slot].active => (
                CRAFT_BLAST_EFFECT,
                self.world.ships[slot].physics.body.position - Vec3::Y * CRAFT_BLAST_DROP,
            ),
            // A craft that has gone inactive since the hit falls back to the
            // impact point rather than reading a stale pose - still its own
            // effect, because what was struck is what chose the file.
            Some(_) => (CRAFT_BLAST_EFFECT, point),
            None => (TRACK_BLAST_EFFECT, point),
        }
    }

    /// Keeps an [`ENGINE_FLARE_EFFECT`] instance on every active craft's
    /// nozzle, where the source authors one.
    ///
    /// **The PS2 port authors an engine flare as a particle effect and the
    /// PSP does not.** On a PSP-sourced race the effect is absent from the
    /// library, nothing attaches, and [`oag_render::exhaust`]'s procedural
    /// flare draws as it always has. On a PS2-sourced one the asset plays
    /// and the procedural quad steps aside - see
    /// [`Race::engine_flare_effect`], which is what the renderer asks.
    ///
    /// The trigger needs no reverse-engineering: an engine flare is on for
    /// as long as the craft is, which is why this one is wired where the
    /// other PS2-only effects are not. Both its emitters are
    /// [`oag_formats::pob::flags::LOOPING`], so it runs until detached.
    ///
    /// Anchored at the `Engine Flare` locator under the craft's *current*
    /// transform, the same point the procedural flare uses, so the two are
    /// interchangeable rather than merely similar.
    pub(super) fn advance_engine_flares(&mut self) {
        let Some(effect) = self.effects.get(ENGINE_FLARE_EFFECT).cloned() else {
            return;
        };
        for slot in 0..MAX_SHIPS {
            let nozzle = self.world.ships[slot]
                .active
                .then(|| self.nozzle_of(slot))
                .flatten();
            match (nozzle, self.engine_flare[slot]) {
                (Some(at), Some(playing)) => self.stage.follow(playing, at),
                (Some(at), None) => self.engine_flare[slot] = self.stage.attach(&effect, at, 1.0),
                // Gone or never had a locator: hand the instance back and
                // let its particles fade rather than cutting them.
                (None, Some(playing)) => {
                    self.stage.detach(playing);
                    self.engine_flare[slot] = None;
                }
                (None, None) => {}
            }
        }
    }

    /// Keeps a [`ROCKET_FLARE_EFFECT`] instance riding every projectile in
    /// the air, and takes it off the ones that are gone.
    ///
    /// **Recovered.** `Rocket_Init` (`0x0885cdb8`) attaches the flare at
    /// launch and it rides the rocket for the whole flight; both emitters are
    /// [`oag_formats::pob::flags::LOOPING`], so the 100-tick duration they
    /// author is not a countdown and the effect does not need re-triggering
    /// to outlast it.
    ///
    /// A detonated rocket has its flare **detached, not killed**: emission
    /// stops and the particles already out finish their own lives, so the
    /// smoke outlives the rocket the way it does in the original. That is
    /// also the bug the invented trail could not avoid - it had to drop the
    /// whole streak the instant the slot vacated, or the next projectile to
    /// take that slot would have inherited it.
    ///
    /// **Called after `projectile::step`**, so a flare is anchored where its
    /// rocket ended the tick.
    pub(super) fn advance_projectile_flares(&mut self) {
        let effect = self.effects.get(ROCKET_FLARE_EFFECT).cloned();
        for (slot, projectile) in self.world.projectiles.slots.iter().enumerate() {
            match (projectile.kind.is_none(), self.projectile_flare[slot]) {
                // Gone: hand the instance back and let it fade out.
                (true, Some(playing)) => {
                    self.stage.detach(playing);
                    self.projectile_flare[slot] = None;
                }
                (true, None) => {}
                (false, Some(playing)) => self.stage.follow(playing, projectile.position),
                // Freshly in the air. `attach` returning `None` means the
                // stage is full of flares already, and this rocket simply
                // flies without one rather than evicting someone else's.
                (false, None) => {
                    if let Some(effect) = effect.as_ref() {
                        self.projectile_flare[slot] =
                            self.stage.attach(effect, projectile.position, 1.0);
                    }
                }
            }
        }
    }

    /// This frame's fallback projectile sprites, in the engine flare's shape.
    ///
    /// `right` and `up` are the camera's, read out of the view matrix the same
    /// way the exhaust's are.
    ///
    /// `modelled` says the caller is drawing the rockets as
    /// [`ROCKET_MODEL_ENTRY`]'s mesh, in which case this draws **nothing**:
    /// the glow around a modelled rocket is [`ROCKET_FLARE_EFFECT`], played
    /// off the disc through [`Race::stage`], and a billboard on top of it
    /// would be a second invented one. What is left here is the case where
    /// the model did not load and a projectile would otherwise be invisible -
    /// see [`PROJECTILE_SPRITE_HALF_SIZE`].
    #[must_use]
    pub fn projectile_sprites(
        &self,
        right: Vec3,
        up: Vec3,
        modelled: bool,
    ) -> Vec<oag_render::mesh::GpuVertex> {
        let mut vertices = Vec::new();
        if modelled {
            return vertices;
        }
        for projectile in &self.world.projectiles.slots {
            if projectile.kind.is_none() {
                continue;
            }
            vertices.extend(exhaust::sprite(
                projectile.position,
                right,
                up,
                PROJECTILE_SPRITE_HALF_SIZE,
                1.0,
            ));
        }
        vertices
    }

    /// Where each live rocket is and how it is oriented, for the model draw.
    ///
    /// **Forward alignment is recovered; the quarter-turn is not.**
    /// `Rocket_Update` (`0x0885d2a8`) rebuilds a basis every tick from the
    /// rocket's normalised velocity and the surface normal under it, hands it to
    /// the model's scene node, and rotates it by a further `-pi/2` about an axis
    /// this project has not resolved - the call is an unresolved import stub.
    /// So this aligns the model along its velocity, which is the evidenced part,
    /// and does **not** invent the extra rotation. See
    /// `docs/ghidra/functions/psp-pulse-usa/rocket-visuals.md`.
    ///
    /// The original's second basis vector is the *track* normal, which this
    /// engine does not carry on a projectile (its rockets fly straight and never
    /// consult the surface - the same page records that gap). World up stands in,
    /// which only decides the model's roll about its own length.
    ///
    /// One entry per live rocket, in slot order, so the caller can zip it
    /// against its drawables.
    ///
    /// # No rendered frame has yet contained a rocket
    ///
    /// Worth stating rather than leaving to be discovered. What *is* checked:
    /// the model loads off a real disc and its long axis is the one aimed down
    /// the velocity here
    /// (`the_rocket_model_is_longest_along_the_axis_it_is_flown_down`), the
    /// bases below are orthonormal and velocity-aligned including the
    /// straight-up degenerate case, and the draw is wired exactly as the ships'
    /// and plumes' are. What is **not**: a captured frame with a rocket in it.
    ///
    /// The headless capture path cannot produce one. `--race` gives a craft that
    /// holds the throttle and does not steer, so over 2400 ticks it never
    /// reaches a `Weapon Pad`, never gets a pickup, and the telemetry line never
    /// reports one held. A first attempt at this misread three pieces of
    /// **track scenery** as a fanned volley - they render identically in
    /// `time_trial`, where no rocket can exist, which is the check that settles
    /// it and the one to repeat before believing any future frame:
    ///
    /// ```sh
    /// cargo run -p oag-game -- --race --mode single_race --hold cross \
    ///     --press square --ticks 900 --screenshot /tmp/on.png
    /// cargo run -p oag-game -- --race --mode time_trial  --hold cross \
    ///     --press square --ticks 900 --screenshot /tmp/off.png
    /// magick compare -metric AE /tmp/on.png /tmp/off.png null:
    /// ```
    ///
    /// Closing it wants a craft that can drive to a pad - the AI, or an input
    /// script replayed through `oag-trace run --script`.
    #[must_use]
    pub fn rocket_model_matrices(&self) -> Vec<Mat4> {
        self.world
            .projectiles
            .slots
            .iter()
            .filter(|projectile| projectile.kind.is_some())
            .map(|projectile| {
                let forward = projectile.velocity.normalize_or_zero();
                if forward == Vec3::ZERO {
                    return Mat4::from_translation(projectile.position);
                }
                // `Vec3::Y` is a poor reference exactly when the rocket is flying
                // straight up or down; `any_orthonormal_vector` is the fallback
                // rather than a silently degenerate basis.
                let reference = if forward.dot(Vec3::Y).abs() > 0.999 {
                    forward.any_orthonormal_vector()
                } else {
                    Vec3::Y
                };
                let side = forward.cross(reference).normalize_or_zero();
                let up = side.cross(forward);
                Mat4::from_cols(
                    side.extend(0.0),
                    up.extend(0.0),
                    forward.extend(0.0),
                    projectile.position.extend(1.0),
                )
            })
            .collect()
    }
}
