//! Pickups and what they fire: the free turbo, the one-slot inventory, and
//! the lock-on sight.
//!
//! Split out of `race.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change. Its tests are
//! `race/tests/weapons.rs`. What a fired weapon then *shows* - the blasts, the
//! flares, the sprites - is [`visuals`], split out under the same rule the day
//! the Mine's and the Missile's own effects were wired. The Quake's and the
//! LeachBeam's per-tick advances are [`single_instance`], split out under the
//! same rule again the day the LeachBeam landed.

use super::*;

mod disruptor;
mod single_instance;
mod visuals;
// Only reached through `crate::race::weapons::<name>` by
// `race/tests/weapons.rs`, which asserts each pure gate without a disc -
// see the doc comments on `visuals::flare_effect_for` and its neighbours.
// `#[cfg(test)]` because nothing in a non-test build calls through this
// path: `visuals.rs` reaches all three directly by their bare names.
#[cfg(test)]
pub(super) use visuals::{
    bounce_effect_for, bounced_this_tick, flare_effect_for, missile_flare_anchors,
};

/// The far plane the reticle's own projection uses.
///
/// Not a visual range: [`sight::DRAW_RANGE`] is, and it is tested in eye space
/// before the projection runs. This only has to be far enough that no craft the
/// sight would draw is clipped by it, which at 250 units of draw range anything
/// past a few thousand is.
const SIGHT_FAR: f32 = 10_000.0;

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
        if !matches!(self.sim.world.race.mode, Mode::TimeTrial | Mode::SpeedLap) {
            return;
        }
        // The same "only into an empty slot" rule a pad follows, so a player who
        // has not spent last lap's turbo does not silently lose this one - they
        // keep the one they have.
        if !self.sim.world.ships[0].pickup.is_empty() {
            return;
        }
        // Gated on the table, so a disc whose weapon file did not load hands
        // out nothing rather than a Turbo with no duration to fire it for.
        if self
            .sim
            .weapons
            .as_ref()
            .and_then(|w| w.simple(oag_tables::weapons::Weapon::Turbo))
            .is_none()
        {
            return;
        }
        self.sim.world.ships[0].pickup.weapon = Some(oag_tables::weapons::Weapon::Turbo);
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
        let Some(weapon) = self.sim.world.ships[0].pickup.weapon else {
            return;
        };
        let Some(weapons) = self.sim.weapons.as_ref() else {
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
            if self.sim.world.ships[0].autopilot_timer > 0.0 {
                self.sim.world.ships[0].autopilot_timer = 0.0;
                return;
            }
            match weapon {
                oag_tables::weapons::Weapon::Turbo => {
                    let Some(simple) = weapons.simple(weapon) else {
                        // The file authors no Turbo. Nothing to fire *with*, so
                        // the pickup is kept rather than spent on nothing.
                        return;
                    };
                    self.sim.world.ships[0].physics.turbo_timer = simple.time;
                    // The same visual a speed pad arms, on the same argument:
                    // the plume is what a boost looks like, and there is one
                    // boost. Not recovered for this path - no capture of a fired
                    // Turbo exists - so it is the plume being reused rather than
                    // a reading of what the original shows.
                    self.view.exhaust[0].boost(exhaust::BOOST_SECONDS);
                }
                oag_tables::weapons::Weapon::Shield => {
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
                    if self.sim.world.ships[0].physics.shield_pickup_timer <= 0.0 {
                        self.sim.world.ships[0].physics.shield_pickup_timer = simple.time;
                        self.view.shield[0].activate();
                    }
                }
                oag_tables::weapons::Weapon::Autopilot => {
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
                    self.sim.world.ships[0].autopilot_timer = simple.time;
                    // The driver has to be told where the craft is before it
                    // flies it, for the windowed-search reason
                    // `Race::set_autopilot` records. A pickup is collected
                    // anywhere on the circuit, so this matters more here than
                    // it does for the operator's flag.
                    self.locate_player_driver();
                }
                oag_tables::weapons::Weapon::Rocket => {
                    let Some(stats) = weapons.rocket() else {
                        // No authored rocket, so nothing to put in the air.
                        return;
                    };
                    let ship = &self.sim.world.ships[0];
                    // **Three, together, fanned by `<Rocket spread>`** - see
                    // `oag_gameplay::projectile::launch` and
                    // `docs/ghidra/functions/psp-pulse-usa/weapon-fire.md`. The
                    // order is the original's, and it matters: it decides which
                    // slot each rocket lands in, and the slot is hashed state.
                    // `None` where the table authors a speed per class and
                    // this race's rung is outside them: no shot, rather than a
                    // shot at some other rung's speed.
                    let Some(shots) = oag_gameplay::projectile::launch(
                        &ship.physics,
                        &ship.handling.dimensions,
                        &stats,
                        &self.sim.class,
                    ) else {
                        return;
                    };
                    // Spent on the *first* shot getting away. A partial volley
                    // is better than a pickup that survives having fired two of
                    // three, and the array cannot fill from one press in a race
                    // this engine can currently run.
                    let mut fired = 0;
                    for (position, velocity) in shots {
                        if self.sim.world.projectiles.spawn(
                            oag_tables::weapons::Weapon::Rocket,
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
                oag_tables::weapons::Weapon::Plasma => {
                    let Some(stats) = weapons.plasma() else {
                        // As the Rocket: nothing to put in the air.
                        return;
                    };
                    let ship = &self.sim.world.ships[0];
                    // **One bolt, not three.** `Weapon_FirePlasma`
                    // (`0x0886a868`) spawns once and clears its own request
                    // bit; the `<Stats>` carry no `spread` to fan a volley
                    // with. See `oag_gameplay::projectile::plasma::launch`.
                    let Some((position, velocity)) = oag_gameplay::projectile::plasma::launch(
                        &ship.physics,
                        &ship.handling.dimensions,
                        &stats,
                        &self.sim.class,
                    ) else {
                        return;
                    };
                    // **Held for a second before it flies.** `Plasma_Init`
                    // (`0x0885bd18`) marks the fresh pool entry charging and
                    // the pool walker holds it on the nose until the countdown
                    // runs out - see
                    // `oag_gameplay::projectile::plasma::CHARGE_SECONDS`, which
                    // also records why the wind-up is one second and not the
                    // `charge_time="3"` the file authors.
                    if !self.sim.world.projectiles.charge_up(
                        position,
                        velocity,
                        0,
                        oag_gameplay::projectile::plasma::CHARGE_SECONDS,
                    ) {
                        // Every slot was taken. Keep the pickup rather than
                        // spend it on a shot that never left - the same rule
                        // the Rocket's arm follows for an empty volley.
                        return;
                    }
                }
                oag_tables::weapons::Weapon::Shuriken => {
                    let Some(stats) = weapons.shuriken() else {
                        // As the Rocket: nothing to put in the air.
                        return;
                    };
                    // **Nothing is drawn unless there is somewhere to put the
                    // blade**, which is the original's own order:
                    // `Weapon_FireShuriken` takes its `rand()` *inside* the
                    // pool-space check. Drawing first and discarding would
                    // advance the seeded generator on a tick the original does
                    // not, and the generator is hashed state.
                    if self.sim.world.projectiles.live()
                        >= oag_gameplay::projectile::MAX_PROJECTILES
                    {
                        return;
                    }
                    // Copied out before the draw because `launch` borrows the
                    // world's generator mutably and the ship state immutably at
                    // once; both are small `Copy` structs.
                    let physics = self.sim.world.ships[0].physics;
                    let dimensions = self.sim.world.ships[0].handling.dimensions;
                    // **One blade, twenty degrees off the nose, side chosen by
                    // a coin.** See `oag_gameplay::projectile::shuriken::launch`
                    // and `docs/ghidra/functions/psp-pulse-usa/shuriken.md`.
                    let Some((position, velocity)) = oag_gameplay::projectile::shuriken::launch(
                        &physics,
                        &dimensions,
                        &stats,
                        &self.sim.class,
                        &mut self.sim.world.rng,
                    ) else {
                        return;
                    };
                    if !self
                        .sim
                        .world
                        .projectiles
                        .throw(position, velocity, 0, stats.fuse)
                    {
                        // Unreachable given the check above, and kept as the
                        // same "keep the pickup rather than spend it on nothing"
                        // rule the Rocket's and the Plasma's arms follow -
                        // `throw` is the only thing that can answer the question
                        // authoritatively, and a silently dropped shot that also
                        // ate the pickup is the failure worth being paranoid
                        // about.
                        return;
                    }
                }
                oag_tables::weapons::Weapon::Missile => {
                    let Some(stats) = weapons.missile() else {
                        // As the Rocket: nothing to put in the air.
                        return;
                    };
                    // **Fired whether or not anything is lockable**, and the
                    // pickup is spent either way. `Ship_FireHeldWeapon`
                    // (`0x08844ae8`) branches on the lock and calls
                    // `Weapon_RequestFire` on both arms - a null target and an
                    // index of `-1` when there is none - and
                    // `Weapon_FireMissile` (`0x088685cc`) clears the held slot
                    // *before* it checks whether the pool has room. So even the
                    // full-pool case costs the player the pickup. The old
                    // "keep it, a missile at nobody is nothing" rule was ours
                    // and was wrong twice over: an unlocked missile flies
                    // ballistically and goes off on its own timer.
                    self.fire_missile(0, &stats);
                }
                oag_tables::weapons::Weapon::Disruptor => {
                    // Pure's bolt. Rolled, locked and spawned in one place
                    // for both paths - see `Race::fire_disruptor`. A full
                    // pool keeps the pickup, as the Rocket's arm does.
                    if !self.fire_disruptor(0) {
                        return;
                    }
                }
                oag_tables::weapons::Weapon::Mine | oag_tables::weapons::Weapon::Bomb => {
                    let Some(drop) =
                        oag_gameplay::projectile::mine::Drop::for_weapon(weapon, weapons)
                    else {
                        // As the Rocket: nothing to lay.
                        return;
                    };
                    // **The pickup is not spent here, and that is recovered.**
                    // `Weapon_DropMines` (`0x088675cc`) clears the craft's
                    // held-weapon slot only in the branch where its counter
                    // reaches zero, so a craft goes on holding the Mine for the
                    // whole half-second the cluster takes to come out. Starting
                    // the drop is the whole action; `Race::lay_mines` does the
                    // rest, one mine at a time, and clears the slot when the
                    // last one is out.
                    //
                    // Returning early therefore skips the `weapon = None` at the
                    // bottom of this function on purpose - it is the one arm
                    // here that must not spend the slot.
                    //
                    // **Called on every press while held, with no
                    // `is_dropping()` guard here** - and none is needed, because
                    // `Held::begin_drop` is itself a no-op mid-cluster. A
                    // mash-fire capture used to log three times the authored
                    // rate (one mine per press-edge instead of one per
                    // `DROP_INTERVAL`) because this call re-armed the counter on
                    // every re-press; see that method's doc comment for the
                    // mechanism and for why the guard is chosen rather than
                    // measured.
                    self.sim.world.ships[0].pickup.begin_drop(drop.count);
                    return;
                }
                oag_tables::weapons::Weapon::LeachBeam => {
                    let Some(stats) = weapons.leach_beam() else {
                        // As the Rocket: nothing to fasten onto anybody.
                        return;
                    };
                    // **The busy check first, and it is a whole-race one.**
                    // `Weapon_FireLeachBeam` (`0x08866658`) returns on
                    // `pool->live != 0` - a world cursor, not a per-craft
                    // cooldown - so a press while *anybody's* beam is up keeps
                    // the pickup. The strictest gate any weapon here has; see
                    // `docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md`.
                    if self.sim.world.leach_beam.is_some() {
                        return;
                    }
                    // The lock is the Missile's own, unmodified - the same
                    // `Ship_AcquireLock` scan off the LeachBeam's own
                    // `<Stats>` window, which `Race::sight_target` already
                    // computes for the reticle. `FUN_0883f540` confirms the
                    // reuse directly: it calls `Ship_AcquireLock` for held
                    // weapon ids `1` and `10` and no others.
                    //
                    // **Fired with or without one**, and the pickup is spent
                    // either way: `Weapon_FireLeachBeam` claims the pool slot
                    // and clears the fire bit *before* it branches on
                    // `craft+0x16c`, and the no-lock arm builds a real
                    // instance that simply expires. The player has fired.
                    self.sim.world.leach_beam = Some(match self.sight_target() {
                        Some(target) => {
                            oag_gameplay::projectile::leach_beam::Beam::locked(0, target, &stats)
                        }
                        None => oag_gameplay::projectile::leach_beam::Beam::unlocked(0, &stats),
                    });
                }
                oag_tables::weapons::Weapon::Quake => {
                    let Some(stats) = weapons.quake() else {
                        // No authored Quake, so nothing to launch - the same
                        // "keep the pickup" rule the Rocket's and the
                        // Plasma's arms follow for a missing table.
                        return;
                    };
                    // **The busy check, first - matching `Weapon_FireQuake`'s
                    // own `if (q->active != 0) return;`, which runs before
                    // the original ever clears the held slot.** Only one
                    // wave can be in flight in the whole race at once, so a
                    // press while one is already travelling keeps the
                    // pickup rather than spending it on nothing. See
                    // `docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md`.
                    if self.sim.world.quake.is_some() {
                        return;
                    }
                    let ship = &self.sim.world.ships[0];
                    let Some(progress) = ship.standing.progress else {
                        // Not yet located on the course - nowhere on the
                        // track's own spline to launch a travelling wave
                        // from. Keep the pickup; this is a startup edge case
                        // a moving craft clears within a tick or two.
                        return;
                    };
                    // The direction sign: dot the firing craft's own forward
                    // against the track's own tangent where it currently
                    // stands, the same dot product `Quake_Init` reads to
                    // pick which way the wave travels. `unwrap_or(1.0)` is
                    // the same "forward, not stalled" default
                    // `oag_gameplay::projectile::quake::Wave::launch` itself
                    // takes for an exactly-perpendicular dot - see that
                    // function's own doc comment for why.
                    let forward_dot_tangent = ship
                        .standing
                        .course_index
                        .and_then(|index| self.sim.course.as_ref()?.tangent(index as usize))
                        .map(|tangent| tangent.dot(ship.physics.body.forward()))
                        .unwrap_or(1.0);
                    self.sim.world.quake = Some(oag_gameplay::projectile::quake::Wave::launch(
                        0,
                        progress,
                        forward_dot_tangent,
                        &stats,
                    ));
                }
                oag_tables::weapons::Weapon::Cannon => {
                    // **Recovered as a non-event *on this path*, not an
                    // oversight.** `Weapon_RequestFire`'s bit `0x2000` - the
                    // Cannon's own - is dispatched by nothing in
                    // `Weapons_DispatchFire`: the Cannon does not fire through
                    // the fire-request system the Rocket and the rest share at
                    // all, so a press *edge* here sets a bit nothing reads.
                    //
                    // The button still fires the weapon, just not from here.
                    // `Cannon_UpdateReload` (`0x0883f424`) reads the **held**
                    // half of the same button - `+0x16` of the craft's control
                    // record, against `+0x15` for the edge this arm is - and
                    // advances its own countdown on every frame the button is
                    // down. So the pickup must survive this press rather than
                    // be spent by it, and `Race::advance_cannons` does the
                    // firing. See
                    // `docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md`.
                    return;
                }
                // The other eight have no effect to run. Deliberately *not* spent:
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
            // **Eliminator refuses this outright.** `MSC_EVENT_ELIM`: "you
            // cannot absorb pickups" - see `Mode::pickups_absorb`. The pickup
            // is kept rather than spent, the same "nothing happened, so
            // nothing is lost" rule the no-op weapon arms above already
            // follow.
            if !self.sim.world.race.mode.pickups_absorb() {
                return;
            }
            let Some(amount) = weapons.absorb(weapon) else {
                return;
            };
            let handling = self.sim.world.ships[0].handling;
            oag_physics::damage::add(
                &mut self.sim.world.ships[0].physics,
                &handling.dimensions,
                amount,
            );
        }
        // `Held::take` rather than a direct write - see its doc comment for
        // why an absorb mid-drop has to clear more than the visible weapon.
        self.sim.world.ships[0].pickup.take();
    }

    /// Lays whatever charge is due from every craft mid-drop, one tick's worth.
    ///
    /// Both rear weapons, because they are one mechanism: a Bomb is a drop of
    /// one. See `oag_gameplay::projectile::mine`.
    ///
    /// **Every craft, in slot order**, because a drop is per-craft state and an
    /// opponent's cluster has to come out at the same rate the player's does.
    /// Slot order is the determinism rule the whole tick follows.
    ///
    /// Called from the tick right after `Race::spend_pickup`, so a mine is laid
    /// from where the craft is *before* this tick's force law - the same instant
    /// a rocket fired on the same tick leaves from. A mine does not move once
    /// laid, so this is the only chance to get its position right.
    ///
    /// Does nothing at all when no craft is mid-drop, which is every craft
    /// almost all of the time: [`oag_gameplay::pickup::Held::advance_drop`]
    /// returns immediately on a zero counter.
    pub(super) fn lay_mines(&mut self) {
        let Some(weapons) = self.sim.weapons.as_ref() else {
            // No table. A craft cannot have started a drop without one - the arm
            // above gates on exactly this - so there is nothing in flight to
            // abandon.
            return;
        };
        for slot in 0..self.sim.world.ship_count as usize {
            if !self.sim.world.ships[slot].active
                || !self.sim.world.ships[slot].pickup.is_dropping()
            {
                continue;
            }
            // **The held weapon, re-read every tick, and it is what says which
            // of the two is coming out.** A drop keeps the pickup in the slot
            // until the last charge is laid - that is the recovered coupling -
            // so the weapon is still there to be asked.
            let Some(weapon) = self.sim.world.ships[slot].pickup.weapon else {
                continue;
            };
            let Some(drop) = oag_gameplay::projectile::mine::Drop::for_weapon(weapon, weapons)
            else {
                continue;
            };
            let due = self.sim.world.ships[slot]
                .pickup
                .advance_drop(self.sim.dt, oag_gameplay::projectile::mine::DROP_INTERVAL);
            if !due {
                continue;
            }
            let ship = &self.sim.world.ships[slot];
            let point = oag_gameplay::projectile::mine::drop_point(
                &ship.physics,
                &ship.handling.dimensions,
            );
            // A full array drops this mine and not the drop: the counter has
            // already been spent, so the cluster goes on laying the rest rather
            // than stalling. That matches `Projectiles::spawn`'s own rule -
            // a shot that cannot be taken is lost, not queued.
            let orientation =
                oag_gameplay::projectile::mine::frozen_pose(ship.physics.body.orientation);
            self.sim
                .world
                .projectiles
                .lay(weapon, point, slot as u8, drop.fuse, orientation);
            // **The Mine only.** `Weapon_DropMines` calls `Mine_Init`, which
            // plays `MINELAUNCH`, once per charge; `Weapon_FireBomb` never
            // calls the play function at all - see `Cue::MineLaunch`'s own
            // doc comment. Both weapons share this loop because they are one
            // mechanism in this engine, but the cue is the Mine's alone.
            if weapon == oag_tables::weapons::Weapon::Mine {
                self.sim.cues.push(crate::audio::sfx::CueEvent::new(
                    crate::audio::sfx::Cue::MineLaunch,
                    slot,
                ));
            }
        }
    }

    /// Fires whatever Cannon the player is holding, for as long as the fire
    /// button is held down.
    ///
    /// **The countdown only advances while fire is held**, which is what makes
    /// this weapon behave as the maintainer describes the original: hold to
    /// fire at the authored `rate`, or tap repeatedly for a few frames of
    /// countdown per tap. `Cannon_UpdateReload` (`0x0883f424`) opens with
    /// `if (*(*(entity+0x94)+0x78) + 0x16 == 0) return;`, and that byte is the
    /// **held** state of `OPT_CTRL_FIRE` in the craft's own control record -
    /// the record the binary itself registers under the string `player_input`.
    /// An earlier revision read the same byte as a *track weapon-pad* flag and
    /// concluded the weapon self-fired; it does not. See
    /// `docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md`.
    ///
    /// **Every craft, not just slot 0** - and an opponent's half is *chosen,
    /// not measured, with no confidence score*. `Cannon_UpdateReload` gates on
    /// the fire-held byte of the firing craft's control record, and for an
    /// opponent that record is `Ai + 0x08`, so the byte is `Ai + 0x1e`.
    /// **Nothing in the image writes `Ai + 0x1e`, at any width, and
    /// `Ai_Construct`'s object is allocated without a zero-fill** while the
    /// craft object beside it is explicitly memset. So the original's gate is
    /// uninitialised heap: the *mechanism* is recovered, the *value* is
    /// undefined behaviour and there is no fact about the disc to score. See
    /// `docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md`.
    ///
    /// **An accident is not a design, so this is tuned rather than copied.**
    /// The project's standing ruling is that opponent behaviour is a design
    /// axis, not a fidelity one. An opponent holds the trigger while
    /// `oag_ai::Driver::holds_fire` says there is somebody in front worth
    /// hitting - see that method for what it is tuned for and for the two gates
    /// it deliberately drops. Held unconditionally instead, which is what the
    /// heap-garbage reading literally predicts, a craft sprays its whole
    /// thirty-round magazine at empty track a second and a half after the pickup
    /// lands, and the player mostly never sees the weapon at all.
    ///
    /// The Cannon's *cadence* is still the disc's - the countdown, the authored
    /// `rate` and the magazine are all measured. Only the decision to hold is
    /// ours.
    ///
    /// **A craft flown by the autopilot fires nothing**, and that half *is*
    /// measured: `Ship_UpdateCraft` (`0x08849618`) re-points `craft+0x78` at a
    /// blend buffer filled from the autopilot's own `Ai` record while
    /// `craft+0x1d4` is non-zero, and the player's autopilot is an `Ai` too - so
    /// it has the same unwritten hole, and the human pad's held byte is no longer
    /// what is read. `Race::flown_for_the_player` is this port's stand-in for that
    /// condition; it covers the operator's `--autopilot` too, which the original
    /// has no equivalent of.
    ///
    /// Called from the tick right after `Race::lay_mines`, for the same
    /// reason that one runs where it does: a round fired this tick leaves
    /// from where the craft was when the tick started.
    pub(super) fn advance_cannons(&mut self, snapshot: &InputSnapshot) {
        let Some(cannon) = self
            .sim
            .weapons
            .as_ref()
            .and_then(oag_tables::weapons::WeaponStats::cannon)
        else {
            // No table, or the table authors no Cannon - Pure's does not.
            // Nothing to arm with no `rounds`/`rate` to read.
            return;
        };
        // The player's own held state, read once. An autopilot blend takes the
        // pad out of the loop entirely, exactly as above.
        let player_holds_fire = !self.flown_for_the_player()
            && snapshot
                .buttons
                .is_held(oag_gameplay::input::Button::Square);
        // Built once for the whole grid, the same way `Race::step_opponents`
        // does it: every craft's view of the field needs the same ordering, and
        // an opponent's trigger reads that view.
        let places = self.places();
        for slot in 0..self.sim.world.ship_count as usize {
            // Slot 0 follows the pad; every other slot asks its own driver.
            let holds_fire = if slot == 0 {
                player_holds_fire
            } else {
                let field = self.field_for(slot, &places);
                self.sim.world.ships[slot]
                    .driver
                    .holds_fire(&oag_ai::Context {
                        line: &self.sim.racing_line,
                        tuning: &self.sim.ai_tuning,
                        pilot: &self.sim.ai_pilots[slot],
                        field: &field,
                        yaw_ceiling: None,
                    })
            };
            if !holds_fire {
                // The trigger is up, so the original's countdown does not move.
                // Deliberately *not* a reset: `Cannon_UpdateReload` leaves
                // `craft+0x158` exactly where it stopped, so letting go and
                // squeezing again resumes rather than restarts - which is why an
                // opponent that loses its target mid-burst picks the burst back
                // up rather than re-arming from a full reload.
                continue;
            }
            self.advance_one_cannon(slot, &cannon);
        }
    }

    /// One craft's Cannon countdown, and the round it puts in the air when the
    /// countdown reaches zero. See [`Self::advance_cannons`] for the gate.
    fn advance_one_cannon(&mut self, slot: usize, cannon: &oag_tables::weapons::CannonStats) {
        if !self.sim.world.ships[slot].active
            || self.sim.world.ships[slot].pickup.weapon != Some(oag_tables::weapons::Weapon::Cannon)
        {
            return;
        }
        let Some(remaining) = self.sim.world.ships[slot].pickup.advance_cannon_reload(
            self.sim.dt,
            cannon.rate,
            // Saturating: a disc that ever authored more than 255 rounds
            // would clamp here rather than panic, and none measured does.
            cannon.rounds.max(0.0).round() as u8,
        ) else {
            // The countdown moved but has not reached zero yet.
            return;
        };
        let ship = &self.sim.world.ships[slot];
        // **The low bit of the count remaining after this round**, not
        // before it - `Weapon_FireCannon` reads `craft->shots & 1` after
        // `Cannon_UpdateReload`'s own decrement, and
        // `Held::advance_cannon_reload` hands back exactly that number.
        // See `oag_gameplay::projectile::cannon::launch`.
        let (position, velocity) = oag_gameplay::projectile::cannon::launch(
            &ship.physics,
            &ship.handling.dimensions,
            remaining & 1 != 0,
        );
        // A full array drops this round and not the countdown, the same
        // rule `Race::lay_mines` follows: the round has already been
        // spent, so a Cannon held by a craft in a saturated pool simply
        // stops putting anything in the air until a slot frees up.
        self.sim.world.projectiles.spawn(
            oag_tables::weapons::Weapon::Cannon,
            position,
            velocity,
            slot as u8,
        );
    }

    /// One tick of the lock-on reticle, and the tone state that goes with it.
    ///
    /// **Recovered** from `HudSight_Update` (`0x0881dbcc`) - the law itself is
    /// in [`oag_race::sight`], and this is only what feeds it. See
    /// `docs/ghidra/functions/psp-pulse-usa/lock-sight.md`.
    ///
    /// # Which craft it points at
    ///
    /// The one `Ship_AcquireLock` would pick for the weapon the player is
    /// **holding**, recomputed every tick. The original runs that function per
    /// frame and stores the winner on the entity; running it here reaches the
    /// same answer without putting a lock on `World`, which would move a
    /// determinism hash for something only the screen reads.
    ///
    /// **Both lockable weapons drive it.** `Ship_AcquireLock` (`0x08844784`) is
    /// one function serving two: it takes the Missile's window from
    /// `stats+0x50`/`+0x54` and the LeachBeam's from `+0x114`/`+0x118`, and
    /// everything after that switch - the `0.9` cone, the along-track screen,
    /// the nearest-by-longitudinal-distance tie-break - is shared. So the two
    /// differ by two numbers, and this reaches them through
    /// `oag_gameplay::projectile::missile::lock_window`.
    ///
    /// The **art** differs too, and that is carried on the reticle rather than
    /// here: [`sight::Sight::set_held`] records which of the two is up, and the
    /// HUD draws the Missile's four brackets plus its inner box, or the
    /// LeachBeam's four arrowheads and no inner.
    ///
    /// # The gate is ours
    ///
    /// The original guards its projection with a condition this project has
    /// read and not understood - see the page's "The gate is the one part not
    /// read". This uses "the held weapon locks, and something is lockable",
    /// which is what the weapon plays like.
    pub(super) fn update_sight(&mut self) {
        if let Some(held) = self.sight_held() {
            self.view.sight.set_held(held);
        }
        let target = self.sight_target();
        let projected = target.and_then(|slot| {
            let world = self.sim.world.ships[slot as usize].physics.body.position;
            // **The title's own virtual screen, not the window's.** The original
            // projects into 480x272 because that *is* its screen; this engine
            // letterboxes the same rectangle into whatever the window is, so
            // the two agree exactly at the authored shape and drift a little as
            // the display aspect is taken away from it. **Ours**, and the
            // alternative - projecting at the window aspect - would put the
            // reticle off the craft on an authored-size capture, which is the
            // frame every comparison against the original is taken at.
            //
            // Off the reticle rather than off a constant, because Wipeout HD
            // authors its HUD in 1920x1080 and the PSP titles in 480x272.
            let screen = self.view.sight.screen();
            let aspect = screen[0] / screen[1];
            let view = self.view();
            // The far plane is irrelevant here - the reticle's own 250-unit
            // range test runs in eye space, before the projection - so this
            // takes a value large enough never to clip a craft the sight would
            // otherwise draw.
            let projection = self.projection(aspect, SIGHT_FAR, self.view.sight_fov);
            sight::project(view, projection * view, world, screen)
        });
        self.view.sight_state = self.view.sight.update(self.sim.dt, projected);
    }

    /// Which of the two lockable weapons the player is holding, or `None`.
    ///
    /// **`None` for a weapon this title's table does not author**, not only for
    /// one that cannot lock: Pure ships no `<Weapon type="LeachBeam">` and no
    /// `leachbeam_sight_*` widgets, so a LeachBeam there has no window to run
    /// and gets no reticle rather than borrowing the Missile's.
    pub(super) fn sight_held(&self) -> Option<sight::Held> {
        let stats = self.sim.weapons.as_ref()?;
        match self.sim.world.ships[0].pickup.weapon? {
            oag_tables::weapons::Weapon::Missile => stats.missile().map(|_| sight::Held::Missile),
            oag_tables::weapons::Weapon::LeachBeam => {
                stats.leach_beam().map(|_| sight::Held::LeachBeam)
            }
            _ => None,
        }
    }

    /// Which craft the reticle is over, or `None`.
    ///
    /// Split out so the choice can be asserted without a camera. Returns
    /// `None` whenever the player is holding something that does not lock,
    /// which is every weapon but the Missile and the LeachBeam.
    ///
    /// # The two arms differ by their window and by where they measure from
    ///
    /// The window is the original's own, off two pairs of `<Stats>` offsets.
    /// The **origin** is not: `Ship_AcquireLock` uses one weapon-independent
    /// origin for both branches, and this engine already moved the Missile's to
    /// the craft's *nose* - a deliberate deviation argued at
    /// [`Race::fire_missile`], where the missile actually starts. The LeachBeam
    /// has no recovered launch offset to move to, so its arm measures from the
    /// craft's own position. **Ours, and the absence of an invention rather
    /// than one**: borrowing the Missile's nose offset for a weapon with no
    /// projectile would be putting a number where the disc authors none. The
    /// two origins are a hull's length apart against a window that starts ten
    /// units out, so nothing visible turns on it.
    pub(super) fn sight_target(&self) -> Option<u8> {
        let held = self.sight_held()?;
        let stats = self.sim.weapons.as_ref()?;
        let count = self.sim.world.ship_count as usize;
        let ship = &self.sim.world.ships[0];
        let (origin, min, max) = match held {
            sight::Held::Missile => {
                let missile = stats.missile()?;
                // From the nose and along the craft's forward, the same two the
                // fire path takes the lock from - see [`Race::fire_missile`] on
                // why the nose and not the centre.
                let (origin, _, _) = oag_gameplay::projectile::missile::launch(
                    &ship.physics,
                    &ship.handling.dimensions,
                    &missile,
                    &self.sim.class,
                );
                (origin, missile.lock_min_dist, missile.lock_max_dist)
            }
            sight::Held::LeachBeam => {
                let leach = stats.leach_beam()?;
                (
                    ship.physics.body.position,
                    leach.lock_min_dist,
                    leach.lock_max_dist,
                )
            }
        };
        oag_gameplay::projectile::missile::lock_window(
            &self.sim.world.ships[..count],
            0,
            origin,
            ship.physics.body.forward(),
            min,
            max,
            self.sim.course.as_ref().map(oag_race::Course::length),
        )
    }

    /// Locks a target if there is one and puts one missile in the air for `slot`.
    ///
    /// Returns whether anything left the rail. **`false` now means the array was
    /// full and nothing else** - a missing lock is no longer a refusal, for the
    /// reason `oag_gameplay::projectile::missile::lock` records at length:
    /// `Ship_FireHeldWeapon` (`0x08844ae8`) fires on both arms of its lock test
    /// and passes a null target on the unlocked one. An unguided missile rides
    /// the floor, glances off walls and detonates on the recovered
    /// `SELF_DETONATE_SECONDS` timer.
    ///
    /// # The lock test is shared; the hold is not
    ///
    /// `oag_gameplay::projectile::missile::lock` - the window, cone and
    /// along-track screen `Ship_AcquireLock` (`0x08844784`) runs - is evaluated
    /// for every slot alike, so a missile's *candidate* target does not depend on
    /// who fired it. See "The player and an opponent use the same rule, on
    /// purpose" below.
    ///
    /// **Whether that candidate is used is a second, separate question, and it is
    /// not shared.** `Ship_FireHeldWeapon` reads the candidate only when
    /// `entity+0x860 & 1` is set, and that bit is written by `HudSight_Update`
    /// alone - the reticle, which this project has found no evidence runs for
    /// anything but the craft the HUD is drawn for. So the craft that owns the
    /// reticle (`slot == 0`) additionally needs [`RaceView::sight`] to say
    /// [`sight::Sight::locked`] - held on screen past [`sight::HOLD_SECONDS`] -
    /// before its candidate is passed through; every other slot's candidate is
    /// used as soon as `lock` finds one, exactly as before this gate landed.
    /// **Not recovered and not ported**: whether an opponent's craft has its own
    /// copy of that bit and something in the original ever sets it - if it does
    /// not, every AI-fired missile in the original is unguided regardless of
    /// `Ship_AcquireLock`'s answer, which would make this engine's opponents
    /// strictly better shots than the original's. See the thread's `## Open`.
    ///
    /// # The player and an opponent use the same rule, on purpose
    ///
    /// `oag_ai::Driver::wants_to_fire` decides whether an *opponent* pulls the
    /// trigger, and its five gates are about that decision - is there somebody
    /// ahead, is the road straight enough, has the trigger rolled. They are not
    /// the weapon's rule. `Ship_AcquireLock` is, and the original runs it for the
    /// player's craft; running it here for both means a missile's candidate
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
    pub fn fire_missile(&mut self, slot: usize, stats: &oag_tables::weapons::MissileStats) -> bool {
        let count = self.sim.world.ship_count as usize;
        let ship = &self.sim.world.ships[slot];
        let (position, velocity, launch_kmh) = oag_gameplay::projectile::missile::launch(
            &ship.physics,
            &ship.handling.dimensions,
            stats,
            &self.sim.class,
        );
        // The lock is taken from the **nose**, where the missile actually starts,
        // rather than from the craft's centre: the near bound of the authored
        // window is ten units and a hull is four long, so measuring from the
        // wrong end moves the boundary by most of a craft.
        let candidate = oag_gameplay::projectile::missile::lock(
            &self.sim.world.ships[..count],
            slot as u8,
            position,
            ship.physics.body.forward(),
            stats,
            self.sim.course.as_ref().map(oag_race::Course::length),
        );
        // Only the craft the reticle is drawn for needs the extra hold - see
        // "The lock test is shared; the hold is not" above. A press before the
        // brackets close still fires, same as an opponent's shot always does;
        // it just carries no candidate through.
        let target = if slot == 0 {
            candidate.filter(|_| self.view.sight.locked())
        } else {
            candidate
        };
        // `target` is passed through as it comes, `None` included: that is the
        // original's null pointer, and `Missile_Update` skips its whole guidance
        // block on it rather than treating it as an error.
        self.sim.world.projectiles.spawn_guided(
            oag_tables::weapons::Weapon::Missile,
            position,
            velocity,
            slot as u8,
            target,
            launch_kmh,
        )
    }
}
