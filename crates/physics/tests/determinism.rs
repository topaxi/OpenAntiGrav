//! The cross-platform determinism gate, over the **actual** simulation.
//!
//! `oag-core`'s `tests/determinism.rs` is the older half of this gate and hashes
//! `oag_core::probe`, a miniature simulation written before there was a real
//! one. It exercises the shared float pipeline and nothing else: no force law,
//! no collision query, no branch that depends on where a ship is. This file is
//! the other half, and it steps [`oag_physics::step`] - the same entry point
//! `oag_game`'s race loop steps - through [`oag_physics::probe`], which is where
//! the scenario, the fixture world and the hashing live.
//!
//! `docs/reverse-engineering/verification-protocol.md` makes this a precondition
//! rather than a nicety: *before any comparison against the original is
//! meaningful, our own simulation must be reproducible.* Until this file
//! existed, the gate that sentence points at did not see the simulation at all.
//!
//! # When this test fails
//!
//! Do not update the constants to make it pass. That converts a real bug into a
//! silent one. The usual suspects, in order of likelihood:
//!
//! 1. A `mul_add` crept into `crates/physics/src`.
//! 2. A build flag enabled fast-math or FMA contraction.
//! 3. `glam`'s `scalar-math` feature got dropped, re-enabling SIMD.
//! 4. A transcendental entered the simulation path. **`crates/physics/src` had
//!    none outside `#[cfg(test)]` when this gate was written**, and that is a
//!    property to preserve rather than a coincidence: IEEE-754 requires correct
//!    rounding for `sqrt` but not for `sin`/`cos`/`exp`, so those are genuinely
//!    platform-dependent.
//! 5. A `HashMap`/`HashSet` reached the collision world or the parameter path.
//!    `CollisionWorld` is ordered storage for exactly this reason - see
//!    `docs/architecture/determinism.md`.
//!
//! A failure here with `oag-core`'s gate passing narrows the cause to this
//! crate; a failure in both is the shared foundation. A failure of
//! [`the_simulation_is_stable_across_repeated_runs`] alone is not a float
//! problem at all - it is uninitialised or address-dependent state, so look for
//! a map or a pointer-derived value before looking at arithmetic.
//!
//! Debug and release are both covered, between two jobs rather than inside this
//! file: CI's `check` job runs the whole workspace's tests in debug and its
//! `determinism` job runs this one in release.
//!
//! Regenerate deliberately, only after establishing that a change of behaviour
//! is intended - a change to the force law changes these hashes and *should*:
//!
//! ```sh
//! cargo run -q -p oag-physics --example determinism_report
//! ```

use oag_core::hash::StateHasher;
use oag_core::math::{Quat, Vec3};
use oag_physics::maglock::MagContact;
use oag_physics::probe::{self, Script};
use oag_physics::{Environment, ShipState, step};

/// `(ticks, script, final_hash, trajectory_hash)`.
///
/// Recorded on x86_64 Linux with the example named in the module docs.
///
/// # History
///
/// - **First recorded 2026-07-29**, when this gate was added. Nothing was
///   regenerated: there was no previous reference for the simulation, because
///   the simulation was not covered by any determinism gate.
const REFERENCE: &[(u32, Script, u64, u64)] = &[
    (
        600,
        Script::Corridor,
        0x878c_7299_c74c_9024,
        0x7b01_4d1c_40fc_e927,
    ),
    (
        3_600,
        Script::Corridor,
        0x6bbc_bf04_f931_4240,
        0xe2ef_5959_3ae4_d468,
    ),
    (
        3_600,
        Script::Aerobatic,
        0xb788_6dbd_345d_4120,
        0xbb3b_2986_eaca_4546,
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

/// Guards the property the reference depends on: that a run is a pure function
/// of its inputs. If this fails, the test above is meaningless even when it
/// passes.
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

/// The two scripts must actually be two experiments. If they produced the same
/// trajectory the third reference row would be dead weight that still passed.
#[test]
fn the_two_scripts_are_different_experiments() {
    assert_ne!(
        probe::run(Script::Corridor, 3_600),
        probe::run(Script::Aerobatic, 3_600)
    );
}

/// Destructuring answers "is the field in the list". It does not answer "does
/// the field reach the hash": a binding can be destructured and then never
/// written, and two adjacent `f32`s can be transposed. This does.
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
        ("leap_timer", |s| s.leap_timer = 1.0),
        ("grounded", |s| s.grounded = 1.0),
        ("grounded_prev", |s| s.grounded_prev = 1.0),
        ("sideshift_timers[0]", |s| s.sideshift_timers[0] = 1.0),
        ("sideshift_timers[1]", |s| s.sideshift_timers[1] = 1.0),
        ("time_since_landing", |s| s.time_since_landing = 2.0),
        ("mag_lock_blend", |s| s.mag_lock_blend = 1.0),
        // The zeroed contact is the case the discriminant byte exists for: it
        // is byte-identical to `None` in every field it has.
        ("mag_contact", |s| {
            s.mag_contact = Some(MagContact {
                point: Vec3::ZERO,
                normal: Vec3::ZERO,
            });
        }),
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

/// The run has to exercise the paths this gate claims to cover. A ship that
/// never leaves its start, never touches a wall or never turns would give
/// perfectly stable hashes over almost none of the crate.
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
    }

    assert!(was_grounded, "the run never found the floor");
    assert!(left_the_ground, "the run never left the floor");
    assert!(touched_wall, "the run never reached a wall");
    assert!(sideshifted, "the sideshift never armed");
    assert!(yawed_left && yawed_right, "the run never yawed both ways");
}
