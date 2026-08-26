//! What projectile flight, impact and blast in [`super`] are asserted to do.
//!
//! Split out of `projectile.rs` under the 200-line cap on inline `#[cfg(test)]`
//! modules; see `scripts/check-file-size.py`.

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

/// A large horizontal floor, wound so its normal points up.
fn floor_at_y(y: f32) -> CollisionWorld {
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
        Surface::Floor,
        0,
    ));
    world
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

/// Over nothing at all, a projectile holds its heading and falls.
///
/// **This test used to assert the opposite** - "nothing pulls a rocket
/// down" - and it was pinning the bug rather than a behaviour. With no
/// surface under it the original accelerates a projectile downward at
/// [`FALL_ACCELERATION`], so an empty world is the *falling* case, not the
/// straight-line one. The straight line is what a projectile does across
/// the two axes it is not being pulled along.
#[test]
fn over_empty_space_a_projectile_keeps_its_heading_and_falls() {
    let mut projectiles = Projectiles::new();
    projectiles.spawn(Weapon::Rocket, Vec3::ZERO, Vec3::Z * 600.0, 0);
    let world = empty_world();
    let dt = 1.0 / 60.0;

    // From tick 2: `to` is computed from the velocity the *previous* tick
    // left, so the first tick's gravity does not move it yet. That lag is
    // the original's own ordering - it projects the position, then probes,
    // then accelerates - and is kept rather than tidied.
    let mut last_drop = 0.0;
    for tick in 1..=10 {
        let impacts = projectiles.advance(
            dt,
            &world,
            &ships(&[]),
            None,
            None,
            oag_formats::handling::SpeedClass::Venom,
        );
        assert!(impacts.iter().all(Option::is_none), "nothing to hit");
        let p = projectiles.slots[0];
        // Forward is untouched: gravity is on Y and nothing steers.
        assert!((p.position.z - 10.0 * tick as f32).abs() < 1e-3, "{p:?}");
        assert_eq!(p.position.x, 0.0, "nothing pushes it sideways");
        // Falling, and falling *faster* every tick - an acceleration, not a
        // fixed sink rate.
        let drop = -p.position.y;
        if tick > 1 {
            assert!(
                drop > last_drop,
                "tick {tick}: expected to keep falling, was {last_drop}, now {drop}"
            );
        }
        last_drop = drop;
    }
}

/// A projectile fired flat over a floor rides it instead of hitting it.
///
/// The regression that matters: before the surface-following model, a
/// volley fired on a real track died in the tick it was fired, because a
/// projectile skimming the floor was detonating on it. Here the floor is
/// directly below and the projectile must survive, settle to
/// [`RIDE_HEIGHT`] above it, and keep its speed.
#[test]
fn a_projectile_over_a_floor_rides_it_rather_than_detonating() {
    let mut projectiles = Projectiles::new();
    projectiles.spawn(Weapon::Rocket, Vec3::Y * 4.0, Vec3::Z * 240.0, 0);
    let world = floor_at_y(0.0);
    let dt = 1.0 / 60.0;

    for tick in 1..=30 {
        let impacts = projectiles.advance(
            dt,
            &world,
            &ships(&[]),
            None,
            None,
            oag_formats::handling::SpeedClass::Venom,
        );
        assert!(
            impacts.iter().all(Option::is_none),
            "tick {tick}: the floor is not a target"
        );
    }
    let p = projectiles.slots[0];
    assert_eq!(p.kind, Some(Weapon::Rocket), "it must still be in the air");
    assert!(
        (p.position.y - RIDE_HEIGHT).abs() < 0.5,
        "expected to settle near the ride height, at {:?}",
        p.position
    );
    assert!(
        (p.velocity.length() - 240.0).abs() < 1.0,
        "riding must not change speed, got {}",
        p.velocity.length()
    );
    assert!(p.position.z > 100.0, "it must have gone somewhere: {p:?}");
}

/// The swept test, and the reason it exists: at an authored class speed a
/// rocket covers more than a tick's worth of wall in one step, so a point
/// test would pass through.
#[test]
fn a_rocket_hits_a_wall_it_would_tunnel_through_in_one_tick() {
    let mut projectiles = Projectiles::new();
    // Deliberately faster than any real rocket - the disc's quickest class
    // authors `1100` km/h, or `306` units a second - because this is the
    // tunnelling guard and it should hold with headroom. At 60 Hz, 800
    // units a second is 13 units a tick against a wall of no thickness.
    projectiles.spawn(Weapon::Rocket, Vec3::ZERO, Vec3::Z * 800.0, 0);
    let world = wall_at_z(20.0);
    let dt = 1.0 / 60.0;

    let mut impact = None;
    for _ in 0..10 {
        let impacts = projectiles.advance(
            dt,
            &world,
            &ships(&[]),
            None,
            None,
            oag_formats::handling::SpeedClass::Venom,
        );
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
            .advance(
                1.0 / 60.0,
                &world,
                &grid,
                None,
                None,
                oag_formats::handling::SpeedClass::Venom,
            )
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
        let impacts = projectiles.advance(
            1.0 / 60.0,
            &world,
            &grid,
            None,
            None,
            oag_formats::handling::SpeedClass::Venom,
        );
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
            .advance(
                1.0 / 60.0,
                &world,
                &grid,
                None,
                None,
                oag_formats::handling::SpeedClass::Venom,
            )
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
        let impacts = projectiles.advance(
            dt,
            &world,
            &ships(&[]),
            None,
            None,
            oag_formats::handling::SpeedClass::Venom,
        );
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
        &mut [],
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
        &mut [],
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
    .expect("the fixture parses");

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
            oag_formats::handling::SpeedClass::Venom,
            oag_physics::DamageRules::default(),
            &mut [],
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
            oag_formats::handling::SpeedClass::Venom,
            oag_physics::DamageRules::default(),
            &mut [],
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

/// The volley: three rockets, together, fanned by `spread` - and the
/// middle one dead ahead.
///
/// **This is the recovered shape**, so it is asserted as a shape rather
/// than loosely: three shots, one origin, one speed, and the outer two
/// symmetric about the craft's forward axis by the authored half-angle.
/// A test that only counted three would pass with all three on the same ray.
#[test]
fn a_launch_fires_three_fanned_about_the_craft_forward() {
    use oag_formats::handling::SpeedClass;

    let state = ShipState::default();
    let dimensions = Dimensions {
        length: 4.0,
        width: 2.0,
        height: 1.0,
        ..Dimensions::default()
    };
    // `spread` of 0.25 rad is about 14 degrees to each side.
    let stats = oag_formats::weapons::parse(
        r#"<WeaponStats>
                 <Weapon type="Global"><Stats slowdown_limit="0"/></Weapon>
                 <Weapon type="Rocket"><Stats absorb="1" blastforce="2" blastradius="3"
                   damage="4" slowdown_time="5" venomspeed="600" flashspeed="700"
                   rapierspeed="800" phantomspeed="900" launchSpeed="50" spread="0.25"/></Weapon>
               </WeaponStats>"#,
    )
    .expect("the fixture parses")
    .rocket()
    .expect("a Rocket");

    let shots = launch(&state, &dimensions, &stats, SpeedClass::Venom);
    assert_eq!(shots.len(), ROCKET_SHOTS);

    let forward = state.body.forward();
    for (nose, velocity) in shots {
        // One origin, ahead of the hull, shared by all three - the original
        // varies the matrix and not the pose.
        assert_eq!(nose, shots[0].0, "the three must share an origin");
        assert!(
            (nose - state.body.position).dot(forward) > 0.0,
            "the launch point is behind the craft: {nose:?}"
        );
        // One speed, unchanged by the fan: the class's plus `launchSpeed`,
        // converted out of the km/h the file authors them in. Spelled as
        // the arithmetic rather than as `180.55` so the unit is legible -
        // this assertion is the guard against the 3.6x reappearing.
        assert!(
            (velocity.length() - (600.0 + 50.0) / KMH_PER_UNIT_PER_SECOND).abs() < 1e-2,
            "expected (600 + 50) km/h as units per second, got {}",
            velocity.length()
        );
    }

    // The middle one is dead ahead, and the outer two are symmetric about
    // it by the authored half-angle.
    let angle = |v: Vec3| forward.angle_between(v.normalize());
    assert!(angle(shots[0].1) < 1e-5, "the first shot must fly straight");
    assert!(
        (angle(shots[1].1) - 0.25).abs() < 1e-4,
        "expected 0.25 rad, got {}",
        angle(shots[1].1)
    );
    assert!(
        (angle(shots[2].1) - 0.25).abs() < 1e-4,
        "expected 0.25 rad, got {}",
        angle(shots[2].1)
    );
    // ...and to *opposite* sides, which is what a fan is. Comparing against
    // the craft's own right axis, because two shots at the same angle from
    // forward could both be to the left.
    let right = state.body.right();
    assert!(
        shots[1].1.dot(right) * shots[2].1.dot(right) < 0.0,
        "the two outer rockets went the same way"
    );
}

/// A file that authors no fan is a file with three rockets on one ray, not
/// an error - and not a crash from normalising a zero.
#[test]
fn a_zero_spread_still_fires_three() {
    use oag_formats::handling::SpeedClass;

    let stats = oag_formats::weapons::parse(
        r#"<WeaponStats>
                 <Weapon type="Global"><Stats slowdown_limit="0"/></Weapon>
                 <Weapon type="Rocket"><Stats absorb="1" blastforce="2" blastradius="3"
                   damage="4" slowdown_time="5" venomspeed="600" flashspeed="700"
                   rapierspeed="800" phantomspeed="900" launchSpeed="0" spread="0"/></Weapon>
               </WeaponStats>"#,
    )
    .expect("parses")
    .rocket()
    .expect("a Rocket");

    let shots = launch(
        &ShipState::default(),
        &Dimensions::default(),
        &stats,
        SpeedClass::Venom,
    );
    assert_eq!(shots.len(), ROCKET_SHOTS);
    for (_, velocity) in shots {
        assert!(velocity.is_finite(), "{velocity:?}");
        assert!((velocity - shots[0].1).length() < 1e-4);
    }
}

/// A Missile's stats, with a wide lock window. Every number invented, ADR-0006.
fn missile_stats() -> oag_formats::weapons::MissileStats {
    oag_formats::weapons::parse(
        r#"<WeaponStats>
             <Weapon type="Global"><Stats slowdown_limit="0"/></Weapon>
             <Weapon type="Missile"><Stats absorb="1" blastforce="10" blastradius="12"
               damage="25" slowdown_time="1" venomspeed="600" flashspeed="700"
               rapierspeed="800" phantomspeed="900" launchSpeed="100"
               lock_min_dist="10" lock_max_dist="200"/></Weapon>
           </WeaponStats>"#,
    )
    .expect("the fixture parses")
    .missile()
    .expect("a Missile")
}

/// **The Missile's sharpest departure from the Rocket**: it mirrors off a wall
/// and carries on, where a rocket detonates on the first face-on hit.
///
/// The reflection is a perfect mirror with no restitution loss, so the speed out
/// matches the speed in - asserted, because a reflection that quietly halved the
/// speed would still look like a bounce.
#[test]
fn a_missile_mirrors_off_a_wall_where_a_rocket_detonates() {
    let stats = missile_stats();
    let world = wall_at_z(60.0);
    let class = oag_formats::handling::SpeedClass::Venom;
    let ships: Vec<crate::world::Ship> = Vec::new();

    let mut projectiles = Projectiles::new();
    // Launch speed equal to the Venom class speed, so the one-second ramp is flat
    // and the only thing that can change the magnitude is the mirror itself.
    projectiles.spawn_guided(Weapon::Missile, Vec3::ZERO, Vec3::Z * 200.0, 0, None, 600.0);
    let mut before = 0.0;
    let mut bounced = None;
    for _ in 0..40 {
        let live = projectiles.slots[0];
        if live.kind.is_some() {
            before = live.velocity.length();
        }
        projectiles.advance(1.0 / 60.0, &world, &ships, Some(&stats), None, class);
        let after = projectiles.slots[0];
        if after.kind.is_some() && after.bounces > 0 {
            bounced = Some((before, after));
            break;
        }
    }
    let (speed_in, after) = bounced.expect("the missile never bounced off the wall");
    assert_eq!(after.bounces, 1);
    assert!(
        after.velocity.z < 0.0,
        "it did not turn around: {:?}",
        after.velocity
    );
    // The mirror itself is lossless, but the tick that contains it also contains
    // one step of the fall term - there is no floor here, so nothing pins the
    // speed - so the bound is one tick of gravity rather than zero.
    assert!(
        (after.velocity.length() - speed_in).abs() < FALL_ACCELERATION / 60.0 + 1e-3,
        "the mirror lost or gained more than gravity explains: {} out against \
         {speed_in} in",
        after.velocity.length()
    );

    // The same shot as a Rocket dies on that wall rather than coming back.
    let mut rockets = Projectiles::new();
    rockets.spawn(Weapon::Rocket, Vec3::ZERO, Vec3::Z * 200.0, 0);
    for _ in 0..40 {
        rockets.advance(1.0 / 60.0, &world, &ships, Some(&stats), None, class);
    }
    assert_eq!(
        rockets.live(),
        0,
        "a rocket survived a wall, so the bounce is not Missile-only"
    );
}

/// The bounce budget is finite: past `MAX_BOUNCES` a missile detonates like
/// anything else. Without this a missile that found a corner could ricochet for
/// its whole lifetime.
#[test]
fn a_missile_gives_up_after_its_bounce_budget() {
    let stats = missile_stats();
    // Two walls facing each other, so a missile between them keeps hitting one.
    //
    // **Both are built here rather than from `wall_at_z`**, for two reasons that
    // each cost a run. The far one must be wound the *other* way: the raycaster is
    // single-sided, so a copy of `wall_at_z` moved to the far end faces away from
    // anything flying toward it and is simply not there - the symptom was one
    // bounce and then a missile sailing off through the scenery. And both must be
    // very tall: nothing pins a missile's speed with no floor under it, so the
    // fall term runs for the whole flight, and at `wall_at_z`'s 100-unit
    // half-height the missile drops out of the bottom of the corridor after three
    // traverses.
    let mut world = CollisionWorld::new();
    let tall = 10_000.0;
    for (z, indices) in [
        (60.0, [[0, 1, 2], [0, 2, 3]]),
        (-60.0, [[0, 2, 1], [0, 3, 2]]),
    ] {
        world.push(TriangleSoup::new(
            vec![
                [-100.0, -tall, z],
                [-100.0, tall, z],
                [100.0, tall, z],
                [100.0, -tall, z],
            ],
            indices.to_vec(),
            Vec::new(),
            Surface::Wall,
            0,
        ));
    }
    let class = oag_formats::handling::SpeedClass::Venom;
    let ships: Vec<crate::world::Ship> = Vec::new();

    let mut projectiles = Projectiles::new();
    // Launch speed equal to the Venom class speed, so the one-second ramp is flat
    // and the only thing that can change the magnitude is the mirror itself.
    projectiles.spawn_guided(Weapon::Missile, Vec3::ZERO, Vec3::Z * 200.0, 0, None, 600.0);
    let mut highest = 0;
    for _ in 0..600 {
        projectiles.advance(1.0 / 60.0, &world, &ships, Some(&stats), None, class);
        highest = highest.max(projectiles.slots[0].bounces);
        if projectiles.live() == 0 {
            break;
        }
    }
    assert_eq!(projectiles.live(), 0, "the missile ricocheted forever");
    assert_eq!(
        highest,
        super::missile::MAX_BOUNCES,
        "it gave up after {highest} bounces rather than {}",
        super::missile::MAX_BOUNCES
    );
}
