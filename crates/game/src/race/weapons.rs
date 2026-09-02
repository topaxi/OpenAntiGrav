//! Pickups and what they fire: the free turbo, the one-slot inventory, and
//! the lock-on sight.
//!
//! Split out of `race.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change. Its tests are
//! `race/tests/weapons.rs`. What a fired weapon then *shows* - the blasts, the
//! flares, the sprites - is [`visuals`], split out under the same rule the day
//! the Mine's and the Missile's own effects were wired.

use super::*;

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
                oag_formats::weapons::Weapon::Plasma => {
                    let Some(stats) = weapons.plasma() else {
                        // As the Rocket: nothing to put in the air.
                        return;
                    };
                    let ship = &self.world.ships[0];
                    // **One bolt, not three.** `Weapon_FirePlasma`
                    // (`0x0886a868`) spawns once and clears its own request
                    // bit; the `<Stats>` carry no `spread` to fan a volley
                    // with. See `oag_gameplay::projectile::plasma::launch`.
                    let (position, velocity) = oag_gameplay::projectile::plasma::launch(
                        &ship.physics,
                        &ship.handling.dimensions,
                        &stats,
                        to_format_class(self.class),
                    );
                    if !self.world.projectiles.spawn(
                        oag_formats::weapons::Weapon::Plasma,
                        position,
                        velocity,
                        0,
                    ) {
                        // Every slot was taken. Keep the pickup rather than
                        // spend it on a shot that never left - the same rule
                        // the Rocket's arm follows for an empty volley.
                        return;
                    }
                }
                oag_formats::weapons::Weapon::Shuriken => {
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
                    if self.world.projectiles.live() >= oag_gameplay::projectile::MAX_PROJECTILES {
                        return;
                    }
                    // Copied out before the draw because `launch` borrows the
                    // world's generator mutably and the ship state immutably at
                    // once; both are small `Copy` structs.
                    let physics = self.world.ships[0].physics;
                    let dimensions = self.world.ships[0].handling.dimensions;
                    // **One blade, twenty degrees off the nose, side chosen by
                    // a coin.** See `oag_gameplay::projectile::shuriken::launch`
                    // and `docs/ghidra/functions/psp-pulse-usa/shuriken.md`.
                    let (position, velocity) = oag_gameplay::projectile::shuriken::launch(
                        &physics,
                        &dimensions,
                        &stats,
                        to_format_class(self.class),
                        &mut self.world.rng,
                    );
                    if !self
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
                oag_formats::weapons::Weapon::Missile => {
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
                oag_formats::weapons::Weapon::Mine | oag_formats::weapons::Weapon::Bomb => {
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
                    self.world.ships[0].pickup.begin_drop(drop.count);
                    return;
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
        let Some(weapons) = self.weapons.as_ref() else {
            // No table. A craft cannot have started a drop without one - the arm
            // above gates on exactly this - so there is nothing in flight to
            // abandon.
            return;
        };
        for slot in 0..self.world.ship_count as usize {
            if !self.world.ships[slot].active || !self.world.ships[slot].pickup.is_dropping() {
                continue;
            }
            // **The held weapon, re-read every tick, and it is what says which
            // of the two is coming out.** A drop keeps the pickup in the slot
            // until the last charge is laid - that is the recovered coupling -
            // so the weapon is still there to be asked.
            let Some(weapon) = self.world.ships[slot].pickup.weapon else {
                continue;
            };
            let Some(drop) = oag_gameplay::projectile::mine::Drop::for_weapon(weapon, weapons)
            else {
                continue;
            };
            let due = self.world.ships[slot]
                .pickup
                .advance_drop(self.dt, oag_gameplay::projectile::mine::DROP_INTERVAL);
            if !due {
                continue;
            }
            let ship = &self.world.ships[slot];
            let point = oag_gameplay::projectile::mine::drop_point(
                &ship.physics,
                &ship.handling.dimensions,
            );
            // A full array drops this mine and not the drop: the counter has
            // already been spent, so the cluster goes on laying the rest rather
            // than stalling. That matches `Projectiles::spawn`'s own rule -
            // a shot that cannot be taken is lost, not queued.
            self.world
                .projectiles
                .lay(weapon, point, slot as u8, drop.fuse);
        }
    }

    /// One tick of the lock-on reticle, and the tone state that goes with it.
    ///
    /// **Recovered** from `HudSight_Update` (`0x0881dbcc`) - the law itself is
    /// in [`crate::race::sight`], and this is only what feeds it. See
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
    /// **Only the Missile drives it today**, and the LeachBeam is the reason
    /// that is worth stating rather than assuming: it locks too - it is the
    /// other weapon `Ship_AcquireLock` reads distances for, at `stats+0x114`
    /// and `+0x118`, and it has its own four `leachbeam_sight_*` widgets over a
    /// hollow-triangle model. What stops it here is that
    /// `oag_formats::weapons` does not parse a `<Weapon type="LeachBeam">`
    /// block at all, so there are no distances to run the window against.
    /// Wiring it is parsing that block and adding an arm here, not new geometry.
    ///
    /// # The gate is ours
    ///
    /// The original guards its projection with a condition this project has
    /// read and not understood - see the page's "The gate is the one part not
    /// read". This uses "the held weapon locks, and something is lockable",
    /// which is what the weapon plays like.
    pub(super) fn update_sight(&mut self) {
        let target = self.sight_target();
        let projected = target.and_then(|slot| {
            let world = self.world.ships[slot as usize].physics.body.position;
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
            let screen = self.sight.screen();
            let aspect = screen[0] / screen[1];
            let view = self.view();
            // The far plane is irrelevant here - the reticle's own 250-unit
            // range test runs in eye space, before the projection - so this
            // takes a value large enough never to clip a craft the sight would
            // otherwise draw.
            let projection = self.projection(aspect, SIGHT_FAR, self.sight_fov);
            sight::project(view, projection * view, world, screen)
        });
        self.sight_state = self.sight.update(self.dt, projected);
    }

    /// Which craft the reticle is over, or `None`.
    ///
    /// Split out so the choice can be asserted without a camera. Returns
    /// `None` whenever the player is holding something that does not lock,
    /// which is every weapon but the Missile and the LeachBeam.
    pub(super) fn sight_target(&self) -> Option<u8> {
        if self.world.ships[0].pickup.weapon != Some(oag_formats::weapons::Weapon::Missile) {
            return None;
        }
        let stats = self
            .weapons
            .as_ref()
            .and_then(oag_formats::weapons::WeaponStats::missile)?;
        let count = self.world.ship_count as usize;
        let ship = &self.world.ships[0];
        // From the nose and along the craft's forward, the same two the fire
        // path takes the lock from - see [`Race::fire_missile`] on why the nose
        // and not the centre.
        let (origin, _, _) = oag_gameplay::projectile::missile::launch(
            &ship.physics,
            &ship.handling.dimensions,
            &stats,
            to_format_class(self.class),
        );
        oag_gameplay::projectile::missile::lock(
            &self.world.ships[..count],
            0,
            origin,
            ship.physics.body.forward(),
            &stats,
            self.course.as_ref().map(oag_race::Course::length),
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
        // `target` is passed through as it comes, `None` included: that is the
        // original's null pointer, and `Missile_Update` skips its whole guidance
        // block on it rather than treating it as an error.
        self.world.projectiles.spawn_guided(
            oag_formats::weapons::Weapon::Missile,
            position,
            velocity,
            slot as u8,
            target,
            launch_kmh,
        )
    }
}
