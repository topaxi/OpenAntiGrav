//! One tick of flight for everything in the pool that flies: the surface
//! probe, the travel sweep, the bounce, the guidance and the reaping.
//!
//! This is [`Projectiles::advance`] and nothing else. It moved here from
//! `projectile.rs` on 2026-09-15 with no change to a line of its body, because
//! that file stood at 997 lines against the 1,000-line ratchet and the
//! Disruptor's arm was about to push it over. A submodule holding one method
//! of the parent's type is the same shape `mine::advance_laid` already has,
//! one level further out.

use super::{
    FALL_ACCELERATION, Impact, KMH_PER_UNIT_PER_SECOND, MAX_FLIGHT_SECONDS, MAX_PROJECTILES,
    Projectile, Projectiles, RIDE_HEIGHT, SURFACE_PROBE_LENGTH, SweepHit, TriggerRadii, cannon,
    disruptor, mine, missile, nearest_hit, plasma, rocket, shuriken,
};
use oag_core::math::Vec3;
use oag_physics::{Ray, Raycaster, Surface};
use oag_tables::weapons::Weapon;

impl Projectiles {
    /// Flies every projectile one tick and reports what stopped.
    ///
    /// The result is indexed by slot, so an entry is `Some` exactly where a
    /// projectile ended its flight this tick. A fixed-size array rather than a
    /// `Vec`, for the reason the module docs give.
    ///
    /// # How a tick is tested
    ///
    /// The swept segment from where the projectile was to where it is going,
    /// which is [`oag_physics::reset::contact`]'s shape and is here for the same
    /// reason: a rocket at a class's authored speed covers something like twenty
    /// units in a tick, so a point test would pass straight through a wall.
    ///
    /// Both a wall and a hull are tested, and **the nearer wins** - a rocket
    /// that would reach a craft only by passing through geometry hits the
    /// geometry. The owner's own hull is excluded outright: a straight-line shot
    /// cannot come back, so the exclusion costs nothing and removes the launch
    /// frame's self-hit without a grace period to tune.
    // Nine parameters: the Disruptor's table block plus the Plasma's, one per
    // weapon that flies at an authored speed. A parameter struct would be the
    // tidy answer and is deferred until a fourth weapon asks for one.
    #[allow(clippy::too_many_arguments)]
    pub fn advance<R: Raycaster + ?Sized>(
        &mut self,
        dt: f32,
        raycaster: &R,
        ships: &[crate::world::Ship],
        missile: Option<&oag_tables::weapons::MissileStats>,
        plasma: Option<&oag_tables::weapons::PlasmaStats>,
        disruptor: Option<&oag_tables::weapons::DisruptorStats>,
        trigger_radii: TriggerRadii,
        class: &str,
    ) -> [Option<Impact>; MAX_PROJECTILES] {
        let mut impacts = [None; MAX_PROJECTILES];
        // Looked up once rather than per bolt: it is a string match on the
        // class name, and every bolt in the air flies at the same one.
        let disruptor_kmh = disruptor.and_then(|stats| stats.speed_for_named(class));
        // Where every projectile started this tick, for the rocket-versus-laid
        // sweep after the loop - the original sweeps the rocket's *previous* to
        // *current* position, and the loop below overwrites the former.
        let starts: [Vec3; MAX_PROJECTILES] = std::array::from_fn(|i| self.slots[i].position);

        for (index, projectile) in self.slots.iter_mut().enumerate() {
            let Some(kind) = projectile.kind else {
                continue;
            };
            let guided = kind == Weapon::Missile;

            // **A charging bolt does not fly**, and the Plasma is the only
            // weapon that has one. `Plasmas_Update` (`0x0886b490`) branches on
            // the entity's own `+0x4c` flag: charging entities take
            // `Plasma_UpdateCharge` (`0x0885c170`) and the countdown at
            // `+0x50`, flying ones take `Plasma_Update`. Nothing else in the
            // tick reaches a charging bolt - no probe, no sweep, no hull test,
            // and no ageing, because the age at `+0x54` is `Plasma_Update`'s
            // to advance. See [`plasma::CHARGE_SECONDS`].
            if projectile.charge > 0.0 {
                // The order is the original's: reseat, *then* count down, so
                // the tick the hold ends still puts the bolt where the craft
                // is now. `Plasma_Launch` (`0x0885bf84`) re-reads the craft's
                // node matrix anyway, which is the same thing said twice.
                let ship = ships.get(projectile.owner as usize);
                let heading = ship.map(|ship| {
                    let (position, heading) =
                        plasma::muzzle(&ship.physics, &ship.handling.dimensions);
                    projectile.position = position;
                    projectile.velocity = heading * projectile.velocity.length();
                    heading
                });
                projectile.charge = (projectile.charge - dt).max(0.0);
                // **Release, on the same tick the countdown crosses zero -
                // `projectile.charge <= 0.0` gates this, not merely `plasma`
                // being `Some`.** `Plasmas_Update` (`0x0886b490`) takes
                // `Plasma_Launch` instead of `Plasma_Update` on this exact
                // tick alone, and `Plasma_Launch` re-reads the firing craft's
                // *current* velocity - not whatever it was moving at when the
                // press started the charge - `p->launch_kmh =
                // length(craft_velocity) * 3.6f + stats->launchspeed`
                // (`docs/ghidra/functions/psp-pulse-usa/plasma.md`'s "the
                // charge is real" section). Without the `charge <= 0.0` guard
                // this ran on *every* charging tick, recomputing the launch
                // speed off whatever the craft was doing at each one, which is
                // invisible against a craft holding a constant velocity
                // through the whole charge - `crates/game/tests/plasma_ground_truth.rs`'s
                // `a_plasma_fired_on_a_real_track_is_one_bolt_and_it_flies`,
                // fired one tick into an accelerating race, is what caught it.
                // `plasma` is `None` for a caller with no weapon table, in
                // which case the bolt keeps whatever magnitude it was
                // charging at, unchanged - the same "no table, no ramp"
                // fallback [`missile::speed_kmh`]'s caller below takes.
                if let (Some(ship), Some(heading), Some(stats)) = (ship, heading, plasma)
                    && projectile.charge <= 0.0
                {
                    let launch_kmh = ship.physics.body.linear_velocity.length()
                        * KMH_PER_UNIT_PER_SECOND
                        + stats.launch_speed;
                    projectile.launch_speed_kmh = launch_kmh;
                    projectile.velocity = heading * (launch_kmh / KMH_PER_UNIT_PER_SECOND);
                }
                continue;
            }

            // **The two rear weapons are the things here that do not fly**, so
            // they take none of what follows: no surface probe, no fall, no
            // sweep, no guidance. A mine or a bomb sits where it was laid,
            // counts its authored fuse down, and goes off when something comes
            // close enough or when the fuse runs out. See [`mine`], which
            // carries the split between the recovered half of that and the
            // read-of-an-authored-attribute half, and which holds both weapons
            // because they are one weapon in two sizes.
            if matches!(kind, Weapon::Mine | Weapon::Bomb) {
                impacts[index] = mine::advance_laid(projectile, kind, dt, ships, trigger_radii);
                if impacts[index].is_some() {
                    *projectile = Projectile::default();
                }
                continue;
            }

            // **The Disruptor flies, and still takes none of what follows**,
            // because its flight is read off its own function and differs
            // from the Rocket's in its order, its probe direction, its ride
            // height and its hit test - four differences [`disruptor`]'s
            // module docs list, and four `if kind ==` branches this function
            // does not need. A bolt that ages out is reaped silently, as a
            // Rocket is; the original spawns its wall explosion there, which
            // is presentation and is recorded on the evidence page.
            if kind == Weapon::Disruptor {
                impacts[index] =
                    disruptor::advance(projectile, dt, raycaster, ships, disruptor_kmh);
                if impacts[index].is_some() || projectile.lifetime <= 0.0 {
                    *projectile = Projectile::default();
                }
                continue;
            }

            // A missile's or a plasma bolt's speed is pinned to its ramp every
            // tick rather than integrated, so the whole flight needs to know
            // how old it is. Age is derived from the lifetime rather than
            // stored beside it: the two would be one number written twice, and
            // the second one is what goes wrong. For the Plasma this is age
            // *since release*, not since the press that started the charge -
            // the charging branch above returns early and never reaches this
            // line, so `lifetime` (and therefore `age`) does not move until
            // the bolt is actually flying.
            let age = MAX_FLIGHT_SECONDS - projectile.lifetime;
            // `None` where the file authors a speed *per* class and this
            // race's rung is outside them - the weapon then flies on its
            // integrated velocity rather than on a ramp borrowed from some
            // other rung. Unreachable on every measured disc: the only ladder
            // with a fifth rung is Pure's, and Pure authors one
            // class-independent speed per weapon.
            //
            // **The Plasma shares [`missile::speed_kmh`] rather than owning a
            // copy.** `Missile_SpeedNow` (`0x0885a038`) and
            // `Plasma_SpeedForClass` (`0x0885c5a4`) each independently test
            // `age < 1.0` and blend with the identical operand order -
            // `launch * (1 - age) + class * age` - which is the same bar this
            // project already used to fold the Rocket's, the Missile's and the
            // Shuriken's `12.0` surface probe into one constant: two
            // functions, read separately, agreeing is worth more than either
            // alone. See [`missile::SPEED_RAMP_SECONDS`]'s own doc comment,
            // which now cites both addresses.
            //
            // **No launch floor for the Plasma.** `Plasma_Launch`'s listing
            // has no `vmax_s` guarding a minimum - that clamp is
            // `Missile_Init`'s own ([`missile::LAUNCH_SPEED_FLOOR_KMH`]), read
            // off a different function, and is not carried over here.
            let pinned_kmh = if guided {
                missile.and_then(|stats| {
                    Some(missile::speed_kmh(
                        projectile.launch_speed_kmh,
                        stats.speed_for_named(class)?,
                        age,
                    ))
                })
            } else if kind == Weapon::Plasma {
                plasma.and_then(|stats| {
                    Some(missile::speed_kmh(
                        projectile.launch_speed_kmh,
                        stats.speed_for_named(class)?,
                        age,
                    ))
                })
            } else if kind == Weapon::Rocket && projectile.launch_speed_kmh > 0.0 {
                // **The Rocket leaves slower than it cruises and is pinned on its
                // first surface hit** - `Rocket_Update` renormalises to
                // `SpeedForClass / 3.6` on the probe-hit arm and only there, so
                // this feeds `speed_units_on_surface` below and nothing else (a
                // Rocket has no target to steer toward). The class speed rides
                // in `launch_speed_kmh`; see [`rocket::LAUNCH_SPEED_SCALE`].
                Some(projectile.launch_speed_kmh)
            } else {
                None
            };

            let from = projectile.position;
            let mut to = from + projectile.velocity * dt;

            // **The surface probe, before the flight sweep** - the order is the
            // original's. Look along the normal being ridden for something to
            // ride; conform to it if it is floor-like, fall if there is nothing.
            //
            // **The Rocket is the odd one out, and it is the only one that
            // looks 6.0.** `Rocket_Update` scales its stored normal by `6.0`;
            // `Missile_Update`, `Plasma_Update` (`0x0885c6cc`) and
            // `Shuriken_Update` (`0x08877bdc`) all scale theirs by `12.0`, each
            // read off its own function. Three weapons agreeing is what turns
            // this from "the Missile is special" into "the Rocket is".
            //
            // **The Plasma read 6.0 here until 2026-09-02**, which was a port
            // bug rather than a reading: it shipped the same day the weapon did
            // and was caught by reading `Shuriken_Update` and noticing the
            // constant a third time.
            let probe_length = if matches!(kind, Weapon::Rocket) {
                SURFACE_PROBE_LENGTH
            } else {
                missile::SURFACE_PROBE_LENGTH
            };
            let probe = Raycaster::raycast(
                raycaster,
                Ray::new(to, -projectile.surface, probe_length),
                None,
                false,
            );
            // **The branch is on the surface class, which is the original's
            // collision code.** `Collision_SweepSegment` (`0x0883198c`)
            // returns the struck collider's surface type - `0` wall, `1`
            // floor, `3` mag floor - or `0x7f` for nothing. `Rocket_Update`
            // (`0x0885d2a8`), read at decompiler level 2026-09-13: `0x7f`
            // falls, `0` and `4` detonate, anything else rides.
            // `Missile_Update` (`0x0885a918`) and `Shuriken_Update`
            // (`0x08877bdc`) take `0`/`4` as *nothing* here - no fall, no
            // ride - and `Plasma_Update` (`0x0885c6cc`) detonates like the
            // Rocket. This branched on the hit normal's angle until
            // 2026-09-13; see [`nearest_hit`] for the sweep half of that bug.
            match probe {
                // A floor: sit at the ride height above it, adopt its normal,
                // and turn the velocity parallel to it without changing speed.
                // Turning rather than reflecting is what makes a projectile
                // *follow* a rolling track instead of bouncing down it.
                Some(hit) if hit.surface.is_hoverable() => {
                    projectile.surface = hit.normal;
                    to = hit.point + hit.normal * RIDE_HEIGHT;
                    // A missile or a plasma bolt re-pins its speed to the ramp
                    // here rather than preserving what it had -
                    // `Missile_Update` and `Plasma_Update` (`0x0885c6cc`,
                    // `default:` arm, `plasma.md`) both normalise and rescale
                    // on this exact branch, and by a **divide** by 3.6 where
                    // the Missile's own guidance path multiplies by a bit
                    // pattern that is not quite 1/3.6. Both roundings are the
                    // original's. **This is also the only branch that re-pins
                    // a Plasma's speed** - `Plasma_Update`'s `0x7f` (no floor)
                    // arm does not rescale, so a bolt fired over a gap keeps
                    // its launch speed, unchanged, until the track comes back
                    // under it. A Shuriken never reaches a `Some` here:
                    // `pinned_kmh` is `None`, so it falls to
                    // `projectile.velocity.length()`, same as always. **A
                    // Rocket does, since 2026-10-01**: its class speed, which
                    // is the step from its 0.75 launch to the cruise.
                    let speed = pinned_kmh.map_or_else(
                        || projectile.velocity.length(),
                        missile::speed_units_on_surface,
                    );
                    let along =
                        projectile.velocity - hit.normal * projectile.velocity.dot(hit.normal);
                    // A projectile aimed straight at the floor has nothing left
                    // after the normal component is removed; keep its heading
                    // rather than zeroing it and let the sweep below resolve it.
                    if along.length_squared() > 1e-6 {
                        projectile.velocity = along.normalize() * speed;
                    }
                }
                // A wall within reach below: the Rocket and the Plasma go off
                // on it, the two bouncing weapons ignore it for this tick.
                // `Rocket_Update` falls through to its travel sweep after this
                // and can spawn a second `WO_ROCKET_EXPLO_TRACK`. **Neither
                // spends a blast on a wall.** The Plasma's craft sweep
                // (`Plasma_SweepCraftHit`, `0x0886afb8`) and the Rocket's
                // (`Rocket_SweepCraftHit`, `0x0886e7ac`, read 2026-09-16)
                // are separate per-tick hull sweeps that never reach this
                // probe, so a wall found here can only be a wall - and both
                // pools' wall teardowns play their `*HITWALL`/`ROCKEXPLWALL`
                // cue, release the trail and reap, touching no craft's
                // damage, slowdown or `entity+0x110` impulse. See
                // `docs/ghidra/functions/psp-pulse-usa/plasma.md`'s "the
                // expiry settles a damage question" section and
                // `rocket-visuals.md`'s "what a rocket hit spends" section.
                Some(hit) => {
                    if matches!(kind, Weapon::Rocket | Weapon::Plasma) {
                        impacts[index] = Some(Impact {
                            point: hit.point,
                            kind,
                            owner: projectile.owner,
                            struck: None,
                            blast: false,
                            effect: None,
                        });
                        *projectile = Projectile::default();
                        continue;
                    }
                }
                // Nothing under it: it falls. The projectile keeps whatever
                // normal it had, so it resumes riding when the track comes
                // back under it.
                None => projectile.velocity -= Vec3::Y * FALL_ACCELERATION * dt,
            }

            let step = to - from;
            let distance = step.length();

            // A projectile with no velocity cannot hit anything by moving, and
            // normalising a zero step would produce a NaN direction that the
            // raycaster would then chase. It still ages out below.
            let hit = if distance > 0.0 {
                nearest_hit(from, to, distance, raycaster, ships, projectile.owner)
            } else {
                None
            };

            let mut bounced = false;
            match hit {
                // **A floor across the step is ridden, not struck.** The
                // original's travel-segment switch takes a floor code to the
                // same push-out a probe hit takes - `hit + normal * 3.0` - for
                // every one of the four weapons; only a wall or a craft ends
                // a flight. This is what carries a projectile over a crest
                // and down into the dip behind it: the probe has lost the
                // floor, the chord meets it steeply, and the answer is to
                // land on it. Until 2026-09-13 that chord detonated.
                //
                // **The Missile only moves; the rest also turn.** `Missile_Update`'s
                // floor arm here is one `hit + normal * g_ride_height` and
                // nothing else - no normal adopted, no velocity written; its
                // guidance rewrites the velocity a few lines later anyway.
                // `Rocket_Update` and `Shuriken_Update` adopt the normal and
                // write `(next - prev) / dt`, then let the next tick's probe
                // rescale it. This engine has no per-tick rescale for the
                // Rocket, so the velocity is turned parallel with its speed
                // kept, as the probe branch does - a choice, recorded as one.
                Some(hit)
                    if hit.struck.is_none() && hit.surface.is_some_and(Surface::is_hoverable) =>
                {
                    projectile.position = hit.point + hit.normal * RIDE_HEIGHT;
                    if !guided {
                        projectile.surface = hit.normal;
                        let along =
                            projectile.velocity - hit.normal * projectile.velocity.dot(hit.normal);
                        if along.length_squared() > 1e-6 {
                            projectile.velocity = along.normalize() * projectile.velocity.length();
                        }
                    }
                }
                Some(hit) => {
                    let SweepHit {
                        point,
                        struck,
                        normal,
                        ..
                    } = hit;
                    // **A missile glances off a wall; a rocket dies on it.** The
                    // original's missile counts wall hits at `self+0x6c`, mirrors its
                    // velocity about the hit normal with no restitution loss, pushes
                    // out along that normal, and only takes the detonating branch
                    // once the count reaches `MAX_BOUNCES`. A hull hit is a different
                    // collision code and always detonates, which is why this arm asks
                    // for `struck.is_none()`.
                    //
                    // **A Shuriken bounces too, by the same law and without a
                    // budget.** `Shuriken_Update`'s travel-segment test calls
                    // `Shuriken_Bounce` (`0x088778ac`) on the branch a rocket dies
                    // on, and that function is `v - 2(v.n)n` with no damping term
                    // anywhere in it - so a blade keeps its speed for ever and what
                    // ends it is its own `fuse`, not a count. Its push-off is its
                    // own literal; see [`shuriken::BOUNCE_PUSH_OFF`].
                    //
                    // `bounces` is still counted for it, because the visual side
                    // reads that counter to know when to play a bounce effect -
                    // nothing gates flight on the number.
                    let push_off = match kind {
                        Weapon::Shuriken => shuriken::BOUNCE_PUSH_OFF,
                        _ => missile::BOUNCE_PUSH_OFF,
                    };
                    let may_bounce = struck.is_none()
                        && match kind {
                            Weapon::Missile => projectile.bounces < missile::MAX_BOUNCES,
                            Weapon::Shuriken => true,
                            _ => false,
                        };
                    if may_bounce {
                        projectile.bounces = projectile.bounces.saturating_add(1);
                        projectile.velocity -= normal * (2.0 * projectile.velocity.dot(normal));
                        projectile.position = point + normal * push_off;
                        bounced = true;
                    } else {
                        // **The Plasma and the Rocket split on `struck`
                        // here.** A wall found by this travel sweep
                        // (`struck: None`) is the same "no blast" reading the
                        // probe branch above and the age timeout both carry.
                        // A craft found here (`struck: Some(_)`) is the pool's
                        // own hull-cylinder sweep's territory -
                        // `Plasma_SweepCraftHit` (`0x0886afb8`) and
                        // `Rocket_SweepCraftHit` (`0x0886e7ac`), both read
                        // 2026-09-16 - which run independently of this raycast
                        // every tick and on a hit call the direct credit and
                        // the force sweep (`Plasma_HitCraft`/
                        // `Plasma_ApplyBlastForce`, `Rocket_HitCraft`/
                        // `Rocket_ApplyBlastForce`). `blast::apply_impacts`
                        // routes either weapon's impact with a `struck` craft
                        // to [`super::blast::blast_direct_hit`] rather than
                        // to [`super::blast::blast`] - see that function's
                        // own doc comment for the credited shape. Missile and
                        // Shuriken, once their bounce budget is spent, are
                        // unchanged and keep `blast: true` unconditionally.
                        impacts[index] = Some(Impact {
                            point,
                            kind,
                            owner: projectile.owner,
                            struck,
                            blast: !matches!(kind, Weapon::Plasma | Weapon::Rocket)
                                || struck.is_some(),
                            effect: None,
                        });
                        *projectile = Projectile::default();
                        continue;
                    }
                }
                None => projectile.position = to,
            }

            // **Guidance runs last, writes only the velocity, and reads `from`.**
            // All three are the original's, and the third is the one that looks
            // wrong: `Missile_Update` steers off the position the missile had at
            // the *start* of the tick, after the move is already committed, so a
            // correction takes effect on the following tick. It is also skipped
            // outright on the tick a missile bounces.
            if let (Some(speed_kmh), Some(target), false) = (pinned_kmh, projectile.target, bounced)
                && let Some(ship) = ships.get(target as usize).filter(|s| s.active)
            {
                projectile.velocity = missile::steer(
                    projectile.velocity,
                    from,
                    ship.physics.body.position,
                    dt,
                    missile::speed_units_guided(speed_kmh),
                );
            }

            projectile.lifetime -= dt;

            // **A missile that hit nothing goes off where it is.** Recovered
            // from the pool's second pass, which tests `3.0 < age` on every live
            // slot and sets the same destroy bit a wall or a craft sets - see
            // [`missile::SELF_DETONATE_SECONDS`]. Tested after the move and
            // against the age at the *end* of the tick, because that is where
            // the original tests it: `Missile_Update` adds `dt` at the top of
            // its own body and the pool's pass runs after it. Strictly greater,
            // as `3.0 < age` is. `blast: false` is the recovered half that is
            // easy to miss - see [`Impact::blast`].
            if guided && MAX_FLIGHT_SECONDS - projectile.lifetime > missile::SELF_DETONATE_SECONDS {
                impacts[index] = Some(Impact {
                    point: projectile.position,
                    kind,
                    owner: projectile.owner,
                    struck: None,
                    blast: false,
                    effect: None,
                });
                *projectile = Projectile::default();
                continue;
            }

            // **A rocket that hit nothing is reaped at five seconds, and a
            // Cannon round at one, both silently.** `RocketPool_Update`'s
            // second pass tests `5.0 < age` and `CannonPool_Update`'s tests
            // `1.0 < age` (an `||` on its own gate), and each retires the
            // round through the same teardown a wall hit takes - trail
            // released, no explosion, no blast - so there is no impact to
            // report. See [`rocket::LIFETIME_SECONDS`] and
            // [`cannon::LIFETIME_SECONDS`]. Tested after the move like the
            // Missile's own timer above, because each pool's pass runs after
            // its per-round update.
            let own_lifetime = match kind {
                Weapon::Rocket => Some(rocket::LIFETIME_SECONDS),
                Weapon::Cannon => Some(cannon::LIFETIME_SECONDS),
                _ => None,
            };
            if own_lifetime.is_some_and(|limit| MAX_FLIGHT_SECONDS - projectile.lifetime > limit) {
                *projectile = Projectile::default();
                continue;
            }

            if projectile.lifetime <= 0.0 {
                // **The Plasma alone detonates here, and it is a recovered
                // negative rather than a recovered blast.** `Plasmas_Update`
                // (`0x0886b490`) sets the same destroy bit at `10.0 < age`
                // that a wall does, and both feed the *identical* teardown
                // pass - `Psys_Release_q`, `Plasma_SpawnDetonation`
                // (`0x0886ac88`, `WO_PLASMA_FLASH`), `PLASMAHITWALL` - with no
                // branch on which one fired. Read at instruction level,
                // 2026-09-16: that teardown calls nothing that touches a
                // craft's shield or `entity+0x110` (the pending-impulse slot
                // every real blast writes, confirmed against
                // `Missile_ApplyBlastForce`'s own write there), and
                // `Weapon_PostBlastImpulse` (`0x0886794c`) - the only function
                // in the binary that spends `damage`/`blastradius`/`blastforce`
                // - has exactly one caller in the whole executable
                // (`FUN_08867b50`, the Mine's own chain; `get_xrefs_to`
                // confirmed it). No `Plasma_ApplyBlast`-shaped function exists
                // anywhere in the binary either. So a Plasma bolt spends its
                // blast on **neither** ending - `blast: false` here matches
                // the wall-hit branch above in every way but that one, which
                // this port does not touch; see the doc comment on
                // `Impact::blast` and `docs/ghidra/functions/psp-pulse-usa/plasma.md`'s
                // "What is not verified" for the flagged tension with that
                // branch's own `blast: true`, carried forward rather than
                // fixed here.
                //
                // **The Shuriken's fuse is the same shape, read 2026-09-30.**
                // `ShurikenPool_Update` (`0x0886ff38`) sets the destroy bit at
                // `fuse < age` and its teardown, `FUN_08870c78`, plays
                // `WO_SHURIKEN_EXPIRE` and starts `ScreenFlash_Start(0)` at the
                // blade. It calls nothing that spends damage, so the ending is
                // `blast: false`, like the Plasma's.
                if matches!(kind, Weapon::Plasma | Weapon::Shuriken) {
                    impacts[index] = Some(Impact {
                        point: projectile.position,
                        kind,
                        owner: projectile.owner,
                        struck: None,
                        blast: false,
                        effect: None,
                    });
                    *projectile = Projectile::default();
                    continue;
                }

                // Reaped, not detonated: nothing was struck, so nothing takes a
                // blast. A projectile that leaves the world simply stops
                // existing.
                //
                // **The cap itself is ours; that there is one is not.** The
                // Rocket's pool (`RocketPool_Update`) takes its own destroy
                // branch on `5.0 < age` and reaches no explosion spawner - the
                // original reaps a stale rocket silently, exactly as this
                // does, and since 2026-09-16 the Rocket takes that branch
                // above at [`rocket::LIFETIME_SECONDS`] and never reaches
                // this one. The Cannon's `1.0` follows it there. A missile
                // never gets here either, having detonated above.
                //
                // **A mine or a bomb never gets here either**, for a different
                // reason: its countdown is the disc's own `timetodie` and
                // running out is a *detonation*, handled in [`advance_laid`]
                // rather than folded into this branch. That is the one place in
                // this module where the same field means two things depending
                // on the weapon, and it is spelled out at both ends.
                *projectile = Projectile::default();
            }
        }

        self.sweep_rockets_through_laid(&starts, trigger_radii, &mut impacts);
        impacts
    }

    /// A rocket that flies through a laid mine or bomb sets it off and is
    /// spent doing so - quietly on both sides.
    ///
    /// **Recovered.** `Rocket_SweepProjectiles` (`0x0886f154`), the third call
    /// in `RocketPool_Update`'s per-rocket loop, tests the rocket's previous-to-
    /// current segment against every live mine (within the Mine's own
    /// `trigger_radius`) and every live bomb (the Bomb's), and raises the
    /// destroy bit on both the rocket and whatever it met. Neither pool's
    /// teardown then spends anything: the mine or bomb plays its explosion
    /// (`MinePool_Update`'s second pass, `Bomb_Detonate` with no victim), the
    /// rocket releases its trail without a cue. See
    /// `docs/ghidra/functions/psp-pulse-usa/rocket-visuals.md`, "What a rocket
    /// hit spends", and `mine.md`'s 2026-09-16 section. A rocket that already
    /// ended this tick (its slot is free) is not swept.
    fn sweep_rockets_through_laid(
        &mut self,
        starts: &[Vec3; MAX_PROJECTILES],
        trigger_radii: TriggerRadii,
        impacts: &mut [Option<Impact>; MAX_PROJECTILES],
    ) {
        for rocket in 0..MAX_PROJECTILES {
            if self.slots[rocket].kind != Some(Weapon::Rocket) || impacts[rocket].is_some() {
                continue;
            }
            let from = starts[rocket];
            let to = self.slots[rocket].position;
            let met = self.slots.iter().enumerate().find_map(|(laid, slot)| {
                let kind = slot.kind?;
                if !matches!(kind, Weapon::Mine | Weapon::Bomb) || impacts[laid].is_some() {
                    return None;
                }
                let radius = trigger_radii.get(kind)?;
                super::geometry::segment_sphere(from, to, slot.position, radius)
                    .map(|_| (laid, kind, slot.position, slot.owner))
            });
            if let Some((laid, kind, centre, owner)) = met {
                impacts[laid] = Some(Impact {
                    point: centre,
                    kind,
                    owner,
                    struck: None,
                    blast: false,
                    effect: None,
                });
                self.slots[laid] = Projectile::default();
                self.slots[rocket] = Projectile::default();
            }
        }
    }
}
