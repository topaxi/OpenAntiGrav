//! What projectile flight, impact and blast in [`super`] are asserted to do.
//!
//! Split from `projectile.rs` under the 200-line cap on inline test modules.

use super::*;
use oag_physics::params::Dimensions;
use oag_physics::{CollisionWorld, ShipState, Surface, TriangleSoup};

mod owner_exempt;
mod seed;

pub(super) fn ships(entries: &[(bool, Vec3)]) -> Vec<crate::test_craft::Ship> {
    entries
        .iter()
        .map(|&(active, position)| {
            let mut ship = crate::test_craft::Ship {
                active,
                ..crate::test_craft::Ship::default()
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

/// Over nothing at all, a projectile holds its heading and falls: with no surface
/// the original accelerates it down at [`FALL_ACCELERATION`]. (This test once
/// asserted "nothing pulls a rocket down" and pinned the bug.)
#[test]
fn over_empty_space_a_projectile_keeps_its_heading_and_falls() {
    let mut projectiles = Projectiles::new();
    projectiles.spawn(Weapon::Rocket, Vec3::ZERO, Vec3::Z * 600.0, 0);
    let world = empty_world();
    let dt = 1.0 / 60.0;

    // From tick 2: `to` uses the previous tick's velocity (the original projects,
    // probes, then accelerates), so tick 1's gravity does not move it yet.
    let mut last_drop = 0.0;
    for tick in 1..=10 {
        let impacts = projectiles.advance(
            dt,
            &world,
            &ships(&[]),
            None,
            None,
            None,
            TriggerRadii::default(),
            "VENOM",
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

/// A projectile fired flat over a floor rides it instead of hitting it. Before
/// surface-following, a volley on a real track died in the tick it was fired. It
/// must survive, settle to [`RIDE_HEIGHT`] and keep its speed.
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
            None,
            TriggerRadii::default(),
            "VENOM",
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

/// The swept test: at an authored class speed a rocket covers more than a tick of
/// wall in one step, so a point test would pass through.
#[test]
fn a_rocket_hits_a_wall_it_would_tunnel_through_in_one_tick() {
    let mut projectiles = Projectiles::new();
    // Faster than any real rocket (the quickest class authors `1100` km/h, `306`
    // units a second): 800 units a second is 13 a tick against a wall of no
    // thickness, so the tunnelling guard holds with headroom.
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
            None,
            TriggerRadii::default(),
            "VENOM",
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

/// A hull hit reports which slot it struck, never the owner's own.
#[test]
fn a_rocket_strikes_a_craft_that_is_not_its_owner() {
    let mut projectiles = Projectiles::new();
    projectiles.spawn(Weapon::Rocket, Vec3::ZERO, Vec3::Z * 600.0, 0);
    // Slot 0 is the owner, on the flight path; a shot that could hit its launcher
    // would report slot 0 and stop at once. Slot 1 is further along.
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
                None,
                TriggerRadii::default(),
                "VENOM",
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

/// An inactive slot is not a craft: slots past `ship_count` hold what the last
/// race left, and a rocket must not detonate on one.
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
            None,
            TriggerRadii::default(),
            "VENOM",
        );
        assert!(
            impacts.iter().all(Option::is_none),
            "a rocket detonated on an empty grid slot"
        );
    }
}

/// The nearer of wall and hull wins, so a rocket cannot reach a craft through a
/// wall.
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
                None,
                TriggerRadii::default(),
                "VENOM",
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

/// A rocket that hits nothing frees its slot **without** an impact: nothing was
/// struck, so nothing takes a blast.
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
            None,
            TriggerRadii::default(),
            "VENOM",
        );
        assert!(
            impacts.iter().all(Option::is_none),
            "a reaped rocket must not blast"
        );
    }
    assert_eq!(projectiles.live(), 0, "the slot leaked");
}

/// `CannonPool_Update` reaps a round at `1.0 < age`, not the shared flight cap:
/// still in the air a tick short of one second, gone with no impact a tick after.
#[test]
fn a_cannon_round_that_hits_nothing_is_reaped_at_one_second() {
    let mut projectiles = Projectiles::new();
    projectiles.spawn(Weapon::Cannon, Vec3::ZERO, Vec3::Z * 600.0, 0);
    let world = empty_world();
    let dt = 1.0 / 60.0;

    let mut live_after = Vec::new();
    for _ in 0..120 {
        let impacts = projectiles.advance(
            dt,
            &world,
            &ships(&[]),
            None,
            None,
            None,
            TriggerRadii::default(),
            "VENOM",
        );
        assert!(
            impacts.iter().all(Option::is_none),
            "a reaped round must not report an impact"
        );
        live_after.push(projectiles.live());
    }
    // Tick 59 leaves the age at 59/60 s, tick 60 at 1.0 exactly (`1.0 < 1.0`
    // is false), tick 61 past it.
    assert_eq!(live_after[59], 1, "reaped before its second was up");
    assert_eq!(live_after[60], 0, "still flying at 1.0167 s");
    assert_eq!(*live_after.last().unwrap(), 0);
}

/// `Rocket_SweepProjectiles`: a rocket flying through a laid mine sets it off
/// quietly (explosion shown, nobody hurt) and is spent without detonating.
#[test]
fn a_rocket_flying_through_a_laid_mine_sets_it_off_and_is_spent() {
    let mut projectiles = Projectiles::new();
    projectiles.spawn(Weapon::Mine, Vec3::new(0.0, 0.0, 30.0), Vec3::ZERO, 1);
    projectiles.spawn(Weapon::Rocket, Vec3::ZERO, Vec3::Z * 600.0, 0);
    let world = empty_world();
    let dt = 1.0 / 60.0;
    let radii = TriggerRadii {
        mine: Some(5.0),
        bomb: None,
    };

    let mut mine_impacts = 0;
    for _ in 0..12 {
        let impacts =
            projectiles.advance(dt, &world, &ships(&[]), None, None, None, radii, "VENOM");
        for impact in impacts.iter().flatten() {
            assert_eq!(impact.kind, Weapon::Mine, "only the mine reports an ending");
            assert!(!impact.blast, "a mine a rocket set off must not blast");
            mine_impacts += 1;
        }
    }
    assert_eq!(mine_impacts, 1, "the mine went off {mine_impacts} times");
    assert_eq!(
        projectiles.live(),
        0,
        "the rocket or the mine outlived the meeting"
    );
}

/// The whole chain through [`step`]: fly, hit, blast. Catches the halves being
/// wired to each other wrongly.
#[test]
fn a_rocket_fired_at_a_parked_craft_takes_its_energy() {
    let stats = oag_tables::weapons::parse(
        r#"<WeaponStats>
                 <Weapon type="Global"><Stats slowdown_limit="0"/></Weapon>
                 <Weapon type="Rocket"><Stats absorb="1" blastforce="10" blastradius="12"
                   damage="25" slowdown_time="1" venomspeed="600" flashspeed="700"
                   rapierspeed="800" phantomspeed="900" launchSpeed="0" spread="1"/></Weapon>
               </WeaponStats>"#,
    )
    .expect("the fixture parses");

    let mut world = crate::test_craft::World::new(1);
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
            &mut world.projectiles,
            &mut world.ships[..world.ship_count as usize],
            1.0 / 60.0,
            &empty,
            Some(&stats),
            "VENOM",
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

/// A race whose weapon table did not load still flies and reaps rockets, but
/// nothing takes damage.
#[test]
fn without_rocket_stats_an_impact_only_frees_its_slot() {
    let mut world = crate::test_craft::World::new(1);
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
            &mut world.projectiles,
            &mut world.ships[..world.ship_count as usize],
            1.0 / 60.0,
            &wall,
            None,
            "VENOM",
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

/// The volley: three rockets, together, fanned by `spread`, the middle one dead
/// ahead. Asserted as a shape (one origin, one speed, the outer two symmetric
/// about forward by the authored half-angle): counting three would pass with all
/// on one ray.
#[test]
fn a_launch_fires_three_fanned_about_the_craft_forward() {
    let mut state = ShipState::default();
    state.body.position = Vec3::new(12.0, -3.0, 40.0);
    // `spread` of 0.25 rad is about 14 degrees to each side.
    let stats = oag_tables::weapons::parse(
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

    let shots = launch(&state, &stats, "VENOM").expect("the fixture authors a Venom rocket speed");
    assert_eq!(shots.len(), ROCKET_SHOTS);

    let forward = state.body.forward();
    for (origin, velocity) in shots {
        // One origin, the craft's own position (measured 2026-10-01, not the nose).
        assert_eq!(origin, shots[0].0, "the three must share an origin");
        assert_eq!(
            origin, state.body.position,
            "a rocket is laid at the craft's position"
        );
        // One speed, unchanged by the fan: 0.75 x the class's alone (measured
        // 2026-10-01; `launchSpeed="50"` plays no part), out of km/h. Spelled as
        // arithmetic so it guards against the 3.6x and `launchSpeed` joining in.
        assert!(
            (velocity.length() - 600.0 * 0.75 / KMH_PER_UNIT_PER_SECOND).abs() < 1e-2,
            "expected 0.75 x 600 km/h as units per second, got {}",
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
    // ...and to opposite sides: compared against the craft's right axis, as two
    // shots at the same angle from forward could both be left.
    let right = state.body.right();
    assert!(
        shots[1].1.dot(right) * shots[2].1.dot(right) < 0.0,
        "the two outer rockets went the same way"
    );
}

/// A file that authors no fan is three rockets on one ray, not an error or a
/// normalise-zero crash.
#[test]
fn a_zero_spread_still_fires_three() {
    let stats = oag_tables::weapons::parse(
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

    let shots = launch(&ShipState::default(), &stats, "VENOM")
        .expect("the fixture authors a Venom rocket speed");
    assert_eq!(shots.len(), ROCKET_SHOTS);
    for (_, velocity) in shots {
        assert!(velocity.is_finite(), "{velocity:?}");
        assert!((velocity - shots[0].1).length() < 1e-4);
    }
}

/// A Missile's stats, with a wide lock window. Every number invented, ADR-0006.
fn missile_stats() -> oag_tables::weapons::MissileStats {
    oag_tables::weapons::parse(
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

/// The Missile mirrors off a wall and carries on, where a rocket detonates on the
/// first face-on hit. The reflection is lossless, so speed out matches speed in
/// (asserted: a halving reflection would still look like a bounce).
#[test]
fn a_missile_mirrors_off_a_wall_where_a_rocket_detonates() {
    let stats = missile_stats();
    let world = wall_at_z(60.0);
    let class = "VENOM";
    let ships: Vec<crate::test_craft::Ship> = Vec::new();

    let mut projectiles = Projectiles::new();
    // Launch speed equal to the Venom class speed, so the one-second ramp is flat
    // and the only thing that can change the magnitude is the mirror itself.
    projectiles.spawn_guided(
        Weapon::Missile,
        Vec3::ZERO,
        Vec3::Z * 200.0,
        0,
        None,
        600.0,
        Vec3::Y,
    );
    let mut before = 0.0;
    let mut bounced = None;
    for _ in 0..40 {
        let live = projectiles.slots[0];
        if live.kind.is_some() {
            before = live.velocity.length();
        }
        projectiles.advance(
            1.0 / 60.0,
            &world,
            &ships,
            Some(&stats),
            None,
            None,
            TriggerRadii::default(),
            class,
        );
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
    // The mirror is lossless, but the tick also holds one step of the fall term
    // (no floor pins the speed), so the bound is one tick of gravity.
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
        rockets.advance(
            1.0 / 60.0,
            &world,
            &ships,
            Some(&stats),
            None,
            None,
            TriggerRadii::default(),
            class,
        );
    }
    assert_eq!(
        rockets.live(),
        0,
        "a rocket survived a wall, so the bounce is not Missile-only"
    );
}

/// The bounce budget is finite: past `MAX_BOUNCES` a missile detonates, rather
/// than ricocheting in a corner for its whole lifetime.
#[test]
fn a_missile_gives_up_after_its_bounce_budget() {
    let stats = missile_stats();
    // Two walls facing each other. Built here, not from `wall_at_z`: the raycaster
    // is single-sided, so the far wall must be wound the other way (else one bounce
    // and the missile sails off), and both must be very tall because no floor pins
    // the speed and the fall term would drop the missile out of the bottom after
    // three traverses.
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
    let class = "VENOM";
    let ships: Vec<crate::test_craft::Ship> = Vec::new();

    let mut projectiles = Projectiles::new();
    // Launch speed equal to the Venom class speed, so only the mirror can change
    // the magnitude.
    projectiles.spawn_guided(
        Weapon::Missile,
        Vec3::ZERO,
        Vec3::Z * 200.0,
        0,
        None,
        600.0,
        Vec3::Y,
    );
    let mut highest = 0;
    for _ in 0..600 {
        projectiles.advance(
            1.0 / 60.0,
            &world,
            &ships,
            Some(&stats),
            None,
            None,
            TriggerRadii::default(),
            class,
        );
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

/// A missile that locked nothing flies straight on and detonates on its own.
///
/// **Recovered** from `Projectiles_Update_q` (`0x08869588`): `3.0 < missile->age`
/// sets the destroy bit; the target is `None`, the original's null pointer, which
/// `Missile_Update` skips guidance on. Flown in an empty world so only the timer
/// can end it.
#[test]
fn an_unguided_missile_detonates_when_its_three_seconds_are_up() {
    let stats = missile_stats();
    let world = empty_world();
    let class = "VENOM";
    let ships: Vec<crate::test_craft::Ship> = Vec::new();

    let mut projectiles = Projectiles::new();
    projectiles.spawn_guided(
        Weapon::Missile,
        Vec3::ZERO,
        Vec3::Z * 200.0,
        0,
        None,
        600.0,
        Vec3::Y,
    );

    let mut ticks = 0_usize;
    let mut ended: Option<Impact> = None;
    while ended.is_none() {
        let impacts = projectiles.advance(
            1.0 / 60.0,
            &world,
            &ships,
            Some(&stats),
            None,
            None,
            TriggerRadii::default(),
            class,
        );
        ticks += 1;
        ended = impacts.into_iter().flatten().next();
        assert!(ticks < 600, "the missile never ended");
    }

    let impact = ended.expect("an impact");
    assert_eq!(impact.struck, None, "an empty world struck a craft");
    assert!(
        !impact.blast,
        "the self-detonation spent a blast the original's teardown never reaches"
    );

    // Three seconds as a literal, not the constant: an assertion computed from
    // `SELF_DETONATE_SECONDS` passes whatever it is changed to. The number is
    // `MissilePool_Update`'s own `3.0 < age`; if it moves, this should fail and be
    // re-read against the disassembly. Within two ticks, not exact: the age is
    // `MAX_FLIGHT_SECONDS - lifetime` and accumulates `f32` rounding over 180 ticks.
    let flown = ticks as f32 / 60.0;
    assert!(
        (flown - 3.0).abs() <= 2.0 / 60.0,
        "it flew {flown}s, not the recovered 3.0s"
    );
    assert_eq!(
        super::missile::SELF_DETONATE_SECONDS,
        3.0,
        "the constant and the number the flight is measured against have parted"
    );
}

/// And the self-detonation hurts nobody, however close they stand: the original's
/// damage (`FUN_08869054`) and blast force (`FUN_08868ea4`) each have two callers
/// and the expiry teardown is neither, reaching only the explosion spawner. See
/// [`Impact::blast`].
#[test]
fn a_self_detonating_missile_damages_nobody_standing_in_it() {
    let mut world = crate::test_craft::World::new(1);
    world.ship_count = 2;
    // Slot 1 is parked near where a missile fired down `+Z` runs out of time,
    // and well inside `blastradius` of it.
    for (slot, position) in [Vec3::ZERO, Vec3::Z * 500.0].into_iter().enumerate() {
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

    let table = oag_tables::weapons::parse(
        r#"<WeaponStats>
             <Weapon type="Global"><Stats slowdown_limit="0"/></Weapon>
             <Weapon type="Missile"><Stats absorb="1" blastforce="10" blastradius="12"
               damage="25" slowdown_time="1" venomspeed="600" flashspeed="700"
               rapierspeed="800" phantomspeed="900" launchSpeed="100"
               lock_min_dist="10" lock_max_dist="200"/></Weapon>
           </WeaponStats>"#,
    )
    .expect("the fixture parses");

    // Fired beside slot 1's line so the swept hull test cannot end the flight early.
    world.projectiles.spawn_guided(
        Weapon::Missile,
        Vec3::X * 40.0,
        Vec3::Z * 200.0,
        0,
        None,
        600.0,
        Vec3::Y,
    );

    let empty = empty_world();
    let mut ended = false;
    for _ in 0..600 {
        let impacts = step(
            &mut world.projectiles,
            &mut world.ships[..world.ship_count as usize],
            1.0 / 60.0,
            &empty,
            Some(&table),
            "VENOM",
            oag_physics::DamageRules::default(),
            &mut [crate::projectile::WeaponHit::default(); 2],
        );
        if let Some(impact) = impacts.into_iter().flatten().next() {
            assert!(!impact.blast, "the timer spent a blast");
            ended = true;
            break;
        }
    }
    assert!(ended, "the missile never ran out of time");
    assert_eq!(
        world.ships[1].physics.shield, 100.0,
        "a craft standing in a self-detonation took damage the original never deals"
    );
    assert_eq!(
        world.ships[0].physics.shield, 100.0,
        "the firing craft took damage from its own expired missile"
    );
}

/// A blade whose fuse runs out reports where it ended and spends no blast:
/// `ShurikenPool_Update`'s teardown plays `WO_SHURIKEN_EXPIRE` and a screen flash,
/// spending no damage.
#[test]
fn a_blade_whose_fuse_runs_out_reports_an_impact_that_spends_no_blast() {
    let geometry = CollisionWorld::new();
    let ships: Vec<crate::test_craft::Ship> = Vec::new();
    let mut projectiles = Projectiles::new();
    assert!(projectiles.throw(Vec3::ZERO, Vec3::Z * 50.0, 3, 2.0, Vec3::Y));

    let mut ticks = 0_usize;
    let mut ended = None;
    while ended.is_none() {
        let impacts = projectiles.advance(
            1.0 / 60.0,
            &geometry,
            &ships,
            None,
            None,
            None,
            TriggerRadii::default(),
            "VENOM",
        );
        ticks += 1;
        ended = impacts.into_iter().flatten().next();
        assert!(ticks < 300, "the blade never ended");
    }
    let impact = ended.expect("an impact");
    assert_eq!(impact.kind, Weapon::Shuriken);
    assert_eq!(impact.owner, 3);
    assert_eq!(impact.struck, None);
    assert!(!impact.blast, "a fuse spends no blast");
    assert!(
        (ticks as f32 / 60.0 - 2.0).abs() <= 2.0 / 60.0,
        "it flew {ticks} ticks, not its authored two seconds"
    );
    assert!(impact.point.z > 50.0, "the impact is where the blade was");
}
