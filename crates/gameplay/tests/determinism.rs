//! The determinism gate's race-level half, against committed constants.
//!
//! `crates/physics/tests/determinism.rs`'s twin, and it exists because that one
//! covers one craft's dynamics and nothing else. What a race carries *around*
//! the dynamics - the inventory, the projectiles, the lap state and the
//! generator's own position - was outside every committed hash until
//! [`oag_gameplay::hash::hash_world`] landed, which
//! `docs/gameplay/pickups.md` recorded as a known hole.
//!
//! # No disc image, deliberately
//!
//! The world here is built in this file: two craft, a synthetic wall, a rocket
//! fired down it. `data/` is gitignored and absent in CI, so a disc-backed run
//! would never execute on Windows or macOS - which is where a portability bug
//! shows up. The same argument `oag_physics::probe`'s module docs make.
//!
//! # What this does *not* cover, so nobody assumes it does
//!
//! The weapon pads' own refresh timers and distance caches, which live on
//! `oag_game::race::Race` rather than in the world - pad timers belong to the
//! track, not to a craft. They are guarded instead by
//! `Race::state_hash`'s tests in `crates/game/src/race.rs`, which run on every
//! `just` because that fixture needs no disc either. Between the two, nothing in
//! the pickup system is left to `just test-data`.

use oag_core::math::Vec3;
use oag_formats::weapons::Weapon;
use oag_gameplay::hash::hash_world;
use oag_gameplay::projectile;
use oag_gameplay::world::World;
use oag_physics::params::Dimensions;
use oag_physics::{CollisionWorld, DamageRules, Surface, TriangleSoup};

/// Our own fixed timestep. ADR-0007.
const TICK: f32 = 1.0 / 60.0;

/// A wall across the flight path, far enough that a rocket flies for a while
/// before reaching it.
fn corridor() -> CollisionWorld {
    let mut world = CollisionWorld::new();
    world.push(TriangleSoup::new(
        vec![
            [-200.0, -200.0, 400.0],
            [-200.0, 200.0, 400.0],
            [200.0, 200.0, 400.0],
            [200.0, -200.0, 400.0],
        ],
        vec![[0, 1, 2], [0, 2, 3]],
        Vec::new(),
        Surface::Wall,
        0,
    ));
    world
}

/// Invented numbers, per ADR-0006 - **not** any ship's or any weapon's.
fn rocket_stats() -> oag_formats::weapons::RocketStats {
    oag_formats::weapons::parse(
        r#"<WeaponStats>
             <Weapon type="Global"><Stats slowdown_limit="0"/></Weapon>
             <Weapon type="Rocket"><Stats absorb="1" blastforce="20" blastradius="30"
               damage="10" slowdown_time="1" venomspeed="500" flashspeed="600"
               rapierspeed="700" phantomspeed="800" launchSpeed="0" spread="1"/></Weapon>
           </WeaponStats>"#,
    )
    .expect("the fixture parses")
    .rocket()
    .expect("a Rocket")
}

/// The scenario: two craft, a rocket in the air, and a draw from the generator
/// every tick.
///
/// Each piece is there because it is a *different* part of the world, and a gate
/// that moved for only one of them would be claiming coverage it does not have:
///
/// - the **projectile array** flies, hits the wall at a tick nobody chose, and
///   frees its slot;
/// - the **blast** spends the second craft's energy pool and shoves it, which
///   reaches `oag_physics::probe::hash_state` through the ship;
/// - the **generator** is drawn from every tick, so its position advances
///   independently of anything visible;
/// - the **inventory and the lap state** are written on fixed ticks.
fn run(ticks: u32) -> (u64, u64) {
    let world_geometry = corridor();
    let stats = rocket_stats();

    let mut world = World::new(0xC0FFEE);
    world.ship_count = 2;
    for (slot, position) in [Vec3::ZERO, Vec3::Z * 380.0].into_iter().enumerate() {
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
        .spawn(Weapon::Rocket, Vec3::ZERO, Vec3::Z * 500.0, 0);

    let mut trajectory = oag_core::hash::StateHasher::new();
    for tick in 0..ticks {
        projectile::step(
            &mut world,
            TICK,
            &world_geometry,
            Some(&stats),
            DamageRules::default(),
        );
        // A draw a tick, so the generator's position is not a function of
        // anything else in the run.
        let _ = world.rng.next_f32();
        if tick == 10 {
            world.ships[0].pickup.weapon = Some(Weapon::Shield);
        }
        if tick == 20 {
            world.race.lap += 1;
            world.race.progress = Some(1.0);
        }
        world.tick += 1;

        oag_gameplay::hash::write_world(&mut trajectory, &world);
    }

    (hash_world(&world), trajectory.finish())
}

/// Committed reference hashes: `(ticks, final, trajectory)`.
///
/// # History
///
/// - **Recorded 2026-08-11**, with the module itself. There is no earlier value
///   to compare against - this is the first race-level gate the project has had.
///
/// **Never edit these to make the test pass**, the same rule
/// `crates/physics/tests/determinism.rs` states at length: a movement here is a
/// change to what a race *does*, and the change is the thing to find. When a
/// movement is legitimate - a new field on `World`, say - record why it moved
/// beneath this comment and isolate the cause first, by removing the new field's
/// own write and checking that the previous constants reproduce bit for bit.
const REFERENCE: &[(u32, u64, u64)] = &[
    (60, 0x0d6b_1685_6498_ed5e, 0xd8f1_9e9e_49a7_9460),
    (600, 0x7b98_740e_2313_1d4f, 0x77b7_b259_b11d_603d),
];

#[test]
fn the_race_state_matches_the_committed_reference() {
    let mut failures = Vec::new();
    for &(ticks, expected_final, expected_trajectory) in REFERENCE {
        let (final_hash, trajectory_hash) = run(ticks);
        if final_hash != expected_final || trajectory_hash != expected_trajectory {
            failures.push(format!(
                "ticks={ticks}\n  final:      expected {expected_final:#018x}, got \
                 {final_hash:#018x}\n  trajectory: expected {expected_trajectory:#018x}, got \
                 {trajectory_hash:#018x}"
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "the race state is not reproducible on this platform ({} / {}):\n\n{}\n\nDo not update \
         the constants to silence this. See the module docs.",
        std::env::consts::OS,
        std::env::consts::ARCH,
        failures.join("\n\n")
    );
}

/// The run has to actually exercise what the module docs claim it does, or the
/// constants above pin an empty scenario. The same guard
/// `oag_physics`'s `the_run_visits_the_paths_it_claims_to_cover` is.
#[test]
fn the_run_visits_the_paths_it_claims_to_cover() {
    let world_geometry = corridor();
    let stats = rocket_stats();

    let mut world = World::new(0xC0FFEE);
    world.ship_count = 2;
    for (slot, position) in [Vec3::ZERO, Vec3::Z * 380.0].into_iter().enumerate() {
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
        .spawn(Weapon::Rocket, Vec3::ZERO, Vec3::Z * 500.0, 0);

    let mut impacts = 0;
    let mut flew = false;
    for _ in 0..600 {
        let reported = projectile::step(
            &mut world,
            TICK,
            &world_geometry,
            Some(&stats),
            DamageRules::default(),
        );
        impacts += reported.iter().flatten().count();
        if world.projectiles.live() > 0 && world.projectiles.slots[0].position.z > 0.0 {
            flew = true;
        }
    }

    assert!(flew, "the rocket never moved, so flight is not covered");
    assert_eq!(impacts, 1, "the rocket never reached the wall");
    assert!(
        world.ships[1].physics.shield < 100.0,
        "the blast reached nobody, so the damage path is not covered"
    );
    assert!(
        world.ships[1].physics.body.linear_velocity.length() > 0.0,
        "the blast pushed nobody, so the impulse path is not covered"
    );
}

/// A second run of the same scenario reproduces the first, bit for bit. Catches
/// a dependency on address order, or on a hasher that carries state between
/// runs, which committed constants alone would not: both runs would be wrong the
/// same way only if the cause were deterministic.
#[test]
fn the_race_state_is_stable_across_repeated_runs() {
    assert_eq!(run(300), run(300));
}
