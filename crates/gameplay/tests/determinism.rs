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
//! `Race::state_hash`'s tests in `crates/game/src/race/tests/hash.rs`, which run on every
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
/// The whole table, since `projectile::step` looks a blast up by weapon.
///
/// **It authors a Rocket and nothing else on purpose.** The scenario below flies
/// a Rocket, and the committed constants are what say the Rocket's flight has not
/// changed; adding a Missile block here would put a second weapon's numbers inside
/// the thing the reference is measuring. A Missile that never flies would move no
/// hash either, but the next person to add a projectile to this scenario should
/// have to think about it rather than find one already half-wired.
fn weapon_stats() -> oag_formats::weapons::WeaponStats {
    oag_formats::weapons::parse(
        r#"<WeaponStats>
             <Weapon type="Global"><Stats slowdown_limit="0"/></Weapon>
             <Weapon type="Rocket"><Stats absorb="1" blastforce="20" blastradius="30"
               damage="10" slowdown_time="1" venomspeed="500" flashspeed="600"
               rapierspeed="700" phantomspeed="800" launchSpeed="0" spread="1"/></Weapon>
           </WeaponStats>"#,
    )
    .expect("the fixture parses")
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
    let stats = weapon_stats();

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
            oag_formats::handling::SpeedClass::Venom,
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
/// - **Moved 2026-08-17 (second time that day)**, when `pickup::Held` grew
///   `last`. The pickup draw refuses to hand out the same weapon twice running -
///   recovered from `WeaponPickup_Grant` (`0x08861d20`) - so what a craft was
///   last given decides what it can be given next, which makes it simulation
///   state. **Isolated the documented way**: with only `write_held`'s second
///   `write_weapon` removed and nothing else changed, the previous constants -
///   `0x2d22_d564_7848_a77a` / `0x81c4_2326_bf0f_8ddc` at 60 ticks and
///   `0x9dee_92a4_2631_48a3` / `0x95b8_b6ad_9886_1d41` at 600 - reproduce bit
///   for bit. This scenario sets `pickup.weapon` directly rather than through
///   `Held::grant`, so `last` is `None` throughout and what moved is one more
///   byte per ship per tick entering the stream.
/// - **Moved 2026-08-17**, when `Projectile` grew the three fields a guided
///   weapon needs: `target`, `bounces` and `launch_speed_kmh`. All three steer a
///   missile - the lock it is chasing, how many walls it has left to glance off,
///   and the speed its ramp blends away from - so all three are simulation state
///   and all three are hashed, across all 128 slots. **Isolated the documented
///   way**: with only the four new `write_*` calls in
///   `oag_gameplay::hash::write_projectile` removed and nothing else changed, the
///   previous constants - `0xf4f1_bc0c_30a7_27fa` / `0x780b_1ec5_4754_1bdc` at 60
///   ticks and `0xf63d_86b8_9120_e523` / `0x6688_3fef_ea90_5441` at 600 -
///   reproduce bit for bit. So what moved is four more writes per slot entering
///   the stream, and **not** the Rocket's flight, which this scenario is the only
///   committed guard on. The scenario flies no missile, so all three fields are
///   `None`/`0`/`0.0` throughout.
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
/// - **Moved a ninth time 2026-08-16**, when [`oag_race::Standing`] gained a lap
///   clock - `lap_start_tick` and `best_lap_ticks`, the two fields `RaceState`
///   already carried for the player alone. Every craft on the grid times its own
///   laps now, which is what "adaptation between races" was blocked on.
///   **Isolated in the order the compiler forces**, which is the cleanest form of
///   this proof: `write_standing` destructures exhaustively, so the fields had to
///   be *named* there before the crate would build. With them named and neither
///   one written to the hasher, the constants this commit replaces -
///   `0x86c3_7eae_232b_caba` / `0x4e1f_368e_1f13_b9ec` at 60 ticks and
///   `0xbd24_86ff_da6a_56b3` / `0x7dc5_c309_e8f9_8311` at 600 - reproduce bit for
///   bit. So the movement is those two writes entering the stream and nothing
///   the simulation does differently. **The scenario here has no course in it**,
///   the same fact the 2026-08-11 entry above turns on, so both fields stay
///   `None` on every tick: what moved is one discriminant byte apiece per craft
///   per tick, in the same way the projectile-pool widening moved empty slots
///   rather than behaviour.
///
/// **Never edit these to make the test pass**, the same rule
/// `crates/physics/tests/determinism.rs` states at length: a movement here is a
/// change to what a race *does*, and the change is the thing to find. When a
/// movement is legitimate - a new field on `World`, say - record why it moved
/// beneath this comment and isolate the cause first, by removing the new field's
/// own write and checking that the previous constants reproduce bit for bit.
const REFERENCE: &[(u32, u64, u64)] = &[
    (60, 0xa88c_cfc7_3a30_f316, 0x4e6e_0a8f_accb_a74c),
    (600, 0xb5fc_13ea_252e_8e43, 0x92e8_8d29_f718_d061),
];

/// The volley scenario: a craft at an angle fires a real fanned Rocket volley
/// through [`projectile::launch`], and every shot flies.
///
/// **Separate from [`run`] on purpose.** The scenario above spawns one rocket
/// with a hand-written velocity, which is exactly why finding D1 of the
/// 2026-08-18 review could live in `launch` untouched by any gate: the spread
/// goes through a `sin_cos`, lands in `Projectile::velocity`, and nothing
/// cross-platform ever called the function that computes it. Adding the volley
/// to `run` would have moved that scenario's constants and mixed one fix's
/// evidence into another's history, so this carries its own reference.
///
/// The craft is deliberately **not** axis-aligned. With a default pose the fan
/// rotates about `Vec3::Y` and two of the three directions come out of the
/// half-angle sine with exactly representable components; a tilted craft makes
/// every component a real product of the rotation, which is what a platform's
/// libm can disagree about in the last bit.
fn run_volley(ticks: u32) -> (u64, u64) {
    use oag_core::math::{Quat, Vec3 as V};

    let world_geometry = corridor();
    let stats = weapon_stats()
        .rocket()
        .expect("the fixture authors a Rocket");

    let mut world = World::new(0xC0FFEE);
    world.ship_count = 2;
    for (slot, position) in [V::ZERO, V::Z * 380.0].into_iter().enumerate() {
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
    // Yaw a little, roll a little: the fan's axis is the craft's own up.
    world.ships[0].physics.body.orientation =
        Quat::from_rotation_y(0.11) * Quat::from_rotation_z(0.23);

    // The volley the front end fires, built the way `race::weapons` builds it.
    let shots = projectile::launch(
        &world.ships[0].physics,
        &world.ships[0].handling.dimensions,
        &stats,
        oag_formats::handling::SpeedClass::Venom,
    );
    for (position, velocity) in shots {
        assert!(
            world
                .projectiles
                .spawn(Weapon::Rocket, position, velocity, 0),
            "the pool refused a shot, so this scenario is not flying a full volley"
        );
    }

    let mut trajectory = oag_core::hash::StateHasher::new();
    for _ in 0..ticks {
        projectile::step(
            &mut world,
            TICK,
            &world_geometry,
            Some(&weapon_stats()),
            oag_formats::handling::SpeedClass::Venom,
            DamageRules::default(),
        );
        let _ = world.rng.next_f32();
        world.tick += 1;
        oag_gameplay::hash::write_world(&mut trajectory, &world);
    }

    (hash_world(&world), trajectory.finish())
}

/// Committed reference hashes for [`run_volley`]: `(ticks, final, trajectory)`.
///
/// # History
///
/// - **Recorded 2026-08-18**, with the scenario, as finding D1's gate. There is
///   no earlier value: no committed hash had ever covered
///   [`projectile::launch`]. Recorded *after* the fix, so what these pin is the
///   deterministic `oag_core::math::quat_from_axis_angle` and not glam's
///   platform `sin_cos` - pinning the hole would have made the hole the
///   reference.
///
/// **Never edit these to make the test pass**, for the same reason
/// [`REFERENCE`] says at length.
const REFERENCE_VOLLEY: &[(u32, u64, u64)] = &[
    (60, 0x8c79_4b77_720c_7444, 0xf356_fa51_5a78_b1a0),
    (600, 0xfa19_77ff_52da_9377, 0x6357_39ef_ff80_8956),
];

#[test]
fn the_fanned_volley_matches_the_committed_reference() {
    let mut failures = Vec::new();
    for &(ticks, expected_final, expected_trajectory) in REFERENCE_VOLLEY {
        let (final_hash, trajectory_hash) = run_volley(ticks);
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
        "the fanned volley is not reproducible on this platform ({} / {}):\n\n{}\n\nDo not \
         update the constants to silence this. See the module docs.",
        std::env::consts::OS,
        std::env::consts::ARCH,
        failures.join("\n\n")
    );
}

/// The volley scenario has to fire a *fan*, or its constants pin three rockets
/// on one ray and the spread never enters the hash at all.
#[test]
fn the_volley_actually_fans() {
    use oag_core::math::{Quat, Vec3 as V};

    let stats = weapon_stats()
        .rocket()
        .expect("the fixture authors a Rocket");
    assert!(stats.spread != 0.0, "the fixture authors no fan");

    let mut world = World::new(0xC0FFEE);
    let ship = &mut world.ships[0];
    ship.handling.dimensions = Dimensions {
        length: 4.0,
        width: 2.0,
        height: 1.0,
        ..Dimensions::default()
    };
    ship.physics.body.orientation = Quat::from_rotation_y(0.11) * Quat::from_rotation_z(0.23);

    let shots = projectile::launch(
        &ship.physics,
        &ship.handling.dimensions,
        &stats,
        oag_formats::handling::SpeedClass::Venom,
    );
    let directions: Vec<V> = shots.iter().map(|&(_, velocity)| velocity).collect();
    assert!(
        directions[1].distance(directions[0]) > 1.0 && directions[2].distance(directions[0]) > 1.0,
        "the three shots share a ray, so the spread is not exercised: {directions:?}"
    );
    // And the fan is off an axis nothing makes exactly representable, which is
    // the property that makes a libm difference visible at all.
    for direction in &directions {
        assert!(
            direction.x != 0.0 && direction.y != 0.0 && direction.z != 0.0,
            "a shot is axis-aligned, so the scenario is weaker than it looks: {direction:?}"
        );
    }
}

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
    let stats = weapon_stats();

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
            oag_formats::handling::SpeedClass::Venom,
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
    assert_eq!(run_volley(300), run_volley(300));
}
