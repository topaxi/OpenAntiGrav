//! The fanned-volley half of the race-level determinism gate.
//!
//! Split from `determinism.rs` under the 1,000-line file ceiling, along the
//! seam it already had: this scenario carries its own reference constants and
//! its own history, and shares only the fixture in `determinism_support`.
//! `REFERENCE` in the history below is `determinism.rs`'s own table.

use oag_gameplay::hash::hash_world;
use oag_gameplay::world::World;
use oag_physics::DamageRules;
use oag_physics::params::Dimensions;
use oag_weapons::projectile;

mod determinism_support;
use determinism_support::{TICK, corridor, weapon_stats};

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
    let fired = projectile::fire_rocket(
        &mut world.projectiles,
        &world.ships[0].physics,
        &stats,
        "VENOM",
        0,
    )
    .expect("Pulse's weapon table authors a Venom rocket speed");
    assert_eq!(
        fired,
        projectile::ROCKET_SHOTS,
        "the pool refused a shot, so this scenario is not flying a full volley"
    );

    let mut trajectory = oag_core::hash::StateHasher::new();
    for _ in 0..ticks {
        projectile::step(
            &mut world.projectiles,
            &mut world.ships[..world.ship_count as usize],
            TICK,
            &world_geometry,
            Some(&weapon_stats()),
            "VENOM",
            DamageRules::default(),
            &mut [],
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
/// - **Moved 2026-08-26**, by `Driver::reflex` joining the hash, and
///   **2026-08-24** by `Ship::autopilot_timer` doing the same. Both isolations
///   and both sets of previous constants are in `REFERENCE`'s history note,
///   which covers both scenarios in one pass rather than twice.
///
/// - **Recorded 2026-08-18**, with the scenario, as finding D1's gate. There is
///   no earlier value: no committed hash had ever covered
///   [`projectile::launch`]. Recorded *after* the fix, so what these pin is the
///   deterministic `oag_core::math::quat_from_axis_angle` and not glam's
///   platform `sin_cos` - pinning the hole would have made the hole the
///   reference.
/// - **Moved 2026-09-16 (second time today)**, the `pending_thrust_scale`
///   addition `REFERENCE`'s own history records, inherited through the same
///   `hash_world` call and isolated there for both scenarios at once. Replaces
///   `0xa901_3ebe_47aa_6426` / `0xfb1e_cac3_3b77_b824` at 60 ticks and
///   `0xae8a_645f_7a5b_05af` / `0xfb29_615f_cef3_e900` at 600.
/// - **Moved 2026-08-19**, the same `pending_impulse` addition `REFERENCE`'s
///   own history records, inherited through the same `hash_world` call and
///   not re-isolated here for the same reason. Replaces
///   `0x8c79_4b77_720c_7444` / `0xf356_fa51_5a78_b1a0` at 60 ticks and
///   `0xfa19_77ff_52da_9377` / `0x6357_39ef_ff80_8956` at 600.
///
/// - **Moved 2026-08-26**, the same `Held::dropping` / `Held::drop_reload`
///   addition `REFERENCE`'s own history records, inherited through the same
///   `hash_world` call and isolated there rather than twice. Replaces
///   `0x87aa_86d6_9f6b_73e8` / `0x525b_5a02_6095_f7a0` at 60 ticks and
///   `0x5cab_0e84_dfbe_0a9b` / `0xce16_0eaa_37a7_9f16` at 600.
///
/// - **Moved 2026-09-05**, the same barrel-roll field addition `REFERENCE`'s
///   own history records, inherited through the same `hash_world` call and not
///   re-isolated here for the same reason. Replaces `0x2d4d_2f6e_a306_041c` /
///   `0x9b66_0ce3_df4f_59f4` at 60 ticks and `0x10cd_79fd_a58d_243b` /
///   `0x8ab3_7516_6ae2_aa42` at 600.
///
/// - **Moved 2026-09-05, later the same day**, the same `roll_payout_timer`
///   plus three-consumer addition `REFERENCE`'s own history records,
///   inherited the same way. Replaces `0x4148_5ed1_decf_60ec` /
///   `0xdfed_3982_a8f4_7a18` at 60 ticks and `0x5a0b_3d23_e346_8677` /
///   `0x7311_e129_616d_bc8e` at 600.
///
/// - **Moved 2026-09-06**, the same `roll_axis_zone` addition `REFERENCE`'s
///   own history records, inherited the same way. Replaces
///   `0x0549_867b_d291_294c` / `0x1f6d_7622_eb31_7338` at 60 ticks and
///   `0x626e_0aee_2d8b_48f7` / `0x78f9_263c_8a07_97ee` at 600.
///
/// - **Moved 2026-09-06, later the same day**, the same `Driver::roll_decided`
///   addition `REFERENCE`'s own history records, inherited through the same
///   `hash_world` call and isolated there rather than twice. Replaces
///   `0x84fa_0540_953f_7dbc` / `0x5544_ea00_8147_b29c` at 60 ticks and
///   `0x71c8_e66d_871f_8f3b` / `0x9c54_22cf_8958_48ea` at 600.
///
/// - **Moved again 2026-09-06, by a merge rather than by one change.** The AI
///   barrel-roll axes and the weapon-slowdown port each moved these constants on
///   their own branch, and neither branch's value survives their merge: the
///   merged tree writes both `Pilot`'s three roll axes and `World`'s
///   `pending_slowdown` into the same stream. The value recorded here is
///   **measured from the merged tree**, not chosen from either side - both
///   causes are already isolated and explained in their own entries above, so
///   what is new here is only their composition.
///
/// - **Moved 2026-09-07**, the same Cannon `Held`-field addition
///   `REFERENCE`'s own history records, inherited through the same
///   `write_held` call and not re-isolated here for the same reason. Replaces
///   `0x4a5b_1790_59a4_23c0` / `0x4e62_4203_11e0_9078` at 60 ticks and
///   `0x2e86_2b63_f59d_657b` / `0x606e_fd2e_04e5_d9ce` at 600.
///
/// - **Moved again 2026-09-07**, the same `World::quake` field addition
///   `REFERENCE`'s own history records, inherited through the same
///   `write_world` call and not re-isolated here for the same reason.
///   Replaces `0x9a80_b71f_63a4_d274` / `0xefd7_1d09_2752_84ac` at 60 ticks
///   and `0x0d5f_98f8_61a6_7ab3` / `0xeb2e_415f_80cc_c0ea` at 600.
///
/// - **Moved again 2026-09-07**, the same `RaceState::lap_splits` /
///   `Standing::lap_splits` addition `REFERENCE`'s own history records,
///   inherited through the same `write_standing`/`write_race` calls and not
///   re-isolated here for the same reason - this scenario never completes a
///   lap either, so every new slot stays `None` and the movement is the same
///   fixed run of bytes. Replaces `0x2d89_9e56_5111_9b1c` /
///   `0x0cc3_9764_975f_5852` at 60 ticks and `0x5feb_a10d_ede2_7e29` /
///   `0xb54d_97c5_c287_eb16` at 600.
///
/// **Never edit these to make the test pass**, for the same reason
/// `REFERENCE` says at length.
/// - **Moved 2026-09-08**, inherited from the `World::leach_beam` addition
///   `REFERENCE`'s own history records, through the same one extra
///   discriminant byte a tick. **This scenario moved where the 2026-09-01 blast
///   change left it alone**, which is the opposite way round from that entry and
///   is the expected shape: a hash-stream addition reaches every scenario
///   equally, and a force-law change only reaches the ones that trigger it.
///   Replaces `0x89da_d0f0_bd19_2e0c` / `0xea20_03ce_ebb3_2762` at 60 ticks and
///   `0x62d3_802f_e05b_0fa9` / `0xf440_21aa_cf59_fd86` at 600. No isolation
///   repeated here - `REFERENCE`'s entry ran it for both scenarios at once.
/// - **Moved again 2026-09-08, same day, inherited from the
///   `Standing::kills`/`deaths` addition `REFERENCE`'s own history
///   records**, through the same `write_standing` call both scenarios share -
///   this one never scores a kill or a death either, so both fields stay a
///   constant `0` through the whole run. Replaces `0x57ff_1511_51c9_3e64` /
///   `0xf488_c189_693d_c7c4` at 60 ticks and `0x4872_7a5a_3abb_9c2b` /
///   `0x189c_0e36_01b8_688a` at 600. No isolation repeated here -
///   `REFERENCE`'s entry ran it for both scenarios at once.
///
/// - **Moved 2026-09-09**, the same `Projectile::charge` addition
///   `REFERENCE`'s own history records, inherited through the same four
///   extra bytes per slot per tick. No isolation repeated here -
///   `REFERENCE`'s entry ran it for both scenarios at once.
///
/// - **Moved 2026-09-11**, the same `oag_ai::Driver::peak_curvature` addition
///   `REFERENCE`'s own history records, inherited through the same
///   `write_driver` call and not re-isolated here for the same reason: this
///   scenario never calls `Driver::drive` either. Replaces
///   `0xdc64_65d2_7fd2_6484` / `0xd117_eae7_9d96_90a4` at 60 ticks and
///   `0x0afa_f530_c285_b40b` / `0x5993_a507_fd8b_40ca` at 600.
///
/// - **Moved 2026-09-15**, the same `Ship::disruption` and
///   `Projectile::effect` addition `REFERENCE`'s own history records,
///   inherited through the same `write_ship` and `write_projectile` calls.
///   No isolation repeated here - `REFERENCE`'s entry ran it for both
///   scenarios at once. Replaces `0x6b88_42dc_ab26_a704` /
///   `0x71ab_7df0_6e22_ba44` at 60 ticks and `0x1d58_0459_a503_3a6b` /
///   `0x1077_6477_39e6_016a` at 600.
///
/// - **Moved 2026-09-16**, the same `World::race` widening and
///   `World::controllers` addition `REFERENCE`'s own history records,
///   inherited through the same `write_world` call. No isolation repeated
///   here, because `REFERENCE`'s entry ran it for both scenarios at once.
///   Replaces `0xc850_5ae4_1c3f_3ed8` / `0x2a14_99ab_3d57_53d8` at 60 ticks
///   and `0xefed_fabd_1add_c7c9` / `0xbb56_3a8f_efeb_6394` at 600.
///
/// - **Moved 2026-09-16 (600-tick trajectory only)**, when the Rocket took
///   the original's own 5.0 s cap (`rocket::LIFETIME_SECONDS`) instead of the
///   shared 10.0. This volley flies into open space, so its three rockets are
///   now reaped at 5 s where they were reaped at 10; the final hash at 600 is
///   unchanged because the slots are empty either way by then, and the 60-tick
///   row is untouched because nothing expires inside a second. Replaces
///   `0xc8f5_f44f_4c4d_c618`; isolated by that arithmetic rather than by a
///   revert.
///
/// - **Moved 2026-10-01 (60-tick final and both trajectories)**, by the
///   Rocket's measured launch: the class speed alone, 0.75 x it until the first
///   surface hit, and the class speed riding in `Projectile::launch_speed_kmh`
///   (`rocket::fire`). Isolated by commit: `f9dce4de` (class speed alone) left
///   this fixture's hash unchanged because it authors `launchSpeed="0"`, and
///   `2b253ead` (0.75 launch, the new hashed field) moved it. The 600-tick
///   final is unchanged, the slots being empty by then. Replaces
///   `0xccab_866c_78e1_d1d6` / `0x668e_a6e7_f71d_d3e4` at 60 ticks and
///   `0x9c20_7c28_2863_b750` (trajectory) at 600.
///
/// - **Moved again 2026-10-01 (same three values)**, by the Rocket leaving from
///   the craft's own position rather than its nose (see `rocket::launch`). Isolated by commit: the previous
///   regeneration was green before it. Replaces `0x84b1_ede3_62c8_f914` /
///   `0xdf2c_fa35_c525_9b68` at 60 ticks and `0xc367_da50_59dd_4747`
///   (trajectory) at 600.
///
/// - **Moved again 2026-10-01**, by the Rocket's riding normal being seeded from
///   the craft's up rather than world up (`Projectiles::spawn_riding`).
///   Isolated by commit: the previous regeneration was green before it.
///   Replaces `0xdb77_1466_f784_d9b9` / `0x299e_b849_daed_4a92` at 60 ticks and
///   `0x4129_e4fd_ccdd_4d1c` (trajectory) at 600.
///
/// - **Moved 2026-10-03**, when `Ship::weapon_ai` (two `u32` clocks for the
///   original's opponent fire law, `oag_ai::weapon_ai`) joined the hash: eight
///   more bytes per ship per tick, all zero here - no clock is ever advanced
///   in this fixture. Isolated the documented way: with the two writes taken
///   out of `write_ship` and nothing else changed, the previous constants
///   reproduced bit for bit. Replaces `0x72ba_a575_f0b3_bdf4` /
///   `0x2e0f_a99e_309c_ed96` at 60 ticks and `0x5b48_d436_7dbc_09e7` /
///   `0x7a1f_821f_3771_eb31` at 600.
///
/// - **Moved 2026-10-04**, when `World::repulsers` (sixteen empty Repulser
///   slots, one discriminant byte each) joined the hash. No Repulser is fired
///   here. Isolated the documented way: with the pool's writes taken out of
///   `write_world` and nothing else changed, the previous constants reproduced
///   bit for bit at both tick counts for both scenarios. Replaces `0x3fa9_0a39_7bac_41b4` /
///   `0xa334_e125_421c_f896` at 60 ticks and `0xfe22_9e45_21ac_9c67` / `0xbc86_1210_e167_7831` at 600.
///
/// - **Moved 2026-10-05**, when `Driver::branching` (which line a driver is on
///   at a fork, `oag_ai::branch`) joined the hash: thirteen more bytes per ship
///   per tick. The synthetic track here has no fork, so every field stays at its
///   default. Isolated the documented way: with the four writes taken out of
///   `write_driver` and nothing else changed, the previous constants reproduced
///   bit for bit at both tick counts. Replaces `0xe655_02d2_4ca9_8eb4` /
///   `0x7d19_2a0c_9ac6_6496` at 60 ticks and `0xeaac_e0f2_34f0_5e27` / `0x998b_ec4e_b275_9eb1` at 600.
///
/// - **Moved 2026-10-05 again**, when `Branching::pending` (the route a driver
///   drew, held until it is near the split) joined the driver's hash: four
///   more bytes per ship per tick, zero on this forkless track. Isolated the
///   same way: with that one write removed the previous constants reproduced
///   bit for bit. Replaces `0x8731_a61e_e41b_a9e8` / `0x9df3_6a61_7315_16d6` at 60 ticks and `0x1a03_923d_2d2f_0787` /
///   `0xaefe_b285_a14e_81a1` at 600.
const REFERENCE_VOLLEY: &[(u32, u64, u64)] = &[
    (60, 0x46d8_ed88_4c17_9c28, 0x315d_2753_13b0_7b16),
    (600, 0xc349_6025_7e65_1567, 0x9648_2860_d2cc_25a1),
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

    let shots = projectile::launch(&ship.physics, &stats, "VENOM")
        .expect("Pulse's weapon table authors a Venom rocket speed");
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

/// A second run of the volley reproduces the first, bit for bit - the same
/// guard `determinism.rs` keeps for its own scenario.
#[test]
fn the_volley_is_stable_across_repeated_runs() {
    assert_eq!(run_volley(300), run_volley(300));
}
