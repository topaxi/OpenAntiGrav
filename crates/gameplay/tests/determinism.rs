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
/// - **Moved 2026-08-12**, and by a *force-law* change rather than a wider hash.
///   `oag_physics` now reads `Antigrav::rebound_jump_time`, which it parsed and
///   ignored: the landing response is armed while a craft is in the air and only
///   once the flight has outlasted that parameter, so a short hop no longer
///   applies `landing_rebound`. `ShipState` also gained `time_airborne`, and
///   `time_since_landing`'s initial value became `Ship_InitCraft`'s recovered
///   `10.0`. See that crate's own determinism history for the isolation; the
///   scenario here inherits the movement because it steps real craft.
/// - **Recorded 2026-08-11**, with the module itself. There is no earlier value
///   to compare against - this is the first race-level gate the project has had.
/// - **Moved 2026-08-11**, when `Ship::driver` joined the hash. The field is the
///   opponent driver's place on the racing line, which seeds next tick's search
///   and so is simulation state; see `oag_gameplay::hash::write_ship`. **The
///   cause was isolated the way the paragraph below requires**: with the
///   driver's own `write_u32` removed and nothing else changed, the previous
///   constants - `0x0d6b_1685_6498_ed5e` / `0xd8f1_9e9e_49a7_9460` at 60 ticks
///   and `0x7b98_740e_2313_1d4f` / `0x77b7_b259_b11d_603d` at 600 - reproduce
///   bit for bit. So the movement is the new field entering the hash and not a
///   change in what the simulation does. The scenario here has no AI in it, so
///   every driver in it is at index `0` throughout.
/// - **Moved again 2026-08-11**, when `Ship::standing` joined the hash. A craft's
///   lap, place on the circuit and finish tick decide the finishing order, so
///   they are simulation state. **Isolated the same way**: with
///   `write_standing`'s call removed and nothing else changed, the previous
///   constants - `0xe8fa_1f54_0742_7f0e` / `0xf195_097c_9824_11b0` at 60 ticks
///   and `0x8f0c_b68a_5b98_18df` / `0x2a7b_fddd_ae47_fd8d` at 600 - reproduce bit
///   for bit. The scenario here has no course in it, so every standing stays at
///   its default and what moved is the field entering the stream, not any value
///   in it.
///
/// - **Moved a third time 2026-08-11**, when `Driver::seed` and `Driver::phase`
///   joined the hash. The seed decides an opponent's whole character - which
///   part of the AI corridor it holds, how hard it commits to a corner - and the
///   phase is the tick count its drift is a function of, so both decide what it
///   steers next; see `oag_ai::Personality`. **Isolated the same way**: with
///   those two `write_u32`s removed and nothing else changed, the previous
///   constants - `0x187f_03ac_8cc2_f9fe` / `0xaa73_ebe5_d207_ccd0` at 60 ticks
///   and `0x8b79_23a3_6397_3ab7` / `0x7bb3_4940_5709_0a5d` at 600 - reproduce
///   bit for bit. The scenario here still has no AI in it, so both fields are
///   `0` throughout and what moved is two more words entering the stream, not
///   any value in it.
///
/// - **Moved a fourth time 2026-08-11**, when `Driver::place` and
///   `Driver::provocation` joined the hash. A driver notices being overtaken by
///   its place getting worse and covers its line harder for a while afterwards,
///   so both decide what it does to the craft around it; see
///   `oag_ai::Driver::stew`. **Isolated the same way**: with those two
///   `write_u32`s removed and nothing else changed, the previous constants -
///   `0x2e8d_8a4d_ab71_199e` / `0x65bc_a8c9_0bc9_07f0` at 60 ticks and
///   `0xf019_f135_fae6_d657` / `0x177a_c6df_4c46_417d` at 600 - reproduce bit
///   for bit. The scenario here still has no AI in it, so both fields are `0`
///   throughout and what moved is two more words entering the stream, not any
///   value in it.
///
/// - **Moved a fifth time 2026-08-11**, when `Driver::pilot` joined the hash.
///   It is a fingerprint of the pilot a craft is flying, and unlike
///   `Ship::handling` - which comes off the player's own disc and is the same
///   everywhere - a pilot can come out of `<config dir>/oag/pilots/` and so
///   **differs between machines by design**. Left out, two machines running
///   "the same race" with different pilot files would agree here and disagree
///   on the race, which is a gate claiming an agreement it does not have.
///   **Isolated the same way**: with that one `write_u32` removed and nothing
///   else changed, the previous constants - `0x9c26_b4b5_c0c7_43be` /
///   `0xc9d7_d405_37ab_9310` at 60 ticks and `0x6c9c_3ab9_8500_0e77` /
///   `0x5c72_e308_83b1_c29d` at 600 - reproduce bit for bit. The scenario here
///   still has no AI in it, so the field is `0` throughout and what moved is one
///   more word entering the stream; `every_driver_in_this_scenario_flies_no_pilot`
///   pins that, because if it ever stopped being true these constants would
///   quietly become machine-dependent.
///
/// - **Moved a sixth time 2026-08-11**, when `Driver::mistake` joined the hash.
///   It counts down a braking point the driver is in the middle of missing, and
///   a craft sailing through one is about to be somewhere a craft that braked is
///   not; see `oag_ai::Driver::blunder`. **Isolated the same way**: with that
///   one `write_u32` removed and nothing else changed, the previous constants -
///   `0xeff4_5f7c_6a67_fa0e` / `0xd894_4c73_75ed_4c20` at 60 ticks and
///   `0x7e38_a669_8476_79e7` / `0xd7d6_954f_c46d_066d` at 600 - reproduce bit
///   for bit. The scenario has no AI, so the field is `0` throughout.
///
/// - **Moved a seventh time 2026-08-11, and this one is unlike the six above:
///   it is a change to what the simulation *does*, not to what is hashed.**
///   Projectiles now follow the track floor instead of flying straight - see
///   `oag_gameplay::projectile` and
///   `docs/ghidra/functions/psp-pulse-usa/rocket-visuals.md`. This scenario
///   flies a rocket down a corridor into a wall, so its whole trajectory
///   differs. That is the intended outcome, not a defect.
///
///   **Every entry above could isolate by removing one `write_*` call; this one
///   cannot**, so it was isolated in two steps instead. The first is the one
///   that matters, because it is what would catch an accidental change riding
///   along:
///
///   1. With **both** the new `Projectile::surface` write *and* the
///      surface-following disabled, and nothing else changed, the constants
///      this commit replaces - `0x2524_032f_46c2_e75e` /
///      `0xc9ba_8f3a_d73d_2530` at 60 and `0x2655_f848_d5d1_9117` /
///      `0xd5df_df48_d023_11bd` at 600 - **reproduce bit for bit**. Nothing
///      else in this change touches the simulation.
///   2. With the field hashed but the flight still straight they read
///      `0x9c82_3577_13b6_c4d7` / `0x9bcf_b626_ad64_1387` at 60 and `0xfc3e_1605_38a7_e79e` / `0x38f8_30b3_2f75_8c5a` at 600. That is the field's own
///      contribution; the rest of the distance to the constants below is the
///      flight model, which is the point of the change.
///
/// - **Moved an eighth time 2026-08-11**, when `MAX_PROJECTILES` went from 16 to
///   128. Sixteen could not hold one simultaneous volley from a full grid (24),
///   let alone an Eliminator race; the original's own pool is 48 and this is
///   deliberately past it. **Isolated the same way the field additions were**:
///   with the constant put back to 16 and nothing else changed, the constants
///   this commit replaces - `0x8a04_20ea_d659_f08e` / `0x3005_5382_1116_2a30`
///   at 60 and `0xd934_4be0_d70e_dc17` / `0x9ca7_8a00_bbbb_29ad` at 600 -
///   reproduce bit for bit. Every slot is hashed whether or not it is occupied, so what moved
///   is 112 more empty slots entering the stream, not anything the simulation
///   does: this scenario fires one rocket and never fills a second slot.
///
/// **Never edit these to make the test pass**, the same rule
/// `crates/physics/tests/determinism.rs` states at length: a movement here is a
/// change to what a race *does*, and the change is the thing to find. When a
/// movement is legitimate - a new field on `World`, say - record why it moved
/// beneath this comment and isolate the cause first, by removing the new field's
/// own write and checking that the previous constants reproduce bit for bit.
const REFERENCE: &[(u32, u64, u64)] = &[
    (60, 0x86c3_7eae_232b_caba, 0x4e1f_368e_1f13_b9ec),
    (600, 0xbd24_86ff_da6a_56b3, 0x7dc5_c309_e8f9_8311),
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

    // **The committed constants are only machine-independent while this holds.**
    // `Driver::pilot` is a digest of a pilot that can come out of the player's
    // own config directory; it is `0` here because this scenario has no AI in
    // it. An edit that gave it one would make the references above depend on
    // whatever is in `~/.config/oag/pilots/`, which is precisely the failure
    // that field was added to make loud rather than silent.
    for ship in &world.ships {
        assert_eq!(
            ship.driver.pilot, 0,
            "a craft here is flying a pilot, so the committed constants now \
             depend on the machine's config directory"
        );
    }
}

/// A second run of the same scenario reproduces the first, bit for bit. Catches
/// a dependency on address order, or on a hasher that carries state between
/// runs, which committed constants alone would not: both runs would be wrong the
/// same way only if the cause were deterministic.
#[test]
fn the_race_state_is_stable_across_repeated_runs() {
    assert_eq!(run(300), run(300));
}
