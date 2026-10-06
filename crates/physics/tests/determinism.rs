//! The cross-platform determinism gate, over the **actual** simulation.
//!
//! `oag-core`'s `tests/determinism.rs` is the older half: it hashes `oag_core::probe`, a
//! miniature simulation, so it covers the shared float pipeline and nothing else. This file
//! steps [`oag_physics::step`], the entry point `oag_game`'s race loop uses, through
//! [`oag_physics::probe`] (scenario, fixture world and hashing).
//!
//! `docs/reverse-engineering/verification-protocol.md` makes this a precondition: *before any
//! comparison against the original is meaningful, our own simulation must be reproducible.*
//!
//! # When this test fails
//!
//! Do not update the constants to make it pass: that turns a real bug into a silent one. In
//! order of likelihood:
//!
//! 1. A `mul_add` crept into `crates/physics/src`.
//! 2. A build flag enabled fast-math or FMA contraction.
//! 3. `glam`'s `scalar-math` feature was dropped, re-enabling SIMD.
//! 4. A transcendental entered the simulation path (`crates/physics/src` had none outside
//!    `#[cfg(test)]` when this gate was written; preserve that). IEEE-754 requires correct
//!    rounding for `sqrt` but not `sin`/`cos`/`exp`, which are platform-dependent.
//! 5. A `HashMap`/`HashSet` reached the collision world or parameter path (`CollisionWorld` is
//!    ordered storage for this reason, `docs/architecture/determinism.md`).
//!
//! A failure here with `oag-core`'s gate passing narrows the cause to this crate; both failing
//! is the shared foundation. A failure of [`the_simulation_is_stable_across_repeated_runs`]
//! alone is not a float problem: it is uninitialised or address-dependent state, so look for a
//! map or pointer-derived value first.
//!
//! Debug and release are covered between two CI jobs: `check` runs the workspace in debug and
//! `determinism` runs this in release. Regenerate deliberately, only after establishing the
//! behaviour change is intended:
//!
//! ```sh
//! cargo run -q -p oag-physics --example physics_determinism_report
//! ```

use oag_core::hash::StateHasher;
use oag_core::math::{Quat, Vec3};
use oag_physics::maglock::MagContact;
use oag_physics::probe::{self, Script};
use oag_physics::{CraftState, Environment, ShipState, step};

/// `(ticks, script, final_hash, trajectory_hash)`, recorded on x86_64 Linux with the example
/// named in the module docs.
///
/// # History
///
/// A *hash-input* change (a `ShipState` field added to `probe::hash_state`) lengthens the FNV-1a
/// stream and moves **every** row, the 600-tick `Corridor` included; a *behaviour* change moves
/// only the rows that reach it. **Check the cause directly before pasting new constants**: remove
/// only the new `hash_state` write(s), keep every other change, and the previous constants must
/// reproduce bit for bit (hash-input), or removing the one suspect call must reproduce the old
/// row (behaviour). Each entry below was checked this way unless noted.
///
/// - **2026-07-29**, first recorded. No earlier reference: no gate covered the simulation.
/// - **2026-07-30, behaviour.** `hull_sample_points`/`hull_extent` scale `<Misc>` dimensions by
///   `hover::TARGET_GLOBAL_SCALE` (`0.75`), as `Ship_InitCraft`'s box-collider setup does
///   (`collision.md`). Only the two 3,600-tick rows moved; 600-tick `Corridor` never reaches a
///   wall.
/// - **2026-08-03, hash input.** `pad_timer`, `pad_direction` added (`engine.rs`,
///   `speedup_pad`). `Environment::default()` has no `pad_hit`, so the fields stay at defaults
///   for every tick and add a constant run of bytes. All three rows moved (the tell).
/// - **2026-08-04, hash input.** `shift_tap_windows`, `shift_armed`, `shift_lockout` for the
///   sideshift gestures (`airbrake.rs`, `advance_sideshift`; `input-bindings.md`). Deleting the
///   four new writes reproduced the 2026-08-03 hashes. `probe::controls` never sets
///   `shift_modifier` or `shift_tap_*`, and its one `Sideshift::Left` (tick 1200) is a direct
///   request that bypasses `shift_lockout`.
/// - **2026-08-08, behaviour, deliberate.** `engine::speedup_pad` implements `<Special
///   speedpad_jump>` (a 5.71-degree tilt toward the hull's up axis while d-pad Up is held), and
///   `probe::environment` is new: the scenario now crosses a speed pad twice. Until then no
///   script set `pad_hit`, so step 15 of 15 was uncovered and a new branch there would have
///   landed with the hashes unmoved. No field added; the 600-tick row did not move (first
///   crossing at tick 1600); `environment` returning the default reproduced the 2026-08-04
///   constants, and the control script is unchanged.
/// - **2026-08-10, hash input.** `shield` added (the energy pool, `entity+0x88`;
///   `damage::apply_contact`). Nothing reads it back. Removing `write_f32(shield)` reproduced
///   the 2026-08-08 constants. `probe::start` fills the pool via `damage::reset` (an empty pool
///   cannot be depleted) and `the_run_visits_the_paths_it_claims_to_cover` asserts energy is
///   spent.
/// - **2026-08-10 (second), hash input.** `craft_state`, `state_timer`: defaults every tick
///   (no script empties a pool). Removing the two writes reproduced the earlier constants.
/// - **2026-08-11, hash input.** `turbo_timer` (`engine::ENGINE_PICKUP_SPEEDUP`): no probe script
///   fires one, and a corridor has no `Weapon Pad`. Removing its write reproduced the six
///   2026-08-10 constants.
/// - **2026-08-11 (second), hash input.** `shield_pickup_timer`. Checked with extra care because
///   `damage::apply_contact` (which the scripts exercise by scraping a wall) now returns early
///   while it runs: with only the write removed and the `apply_contact` guard,
///   `advance_shield_pickup` call and `damage::reset` clearing all in place, the earlier three
///   constants reproduced, so the new branch is never taken.
/// - **2026-08-12, behaviour.** `Ship_CastHoverProbes` branches on `craft+0x2ec <= 50.0`; above
///   it one ray is cast and the rear probe's hit record is manufactured from the front's
///   ([`hover::FAST_PROBE_SPEED`]). This crate cast two rays at every speed and shed half its
///   suspension over a lip. No field added. On the disc's twelve circuits a lone craft went from
///   seven clean laps to nine, `05_Track` from never completing a second lap to a clean one, and
///   `09_Track` from six recoveries to three.
/// - **2026-08-12 (second), behaviour.** `Ship_UpdateCraft` (`0x08849df0`) keeps an airborne
///   clock at `craft+0x284` and `Ship_HoverTwoPoint` zeroes the landing clock **while airborne**
///   once it passes `Antigrav::rebound_jump_time`; a shorter hop never arms `landing_rebound`.
///   `ShipState` gains `time_airborne`, and `time_since_landing` starts at the recovered `10.0`
///   (was an invented `1.0`; both sit outside the 0.2 s window). Isolated by removing only
///   `write_f32(time_airborne)`: all three rows still moved, since every script leaves the
///   ground and the landing response fires on fewer ticks.
/// - **2026-08-19, hash input.** `pending_impulse` (`entity->0x4c + 0x110`), consumed by
///   `wall::apply_pending_impulse` (`Ship_ApplyCollisionImpulse`) every tick from `step`. No
///   producer is ported (`contact-response.md`), so the field is `Vec3::ZERO` on every entry and
///   the call is a true no-op. Removing the write reproduced the 2026-08-11 constants.
/// - **2026-09-05, hash input.** `roll_taps`, `roll_tap_timer`, `roll_phase`, `roll_target` for
///   the barrel roll (`barrel_roll`, `input-bindings.md`), not yet wired into `forces::evaluate`;
///   nothing calls `record_tap` or `arm`. Removing the six new writes reproduced the 2026-08-19
///   constants.
/// - **2026-09-05 (later), hash input.** `roll_payout_timer` (ours for `craft+0x1c0 & 0x400`),
///   with three consumer branches wired into terms exercised every tick
///   (`airbrake::lateral_grip`, `hover::probe_from_hit`'s `rebound_override`, `engine::engine`'s
///   turbo add), plus `barrel_roll::release` on landing. The timer never leaves `0.0`, so every
///   new `else` arm is what ran before. Removing the one write reproduced the earlier constants.
/// - **2026-09-06, hash input; checked, not assumed.** The roll became *reachable*:
///   `ShipControls` gained d-pad tap edges, `barrel_roll::advance_gesture` reads them and the
///   axis, and `ShipState` gained `roll_axis_zone`. **Still no script arms one**:
///   `probe::controls` holds `steer_x` at `+-0.8` (slalom) and `+-0.6` (wall runs), inside
///   `barrel_roll::AXIS_TAP_THRESHOLD` (`0.9`), and sets neither tap field. Removing the one
///   `roll_axis_zone` write reproduced the 2026-09-05 constants. A script pushing the axis past
///   `0.9` in an alternation would move these for a real reason.
/// - **2026-09-29, behaviour.** The hull port of the sunk-craft recovery: floor contacts
///   (`Collision_BoxAgainstMesh`), `Collision_AddContact`'s projection gate, and
///   `Body_StepWorld`'s pass 1 (`wall::pre_integration_clip`, `0x0884f70c`). Only the 3,600-tick
///   `Aerobatic` row moved. Removing the `pre_integration_clip` call (all else left in)
///   reproduced the old `Aerobatic` pair, so pass 1 is the whole movement: it fires on the wall
///   runs where the swept ray it replaced never did.
/// - **2026-09-30, behaviour (the two 3,600-tick rows).** `airbrake::evaluate`'s forward `drag`
///   term multiplied `steerX` on `-1..=1` where `Ship_UpdateAirbrakes` reads it on `+/-100`
///   (`Ship_UpdateSteering` compares that field to the `+/-100` ramped state at `0x088487b8`), so
///   it ran 100x weak. Found by a one-tick pose walk of `talons-junction-clean-lap.csv`
///   (`lap_window_ground_truth.rs` in `oag-trace`). The 600-tick row did not move: the term needs
///   an airbrake imbalance and a steering deflection at once.
const REFERENCE: &[(u32, Script, u64, u64)] = &[
    (
        600,
        Script::Corridor,
        0x7e0e_8ab7_7ca6_7b3b,
        0x67fa_c2e0_9bb5_db28,
    ),
    (
        3_600,
        Script::Corridor,
        0x0475_b934_4503_366e,
        0xe5a7_709d_5dbd_3343,
    ),
    (
        3_600,
        Script::Aerobatic,
        0xccf3_51b3_110c_e688,
        0x4f50_920e_9828_3d82,
    ),
];

#[test]
fn the_simulation_matches_the_committed_reference() {
    let mut failures = Vec::new();

    for &(ticks, script, expected_final, expected_trajectory) in REFERENCE {
        let got = probe::run(script, ticks);
        if got.final_hash != expected_final || got.trajectory_hash != expected_trajectory {
            failures.push(format!(
                "ticks={ticks} script={script:?}\n  \
                 final:      expected {expected_final:#018x}, got {:#018x}\n  \
                 trajectory: expected {expected_trajectory:#018x}, got {:#018x}",
                got.final_hash, got.trajectory_hash
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "the ship simulation is not reproducible on this platform ({} / {}):\n\n{}\n\n\
         Do not update the constants to silence this. See the module docs.",
        std::env::consts::OS,
        std::env::consts::ARCH,
        failures.join("\n\n")
    );
}

/// Guards the property the reference depends on: a run is a pure function of its inputs. If
/// this fails the test above is meaningless even when it passes.
#[test]
fn the_simulation_is_stable_across_repeated_runs() {
    for &(ticks, script, _, _) in REFERENCE {
        let first = probe::run(script, ticks);
        for attempt in 0..4 {
            assert_eq!(
                probe::run(script, ticks),
                first,
                "run {attempt} diverged for ticks={ticks} script={script:?}"
            );
        }
    }
}

/// The two scripts must be two experiments, or the third row is dead weight that still passes.
#[test]
fn the_two_scripts_are_different_experiments() {
    assert_ne!(
        probe::run(Script::Corridor, 3_600),
        probe::run(Script::Aerobatic, 3_600)
    );
}

/// Destructuring answers "is the field in the list", not "does it reach the hash": a binding can
/// be destructured and never written, and two adjacent `f32`s can be transposed. This does.
#[test]
fn every_hashed_field_reaches_the_hash() {
    fn hash_of(state: &ShipState) -> u64 {
        let mut hasher = StateHasher::new();
        probe::hash_state(&mut hasher, state);
        hasher.finish()
    }

    let base = ShipState::default();
    let unmoved = hash_of(&base);

    /// One field's perturbation: what it is called, and how to move it.
    type Move = (&'static str, fn(&mut ShipState));

    let moves: &[Move] = &[
        ("body.position", |s| s.body.position.x = 1.0),
        ("body.orientation", |s| {
            s.body.orientation = Quat::from_xyzw(0.0, 1.0, 0.0, 0.0);
        }),
        ("body.linear_velocity", |s| s.body.linear_velocity.y = 1.0),
        ("body.angular_velocity", |s| s.body.angular_velocity.z = 1.0),
        ("body.force", |s| s.body.force.x = 1.0),
        ("body.torque", |s| s.body.torque.y = 1.0),
        ("body.mass", |s| s.body.mass = 2.0),
        ("body.inertia", |s| s.body.inertia.z = 3.0),
        ("airbrake_left", |s| s.airbrake_left = 1.0),
        ("airbrake_right", |s| s.airbrake_right = 1.0),
        ("thrust", |s| s.thrust = 1.0),
        ("brake", |s| s.brake = 1.0),
        ("steer", |s| s.steer = 1.0),
        ("reverse_controls", |s| s.reverse_controls = 1.0),
        ("stun_timer", |s| s.stun_timer = 1.0),
        ("wall_contact_prev", |s| s.wall_contact_prev = true),
        ("pending_impulse", |s| s.pending_impulse.z = 1.0),
        ("slowdown_timer", |s| s.slowdown_timer = 1.0),
        ("grounded", |s| s.grounded = 1.0),
        ("grounded_prev", |s| s.grounded_prev = 1.0),
        ("sideshift_timers[0]", |s| s.sideshift_timers[0] = 1.0),
        ("sideshift_timers[1]", |s| s.sideshift_timers[1] = 1.0),
        ("time_since_landing", |s| s.time_since_landing = 2.0),
        ("mag_lock_blend", |s| s.mag_lock_blend = 1.0),
        ("pad_timer", |s| s.pad_timer = 1.0),
        ("pad_direction", |s| s.pad_direction.z = 1.0),
        // The zeroed contact is what the discriminant byte exists for: byte-identical to `None`
        // in every field it has.
        ("mag_contact", |s| {
            s.mag_contact = Some(MagContact {
                point: Vec3::ZERO,
                normal: Vec3::ZERO,
            });
        }),
        ("shield", |s| s.shield = 1.0),
        ("craft_state", |s| s.craft_state = CraftState::Eliminated),
        ("state_timer", |s| s.state_timer = 1.0),
    ];

    for (name, apply) in moves {
        let mut moved = base;
        apply(&mut moved);
        assert_ne!(
            hash_of(&moved),
            unmoved,
            "moving {name} did not change the hash, so it is not covered"
        );
    }
}

/// The run has to exercise the paths the gate claims to cover: a ship that never leaves its
/// start, touches a wall or turns gives stable hashes over almost none of the crate.
#[test]
fn the_run_visits_the_paths_it_claims_to_cover() {
    let handling = probe::handling();
    let world = probe::corridor();
    let environment = Environment::default();
    let mut state = probe::start(&handling);

    let mut touched_wall = false;
    let mut left_the_ground = false;
    let mut was_grounded = false;
    let mut yawed_left = false;
    let mut yawed_right = false;
    let mut sideshifted = false;
    let full_pool = handling.dimensions.shield;
    let mut lost_energy = false;

    for tick in 0..3_600 {
        let evaluated = step(
            &mut state,
            &probe::controls(Script::Corridor, tick),
            &handling,
            &environment,
            &world,
            1.0 / 60.0,
        );
        touched_wall |= evaluated.wall.contacts > 0;
        was_grounded |= state.grounded > 0.0;
        left_the_ground |= state.grounded == 0.0;
        sideshifted |= state.sideshift_timers.iter().any(|t| *t > 0.0);
        yawed_left |= state.body.angular_velocity.y > 1e-3;
        yawed_right |= state.body.angular_velocity.y < -1e-3;
        assert!(
            state.body.position.is_finite() && state.body.linear_velocity.is_finite(),
            "tick {tick}: the fixture run went non-finite, so the hashes mean nothing"
        );
        // The claims the history above rests on, asserted not just written: the boost and the
        // pending impulse are inert here, so their fields hold defaults and those hashes moved
        // only because the stream got longer. A future `Environment` default carrying a pad hit
        // would otherwise quietly turn the notes into a lie.
        assert_eq!(state.pad_timer, 0.0, "tick {tick}: the boost armed");
        assert_eq!(state.pad_direction, Vec3::ZERO);
        assert_eq!(
            state.pending_impulse,
            Vec3::ZERO,
            "tick {tick}: an impulse posted"
        );
        lost_energy |= state.shield < full_pool;
    }

    assert!(was_grounded, "the run never found the floor");
    assert!(left_the_ground, "the run never left the floor");
    assert!(touched_wall, "the run never reached a wall");
    assert!(sideshifted, "the sideshift never armed");
    assert!(yawed_left && yawed_right, "the run never yawed both ways");
    // The pool is hashed, and a hashed field that never moves is coverage in name only: a new
    // branch in the damage law would land with the hashes unchanged. `Environment::default()`
    // has damage on, so the wall contacts must cost something.
    assert!(lost_energy, "the run never spent any energy on a wall");
}
