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
//! **Recovered.** The numbers, and **how many rockets a press puts in the
//! air**. `<Weapon type="Rocket"><Stats>` authors `damage`, `blastforce`,
//! `blastradius`, `launchSpeed`, `spread` and a separate flight speed per speed
//! class, at confidence 92 (`docs/formats/weapon-stats.md`). `Weapon_FireRocket`
//! (`0x0886e104`) then spawns **three at once** - straight ahead, `+spread` and
//! `-spread` - at confidence 88
//! (`docs/ghidra/functions/psp-pulse-usa/weapon-fire.md`). Also recovered is
//! what happens at the far end: `Ship_Damage` (`0x088439ac`) takes a `source`
//! argument whose value 2 is a weapon, and the weapons-off halving and the clamp
//! are its own (`.../shield.md`).
//!
//! Also recovered, and now implemented: **a projectile follows the floor.**
//! `Rocket_Update` (`0x0885d2a8`) probes [`SURFACE_PROBE_LENGTH`] units toward
//! the surface every tick, rides [`RIDE_HEIGHT`] above what it finds, and falls
//! at [`FALL_ACCELERATION`] when it finds nothing
//! (`.../rocket-visuals.md`). Confirmed in PPSSPP, where the frames show rockets
//! hugging the track a body-length off the racing line, and corroborated by a
//! maintainer who has played the game: projectiles skim the floor and stop only
//! on a craft or a wall.
//!
//! **This is why the module was rewritten rather than tuned.** Flying straight,
//! a volley died in the tick it was fired - measured on a real track, all three
//! rockets gone before the next frame, which is what "they look like three dots
//! and disappear" was.
//!
//! **Ours.**
//!
//! - **Wall versus floor.** The original picks "detonate" or "deflect" from a
//!   collision *code* its query returns; this engine's raycaster has no such
//!   code, so the test is geometric. See [`WALL_FACING`] and [`RIDEABLE_COS`],
//!   the two thresholds that decision rests on.
//! - **Turning the velocity parallel to the surface rather than reflecting it.**
//!   The original recomputes velocity from a corrected position, which is not
//!   quite either; this preserves speed and follows the track, which is the
//!   behaviour described.
//! - **That the rule is weapon-agnostic.** It lives in [`Projectiles::advance`],
//!   so the Missile and the rest inherit it when they land. A maintainer's
//!   account puts the Missile on the floor too and is **less sure about the
//!   Cannon**; nothing here has read either, so if the Cannon turns out to fire
//!   flat this is the place that has to grow a per-weapon flag.
//! - **The axis the fan rotates about.** See [`launch`].
//! - **A sphere for a hull**, and one that is *wider* than the hull on two of
//!   its three axes. The original tests a projectile against something and
//!   nothing says what. See [`hull_radius`], which is exact about which way it
//!   errs.
//! - **Full damage everywhere inside `blastradius`,** with no falloff. See
//!   [`Impact`].
//! - **The launch offset and the lifetime cap.**
//!
//! # Fixed-size, like everything else in the world
//!
//! [`MAX_PROJECTILES`] slots of plain `Copy` data, allocated once inside
//! [`crate::World`]. No `Vec`, so a world snapshot stays one `memcpy`-shaped
//! operation - `docs/architecture/adr/0003-no-ecs.md`. A full array drops the
//! shot rather than growing, and [`Projectiles::spawn`] says so.

pub mod missile;

use oag_core::math::Vec3;
use oag_formats::weapons::{RocketStats, Weapon};
use oag_physics::params::Dimensions;
use oag_physics::{Ray, Raycaster, ShipState};

/// The most projectiles that can be in the air at once.
///
/// **Ours, and deliberately larger than the original's.**
///
/// The original's pool caps at **48** - `Weapon_FireRocket` bounds-checks
/// `live < 0x2e` before putting three more in the air, against a cap of `0x30`
/// elsewhere. That is the floor this has to clear, not the target: it is a 2007
/// handheld's budget, and an Eliminator race in the original genuinely runs out
/// and drops shots.
///
/// **This used to be 16, and 16 was too small to be correct**, not merely tight:
/// a Rocket puts [`ROCKET_SHOTS`] in the air per press, so one simultaneous
/// volley from a full grid is 24 and would have silently dropped a third of
/// itself at [`Projectiles::spawn`]. Surface-following flight made it worse by
/// keeping projectiles alive far longer than straight flight into the nearest
/// wall did.
///
/// 128 is eight craft with sixteen apiece in the air at once, which no rate of
/// fire this engine can reach will exhaust. Fixed-size rather than a `Vec`
/// because [ADR-0003](../../../docs/architecture/adr/0003-no-ecs.md) requires
/// the world to snapshot in one `memcpy`-shaped operation; at roughly 48 bytes
/// a slot the whole array is about 6 KiB, which is not a number worth
/// economising on.
pub const MAX_PROJECTILES: usize = 128;

/// How long a projectile flies before it gives up, in seconds.
///
/// **Ours, and a safety net rather than a mechanic.** A rocket that leaves the
/// track through a gap in the collision soup would otherwise hold its slot for
/// the whole race. Long enough that no rocket fired at anything reachable
/// expires first: the disc's slowest class authors `800` km/h, which is `222`
/// units a second (see [`launch`] on the unit), so ten seconds is better than
/// two kilometres - more than a lap of any Pulse circuit is wide.
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
    /// Where it is going, in world units a second.
    ///
    /// **Not constant.** It is turned parallel to the surface each tick the
    /// projectile finds one, and pulled down by [`FALL_ACCELERATION`] each tick
    /// it does not. Speed is preserved across a turn; only the direction moves.
    pub velocity: Vec3,
    /// The surface normal the projectile is riding, normalised.
    ///
    /// **Recovered as a concept**: the original keeps one at `self+0x100`,
    /// initialises it from the firing craft and re-probes along it every tick
    /// (`Rocket_Update`, `0x0885d2a8`). It is what makes a projectile follow a
    /// banked or rolling track rather than a horizontal plane - the probe goes
    /// along *this*, not along world down.
    ///
    /// Seeded to [`Vec3::Y`] by [`Projectiles::spawn`] rather than to the firing
    /// craft's up, which is **ours**: the original seeds it from the craft and
    /// this engine's spawn call does not carry one. The first probe corrects it,
    /// so the cost is at most one tick of a wrong probe direction on a steeply
    /// banked launch.
    pub surface: Vec3,
    /// Which ship slot fired it.
    ///
    /// Read for two things: the shot cannot hit its own launcher in flight, and
    /// an [`Impact`] carries it so a future kill-credit or telemetry pass has
    /// it. **It is not excluded from the blast** - see [`Impact`].
    pub owner: u8,
    /// Seconds left before [`MAX_FLIGHT_SECONDS`] reaps it.
    pub lifetime: f32,
    /// The ship slot a guided projectile is chasing, or `None` for an unguided
    /// one and for a guided one that locked nothing.
    ///
    /// **A slot index where the original keeps a pointer.** `Missile_Init` copies
    /// a raw pointer to the target object into `self+0xe0`; an index is what
    /// survives being inside a `Copy` world snapshot, and it is what
    /// [`crate::hash`] can hash.
    ///
    /// Set once at launch and **never re-evaluated** - that is the original's
    /// behaviour, not a simplification: `Missile_Update` reads `+0xe0` twice and
    /// writes it never, so there is no re-targeting, no range re-check and no
    /// give-up when the target gets away. See [`missile::lock`].
    pub target: Option<u8>,
    /// How many walls this projectile has already glanced off.
    ///
    /// Only a Missile ever raises it - see [`missile::MAX_BOUNCES`]. A Rocket
    /// detonates on its first wall, so its counter is always zero.
    pub bounces: u8,
    /// The speed this projectile left the rail at, in km/h.
    ///
    /// Only a Missile reads it, as the base its speed ramp blends away from over
    /// [`missile::SPEED_RAMP_SECONDS`]; see [`missile::speed_kmh`]. It is
    /// per-shot rather than per-weapon because it carries the firing craft's own
    /// speed at the moment of launch.
    pub launch_speed_kmh: f32,
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
                surface: Vec3::Y,
                target: None,
                bounces: 0,
                launch_speed_kmh: 0.0,
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
        self.spawn_guided(kind, position, velocity, owner, None, 0.0)
    }

    /// The same, for a projectile that carries a lock and a launch speed.
    ///
    /// [`Self::spawn`] is this with both left empty, so an unguided weapon needs
    /// no new call site and - more to the point - lands in exactly the same slot
    /// with exactly the same fields it always did. That is what keeps a
    /// rocket-only run's determinism hash a question about the hash *stream*
    /// rather than about the rocket's flight.
    pub fn spawn_guided(
        &mut self,
        kind: Weapon,
        position: Vec3,
        velocity: Vec3,
        owner: u8,
        target: Option<u8>,
        launch_speed_kmh: f32,
    ) -> bool {
        let Some(slot) = self.slots.iter_mut().find(|p| p.kind.is_none()) else {
            return false;
        };
        *slot = Projectile {
            kind: Some(kind),
            position,
            velocity,
            owner,
            lifetime: MAX_FLIGHT_SECONDS,
            // World up until the first probe corrects it - see the field.
            surface: Vec3::Y,
            target,
            bounces: 0,
            launch_speed_kmh,
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
        missile: Option<&oag_formats::weapons::MissileStats>,
        class: oag_formats::handling::SpeedClass,
    ) -> [Option<Impact>; MAX_PROJECTILES] {
        let mut impacts = [None; MAX_PROJECTILES];

        for (index, projectile) in self.slots.iter_mut().enumerate() {
            let Some(kind) = projectile.kind else {
                continue;
            };
            let guided = kind == Weapon::Missile;

            // A missile's speed is pinned to its ramp every tick rather than
            // integrated, so the whole flight needs to know how old it is. Age
            // is derived from the lifetime rather than stored beside it: the two
            // would be one number written twice, and the second one is what goes
            // wrong.
            let age = MAX_FLIGHT_SECONDS - projectile.lifetime;
            let pinned_kmh = missile.filter(|_| guided).map(|stats| {
                missile::speed_kmh(projectile.launch_speed_kmh, stats.speed_for(class), age)
            });

            let from = projectile.position;
            let mut to = from + projectile.velocity * dt;

            // **The surface probe, before the flight sweep** - the order is the
            // original's. Look along the normal being ridden for something to
            // ride; conform to it if it is floor-like, fall if there is nothing.
            //
            // A missile looks **twice as far** as a rocket: `Missile_Update`
            // scales its normal by 12.0 where `Rocket_Update` uses 6.0. Two
            // recovered numbers, so two constants.
            let probe_length = if guided {
                missile::SURFACE_PROBE_LENGTH
            } else {
                SURFACE_PROBE_LENGTH
            };
            let probe = Raycaster::raycast(
                raycaster,
                Ray::new(to, -projectile.surface, probe_length),
                None,
                false,
            );
            match probe {
                // Rideable: sit at the ride height above it, adopt its normal,
                // and turn the velocity parallel to it without changing speed.
                // Turning rather than reflecting is what makes a projectile
                // *follow* a rolling track instead of bouncing down it.
                Some(hit) if hit.normal.dot(projectile.surface) > RIDEABLE_COS => {
                    projectile.surface = hit.normal;
                    to = hit.point + hit.normal * RIDE_HEIGHT;
                    // A missile re-pins its speed to the ramp here rather than
                    // preserving what it had - `Missile_Update` normalises and
                    // rescales on this exact branch, and by a **divide** by 3.6
                    // where its guidance path multiplies by a bit pattern that is
                    // not quite 1/3.6. Both roundings are the original's.
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
                // Nothing under it, or only something too steep to ride: it
                // falls. The projectile keeps whatever normal it had, so it
                // resumes riding when the track comes back under it.
                _ => projectile.velocity -= Vec3::Y * FALL_ACCELERATION * dt,
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
            if let Some((point, struck, normal)) = hit {
                // **A missile glances off a wall; a rocket dies on it.** The
                // original's missile counts wall hits at `self+0x6c`, mirrors its
                // velocity about the hit normal with no restitution loss, pushes
                // out along that normal, and only takes the detonating branch
                // once the count reaches `MAX_BOUNCES`. A hull hit is a different
                // collision code and always detonates, which is why this arm asks
                // for `struck.is_none()`.
                let may_bounce =
                    guided && struck.is_none() && projectile.bounces < missile::MAX_BOUNCES;
                if may_bounce {
                    projectile.bounces += 1;
                    projectile.velocity -= normal * (2.0 * projectile.velocity.dot(normal));
                    projectile.position = point + normal * missile::BOUNCE_PUSH_OFF;
                    bounced = true;
                } else {
                    impacts[index] = Some(Impact {
                        point,
                        kind,
                        owner: projectile.owner,
                        struck,
                    });
                    *projectile = Projectile::default();
                    continue;
                }
            } else {
                projectile.position = to;
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
            if projectile.lifetime <= 0.0 {
                // Reaped, not detonated: nothing was struck, so nothing takes a
                // blast. A projectile that leaves the world simply stops
                // existing. **Ours** - the original's missile has no lifetime cap
                // at all, only the bounce budget, so this is the same safety net
                // the Rocket already had rather than a recovered rule.
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
) -> Option<(Vec3, Option<u8>, Vec3)> {
    let direction = (to - from) / distance;

    // `include_reset` is false: a `Reset Collision` volume is a respawn trigger
    // rather than a surface, and a rocket detonating on one would blow up in
    // mid-air over the run-off.
    //
    // **Only a face-on hit stops it.** A projectile riding the track clips the
    // floor constantly - that is what riding it means - and detonating on those
    // is exactly the bug this model exists to fix: before it, three rockets
    // died in the tick they were fired. A grazing hit is the floor and is
    // ignored here, having already been handled by the surface probe; a hit the
    // projectile runs *into* is a wall and stops it. See [`WALL_FACING`].
    let mut best = Raycaster::raycast(raycaster, Ray::new(from, direction, distance), None, false)
        .filter(|hit| -hit.normal.dot(direction) > WALL_FACING)
        .map(|hit| (hit.distance, hit.point, None, hit.normal));

    for (slot, ship) in ships.iter().enumerate() {
        if !ship.active || slot as u8 == owner {
            continue;
        }
        let radius = hull_radius(&ship.handling.dimensions);
        let Some(t) = segment_sphere(from, to, ship.physics.body.position, radius) else {
            continue;
        };
        let travelled = t * distance;
        if best.is_none_or(|(nearest, _, _, _)| travelled < nearest) {
            // A hull hit carries the incoming direction reversed where a geometry
            // hit carries a surface normal. Nothing reads it - a hull hit always
            // detonates, never bounces - and it is here so the tuple has one
            // shape rather than an `Option` nobody unwraps.
            best = Some((
                travelled,
                from + direction * travelled,
                Some(slot as u8),
                -direction,
            ));
        }
    }

    best.map(|(_, point, struck, normal)| (point, struck, normal))
}

/// Flies the world's projectiles and detonates whatever stopped, in one call.
///
/// The seam a race loop wants: `oag-gameplay` owns both halves - the array and
/// the weapon table that says what a hit is worth - and the composition root
/// owns neither. Returns the impacts so a caller can put a spark where each one
/// landed, which is the only thing left for it to do.
///
/// `weapons` is `None` for a race whose weapon table did not load, in which case
/// an impact does nothing but free its slot. That is the same "gated on the
/// table" rule the pickup grant follows, and for the same reason: a blast with
/// no authored radius or damage would be an invented number. The same is true per
/// *weapon*: a table that authors no Missile gives a missile impact no blast.
///
/// # The blast is looked up per weapon, and it used to not be
///
/// This took `Option<&RocketStats>` and spent it on **every** impact whatever
/// `Impact::kind` said, which was invisible while the Rocket was the only thing
/// in the air and would have quietly given a missile the rocket's `damage`,
/// `blastradius` and `blastforce`. The two differ on the shipped disc - the
/// Missile hits harder and pushes harder - so the bug would have read as a tuning
/// disagreement rather than as a wiring one.
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
    weapons: Option<&oag_formats::weapons::WeaponStats>,
    class: oag_formats::handling::SpeedClass,
    rules: oag_physics::DamageRules,
) -> [Option<Impact>; MAX_PROJECTILES] {
    let count = world.ship_count as usize;
    let missile_stats = weapons.and_then(oag_formats::weapons::WeaponStats::missile);
    let impacts = world.projectiles.advance(
        dt,
        raycaster,
        &world.ships[..count],
        missile_stats.as_ref(),
        class,
    );

    for impact in impacts.iter().flatten() {
        let Some((radius, damage, force)) = blast_stats(weapons, impact.kind) else {
            continue;
        };
        blast(
            &mut world.ships[..count],
            impact.point,
            radius,
            damage,
            force,
            rules,
        );
    }

    impacts
}

/// One weapon's `blastradius`, `damage` and `blastforce`, or `None`.
///
/// `None` for a table that did not load, for a weapon it does not author, and for
/// a weapon whose block this crate does not decode - all three are the same
/// answer to the caller, which is "spend no blast". A weapon that reaches an
/// impact with no authored numbers is a bug upstream in
/// [`crate::pickup::IMPLEMENTED`], not something to paper over with a default.
fn blast_stats(
    weapons: Option<&oag_formats::weapons::WeaponStats>,
    kind: Weapon,
) -> Option<(f32, f32, f32)> {
    let weapons = weapons?;
    match kind {
        Weapon::Rocket => weapons
            .rocket()
            .map(|s| (s.blastradius, s.damage, s.blastforce)),
        Weapon::Missile => weapons
            .missile()
            .map(|s| (s.blastradius, s.damage, s.blastforce)),
        _ => None,
    }
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

/// How many rockets one press puts in the air.
///
/// **Recovered, confidence 88.** `Weapon_FireRocket` (`0x0886e104`) makes three
/// literal calls to one spawn helper in a single invocation, with no timer
/// between them: one through the craft's own matrix, one through it rotated by
/// `+spread`, one by `-spread`. See
/// `docs/ghidra/functions/psp-pulse-usa/weapon-fire.md`.
pub const ROCKET_SHOTS: usize = 3;

/// How many km/h one world unit per second is, for the authored weapon speeds.
///
/// **Recovered, confidence 84**, as the divisor both of
/// `Rocket_SpeedForClass`'s callers apply - see [`launch`]. It is
/// [`oag_core::math::SPEED_TO_KMH`], the same physical conversion the exhaust
/// ramp and the HUD's readout run the other way, named for this direction here.
///
/// Kept as a division rather than a multiply by a baked reciprocal, which is
/// what `Rocket_Update` itself does and which keeps the arithmetic one
/// operation instead of two roundings.
pub const KMH_PER_UNIT_PER_SECOND: f32 = oag_core::math::SPEED_TO_KMH;

/// How far a projectile looks toward the surface for something to ride.
///
/// **Recovered, confidence 82.** `Rocket_Update` (`0x0885d2a8`) scales the
/// stored normal by `6.0` and casts along it from the projected position, every
/// tick, before anything else. See
/// `docs/ghidra/functions/psp-pulse-usa/rocket-visuals.md`.
pub const SURFACE_PROBE_LENGTH: f32 = 6.0;

/// How far above the surface a projectile rides.
///
/// **Recovered, confidence 82.** The same function pushes out along the hit
/// normal by `3.0` after a surface hit, which is what holds a projectile off the
/// track rather than in it.
pub const RIDE_HEIGHT: f32 = 3.0;

/// How fast a projectile with nothing under it falls, in units per second squared.
///
/// **Recovered, confidence 82.** `velocity.y -= dt * 50.0` on the branch where
/// the surface probe finds nothing - a projectile that flies off the edge of the
/// track drops instead of sailing on forever.
pub const FALL_ACCELERATION: f32 = 50.0;

/// How square-on a geometry hit must be to count as a wall rather than the floor.
///
/// **Ours, and the one judgement call in the surface-following model.** The
/// original distinguishes "detonate" from "deflect" by a *collision code* its
/// query returns (`0` and `4` detonate, other non-zero values deflect) and this
/// engine's raycaster has no such code. What it has is the hit normal, so the
/// test is geometric: a projectile skimming the floor meets it edge-on, and one
/// flying into a wall meets it face-on. `0.25` is about 75 degrees off the
/// surface - generous, because a false *wall* stops a shot dead and a false
/// *floor* only lets it skim one more tick.
pub const WALL_FACING: f32 = 0.25;

/// How closely a probed surface must match the one being ridden to be ridden too.
///
/// **Ours.** Stops a projectile from treating a wall it happens to probe into as
/// a new floor and climbing it. `0.5` is 60 degrees, which passes any bank or
/// roll a Pulse circuit authors and rejects anything vertical.
pub const RIDEABLE_COS: f32 = 0.5;

/// Where a craft launches its rockets from, and how fast.
///
/// Returns [`ROCKET_SHOTS`] `(position, velocity)` pairs in the original's own
/// order - straight ahead, `+spread`, `-spread` - because that order decides
/// which projectile slot each lands in, and the slot is hashed state.
///
/// # What is recovered here, and what is not
///
/// **Recovered.** That there are three; that they leave *together* rather than
/// as a burst; that they share the craft's position and differ only by a
/// rotation built from `<Rocket spread>` and its negation; and that `spread` is
/// an angle in radians (from the `vcst_s(5)` = `2/pi` the original multiplies by
/// before `vcos_s`/`vsin_s`, which is Allegrex's radians-to-quarter-turns
/// conversion).
///
/// **Ours.**
///
/// - **The axis the fan rotates about.** The original builds its rotation
///   through four `vpfxs`-prefixed lanes and reading the axis back off the
///   prefixes was not attempted. The craft's **up** axis is what a lateral
///   spread of forward-firing rockets wants, and it is what this uses.
/// - **The launch offset.** The original passes the craft's pose for the
///   position and varies only the matrix, so all three share an origin - that
///   part is recovered. Pushing that origin forward by the hull's own extent,
///   so a rocket starts outside the craft that fired it, is this engine's, and
///   it uses the craft's *unrotated* forward so the three still share it.
/// - **The speed being the class's plus `launchSpeed`** rather than one or the
///   other, and the craft's own velocity not being inherited. The original's
///   flight speed is the class's **alone** - neither `Rocket_Init` nor
///   `Rocket_Update` mentions `launchSpeed` - so the sum is this engine's
///   choice and stays one, flagged here rather than quietly corrected: what
///   `launchSpeed` *is* for has not been found.
/// - **Treating `launchSpeed` as km/h too.** The four class speeds are measured
///   (see below); `launchSpeed` is authored in the same `<Stats>` block and in
///   the same range, so it is converted with them. Nothing reads it, so nothing
///   confirms it.
///
/// # The authored speeds are km/h, not units per second
///
/// **Recovered, confidence 84** - `docs/ghidra/functions/psp-pulse-usa/rocket-visuals.md`.
/// `Rocket_SpeedForClass` (`0x0885d1b0`) returns the authored float for the
/// current class and does no arithmetic, and **both** of its callers divide by
/// `3.6` before it becomes a velocity. Spending the numbers as units per
/// second, which this did until 2026-08-11, flies a rocket **3.6x too fast**:
/// the disc authors `venomspeed="800" launchSpeed="200"`, so a Venom rocket ran
/// at `1000` units/s, which the HUD's own `* 3.6` would read as **3600 km/h**
/// against a craft that tops out near 600. Converted it is `278` units/s, or
/// 1000 km/h - faster than the craft, which is what a rocket should be.
///
/// The conversion is at the call site rather than inside
/// [`RocketStats::speed_for`] on purpose, mirroring the original: the lookup
/// hands back the authored figure and the consumer spends it.
#[must_use]
pub fn launch(
    state: &ShipState,
    dimensions: &Dimensions,
    stats: &RocketStats,
    class: oag_formats::handling::SpeedClass,
) -> [(Vec3, Vec3); ROCKET_SHOTS] {
    let forward = state.body.forward();
    let up = state.body.up();
    let nose = state.body.position
        + forward * oag_physics::wall::hull_extent(&state.body, dimensions, forward);
    let speed = (stats.speed_for(class) + stats.launch_speed) / KMH_PER_UNIT_PER_SECOND;

    // The original's own order. A zero `spread` collapses all three onto the
    // same ray rather than erroring: that is a file that authors no fan, not a
    // broken weapon.
    [0.0, stats.spread, -stats.spread].map(|angle| {
        let direction = oag_core::math::quat_from_axis_angle(up, angle) * forward;
        (nose, direction * speed)
    })
}

#[cfg(test)]
mod tests;
