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
use oag_physics::{CraftState, Environment, ShipState, step};

/// `(ticks, script, final_hash, trajectory_hash)`.
///
/// Recorded on x86_64 Linux with the example named in the module docs.
///
/// # History
///
/// - **First recorded 2026-07-29**, when this gate was added. Nothing was
///   regenerated: there was no previous reference for the simulation, because
///   the simulation was not covered by any determinism gate.
/// - **Regenerated 2026-07-30.** `hull_sample_points`/`hull_extent`
///   (`crates/physics/src/wall.rs`) now scale `<Misc>` hull dimensions by
///   `hover::TARGET_GLOBAL_SCALE` (`0.75`) before building the collision box,
///   matching `Ship_InitCraft`'s box-collider setup read at instruction level -
///   see `docs/ghidra/functions/psp-pulse-usa/collision.md`. Only the two
///   3,600-tick hashes moved; the 600-tick `Corridor` entry is untouched
///   because that scenario never reaches a wall in 600 ticks, which is the
///   expected shape of a change scoped to contact geometry.
/// - **Regenerated 2026-08-03**, and **no behaviour changed**. `ShipState` gained
///   `pad_timer` and `pad_direction` for the speed-pad boost
///   (`crates/physics/src/engine.rs`, `speedup_pad`), so `probe::hash_state`
///   writes four more `f32`s per tick and the FNV-1a stream is longer. The boost
///   itself cannot have run: `probe::run` drives every script with
///   `Environment::default()`, whose `pad_hit` is `None`, and with no hit the
///   timer never leaves `0.0` and the term returns `Vec3::ZERO` before touching
///   an accumulator. So the two fields hold their defaults for all 3,600 ticks of
///   every script and contribute a *constant* run of bytes.
///
///   **All three rows moved this time, including the 600-tick `Corridor`**, which
///   is the tell that this is a hash-input change rather than a force-law one: a
///   change to behaviour reaches the scenarios that exercise it, a change to what
///   is hashed reaches all of them equally.
/// - **Regenerated 2026-08-04**, and **no behaviour changed**. `ShipState` gained
///   `shift_tap_windows`, `shift_armed` and `shift_lockout` for the two sideshift
///   gestures the original triggers on (`crates/physics/src/airbrake.rs`,
///   `advance_sideshift`; see
///   `docs/ghidra/functions/psp-pulse-usa/input-bindings.md`), so `probe::hash_state`
///   writes three more `f32`s and a `u8` per tick.
///
///   All three rows moved again, same tell as above. This time it was also
///   **checked directly** rather than argued from the shape: with exactly the
///   four new writes deleted from `hash_state` and nothing else changed, the run
///   reproduces the 2026-08-03 hashes bit for bit. So the trajectory is
///   untouched and only the stream is longer. That is the check to repeat before
///   pasting new constants in here - it is cheap, and it is the difference
///   between "the hash moved because I added a field" and "the hash moved and I
///   assumed that was why".
///
///   The reason behaviour cannot have changed: `probe::controls` never sets
///   `shift_modifier` or either `shift_tap_*`, so no gesture can arm on any
///   script, and the one `Sideshift::Left` at tick 1200 goes through
///   `ShipControls::sideshift`, which is a direct request and deliberately
///   bypasses `shift_lockout`. The 600-tick `Corridor` row does not even reach
///   that tick, so on that row all four new fields hold their defaults for the
///   whole run - and it moved anyway, which is the tell.
/// - **Regenerated 2026-08-08, and this time behaviour *did* change - on
///   purpose.** Two things landed together and both are deliberate:
///
///   1. `crate::engine::speedup_pad` implements `<Special speedpad_jump>`, the
///      original's `if (controls->0x24 & 1) dir += craft+0x160 * jump` - a
///      5.71-degree tilt of the boost toward the hull's up axis while d-pad Up is
///      held. See that function's docs and
///      `docs/ghidra/functions/psp-pulse-usa/engine.md`.
///   2. **`probe::environment` is new and the scenario now crosses a speed pad,
///      twice.** Until now no script ever set `pad_hit`, so step 15 of 15 was
///      the one force term this gate did not cover at all - and a new branch
///      inside an uncovered term would have landed with the hashes not moving,
///      which is the worst possible outcome for a gate.
///
///   **No `ShipState` field was added**, so unlike the four entries above this is
///   not a longer hash stream: the movement is trajectory, not bookkeeping. Two
///   things say so, and both were checked rather than argued:
///
///   - **The 600-tick `Corridor` row does not move.** The first crossing is at
///     tick 1600. A change to what is *hashed* reaches every row equally; a
///     change to *behaviour* reaches only the rows that reach it.
///   - **With `environment` returning `Environment::default()` and nothing else
///     touched, all three rows reproduce the 2026-08-04 constants bit for bit.**
///     That is the check the entry above asks for, and it isolates the cause to
///     the two crossings alone - the two crossings are deliberately placed inside
///     stretches of `probe::controls` that already hold the pitch axis on either
///     side of the tilt's threshold, so **not one byte of the input script
///     changed** and there is no second cause to disentangle.
/// - **Regenerated 2026-08-10, and behaviour did *not* change.** `ShipState`
///   gained `shield`, the energy pool at the original's `entity+0x88`, and
///   `crate::damage::apply_contact` spends it out of the frame's contact
///   impulses. **Nothing reads the pool back**, so no force term can see it and
///   the trajectory is untouched; the movement is a longer hash stream, the same
///   kind as the four bookkeeping entries above.
///
///   That is checked rather than argued, by the isolation those entries ask for:
///   with `hasher.write_f32(shield)` alone commented out and every other change
///   in place, **all six constants from 2026-08-08 reproduce bit for bit**. So
///   the field reaches the hash and reaches nothing else.
///
///   `probe::start` now fills the pool through `crate::damage::reset`, because
///   `ShipState::default()` starts it at zero and an already-empty pool cannot be
///   depleted - `the_run_visits_the_paths_it_claims_to_cover` asserts the run
///   actually spends energy, for the same reason the speed-pad entry above exists.
/// - **Regenerated 2026-08-10 a second time, and behaviour did not change
///   either.** `ShipState` gained `craft_state` and `state_timer`, the three
///   states the energy pool reaches (`crate::damage::CraftState`) and their
///   timer. Nothing in the force law reads them and no probe script empties a
///   pool, so both hold their defaults for every tick of every run - a longer
///   hash stream and nothing else. Checked the same way: with the two writes at
///   the end of `hash_state` commented out and every other change in place, the
///   constants from earlier the same day reproduce bit for bit.
const REFERENCE: &[(u32, Script, u64, u64)] = &[
    (
        600,
        Script::Corridor,
        0xad32_1a4c_5459_85a8,
        0x9220_7924_fa4d_dafb,
    ),
    (
        3_600,
        Script::Corridor,
        0x9eb5_732a_f116_de6c,
        0x8799_28cb_63d0_6e1a,
    ),
    (
        3_600,
        Script::Aerobatic,
        0x9cb3_8272_be8b_262b,
        0x7189_75cb_27ce_9bc1,
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
        ("pad_timer", |s| s.pad_timer = 1.0),
        ("pad_direction", |s| s.pad_direction.z = 1.0),
        // The zeroed contact is the case the discriminant byte exists for: it
        // is byte-identical to `None` in every field it has.
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
        // The claim the 2026-08-03 regeneration rests on: the boost is inert
        // here, so its two fields hold their defaults for the whole run and the
        // hashes moved only because the stream got longer. Asserted rather than
        // asserted-in-prose, because a future `Environment` default carrying a
        // pad hit would quietly turn that history note into a lie.
        assert_eq!(state.pad_timer, 0.0, "tick {tick}: the boost armed");
        assert_eq!(state.pad_direction, Vec3::ZERO);
        lost_energy |= state.shield < full_pool;
    }

    assert!(was_grounded, "the run never found the floor");
    assert!(left_the_ground, "the run never left the floor");
    assert!(touched_wall, "the run never reached a wall");
    assert!(sideshifted, "the sideshift never armed");
    assert!(yawed_left && yawed_right, "the run never yawed both ways");
    // The pool is hashed, and a hashed field that never moves is coverage in
    // name only - a new branch inside the damage law would land with the
    // reference hashes unchanged, which is the worst outcome this gate has.
    // `Environment::default()` has damage on, so the wall contacts above must
    // cost something.
    assert!(lost_energy, "the run never spent any energy on a wall");
}
