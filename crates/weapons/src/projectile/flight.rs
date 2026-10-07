//! One tick of flight for everything in the pool that flies: the surface probe,
//! the travel sweep, the bounce, the guidance and the reaping.
//!
//! This is [`Projectiles::advance`] alone, split from `projectile.rs` to stay
//! under the 1,000-line ratchet.

use super::{
    FALL_ACCELERATION, Impact, KMH_PER_UNIT_PER_SECOND, MAX_FLIGHT_SECONDS, MAX_PROJECTILES,
    Projectile, Projectiles, RIDE_HEIGHT, SURFACE_PROBE_LENGTH, SweepHit, TriggerRadii, cannon,
    disruptor, mine, missile, nearest_hit, plasma, rocket, shuriken,
};
use crate::Craft;
use oag_core::math::Vec3;
use oag_physics::{Ray, Raycaster, Surface};
use oag_tables::weapons::Weapon;

impl Projectiles {
    /// Flies every projectile one tick and reports what stopped.
    ///
    /// The result is indexed by slot: `Some` where a projectile ended this tick.
    ///
    /// Each tick tests the swept segment, as [`oag_physics::reset::contact`] does:
    /// a rocket covers about twenty units a tick, so a point test would pass
    /// through a wall. Wall and hull are both tested and the nearer wins. The
    /// owner's own hull is excluded outright (a straight shot cannot come back),
    /// which removes the launch frame's self-hit without a grace period.
    // Nine parameters, one table block per weapon that flies at an authored speed;
    // a parameter struct is deferred until a fourth weapon asks for one.
    #[allow(clippy::too_many_arguments)]
    pub fn advance<R: Raycaster + ?Sized, S: Craft>(
        &mut self,
        dt: f32,
        raycaster: &R,
        ships: &[S],
        missile: Option<&oag_tables::weapons::MissileStats>,
        plasma: Option<&oag_tables::weapons::PlasmaStats>,
        disruptor: Option<&oag_tables::weapons::DisruptorStats>,
        trigger_radii: TriggerRadii,
        class: &str,
    ) -> [Option<Impact>; MAX_PROJECTILES] {
        let mut impacts = [None; MAX_PROJECTILES];
        // Looked up once: a string match on the class name, the same for every bolt.
        let disruptor_kmh = disruptor.and_then(|stats| stats.speed_for_named(class));
        // Where each projectile started this tick, for the rocket-versus-laid sweep
        // after the loop (the loop overwrites the position).
        let starts: [Vec3; MAX_PROJECTILES] = std::array::from_fn(|i| self.slots[i].position);

        for (index, projectile) in self.slots.iter_mut().enumerate() {
            let Some(kind) = projectile.kind else {
                continue;
            };
            let guided = kind == Weapon::Missile;

            // A charging bolt does not fly (Plasma only). `Plasmas_Update`
            // (`0x0886b490`) branches on `+0x4c`: charging entities take
            // `Plasma_UpdateCharge` (`0x0885c170`) and the countdown at `+0x50`.
            // No probe, sweep, hull test or ageing reaches it. See
            // [`plasma::CHARGE_SECONDS`].
            if projectile.charge > 0.0 {
                // Reseat, then count down, so the tick the hold ends still puts
                // the bolt where the craft is now.
                let ship = ships.get(projectile.owner as usize);
                let heading = ship.map(|ship| {
                    let (position, heading) = plasma::muzzle(ship.physics(), &ship.dimensions());
                    projectile.position = position;
                    projectile.velocity = heading * projectile.velocity.length();
                    heading
                });
                projectile.charge = (projectile.charge - dt).max(0.0);
                // Release on the tick the countdown crosses zero: `Plasmas_Update`
                // takes `Plasma_Launch` instead of `Plasma_Update` on that tick
                // alone, and it re-reads the craft's current velocity,
                // `p->launch_kmh = length(craft_velocity) * 3.6f +
                // stats->launchspeed` (`docs/ghidra/functions/psp-pulse-usa/plasma.md`,
                // "the charge is real"). Without the `charge <= 0.0` guard the
                // launch speed was recomputed every charging tick, which
                // `crates/game/tests/plasma_ground_truth.rs`'s
                // `a_plasma_fired_on_a_real_track_is_one_bolt_and_it_flies` caught.
                // `plasma` `None` (no weapon table) keeps the charging magnitude.
                if let (Some(ship), Some(heading), Some(stats)) = (ship, heading, plasma)
                    && projectile.charge <= 0.0
                {
                    let launch_kmh = ship.physics().body.linear_velocity.length()
                        * KMH_PER_UNIT_PER_SECOND
                        + stats.launch_speed;
                    projectile.launch_speed_kmh = launch_kmh;
                    // `Plasma_Launch` writes `-craft+0xb10` as the ridden normal.
                    projectile.surface = ship.physics().body.up();
                    projectile.velocity = heading * (launch_kmh / KMH_PER_UNIT_PER_SECOND);
                }
                continue;
            }

            // A mine or bomb does not fly: no probe, fall, sweep or guidance. It
            // sits, counts its authored fuse down and goes off on proximity or
            // expiry. See [`mine`].
            if matches!(kind, Weapon::Mine | Weapon::Bomb) {
                impacts[index] = mine::advance_laid(projectile, kind, dt, ships, trigger_radii);
                if impacts[index].is_some() {
                    *projectile = Projectile::default();
                }
                continue;
            }

            // The Disruptor flies on its own function, differing from the Rocket
            // in order, probe direction, ride height and hit test (see
            // [`disruptor`]). An aged-out bolt is reaped silently; the original's
            // wall explosion is presentation, noted on the evidence page.
            if kind == Weapon::Disruptor {
                impacts[index] =
                    disruptor::advance(projectile, dt, raycaster, ships, disruptor_kmh);
                if impacts[index].is_some() || projectile.lifetime <= 0.0 {
                    *projectile = Projectile::default();
                }
                continue;
            }

            // A missile's or plasma bolt's speed is pinned to its ramp each tick,
            // so the flight needs its age, derived from the lifetime (for the
            // Plasma, age since release: the charging branch returns first).
            let age = MAX_FLIGHT_SECONDS - projectile.lifetime;
            // `None` where the file authors a speed per class and this race's rung
            // is outside them: the weapon flies on its integrated velocity.
            // Unreachable on every measured disc.
            //
            // The Plasma shares [`missile::speed_kmh`]: `Missile_SpeedNow`
            // (`0x0885a038`) and `Plasma_SpeedForClass` (`0x0885c5a4`) read
            // separately agree (see [`missile::SPEED_RAMP_SECONDS`]). No launch
            // floor for the Plasma: that `vmax_s` is `Missile_Init`'s.
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
                // The Rocket leaves slower than it cruises and is pinned on its
                // first surface hit: `Rocket_Update` renormalises to
                // `SpeedForClass / 3.6` on the probe-hit arm only, so this feeds
                // `speed_units_on_surface` below. The class speed rides in
                // `launch_speed_kmh`; see [`rocket::LAUNCH_SPEED_SCALE`].
                Some(projectile.launch_speed_kmh)
            } else {
                None
            };

            let from = projectile.position;
            let mut to = from + projectile.velocity * dt;

            // The surface probe runs before the flight sweep, as in the original:
            // look along the ridden normal; conform if floor-like, fall if
            // nothing. Only the Rocket probes 6.0: `Rocket_Update` scales by
            // `6.0`; `Missile_Update`, `Plasma_Update` (`0x0885c6cc`) and
            // `Shuriken_Update` (`0x08877bdc`) by `12.0`, each read off its own
            // function.
            let probe_length = if matches!(kind, Weapon::Rocket) {
                SURFACE_PROBE_LENGTH
            } else {
                missile::SURFACE_PROBE_LENGTH
            };
            // The Cannon flies no probe at all: `Cannon_UpdateRound` (`0x0886593c`)
            // moves `position + velocity * dt` and sweeps that segment, with no
            // surface probe, no ride height and no fall. A probe along a seeded
            // world up snapped a round off the muzzle on any banked track.
            let probe = if kind == Weapon::Cannon {
                None
            } else {
                Raycaster::raycast(
                    raycaster,
                    Ray::new(to, -projectile.surface, probe_length),
                    None,
                    false,
                )
            };
            // The branch is on the surface class, the original's collision code.
            // `Collision_SweepSegment` (`0x0883198c`) returns `0` wall, `1` floor,
            // `3` mag floor or `0x7f` nothing. `Rocket_Update` (`0x0885d2a8`):
            // `0x7f` falls, `0` and `4` detonate, anything else rides.
            // `Missile_Update` (`0x0885a918`) and `Shuriken_Update` take `0`/`4`
            // as nothing (no fall, no ride); `Plasma_Update` detonates like the
            // Rocket. See [`nearest_hit`] for the sweep half.
            match probe {
                // A floor: sit at the ride height, adopt its normal, turn the
                // velocity parallel without changing speed, so a projectile
                // follows a rolling track instead of bouncing down it.
                Some(hit) if hit.surface.is_hoverable() => {
                    projectile.surface = hit.normal;
                    to = hit.point + hit.normal * RIDE_HEIGHT;
                    // A missile or plasma bolt re-pins its speed here:
                    // `Missile_Update` and `Plasma_Update` (`0x0885c6cc`, `default:`
                    // arm, `plasma.md`) normalise and rescale on this branch, by a
                    // divide by 3.6 (the Missile's guidance multiplies by a bit
                    // pattern that is not quite 1/3.6); both roundings are the
                    // original's. It is the only branch that re-pins a Plasma
                    // (the `0x7f` arm does not rescale). A Shuriken has `None`
                    // here and keeps its velocity length. A Rocket re-pins to its
                    // class speed, the step from its 0.75 launch to cruise.
                    let speed = pinned_kmh.map_or_else(
                        || projectile.velocity.length(),
                        missile::speed_units_on_surface,
                    );
                    if kind == Weapon::Rocket {
                        // The Rocket steers toward the ride point: `Rocket_Update`
                        // sets velocity to `(ride point - from) / dt`, rescales to
                        // the class speed and re-integrates from `from`. See
                        // `rocket-visuals.md`, 2026-10-01 second pass.
                        let toward = (to - from) / dt;
                        if toward.length_squared() > 0.0 {
                            projectile.velocity = toward.normalize() * speed;
                            to = from + projectile.velocity * dt;
                        } else {
                            projectile.velocity = toward;
                        }
                    } else {
                        let along =
                            projectile.velocity - hit.normal * projectile.velocity.dot(hit.normal);
                        // Aimed straight at the floor leaves nothing; keep the
                        // heading and let the sweep below resolve it.
                        if along.length_squared() > 1e-6 {
                            projectile.velocity = along.normalize() * speed;
                        }
                    }
                }
                // A wall within reach below: the Rocket and Plasma go off on it,
                // the bouncing weapons ignore it this tick. Neither spends a blast
                // on a wall: their craft sweeps (`Plasma_SweepCraftHit`
                // `0x0886afb8`, `Rocket_SweepCraftHit` `0x0886e7ac`) never reach
                // this probe, and the wall teardowns only play a cue, release the
                // trail and reap. See `docs/ghidra/functions/psp-pulse-usa/plasma.md`
                // ("the expiry settles a damage question") and `rocket-visuals.md`
                // ("what a rocket hit spends").
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
                // Nothing under it: it falls, keeping its normal so it resumes
                // riding when the track returns.
                None if kind != Weapon::Cannon => {
                    projectile.velocity -= Vec3::Y * FALL_ACCELERATION * dt;
                }
                None => {}
            }

            let step = to - from;
            let distance = step.length();

            // A zero step cannot hit anything and would normalise to NaN.
            let hit = if distance > 0.0 {
                nearest_hit(from, to, distance, raycaster, ships, projectile.owner)
            } else {
                None
            };

            let mut bounced = false;
            match hit {
                // A floor across the step is ridden, not struck: the original's
                // travel-segment switch takes a floor code to the same push-out
                // (`hit + normal * 3.0`) for all four weapons; only a wall or craft
                // ends a flight. It carries a projectile over a crest into the dip.
                //
                // The Missile only moves: `Missile_Update`'s floor arm writes no
                // normal or velocity (its guidance rewrites it). `Rocket_Update`
                // and `Shuriken_Update` adopt the normal and write
                // `(next - prev) / dt`. This engine has no per-tick rescale for the
                // Rocket, so velocity is turned parallel with speed kept (chosen,
                // not measured).
                Some(hit)
                    if kind != Weapon::Cannon
                        && hit.struck.is_none()
                        && hit.surface.is_some_and(Surface::is_hoverable) =>
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
                    // A missile glances off a wall, a rocket dies on it: the
                    // missile counts wall hits at `self+0x6c`, mirrors velocity
                    // about the normal losslessly, pushes out along it, and
                    // detonates at `MAX_BOUNCES`. A hull hit always detonates, hence
                    // `struck.is_none()`.
                    //
                    // A Shuriken bounces without a budget: `Shuriken_Bounce`
                    // (`0x088778ac`) is `v - 2(v.n)n` with no damping, so only its
                    // `fuse` ends it. Push-off: [`shuriken::BOUNCE_PUSH_OFF`].
                    // `bounces` is still counted for the visual side's bounce
                    // effect.
                    let push_off = match kind {
                        Weapon::Shuriken => shuriken::BOUNCE_PUSH_OFF,
                        Weapon::Cannon => cannon::BOUNCE_PUSH_OFF,
                        _ => missile::BOUNCE_PUSH_OFF,
                    };
                    let may_bounce = struck.is_none()
                        && match kind {
                            Weapon::Missile => projectile.bounces < missile::MAX_BOUNCES,
                            Weapon::Shuriken => true,
                            // `Cannon_UpdateRound`'s third arm: a floor or mag floor
                            // is neither a wall (`0`/`4`) nor nothing (`0x7f`), so
                            // it reflects the velocity and nudges the round off.
                            Weapon::Cannon => hit.surface.is_some_and(Surface::is_hoverable),
                            _ => false,
                        };
                    if may_bounce {
                        projectile.bounces = projectile.bounces.saturating_add(1);
                        projectile.velocity -= normal * (2.0 * projectile.velocity.dot(normal));
                        projectile.position = point + normal * push_off;
                        bounced = true;
                    } else {
                        // Plasma and Rocket split on `struck`: a wall found by this
                        // sweep (`struck: None`) spends no blast, as above. A craft
                        // found here belongs to the pool's own hull sweep
                        // (`Plasma_SweepCraftHit`, `Rocket_SweepCraftHit`; `*_HitCraft`
                        // and `*_ApplyBlastForce`), so `blast::apply_impacts` routes a
                        // `struck` impact to [`super::blast::blast_direct_hit`]. Missile
                        // and Shuriken keep `blast: true`.
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

            // Guidance runs last, writes only velocity and reads `from`:
            // `Missile_Update` steers off the position at the start of the tick,
            // so a correction lands a tick later. Skipped on a bounce.
            if let (Some(speed_kmh), Some(target), false) = (pinned_kmh, projectile.target, bounced)
                && let Some(ship) = ships.get(target as usize).filter(|s| s.active())
            {
                projectile.velocity = missile::steer(
                    projectile.velocity,
                    from,
                    ship.physics().body.position,
                    dt,
                    missile::speed_units_guided(speed_kmh),
                );
            }

            projectile.lifetime -= dt;

            // A missile that hit nothing goes off where it is: the pool's second
            // pass tests `3.0 < age` after the move (see
            // [`missile::SELF_DETONATE_SECONDS`]), strictly greater. `blast:
            // false`, see [`Impact::blast`].
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

            // A rocket that hit nothing is reaped at five seconds, a Cannon round
            // at one, silently: `RocketPool_Update` tests `5.0 < age` and
            // `CannonPool_Update` `1.0 < age`, retiring through the wall-hit
            // teardown with no explosion or blast. See
            // [`rocket::LIFETIME_SECONDS`] and [`cannon::LIFETIME_SECONDS`].
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
                // The Plasma alone detonates here, a recovered negative:
                // `Plasmas_Update` (`0x0886b490`) sets the same destroy bit at
                // `10.0 < age` as a wall, feeding the identical teardown
                // (`Psys_Release_q`, `Plasma_SpawnDetonation` `0x0886ac88`
                // `WO_PLASMA_FLASH`, `PLASMAHITWALL`). Read at instruction level
                // 2026-09-16, it touches no craft's shield or `entity+0x110`, and
                // `Weapon_PostBlastImpulse` (`0x0886794c`) has one caller
                // (`FUN_08867b50`, the Mine's, by `get_xrefs_to`). So `blast: false`;
                // see `Impact::blast` and `docs/ghidra/functions/psp-pulse-usa/plasma.md`
                // ("What is not verified") for the tension with the wall-hit
                // branch's `blast: true`, carried forward.
                //
                // The Shuriken's fuse is the same shape (read 2026-09-30):
                // `ShurikenPool_Update` (`0x0886ff38`) sets the destroy bit at
                // `fuse < age`; teardown `FUN_08870c78` plays `WO_SHURIKEN_EXPIRE`
                // and `ScreenFlash_Start(0)` and spends no damage.
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

                // Reaped, not detonated: nothing was struck. The cap is ours, that
                // there is one is not: `RocketPool_Update` reaps at `5.0 < age`
                // with no explosion (the Rocket and Cannon take that branch above
                // and never reach this). A missile detonated above. A mine or bomb
                // never gets here: its `timetodie` running out is a detonation
                // handled in [`advance_laid`].
                *projectile = Projectile::default();
            }
        }

        self.sweep_rockets_through_laid(&starts, trigger_radii, &mut impacts);
        impacts
    }

    /// A rocket flying through a laid mine or bomb sets it off and is spent,
    /// quietly on both sides.
    ///
    /// **Recovered.** `Rocket_SweepProjectiles` (`0x0886f154`), third call in
    /// `RocketPool_Update`'s loop, tests the rocket's segment against every live
    /// mine (the Mine's `trigger_radius`) and bomb (the Bomb's) and raises the
    /// destroy bit on both; neither teardown spends anything. See
    /// `docs/ghidra/functions/psp-pulse-usa/rocket-visuals.md` ("What a rocket
    /// hit spends") and `mine.md`'s 2026-09-16 section. A rocket already ended
    /// this tick is not swept.
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
