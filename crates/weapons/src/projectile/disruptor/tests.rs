//! What the Disruptor's flight in [`super`] is asserted to do. Its own file
//! for the reason every other weapon's is: the inline test ceiling.

use super::*;
use crate::projectile::{MAX_FLIGHT_SECONDS as SAFETY_NET, Projectiles, TriggerRadii};
use crate::test_craft::Ship;
use oag_physics::{CollisionWorld, Surface, TriangleSoup};
use oag_tables::weapons::{DisruptorEffect, DisruptorEffectKind as Kind, DisruptorStats};

const TICK: f32 = 1.0 / 60.0;

fn ship_at(position: Vec3, active: bool) -> Ship {
    let mut ship = Ship {
        active,
        ..Ship::default()
    };
    ship.physics.body.position = position;
    ship
}

fn quad(y: f32, surface: Surface) -> CollisionWorld {
    let mut world = CollisionWorld::new();
    world.push(TriangleSoup::new(
        vec![
            [-500.0, y, -500.0],
            [-500.0, y, 500.0],
            [500.0, y, 500.0],
            [500.0, y, -500.0],
        ],
        vec![[0, 1, 2], [0, 2, 3]],
        Vec::new(),
        surface,
        0,
    ));
    world
}

/// The default body faces `-Z`, so everything below flies that way.
const FORWARD: Vec3 = Vec3::NEG_Z;

/// A wall across `z`.
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

fn stats(speed: f32) -> DisruptorStats {
    let mut effects = [None; 10];
    effects[Kind::Stall as usize] = Some(DisruptorEffect {
        time: 2.0,
        amount: None,
        speed_percent: None,
    });
    DisruptorStats {
        absorb: 1.0,
        speed,
        effects,
    }
}

fn fire(projectiles: &mut Projectiles, target: Option<u8>) {
    let ship = ship_at(Vec3::ZERO, true);
    let (position, velocity, up) = launch(&ship.physics, &ship.handling.dimensions);
    assert!(projectiles.fire_disruptor(position, velocity, up, 0, target, Kind::Stall));
}

fn tick(
    projectiles: &mut Projectiles,
    world: &CollisionWorld,
    ships: &[Ship],
    stats: Option<&DisruptorStats>,
) -> Option<Impact> {
    projectiles.advance(
        TICK,
        world,
        ships,
        None,
        None,
        stats,
        TriggerRadii::default(),
        "VENOM",
    )[0]
}

/// 500 km/h along the craft's forward, from its centre, probing along its up.
#[test]
fn a_bolt_leaves_at_the_literal_launch_speed_carrying_its_effect() {
    let mut projectiles = Projectiles::new();
    fire(&mut projectiles, Some(3));
    let bolt = projectiles.slots[0];
    assert_eq!(bolt.kind, Some(Weapon::Disruptor));
    assert_eq!(bolt.effect, Some(Kind::Stall));
    assert_eq!(bolt.target, Some(3));
    assert_eq!(bolt.position, Vec3::ZERO);
    assert_eq!(bolt.surface, Vec3::Y);
    assert_eq!(bolt.velocity.normalize(), FORWARD);
    let kmh = bolt.velocity.length() * crate::projectile::KMH_PER_UNIT_PER_SECOND;
    assert!((kmh - LAUNCH_KMH).abs() < 0.01, "{kmh}");
    assert_eq!(bolt.lifetime, SAFETY_NET);
}

/// Over nothing the bolt falls at the recovered rate and keeps its heading.
#[test]
fn over_nothing_a_bolt_falls() {
    let mut projectiles = Projectiles::new();
    fire(&mut projectiles, None);
    let world = CollisionWorld::new();
    for _ in 0..10 {
        assert!(tick(&mut projectiles, &world, &[], None).is_none());
    }
    let bolt = projectiles.slots[0];
    assert!((bolt.velocity.y - (-FALL_ACCELERATION * TICK * 10.0)).abs() < 1e-3);
    assert!(bolt.position.z < 0.0 && bolt.position.x == 0.0);
}

/// Over a floor it settles toward the ride height and re-pins to the class
/// speed - `speed + 80` for Venom, the second rung.
#[test]
fn over_a_floor_a_bolt_rides_at_six_units_and_at_the_class_speed() {
    let mut projectiles = Projectiles::new();
    fire(&mut projectiles, None);
    let world = quad(-3.0, Surface::Floor);
    let stats = stats(300.0);
    for _ in 0..120 {
        assert!(tick(&mut projectiles, &world, &[], Some(&stats)).is_none());
    }
    let bolt = projectiles.slots[0];
    assert!(
        (bolt.position.y - (-3.0 + RIDE_HEIGHT)).abs() < 0.5,
        "{}",
        bolt.position.y
    );
    let kmh = bolt.velocity.length() * crate::projectile::KMH_PER_UNIT_PER_SECOND;
    assert!((kmh - 380.0).abs() < 1.0, "{kmh}");
}

/// A wall across the sweep ends the flight with no effect and no blast.
#[test]
fn a_wall_stops_a_bolt_and_takes_nothing() {
    let mut projectiles = Projectiles::new();
    fire(&mut projectiles, None);
    let world = wall_at_z(-20.0);
    let mut stopped = None;
    for _ in 0..60 {
        if let Some(impact) = tick(&mut projectiles, &world, &[], None) {
            stopped = Some(impact);
            break;
        }
    }
    let impact = stopped.expect("the wall was 20 units away at 139 units a second");
    assert_eq!(impact.kind, Weapon::Disruptor);
    assert_eq!(
        (impact.struck, impact.blast, impact.effect),
        (None, false, None)
    );
    assert_eq!(projectiles.live(), 0);
}

/// A craft inside the six-unit cylinder is struck, and the impact carries the
/// effect to it. The owner is never a target.
#[test]
fn a_craft_within_six_units_of_the_path_is_struck_with_the_effect() {
    let mut projectiles = Projectiles::new();
    fire(&mut projectiles, None);
    let ships = [
        ship_at(Vec3::ZERO, true),
        ship_at(Vec3::new(5.0, 0.0, -30.0), true),
    ];
    let world = CollisionWorld::new();
    let mut struck = None;
    for _ in 0..60 {
        if let Some(impact) = tick(&mut projectiles, &world, &ships, None) {
            struck = Some(impact);
            break;
        }
    }
    let impact = struck.expect("a craft five units off the line is inside the cylinder");
    assert_eq!(impact.struck, Some(1));
    assert_eq!(impact.effect, Some(Kind::Stall));
    assert!(impact.blast);

    // Seven units off is a miss.
    let mut projectiles = Projectiles::new();
    fire(&mut projectiles, None);
    let ships = [
        ship_at(Vec3::ZERO, true),
        ship_at(Vec3::new(7.0, 0.0, -30.0), true),
    ];
    for _ in 0..60 {
        assert!(tick(&mut projectiles, &world, &ships, None).is_none());
    }
}

#[test]
fn the_cylinder_has_a_unit_of_slack_past_each_end() {
    let from = Vec3::ZERO;
    let to = Vec3::Z * 10.0;
    assert!(cylinder_hit(from, to, Vec3::Z * 10.9));
    assert!(!cylinder_hit(from, to, Vec3::Z * 11.1));
    assert!(cylinder_hit(from, to, Vec3::Z * -0.9));
    assert!(!cylinder_hit(from, to, Vec3::Z * -1.1));
    assert!(cylinder_hit(from, to, Vec3::new(HIT_RADIUS, 0.0, 5.0)));
    assert!(!cylinder_hit(
        from,
        to,
        Vec3::new(HIT_RADIUS + 0.01, 0.0, 5.0)
    ));
    assert!(cylinder_hit(from, from, Vec3::X * 5.0));
}

/// A locked bolt turns toward its target a chord of `1.0 * dt` a tick.
#[test]
fn a_locked_bolt_homes_gently_and_holds_the_class_speed() {
    let speed = 100.0;
    let velocity = Vec3::Z * speed;
    let target = Vec3::new(50.0, 0.0, 50.0);
    let turned = steer(velocity, Vec3::ZERO, target, TICK, speed);
    // Toward +X by one chord of `TICK` along the unit error, at the pinned
    // speed (very slightly over, as the original does not renormalise).
    assert!(turned.x > 0.0);
    let error = (target.normalize() - Vec3::Z).normalize();
    let expected = (Vec3::Z + error * TICK) * speed;
    assert!((turned - expected).length() < 1e-3, "{turned:?}");
    // Already aligned: the heading is kept, the speed re-pinned.
    assert_eq!(
        steer(Vec3::Z * 5.0, Vec3::ZERO, Vec3::Z * 9.0, TICK, speed),
        Vec3::Z * speed
    );
    // Degenerate inputs come back untouched.
    assert_eq!(
        steer(Vec3::ZERO, Vec3::ZERO, Vec3::Z, TICK, speed),
        Vec3::ZERO
    );
}

/// Through the pool: a bolt with a lock on an active craft bends toward it,
/// and one whose target is gone flies straight.
#[test]
fn the_pool_guides_a_locked_bolt_and_not_an_orphaned_one() {
    let stats = stats(300.0);
    let world = CollisionWorld::new();
    let ships = [
        ship_at(Vec3::ZERO, true),
        ship_at(Vec3::new(40.0, 0.0, -200.0), true),
        ship_at(Vec3::new(40.0, 0.0, -200.0), false),
    ];
    let mut locked = Projectiles::new();
    fire(&mut locked, Some(1));
    let mut orphaned = Projectiles::new();
    fire(&mut orphaned, Some(2));
    for _ in 0..30 {
        tick(&mut locked, &world, &ships, Some(&stats));
        tick(&mut orphaned, &world, &ships, Some(&stats));
    }
    assert!(locked.slots[0].velocity.x > 0.0);
    assert_eq!(orphaned.slots[0].velocity.x, 0.0);
}

/// Ten seconds and it is gone, having hit nothing and told nobody.
#[test]
fn a_bolt_that_finds_nothing_is_reaped_at_ten_seconds() {
    let mut projectiles = Projectiles::new();
    fire(&mut projectiles, None);
    let world = CollisionWorld::new();
    let ticks = (MAX_FLIGHT_SECONDS / TICK) as usize;
    for _ in 0..ticks + 2 {
        assert!(tick(&mut projectiles, &world, &[], None).is_none());
    }
    assert_eq!(projectiles.live(), 0);
}
