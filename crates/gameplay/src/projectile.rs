//! Things a weapon puts in the air, and what they hit.
//!
//! Today that is the Rocket alone. The Missile, the Plasma bolt, the Bomb, the
//! Mine and the Shuriken all belong here when they land; what each of them adds
//! is a launch rule and an impact rule, not a second flight model.
//!
//! # What is recovered and what is ours
//!
//! Stated at the top, the way [`crate::pickup`] states it, because this module
//! is **almost entirely ours** and that is easy to lose.
//!
//! **Recovered.** The numbers, and only the numbers:
//! `<Weapon type="Rocket"><Stats>` authors `damage`, `blastforce`,
//! `blastradius`, `launchSpeed` and a separate flight speed per speed class, at
//! confidence 92 (`docs/formats/weapon-stats.md`). Also recovered is what
//! happens at the far end - `Ship_Damage` (`0x088439ac`) takes a `source`
//! argument whose value 2 is a weapon, and the weapons-off halving and the
//! clamp are its own (`docs/ghidra/functions/psp-pulse-usa/shield.md`).
//!
//! **Ours, and there is no way for it not to be.** No firing call site, no
//! projectile class and no flight update has been found anywhere in the
//! executable. So:
//!
//! - **Straight-line flight at a constant speed.** A rocket is the one weapon
//!   whose name says it does not steer, and the Missile's `lock_max_dist` /
//!   `lock_min_dist` - which the Rocket does not carry - is what says the two
//!   differ in exactly that. But no integrator has been read.
//! - **One rocket per fire.** See [`oag_formats::weapons::RocketStats`], which
//!   records the `spread` evidence pointing the other way.
//! - **A sphere for a hull**, and one that is *wider* than the hull on two of
//!   its three axes. The original tests a projectile against something and
//!   nothing says what. See [`hull_radius`], which is exact about which way it
//!   errs.
//! - **Full damage everywhere inside `blastradius`,** with no falloff. See
//!   [`Impact`].
//! - **The launch offset, the lifetime cap, and the slot count.**
//!
//! # Fixed-size, like everything else in the world
//!
//! [`MAX_PROJECTILES`] slots of plain `Copy` data, allocated once inside
//! [`crate::World`]. No `Vec`, so a world snapshot stays one `memcpy`-shaped
//! operation - `docs/architecture/adr/0003-no-ecs.md`. A full array drops the
//! shot rather than growing, and [`Projectiles::spawn`] says so.

use oag_core::math::Vec3;
use oag_formats::weapons::{RocketStats, Weapon};
use oag_physics::params::Dimensions;
use oag_physics::{Ray, Raycaster, ShipState};

/// The most projectiles that can be in the air at once.
///
/// **Ours.** Eight craft with one weapon each cannot exceed eight in flight
/// today, and the headroom above that is deliberate: a Rocket fired as a volley,
/// which `spread` hints at (see [`RocketStats`]), would want three slots per
/// shot, and sixteen absorbs that without moving the array's size and with it
/// every committed world hash.
pub const MAX_PROJECTILES: usize = 16;

/// How long a projectile flies before it gives up, in seconds.
///
/// **Ours, and a safety net rather than a mechanic.** A rocket that leaves the
/// track through a gap in the collision soup would otherwise hold its slot for
/// the whole race. Long enough that no rocket fired at anything reachable
/// expires first: at the slowest authored class speed it covers several
/// kilometres, which is more than a lap of any Pulse circuit is wide.
pub const MAX_FLIGHT_SECONDS: f32 = 10.0;

/// One thing in the air.
///
/// A free slot is one whose [`Self::kind`] is `None`; the rest of the fields are
/// then stale and must not be read. Kept as one array of plain structs rather
/// than as a list with a length, so the array's contents depend on nothing but
/// the simulation.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Projectile {
    /// Which weapon fired it, or `None` for a free slot.
    pub kind: Option<Weapon>,
    /// Where it is, in world space.
    pub position: Vec3,
    /// Where it is going, in world units a second. Constant for its whole life:
    /// nothing steers, and no gravity is applied.
    pub velocity: Vec3,
    /// Which ship slot fired it.
    ///
    /// Read for two things: the shot cannot hit its own launcher in flight, and
    /// an [`Impact`] carries it so a future kill-credit or telemetry pass has
    /// it. **It is not excluded from the blast** - see [`Impact`].
    pub owner: u8,
    /// Seconds left before [`MAX_FLIGHT_SECONDS`] reaps it.
    pub lifetime: f32,
}

/// What a projectile did when it stopped.
///
/// # The blast rule, which is ours
///
/// Everything within `blastradius` of [`Self::point`] takes the **full**
/// `damage`. The original may well fall off with distance and nothing has been
/// read that says so, so a falloff curve would be invented detail on top of an
/// already-invented mechanic. Full damage inside a hard radius is the reading
/// that has one number in it and that number is the disc's.
///
/// **The firing craft is not excluded.** A rocket launched into a wall at close
/// range hurts the ship that fired it, which follows from the blast being a
/// position rather than an ownership question, and is the behaviour a player can
/// discover. Nothing says the original agrees.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Impact {
    /// Where the blast is centred.
    pub point: Vec3,
    /// Which weapon it was, so the caller can look its stats back up.
    pub kind: Weapon,
    /// The ship slot that fired it.
    pub owner: u8,
    /// The ship slot it struck directly, or `None` for a hit on geometry.
    ///
    /// A direct hit takes blast damage like any other craft inside the radius -
    /// this is here to tell the two apart for a future direct-hit bonus or a
    /// sound cue, not because the damage differs today.
    pub struck: Option<u8>,
}

/// Everything in the air, in one fixed-size array.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Projectiles {
    /// The slots. Public because [`crate::hash`] destructures them and a
    /// renderer walks them; a caller must test [`Projectile::kind`] before
    /// reading any other field.
    pub slots: [Projectile; MAX_PROJECTILES],
}

impl Default for Projectiles {
    fn default() -> Self {
        Self::new()
    }
}

impl Projectiles {
    /// An empty array.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            slots: [Projectile {
                kind: None,
                position: Vec3::ZERO,
                velocity: Vec3::ZERO,
                owner: 0,
                lifetime: 0.0,
            }; MAX_PROJECTILES],
        }
    }

    /// How many slots are in use.
    #[must_use]
    pub fn live(&self) -> usize {
        self.slots.iter().filter(|p| p.kind.is_some()).count()
    }

    /// Empties every slot.
    ///
    /// **Nothing calls this today**, and that is not an oversight to fix by
    /// finding a caller: `oag_game::race::Race::start` builds a whole new
    /// `World`, so a race begins with a fresh array and cannot inherit a rocket
    /// from the previous run. It is here for a caller that *reuses* a `World`
    /// (a replay scrubbing to a keyframe, or a restart that keeps the
    /// allocation), because for such a caller the alternative is a rocket
    /// arriving at a craft that never fired one.
    ///
    /// A respawn deliberately does **not** call it: a craft put back on the
    /// track has not unfired anything it launched.
    pub fn clear(&mut self) {
        *self = Self::new();
    }

    /// Puts one in the air, or does nothing if every slot is taken.
    ///
    /// **The first free slot in index order**, never a search that depends on
    /// anything but the array: which slot a shot lands in is simulation state,
    /// and a scan ordered by age or distance would make it depend on the whole
    /// history.
    ///
    /// A full array **drops the shot silently**. That is a real state only a
    /// bug can reach today - eight craft hold one pickup each - and the
    /// alternative, evicting the oldest, would let a full array change what an
    /// already-fired rocket does.
    ///
    /// Returns whether the shot was taken, so a caller can decline to spend the
    /// pickup on nothing.
    pub fn spawn(&mut self, kind: Weapon, position: Vec3, velocity: Vec3, owner: u8) -> bool {
        let Some(slot) = self.slots.iter_mut().find(|p| p.kind.is_none()) else {
            return false;
        };
        *slot = Projectile {
            kind: Some(kind),
            position,
            velocity,
            owner,
            lifetime: MAX_FLIGHT_SECONDS,
        };
        true
    }

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
    pub fn advance<R: Raycaster + ?Sized>(
        &mut self,
        dt: f32,
        raycaster: &R,
        ships: &[crate::world::Ship],
    ) -> [Option<Impact>; MAX_PROJECTILES] {
        let mut impacts = [None; MAX_PROJECTILES];

        for (index, projectile) in self.slots.iter_mut().enumerate() {
            let Some(kind) = projectile.kind else {
                continue;
            };

            let from = projectile.position;
            let to = from + projectile.velocity * dt;
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

            if let Some((point, struck)) = hit {
                impacts[index] = Some(Impact {
                    point,
                    kind,
                    owner: projectile.owner,
                    struck,
                });
                *projectile = Projectile::default();
                continue;
            }

            projectile.position = to;
            projectile.lifetime -= dt;
            if projectile.lifetime <= 0.0 {
                // Reaped, not detonated: nothing was struck, so nothing takes a
                // blast. A rocket that leaves the world simply stops existing.
                *projectile = Projectile::default();
            }
        }

        impacts
    }
}

/// The nearest of the geometry hit and the hull hits along one tick's step.
fn nearest_hit<R: Raycaster + ?Sized>(
    from: Vec3,
    to: Vec3,
    distance: f32,
    raycaster: &R,
    ships: &[crate::world::Ship],
    owner: u8,
) -> Option<(Vec3, Option<u8>)> {
    let direction = (to - from) / distance;

    // `include_reset` is false: a `Reset Collision` volume is a respawn trigger
    // rather than a surface, and a rocket detonating on one would blow up in
    // mid-air over the run-off.
    let mut best = Raycaster::raycast(raycaster, Ray::new(from, direction, distance), None, false)
        .map(|hit| (hit.distance, hit.point, None));

    for (slot, ship) in ships.iter().enumerate() {
        if !ship.active || slot as u8 == owner {
            continue;
        }
        let radius = hull_radius(&ship.handling.dimensions);
        let Some(t) = segment_sphere(from, to, ship.physics.body.position, radius) else {
            continue;
        };
        let travelled = t * distance;
        if best.is_none_or(|(nearest, _, _)| travelled < nearest) {
            best = Some((travelled, from + direction * travelled, Some(slot as u8)));
        }
    }

    best.map(|(_, point, struck)| (point, struck))
}

/// Flies the world's projectiles and detonates whatever stopped, in one call.
///
/// The seam a race loop wants: `oag-gameplay` owns both halves - the array and
/// the weapon table that says what a hit is worth - and the composition root
/// owns neither. Returns the impacts so a caller can put a spark where each one
/// landed, which is the only thing left for it to do.
///
/// `rocket` is `None` for a race whose weapon table did not load, in which case
/// an impact does nothing but free its slot. That is the same "gated on the
/// table" rule the pickup grant follows, and for the same reason: a blast with
/// no authored radius or damage would be an invented number.
///
/// # Ordering
///
/// Every projectile is flown before any blast is applied, so a rocket cannot be
/// deflected by a blast from another rocket that happened to be earlier in the
/// array. Two simultaneous impacts therefore both see the pre-blast poses, which
/// is the only ordering that does not make the array index part of the physics.
pub fn step<R: Raycaster + ?Sized>(
    world: &mut crate::World,
    dt: f32,
    raycaster: &R,
    rocket: Option<&RocketStats>,
    rules: oag_physics::DamageRules,
) -> [Option<Impact>; MAX_PROJECTILES] {
    let count = world.ship_count as usize;
    let impacts = world
        .projectiles
        .advance(dt, raycaster, &world.ships[..count]);

    if let Some(stats) = rocket {
        for impact in impacts.iter().flatten() {
            blast(
                &mut world.ships[..count],
                impact.point,
                stats.blastradius,
                stats.damage,
                stats.blastforce,
                rules,
            );
        }
    }

    impacts
}

/// Spends one blast against every craft inside its radius.
///
/// Full `damage` and a `force` impulse directed away from `point`, with no
/// falloff and with the firing craft included - the choices [`Impact`] records
/// and defends. Damage goes through [`oag_physics::damage::apply_weapon`], so
/// the state gate, the weapons-off halving and the clamp are the recovered ones.
///
/// Returns how many craft it reached, which is what a caller asserting "the
/// blast did something" wants and what a test asserting "and nothing outside the
/// radius" needs the other half of.
pub fn blast(
    ships: &mut [crate::world::Ship],
    point: Vec3,
    radius: f32,
    damage: f32,
    force: f32,
    rules: oag_physics::DamageRules,
) -> usize {
    let mut reached = 0;
    for ship in ships.iter_mut().filter(|s| s.active) {
        let offset = ship.physics.body.position - point;
        if offset.length() > radius {
            continue;
        }
        reached += 1;

        let dimensions = ship.handling.dimensions;
        oag_physics::damage::apply_weapon(&mut ship.physics, &dimensions, damage, rules);

        // A craft exactly on the blast centre has no direction to be pushed in.
        // World up rather than a zero push or a normalised NaN: something has to
        // happen, and up is the one direction that does not depend on an
        // arbitrary axis of the craft or of the track.
        let direction = if offset.length() > 1e-4 {
            offset.normalize()
        } else {
            Vec3::Y
        };
        ship.physics.body.apply_impulse(direction * force);
    }
    reached
}

/// The sphere a craft is tested against.
///
/// **Ours.** `<Misc>` authors a `length`, a `width` and a `height` and the
/// physics builds a box from them (`oag_physics::wall::hull_extent`), but
/// nothing has been read about what a *projectile* is tested against.
///
/// Half the largest dimension is the sphere that **circumscribes** the box's
/// longest axis, and it is worth being exact about which way that errs. A hull
/// of `length 4, width 2, height 1` has half-extents `(2.0, 1.0, 0.5)` and a
/// radius of `2.0`, so the sphere matches the box nose-to-tail and **bulges
/// past it on the other two axes**: a rocket passing 1.8 units to the side hits,
/// where the box would have missed. So this is the *generous* reading, not the
/// conservative one - it favours the shooter, and a near miss can register as a
/// hit.
///
/// That is a defensible placeholder rather than the right answer: the smallest
/// half-extent (`0.5` here) would be conservative and would make most visually
/// solid hits miss, which reads as a broken weapon. Sizing to the hull's length
/// keeps a craft-sized target. Whoever recovers what the original tests should
/// replace the whole function rather than tune this number.
///
/// Reusing the box would mean a segment-vs-oriented-box test for a mechanic
/// where nothing is known about the original's own shape, which is precision
/// with no evidence under it.
#[must_use]
pub fn hull_radius(dimensions: &Dimensions) -> f32 {
    0.5 * dimensions
        .length
        .max(dimensions.width)
        .max(dimensions.height)
}

/// Where a segment first enters a sphere, as a fraction of the segment.
///
/// `None` when it misses, or when both roots lie outside `0..=1`. A segment that
/// *starts* inside returns `0.0`, which is the answer a launch that overlaps a
/// hull needs.
fn segment_sphere(p0: Vec3, p1: Vec3, centre: Vec3, radius: f32) -> Option<f32> {
    let d = p1 - p0;
    let m = p0 - centre;
    let a = d.dot(d);
    if a <= 0.0 {
        return None;
    }
    let b = m.dot(d);
    let c = m.dot(m) - radius * radius;

    // Already inside. Not folded into the quadratic below: with `c <= 0` the
    // near root is negative and would be rejected, which would let a projectile
    // spawned inside a hull fly out through it.
    if c <= 0.0 {
        return Some(0.0);
    }
    // Heading away, and outside.
    if b >= 0.0 {
        return None;
    }

    let discriminant = b * b - a * c;
    if discriminant < 0.0 {
        return None;
    }
    let t = (-b - discriminant.sqrt()) / a;
    if (0.0..=1.0).contains(&t) {
        Some(t)
    } else {
        None
    }
}

/// Where a craft launches a projectile from, and how fast.
///
/// **Ours, both halves.**
///
/// The origin is the ship's nose - its centre of mass pushed forward by the
/// hull's own extent along its facing, through
/// [`oag_physics::wall::hull_extent`], which is the same function the contact
/// solver measures the hull with. So a rocket starts outside the craft that
/// fired it rather than inside it, and no grace period is needed to stop a
/// launch registering as a self-hit.
///
/// The speed is the class's own `<Rocket venomspeed|...>` **plus**
/// `launchSpeed`. That reading of the pair is a guess: the file authors both,
/// the names suggest a cruise speed and a launch addition, and nothing has been
/// read that combines them. The craft's own velocity is **not** inherited -
/// which is the other plausible reading of what `launchSpeed` is for, and is
/// recorded here as the alternative rather than silently not done.
#[must_use]
pub fn launch(
    state: &ShipState,
    dimensions: &Dimensions,
    stats: &RocketStats,
    class: oag_formats::handling::SpeedClass,
) -> (Vec3, Vec3) {
    let forward = state.body.forward();
    let nose = state.body.position
        + forward * oag_physics::wall::hull_extent(&state.body, dimensions, forward);
    let speed = stats.speed_for(class) + stats.launch_speed;
    (nose, forward * speed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use oag_physics::{CollisionWorld, Surface, TriangleSoup};

    fn ships(entries: &[(bool, Vec3)]) -> Vec<crate::world::Ship> {
        entries
            .iter()
            .map(|&(active, position)| {
                let mut ship = crate::world::Ship {
                    active,
                    ..crate::world::Ship::default()
                };
                ship.physics.body.position = position;
                ship.handling.dimensions = Dimensions {
                    length: 4.0,
                    width: 2.0,
                    height: 1.0,
                    ..Dimensions::default()
                };
                ship
            })
            .collect()
    }

    /// A wall at `z = 100`, facing back down `-Z` at anything flying `+Z`.
    fn wall_at_z(z: f32) -> CollisionWorld {
        let mut world = CollisionWorld::new();
        world.push(TriangleSoup::new(
            vec![
                [-100.0, -100.0, z],
                [-100.0, 100.0, z],
                [100.0, 100.0, z],
                [100.0, -100.0, z],
            ],
            vec![[0, 1, 2], [0, 2, 3]],
            Vec::new(),
            Surface::Wall,
            0,
        ));
        world
    }

    fn empty_world() -> CollisionWorld {
        CollisionWorld::new()
    }

    #[test]
    fn a_spawned_projectile_takes_the_first_free_slot_and_the_array_never_grows() {
        let mut projectiles = Projectiles::new();
        for _ in 0..MAX_PROJECTILES {
            assert!(projectiles.spawn(Weapon::Rocket, Vec3::ZERO, Vec3::Z, 0));
        }
        assert_eq!(projectiles.live(), MAX_PROJECTILES);
        assert!(
            !projectiles.spawn(Weapon::Rocket, Vec3::ZERO, Vec3::Z, 0),
            "a full array must decline rather than evict"
        );
        assert_eq!(projectiles.slots.len(), MAX_PROJECTILES);
    }

    /// Straight line, constant speed, no gravity and no steering. The whole
    /// flight model, and the thing every other test here rests on.
    #[test]
    fn flight_is_straight_and_at_a_constant_speed() {
        let mut projectiles = Projectiles::new();
        projectiles.spawn(Weapon::Rocket, Vec3::ZERO, Vec3::Z * 600.0, 0);
        let world = empty_world();
        let dt = 1.0 / 60.0;

        for tick in 1..=10 {
            let impacts = projectiles.advance(dt, &world, &ships(&[]));
            assert!(impacts.iter().all(Option::is_none));
            let p = projectiles.slots[0];
            assert!((p.position.z - 10.0 * tick as f32).abs() < 1e-3, "{p:?}");
            assert_eq!(p.position.x, 0.0);
            assert_eq!(p.position.y, 0.0, "nothing pulls a rocket down");
        }
    }

    /// The swept test, and the reason it exists: at an authored class speed a
    /// rocket covers more than a tick's worth of wall in one step, so a point
    /// test would pass through.
    #[test]
    fn a_rocket_hits_a_wall_it_would_tunnel_through_in_one_tick() {
        let mut projectiles = Projectiles::new();
        // 800 units a second is the slowest class's authored rocket speed on the
        // disc; at 60 Hz that is 13 units a tick against a wall of no thickness.
        projectiles.spawn(Weapon::Rocket, Vec3::ZERO, Vec3::Z * 800.0, 0);
        let world = wall_at_z(20.0);
        let dt = 1.0 / 60.0;

        let mut impact = None;
        for _ in 0..10 {
            let impacts = projectiles.advance(dt, &world, &ships(&[]));
            if let Some(hit) = impacts.into_iter().flatten().next() {
                impact = Some(hit);
                break;
            }
        }
        let impact = impact.expect("a rocket flew through a wall");
        assert!((impact.point.z - 20.0).abs() < 1e-3, "{impact:?}");
        assert_eq!(impact.struck, None, "a wall is not a craft");
        assert_eq!(projectiles.live(), 0, "an impact must free the slot");
    }

    /// A hull hit reports which slot it struck, and the owner's own hull is
    /// never it.
    #[test]
    fn a_rocket_strikes_a_craft_that_is_not_its_owner() {
        let mut projectiles = Projectiles::new();
        projectiles.spawn(Weapon::Rocket, Vec3::ZERO, Vec3::Z * 600.0, 0);
        // Slot 0 is the owner and sits directly on the flight path; slot 1 is
        // further along it. A shot that could hit its own launcher would report
        // slot 0 and stop at once.
        let grid = ships(&[(true, Vec3::new(0.0, 0.0, 5.0)), (true, Vec3::Z * 30.0)]);
        let world = empty_world();

        let mut impact = None;
        for _ in 0..20 {
            if let Some(hit) = projectiles
                .advance(1.0 / 60.0, &world, &grid)
                .into_iter()
                .flatten()
                .next()
            {
                impact = Some(hit);
                break;
            }
        }
        let impact = impact.expect("a rocket flew through a craft");
        assert_eq!(impact.struck, Some(1), "a rocket hit its own launcher");
        assert_eq!(impact.owner, 0);
    }

    /// An inactive slot is not a craft. Slots past `ship_count` hold whatever
    /// the last race left in them, and a rocket must not detonate on one.
    #[test]
    fn an_inactive_slot_is_not_a_target() {
        let mut projectiles = Projectiles::new();
        projectiles.spawn(Weapon::Rocket, Vec3::ZERO, Vec3::Z * 600.0, 0);
        let grid = ships(&[(true, Vec3::ZERO), (false, Vec3::Z * 30.0)]);
        let world = empty_world();

        for _ in 0..20 {
            let impacts = projectiles.advance(1.0 / 60.0, &world, &grid);
            assert!(
                impacts.iter().all(Option::is_none),
                "a rocket detonated on an empty grid slot"
            );
        }
    }

    /// The nearer of the two wins, which is what stops a rocket reaching a craft
    /// through a wall.
    #[test]
    fn geometry_in_front_of_a_craft_stops_the_rocket_first() {
        let mut projectiles = Projectiles::new();
        projectiles.spawn(Weapon::Rocket, Vec3::ZERO, Vec3::Z * 600.0, 0);
        let world = wall_at_z(20.0);
        let grid = ships(&[(true, Vec3::ZERO), (true, Vec3::Z * 40.0)]);

        let mut impact = None;
        for _ in 0..20 {
            if let Some(hit) = projectiles
                .advance(1.0 / 60.0, &world, &grid)
                .into_iter()
                .flatten()
                .next()
            {
                impact = Some(hit);
                break;
            }
        }
        let impact = impact.expect("nothing stopped the rocket");
        assert_eq!(
            impact.struck, None,
            "the rocket reached a craft through a wall"
        );
        assert!((impact.point.z - 20.0).abs() < 1e-3);
    }

    /// A rocket that hits nothing frees its slot instead of holding it forever,
    /// and it does so **without** reporting an impact - nothing was struck, so
    /// nothing takes a blast.
    #[test]
    fn a_rocket_that_hits_nothing_is_reaped_without_detonating() {
        let mut projectiles = Projectiles::new();
        projectiles.spawn(Weapon::Rocket, Vec3::ZERO, Vec3::Z * 600.0, 0);
        let world = empty_world();
        let dt = 1.0 / 60.0;

        let ticks = (MAX_FLIGHT_SECONDS / dt).ceil() as u32 + 2;
        for _ in 0..ticks {
            let impacts = projectiles.advance(dt, &world, &ships(&[]));
            assert!(
                impacts.iter().all(Option::is_none),
                "a reaped rocket must not blast"
            );
        }
        assert_eq!(projectiles.live(), 0, "the slot leaked");
    }

    /// The blast, both halves: energy off the pool and velocity away from the
    /// centre, for everything inside the radius and nothing outside it.
    ///
    /// The craft outside is the assertion that matters - a blast that reached
    /// every craft on the track would pass any test that only looked at the one
    /// that was hit.
    #[test]
    fn a_blast_reaches_inside_the_radius_and_stops_at_it() {
        let mut grid = ships(&[
            (true, Vec3::ZERO),                // dead centre
            (true, Vec3::new(0.0, 0.0, 8.0)),  // inside a radius of 10
            (true, Vec3::new(0.0, 0.0, 40.0)), // well outside it
        ]);
        for ship in &mut grid {
            ship.handling.dimensions.shield = 100.0;
            ship.physics.shield = 100.0;
            ship.physics.body.mass = 2.0;
        }

        let reached = blast(
            &mut grid,
            Vec3::ZERO,
            10.0,
            30.0,
            100.0,
            oag_physics::DamageRules::default(),
        );
        assert_eq!(reached, 2, "the blast reached {reached} craft");

        assert_eq!(grid[0].physics.shield, 70.0, "the craft at the centre");
        assert_eq!(grid[1].physics.shield, 70.0, "the craft inside the radius");
        assert_eq!(
            grid[2].physics.shield, 100.0,
            "a craft outside the radius took damage"
        );

        // `dv = J / m`, so 100 units of force on a mass of 2 is 50 units of
        // velocity, directed away from the centre.
        let pushed = grid[1].physics.body.linear_velocity;
        assert!((pushed.z - 50.0).abs() < 1e-3, "pushed {pushed:?}");
        assert_eq!(
            grid[2].physics.body.linear_velocity,
            Vec3::ZERO,
            "a craft outside the radius was pushed"
        );
        // The craft exactly on the centre has no direction, and gets world up
        // rather than a NaN.
        let centred = grid[0].physics.body.linear_velocity;
        assert!(centred.is_finite(), "a centred craft got {centred:?}");
        assert!((centred.y - 50.0).abs() < 1e-3, "{centred:?}");
    }

    /// A shielded craft inside the radius takes neither half. The damage gate is
    /// `oag_physics::damage`'s, but the *impulse* is applied here and has no
    /// gate of its own - so this is the test that says whether a shield stops a
    /// rocket shoving a craft off the racing line.
    ///
    /// **It does not**, and that is deliberate: the shield refuses damage, which
    /// is the one thing about it with a duration behind it. Extending it to
    /// refuse momentum would be a second invented rule stacked on the first.
    #[test]
    fn a_shielded_craft_keeps_its_energy_and_still_gets_shoved() {
        let mut grid = ships(&[(true, Vec3::new(0.0, 0.0, 5.0))]);
        grid[0].handling.dimensions.shield = 100.0;
        grid[0].physics.shield = 100.0;
        grid[0].physics.body.mass = 1.0;
        grid[0].physics.shield_pickup_timer = 1.0;

        blast(
            &mut grid,
            Vec3::ZERO,
            10.0,
            30.0,
            10.0,
            oag_physics::DamageRules::default(),
        );
        assert_eq!(grid[0].physics.shield, 100.0, "the shield let damage in");
        assert!(
            grid[0].physics.body.linear_velocity.length() > 0.0,
            "the shield also stopped the shove, which it should not"
        );
    }

    /// The whole chain through [`step`]: fly, hit, blast. The one test that
    /// would catch the halves being wired to each other wrongly rather than each
    /// being right on its own.
    #[test]
    fn a_rocket_fired_at_a_parked_craft_takes_its_energy() {
        let stats = oag_formats::weapons::parse(
            r#"<WeaponStats>
                 <Weapon type="Global"><Stats slowdown_limit="0"/></Weapon>
                 <Weapon type="Rocket"><Stats absorb="1" blastforce="10" blastradius="12"
                   damage="25" slowdown_time="1" venomspeed="600" flashspeed="700"
                   rapierspeed="800" phantomspeed="900" launchSpeed="0" spread="1"/></Weapon>
               </WeaponStats>"#,
        )
        .expect("the fixture parses")
        .rocket()
        .expect("a Rocket");

        let mut world = crate::World::new(1);
        world.ship_count = 2;
        for (slot, position) in [Vec3::ZERO, Vec3::Z * 60.0].into_iter().enumerate() {
            let ship = &mut world.ships[slot];
            ship.active = true;
            ship.handling.dimensions = Dimensions {
                length: 4.0,
                width: 2.0,
                height: 1.0,
                shield: 100.0,
                ..Dimensions::default()
            };
            ship.physics.shield = 100.0;
            ship.physics.body.mass = 1.0;
            ship.physics.body.position = position;
        }

        world
            .projectiles
            .spawn(Weapon::Rocket, Vec3::ZERO, Vec3::Z * 600.0, 0);

        let empty = empty_world();
        let mut hit = false;
        for _ in 0..30 {
            let impacts = step(
                &mut world,
                1.0 / 60.0,
                &empty,
                Some(&stats),
                oag_physics::DamageRules::default(),
            );
            if impacts.iter().flatten().count() > 0 {
                hit = true;
                break;
            }
        }
        assert!(hit, "the rocket never reached the parked craft");
        assert_eq!(
            world.ships[1].physics.shield, 75.0,
            "the target took no damage"
        );
        assert_eq!(
            world.ships[0].physics.shield, 100.0,
            "the firing craft was 60 units away and inside no radius"
        );
        assert_eq!(world.projectiles.live(), 0, "the slot leaked");
    }

    /// A race whose weapon table did not load still flies and reaps rockets; it
    /// just cannot say what a hit is worth, so nothing takes damage.
    #[test]
    fn without_rocket_stats_an_impact_only_frees_its_slot() {
        let mut world = crate::World::new(1);
        world.ship_count = 1;
        world.ships[0].active = true;
        world.ships[0].handling.dimensions.shield = 100.0;
        world.ships[0].physics.shield = 100.0;

        world
            .projectiles
            .spawn(Weapon::Rocket, Vec3::ZERO, Vec3::Z * 800.0, 1);
        let wall = wall_at_z(20.0);
        for _ in 0..10 {
            step(
                &mut world,
                1.0 / 60.0,
                &wall,
                None,
                oag_physics::DamageRules::default(),
            );
        }
        assert_eq!(world.projectiles.live(), 0);
        assert_eq!(world.ships[0].physics.shield, 100.0);
    }

    #[test]
    fn clearing_empties_every_slot() {
        let mut projectiles = Projectiles::new();
        projectiles.spawn(Weapon::Rocket, Vec3::ZERO, Vec3::Z, 0);
        projectiles.clear();
        assert_eq!(projectiles.live(), 0);
    }

    /// The two degenerate cases the quadratic would otherwise get wrong: a
    /// segment that starts inside must report the start, and one aimed away from
    /// a sphere it is outside of must report nothing however long it is.
    #[test]
    fn the_sphere_test_handles_starting_inside_and_pointing_away() {
        let centre = Vec3::new(0.0, 0.0, 10.0);
        assert_eq!(
            segment_sphere(centre, centre + Vec3::Z * 100.0, centre, 2.0),
            Some(0.0)
        );
        assert_eq!(
            segment_sphere(Vec3::ZERO, Vec3::Z * -100.0, centre, 2.0),
            None,
            "a segment aimed away from a sphere hit it"
        );
        assert_eq!(
            segment_sphere(Vec3::ZERO, Vec3::X * 100.0, centre, 2.0),
            None,
            "a segment that passes wide hit it"
        );
        // And one that reaches exactly the near face.
        let t = segment_sphere(Vec3::ZERO, Vec3::Z * 8.0, centre, 2.0).expect("a grazing hit");
        assert!((t - 1.0).abs() < 1e-4, "entered at {t}");
    }

    /// The radius is half the *largest* dimension, so a long craft is not
    /// modelled by its narrowest axis - **and the sphere is therefore wider
    /// than the hull**, which is the part [`hull_radius`]' docs are careful
    /// about and which this pins rather than leaves to the prose.
    #[test]
    fn the_hull_radius_circumscribes_the_longest_axis_and_bulges_past_the_rest() {
        let dimensions = Dimensions {
            length: 4.0,
            width: 2.0,
            height: 1.0,
            ..Dimensions::default()
        };
        let radius = hull_radius(&dimensions);
        assert_eq!(radius, 2.0);
        // Nose to tail it matches the box exactly...
        assert_eq!(radius, dimensions.length * 0.5);
        // ...and on both other axes it reaches further, so a shot that the box
        // would miss still hits. Generous, not conservative.
        assert!(radius > dimensions.width * 0.5);
        assert!(radius > dimensions.height * 0.5);
    }

    /// A launch starts outside the hull that fired it and carries the class's
    /// own speed plus `launchSpeed` - not one or the other.
    #[test]
    fn a_launch_clears_the_hull_and_reads_both_speeds() {
        use oag_formats::handling::SpeedClass;

        let mut state = ShipState::default();
        state.body.position = Vec3::new(0.0, 0.0, 0.0);
        let dimensions = Dimensions {
            length: 4.0,
            width: 2.0,
            height: 1.0,
            ..Dimensions::default()
        };
        let stats = oag_formats::weapons::parse(
            r#"<WeaponStats>
                 <Weapon type="Global"><Stats slowdown_limit="0"/></Weapon>
                 <Weapon type="Rocket"><Stats absorb="1" blastforce="2" blastradius="3"
                   damage="4" slowdown_time="5" venomspeed="600" flashspeed="700"
                   rapierspeed="800" phantomspeed="900" launchSpeed="50" spread="6"/></Weapon>
               </WeaponStats>"#,
        )
        .expect("the fixture parses")
        .rocket()
        .expect("a Rocket");

        let (nose, velocity) = launch(&state, &dimensions, &stats, SpeedClass::Venom);
        // `Body::forward` is `-Z`, so a nose ahead of the origin has a negative
        // `z`. Asserted through the body's own axis rather than as a sign, so
        // this test says "outside the hull" rather than "in this direction".
        assert!(
            (nose - state.body.position).dot(state.body.forward()) > 0.0,
            "the launch point is behind the craft: {nose:?}"
        );
        assert!(
            nose.length() > 0.0,
            "the launch point is inside the hull: {nose:?}"
        );
        assert!(
            (velocity.length() - 650.0).abs() < 1e-2,
            "expected the class speed plus launchSpeed, got {}",
            velocity.length()
        );
    }
}
