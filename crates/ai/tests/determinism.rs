//! The cross-platform determinism gate, over the **drivers**.
//!
//! The third of three: `oag-core`'s `tests/determinism.rs` hashes the math
//! foundation, `oag-physics`'s the force law under *scripted* input, and this
//! one the part that decides what the input should be, through
//! [`oag_ai::probe`].
//!
//! # Why it was added
//!
//! [`oag_ai::Line::curvature`] called the platform's own `acos` from the day it
//! was written. `docs/architecture/determinism.md` forbids that (IEEE-754
//! requires correct rounding for `sqrt`, not transcendentals), and the angle sets
//! the speed target every craft brakes against. Nothing failed because **nothing
//! cross-platform ever ran a driver**. Measured before the fix, this machine's
//! `f32::acos` and `libm::acosf` differed on 8.6 % of samples by up to one ULP
//! (`3e-8` in the band a racing line lands in): one bit in the world hash.
//!
//! **Verified to catch it, not merely asserted**: with `oag_core::math::acos`
//! put back on `f32::acos`, [`the_drivers_match_the_committed_reference`] fails
//! on the `field`/600-tick trajectory hash (`0x3fb6_79e7_685e_a9b3` against the
//! then-committed `0xe2c9_292e_b42a_9aeb`, the 2026-08-15 run's; [`REFERENCE`]
//! has moved since). **The `solo`/600 row still matched**: one craft for ten
//! seconds does not accumulate a one-ULP angle into visible state, four reacting
//! to each other do. That is why the `Field` rows exist; dropping them for speed
//! takes the gate's teeth with them.
//!
//! # When this test fails
//!
//! Do not update the constants to make it pass. In order of likelihood:
//!
//! 1. A transcendental entered `crates/ai/src` or `crates/physics/src` (stepped
//!    here too). Bring the implementation in, as `oag_core::math::acos` did.
//! 2. A `Driver` field was added, removed or reordered. All are hashed on
//!    purpose (`probe::write_craft`), so the constants move: isolate it as
//!    `oag_gameplay`'s gate documents, re-running with only the new field's
//!    `write_*` removed and checking the old constants reproduce bit for bit.
//! 3. A personality draw was inserted anywhere but the end, which re-rolls every
//!    later axis for every pilot.
//! 4. A `HashMap`/`HashSet` reached the driver or the field.
//! 5. **`libm` was updated**: `Cargo.lock` pins it and a reworked `acosf` moves
//!    these hashes. Check `git diff Cargo.lock` first.
//!
//! A failure here with the other two gates passing narrows the cause to this
//! crate. [`the_drivers_are_stable_across_repeated_runs`] failing alone is not a
//! float problem: it is uninitialised or address-dependent state.
//!
//! Regenerate deliberately, only once a behaviour change is established as
//! intended:
//!
//! ```sh
//! cargo run -q -p oag-ai --example ai_determinism_report
//! ```

use oag_ai::probe::{self, Scenario};

/// `(ticks, scenario, final_hash, trajectory_hash)`, recorded on x86_64 Linux
/// with the example named in the module docs.
///
/// # History
///
/// Each regeneration was a deliberate behaviour change; which rows moved says
/// what the change reached. `oag-core`, `oag-physics` and `oag-gameplay`'s gates
/// passed unchanged each time unless noted, and the scenario's own curvature
/// spread stayed `0.000255..0.020490` / `0.000036..0.026295`
/// ([`the_scenario_still_exercises_corners_and_craft_that_drive`] measures the
/// line, not what a driver read off it).
///
/// - **2026-08-15**: first recorded with the `acos` fix. No previous reference.
/// - **2026-08-24, `Field` rows, twice**: `Driver::ram` measures corridor
///   clearance from the craft and asks for the widened `RAM_CLEARANCE`; then a
///   ram may only target the player's slot (`driver::ram`'s `PLAYER_SLOT`).
///   **`Solo` did not move on either**: a craft with nobody alongside never
///   reaches the gate, the check that each change stayed on its path.
/// - **2026-09-06, all rows**: [`oag_ai::Line::curvature`]'s three walks all
///   root at `index` instead of chaining off each other's landing index, moving
///   every angle by a sub-sample amount (hence `Solo` moved). **Isolated**: the
///   same commit's zero-chord guard applied alone reproduced all rows bit for
///   bit.
/// - **2026-09-06, all rows, later**: [`oag_ai::Tuning::curvature_span`]'s
///   ceiling (eleven units), changing what the estimator measures over on every
///   reading above a standstill. **Isolated**: the same change's yaw-rate term in
///   `corner_target` alone reproduced all rows (it binds above `k` about 0.0125,
///   which the uncapped readings stay under), and the ceiling set back to `None`
///   reproduced them again, so the ceiling is the whole movement. **Recorded
///   twice that day**: the first chosen value was ten and turned two disc-backed
///   field tests red; these are the eleven-unit hashes (a `Solo` row reading
///   `0x94d4_044f...` is the rejected value's).
/// - **2026-09-06, all rows, third**: [`oag_ai::Tuning::look_speed`] `0.35` to
///   `0.30`, which sets the steering lookahead on every reading, so all three
///   moved.
/// - **2026-09-07, `Field` rows**: [`oag_ai::Driver::social`] reads
///   `ctx.field.alongside` (preferred over `behind`). `Solo` carries no rival and
///   reproduced bit for bit (`0x65d7_0dd3_7566_c624`).
/// - **2026-09-08, `Field` rows**: `Driver::social`'s `CONTACT_FLOOR`. `Solo`
///   reproduced bit for bit again (`0x65d7_0dd3_7566_c624`, `0xb1f8_d6c8_1437_06ee`),
///   confirming the floor fires only alongside a rival. `Driver::holds_fire`,
///   added the same day, cannot reach this file: no weapon exists in the probe.
/// - **2026-09-11, all rows**: the differential airbrake's
///   `Driver::peak_curvature` and `driver::pace::trail`'s gate built on it. Not a
///   pure field addition: this scenario calls `Driver::drive` every tick, and
///   `Solo` moved because the differential is line-following arithmetic reached
///   on its own corners (curvature up to 0.026295). **Isolated**: with the
///   `peak_curvature` write removed from `write_craft` and the gate kept, the
///   hashes were `0x00ba_9d38_3cae_9ba4` / `0x7f2b_8f0b_f392_44fa` (`Solo`),
///   `0x7a39_e5be_5990_378b` / `0x72cc_ce44_8811_242c` (`Field`/600) and
///   `0xf426_db64_a7e7_393b` / `0x986a_1ec6_55d7_a781` (`Field`/1,800), still
///   moved from before: the gate alone accounts for it.
/// - **2026-09-11, all rows, later**: [`oag_ai::Tuning::trail_deadband`],
///   [`oag_ai::Tuning::trail_gain`], [`oag_ai::Tuning::trail_saturation`]
///   (`0.7`/`0.05`/`3.0`; sweep in `docs/gameplay/ai.md`, "Tuning sweep tables").
/// - **2026-09-30, all rows**: the player's physics, no driver code:
///   `oag_physics::airbrake::evaluate`'s forward `drag` ran 100x weak (raw
///   `steerX` on `-1..=1` where the original holds `+/-100`); see `oag-physics`'
///   gate. The AI flies the player's physics, so every row moves.
/// - **2026-09-30, all rows**: `Line::curvature`'s `bend_angle` (yaw plus the
///   *convex* pitch). This line is flat so no corner moved: the yaw from chords
///   flattened onto the ground plane differs from the plain chord angle in the
///   last bit. No transcendental added; `oag_core::math::acos` is the only one.
const REFERENCE: [(u32, Scenario, u64, u64); 3] = [
    (
        600,
        Scenario::Solo,
        0xb27b_df5d_9fd5_c28d,
        0x74b5_77e2_abbc_dc2d,
    ),
    (
        600,
        Scenario::Field,
        0x5249_34d8_34dd_e4e6,
        0xfac7_6d7b_763e_551e,
    ),
    (
        1_800,
        Scenario::Field,
        0x9548_b23b_b132_b7b7,
        0xb7da_882b_82c1_cdb7,
    ),
];

#[test]
fn the_drivers_match_the_committed_reference() {
    for &(ticks, scenario, final_hash, trajectory_hash) in &REFERENCE {
        let result = probe::run(scenario, ticks);
        assert_eq!(
            result.final_hash, final_hash,
            "final state hash moved for {scenario} over {ticks} ticks \
             (got {:#018x}, expected {final_hash:#018x}) - read this file's \
             header before touching the constant",
            result.final_hash
        );
        assert_eq!(
            result.trajectory_hash, trajectory_hash,
            "trajectory hash moved for {scenario} over {ticks} ticks \
             (got {:#018x}, expected {trajectory_hash:#018x}) - the run \
             diverged mid-way even if it ended in the same place",
            result.trajectory_hash
        );
    }
}

#[test]
fn the_drivers_are_stable_across_repeated_runs() {
    for &(ticks, scenario, _, _) in &REFERENCE {
        let first = probe::run(scenario, ticks);
        let second = probe::run(scenario, ticks);
        assert_eq!(
            first, second,
            "two runs of {scenario} over {ticks} ticks disagreed in the same \
             process, which is not a float portability problem - look for \
             uninitialised or address-dependent state"
        );
    }
}

/// **The guard on the guard.** Hashes agree perfectly over a scenario that tests
/// nothing, so this asserts the scenario is still worth hashing. It stops being
/// so if the circuit flattens into one radius (every `acos` sees one angle, and
/// an error monotone in it moves nothing) or the craft stop driving (spun off,
/// or placed beside the line), leaving a reproducible run where almost nothing
/// happens. The bounds are loose: not a controller regression (`closed_loop.rs`)
/// but a tripwire on the fixture.
#[test]
fn the_scenario_still_exercises_corners_and_craft_that_drive() {
    let result = probe::run(Scenario::Field, 1_800);

    let (lowest, highest) = result.curvature;
    assert!(
        lowest < 0.001,
        "the circuit no longer has anything straight in it: curvature bottoms \
         out at {lowest} per unit"
    );
    assert!(
        highest > 0.01,
        "the circuit no longer has a real corner in it: curvature peaks at \
         {highest} per unit, a radius of {}",
        1.0 / highest
    );
    assert!(
        highest / lowest > 10.0,
        "every corner on this circuit is now the same radius ({lowest}..{highest} \
         per unit), so an error in the angle that is monotone in curvature \
         would move nothing"
    );
    assert!(
        result.travelled > 1_500.0,
        "the least travelled craft went {} units in 1,800 ticks, which is not a \
         craft driving a circuit - the hashes above would still agree",
        result.travelled
    );
}

/// The speed plan learned on the probe's circuit, hashed ([`probe::speed_plan`]).
/// Added 2026-10-03 with the plan; nothing above moved, every scenario there
/// drives with `plan: None`.
const PLAN_REFERENCE: u64 = 0xc6b7_1a41_c017_d1b5;

#[test]
fn the_speed_plan_matches_the_committed_reference() {
    let (hash, report) = probe::speed_plan();
    println!("speed plan: {hash:#018x} {report:?}");
    assert!(
        report.verify_lap_ticks.is_some(),
        "the probe's plan never completed its verification lap, so the hash pins a failure: {report:?}"
    );
    assert_eq!(
        hash, PLAN_REFERENCE,
        "the speed plan on the probe's circuit is not the committed one: {hash:#018x}"
    );
}
