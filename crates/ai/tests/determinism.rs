//! The cross-platform determinism gate, over the **drivers**.
//!
//! The third of three. `oag-core`'s `tests/determinism.rs` hashes the math
//! foundation and `oag-physics`'s hashes the force law under a *scripted*
//! input - so between them they cover every arithmetic path in the simulation
//! except the one that decides what the input should be. This file covers that
//! one, through [`oag_ai::probe`].
//!
//! # Why it was added
//!
//! [`oag_ai::Line::curvature`] called the platform's own `acos` from the day it
//! was written. `docs/architecture/determinism.md` forbids that - IEEE-754
//! requires correct rounding for `sqrt` and not for the transcendentals, so two
//! targets may legitimately differ in the last bit - and the angle it computes
//! sets the speed target every craft brakes against, which is simulation state.
//! Nothing failed, because **nothing cross-platform ever ran a driver**.
//!
//! Measured before the fix, over the domain a clamped unit-vector dot actually
//! produces: this machine's `f32::acos` and `libm::acosf` differ on 8.6 % of
//! samples, by up to one ULP (`2.4e-7` rad at the extreme, `3e-8` in the band a
//! racing line lands in). One ULP in a speed target is one bit in the world
//! hash, which is the whole of what a determinism gate is about.
//!
//! `curvature` calls `oag_core::math::acos` now, and **this gate was verified to
//! catch the difference rather than merely asserted to**: with
//! `oag_core::math::acos` temporarily put back on `f32::acos`,
//! [`the_drivers_match_the_committed_reference`] fails on the `field`/600-tick
//! trajectory hash (`0x3fb6_79e7_685e_a9b3` against the then-committed
//! `0xe2c9_292e_b42a_9aeb`; both numbers are the 2026-08-15 run's, and
//! [`REFERENCE`] has moved once since - see its own history).
//!
//! **What that same check also showed, and it is the reason `Field` rows are in
//! the reference at all**: the `solo`/600 row still matched. One craft
//! following a line for ten seconds does not accumulate a one-ULP angle into a
//! visible state difference; four craft reacting to each other do. A future
//! edit that drops the `Field` scenarios for being slower would quietly take
//! the gate's teeth with it.
//!
//! # When this test fails
//!
//! Do not update the constants to make it pass; that converts a real bug into a
//! silent one. In order of likelihood:
//!
//! 1. A transcendental entered `crates/ai/src` - or `crates/physics/src`, which
//!    this gate steps too. `oag_core::math::acos` is the pattern to follow:
//!    bring the implementation in, do not call the platform's.
//! 2. A `Driver` field was added, removed or reordered. Every one of them is
//!    hashed on purpose (`probe::write_craft`), so this is expected to move the
//!    constants - isolate it the way `oag_gameplay`'s gate documents, by
//!    re-running with only the new field's `write_*` removed and checking the
//!    old constants reproduce bit for bit.
//! 3. A personality draw was inserted anywhere but the end of the order, which
//!    silently re-rolls every later axis for every pilot.
//! 4. A `HashMap`/`HashSet` reached the driver or the field.
//! 5. **`libm` was updated.** These hashes are a function of that crate's
//!    implementation as much as of ours - `Cargo.lock` pins it, and a
//!    `cargo update` that lands a reworked `acosf` moves them. Check
//!    `git diff Cargo.lock` before going looking for a bug in the simulation.
//!
//! A failure here with the other two gates passing narrows the cause to this
//! crate. A failure of [`the_drivers_are_stable_across_repeated_runs`] alone is
//! not a float problem at all - it is uninitialised or address-dependent state.
//!
//! Regenerate deliberately, only once a change of behaviour is established as
//! intended:
//!
//! ```sh
//! cargo run -q -p oag-ai --example ai_determinism_report
//! ```

use oag_ai::probe::{self, Scenario};

/// `(ticks, scenario, final_hash, trajectory_hash)`.
///
/// Recorded on x86_64 Linux with the example named in the module docs.
///
/// # History
///
/// - **First recorded 2026-08-15**, when this gate was added alongside the
///   `acos` fix it exists to guard. Nothing was regenerated: there was no
///   previous reference, because no determinism gate had ever run a driver.
/// - **Both `Field` rows regenerated twice on 2026-08-24**, both times for a
///   deliberate change of behaviour and not a bug. First: `Driver::ram` measures
///   its corridor clearance from where the craft is rather than from the line,
///   and asks for `oag_ai::driver`'s widened `RAM_CLEARANCE`. Second: a ram may
///   only target the player's slot, because AI-on-AI shoving spirals in a clump
///   (see `driver::ram`'s `PLAYER_SLOT`). Fewer shifts fire either way, so a
///   field that shoves diverges. **`Scenario::Solo` did not move on either**,
///   which is the check that says each change is confined to the path it claims:
///   a craft with nobody alongside never reaches the gate. `oag_gameplay`'s own
///   gate did not move either - its scenario flies no pilot.
/// - **All three rows regenerated 2026-09-06**, for the chord spacing in
///   [`oag_ai::Line::curvature`]. The three walks used to chain off each
///   other's landing *index* rather than run from `index`, so the chords were
///   never quite `span` apart - by under a sample on a well-formed line, and by
///   the whole segment where one is longer than `span`. Rooting all three at
///   `index` moves the angle everywhere by that sub-sample amount, which is why
///   `Solo` moved this time when it did not on 2026-08-24: this is not a path a
///   craft has to reach, it is every curvature reading the estimator takes.
///
///   **Isolated before regenerating**, the way item 2 below asks for. The same
///   commit also returns zero on a chord of no length, and that half was
///   applied *alone*, with the old chaining left in place: all three rows
///   reproduced bit for bit. So the guard is confined to the degenerate case it
///   names - the synthetic scenarios never produce a zero chord - and the
///   chord spacing is the whole of the movement. `oag-core`, `oag-physics` and
///   `oag-gameplay`'s gates all still pass unchanged.
/// - **All three rows regenerated again 2026-09-06**, later the same day, for
///   [`oag_ai::Tuning::curvature_span`] - a **ceiling on the chord** rather
///   than a change to how the three walks are spaced. The span used to be half
///   the driver's own lookahead, so it grew with speed; capping it at eleven
///   units changes what the estimator measures over on every reading above a
///   standstill, and therefore every speed target on both scenarios. Same class
///   of cause as the item above, different cause: **do not read this entry as
///   that one.**
///
///   **Isolated before regenerating.** The same change also adds the yaw-rate
///   term to `driver::pace::corner_target`, and that half was applied *alone*:
///   all three rows reproduced bit for bit, because the term binds above
///   `k = max_turn_rate^2 / lateral_accel` (about 0.0125) and with the uncapped
///   span the driver's own readings on these fixtures stay under it. Then, with
///   the yaw term in place, setting `curvature_span` back to `None` reproduced
///   all three rows bit for bit again - so **the ceiling is the whole of the
///   movement** and the yaw term contributes none of it. `oag-core`,
///   `oag-physics` and `oag-gameplay`'s gates all still pass unchanged; the
///   scenario's own curvature spread is untouched (0.000255..0.020490 and
///   0.000036..0.026295, exactly as before), because
///   [`the_scenario_still_exercises_corners_and_craft_that_drive`] measures the
///   line rather than what a driver read off it.
///
///   **Recorded twice on the day**, because the first value the sweep chose was
///   ten and it turned two disc-backed field tests red - see
///   [`oag_ai::Tuning::curvature_span`]. These are the eleven-unit hashes. A
///   reference that still read `0x94d4_044f...` on the `Solo` row would be the
///   rejected value's.
/// - **All three rows regenerated 2026-09-06**, a third time that day, for
///   [`oag_ai::Tuning::look_speed`] - lowered from `0.35` to `0.30` to fix
///   `13_Track`'s own tracking-convergence lag (see that constant's own doc
///   comment for the sweep). `look_speed` sets `Driver::steering`'s lookahead
///   directly, which every reading above a standstill uses on both scenarios,
///   so **all three rows move this time** rather than only `Field` - unlike
///   the `curvature_span` entry above, which only bound above a speed neither
///   fixture reaches in `Solo`. `oag-core`, `oag-physics` and
///   `oag-gameplay`'s gates all still pass unchanged, and the scenario's own
///   curvature spread is untouched (`0.000255..0.020490` and
///   `0.000036..0.026295`, exactly as before), which is what
///   [`the_scenario_still_exercises_corners_and_craft_that_drive`] measures:
///   the line itself did not change, only what the driver does with it.
/// - **The two `Field` rows regenerated 2026-09-07**, for
///   [`oag_ai::Driver::social`] reading `ctx.field.alongside` (preferring it
///   over `ctx.field.behind`) rather than `behind` alone - see that
///   function's own doc comment for why. `Solo` never carries a rival at
///   all, so `alongside` and `behind` are both always `None` there and that
///   row reproduced bit for bit (`0x65d7_0dd3_7566_c624` unchanged) - only
///   `Field` moved, on both the 600- and 1,800-tick rows, because both put
///   more than one craft on the circuit. `oag-core`, `oag-physics` and
///   `oag-gameplay`'s gates all still pass unchanged, and the scenario's own
///   curvature spread is untouched, which is what
///   [`the_scenario_still_exercises_corners_and_craft_that_drive`] measures:
///   the line did not change, only how a driver reacts to a touching rival.
/// - **The two `Field` rows regenerated again 2026-09-08**, for
///   [`oag_ai::Driver::social`]'s new `CONTACT_FLOOR` - the minimum yield every
///   driver now gives a rival it is already alongside, whatever its personality
///   says. See that constant for what it is for and why it is chosen rather than
///   measured. The signature is exactly the previous regeneration's, which is
///   the check that it went where it was meant to: `Solo` carries no rival, so
///   `alongside` is always `None` there and the floor can never fire, and that
///   row reproduced **bit for bit** again (`0x65d7_0dd3_7566_c624` and
///   `0xb1f8_d6c8_1437_06ee`, both unchanged for the second time). Only the two
///   `Field` rows moved. `oag-core`, `oag-physics` and `oag-gameplay`'s gates
///   all still pass unchanged, and
///   [`the_scenario_still_exercises_corners_and_craft_that_drive`] still holds,
///   so the line is untouched and only the driver's response to contact moved.
///   `Driver::holds_fire`, added in the same change, cannot reach this file at
///   all - no weapon exists in [`oag_ai::probe`]'s scenarios.
/// - **All three rows regenerated 2026-09-11**, for the differential
///   airbrake's `Driver::peak_curvature` field and `driver::pace::trail`'s
///   new gate built on it - see that function's own doc for the real-track
///   measurement that replaced `speed < target`. **This is not a pure
///   field-addition entry like the ones above it**: unlike
///   `oag_gameplay`'s own determinism scenario, this one *does* call
///   `Driver::drive` on every tick, so `trail`'s new gate genuinely changes
///   what the differential does and every downstream position and velocity
///   with it. **`Solo` moved this time**, unlike the two 2026-09-07/08
///   `social` entries above: the differential is pure line-following
///   arithmetic, reached by a lone craft on this scenario's own corners
///   (curvature up to 0.026295), where `social` needs a rival `Solo` never
///   carries.
///
///   **Isolated to confirm the field itself is not doing the work**: with
///   `driver.peak_curvature`'s write commented out of `write_craft`,
///   keeping the new gate logic, all three hashes reproduced
///   `0x00ba_9d38_3cae_9ba4` / `0x7f2b_8f0b_f392_44fa` (`Solo`),
///   `0x7a39_e5be_5990_378b` / `0x72cc_ce44_8811_242c` (`Field`/600) and
///   `0xf426_db64_a7e7_393b` / `0x986a_1ec6_55d7_a781` (`Field`/1,800) -
///   still moved from the pre-2026-09-11 constants, so the gate change alone
///   accounts for the movement and the field's own bytes are not required to
///   see it. `oag-core` and `oag-physics`'s gates are untouched (`trail` is
///   `oag-ai` only), and
///   [`the_scenario_still_exercises_corners_and_craft_that_drive`] still
///   holds - the line did not change, only what a saturated driver does on
///   it.
/// - **All three rows regenerated again 2026-09-11, later the same day**, for
///   [`oag_ai::Tuning::trail_deadband`], [`oag_ai::Tuning::trail_gain`] and
///   [`oag_ai::Tuning::trail_saturation`] - reported from play: the gate above
///   fixed *when* the differential fires, but a live `--race --autopilot` run
///   showed it firing at a magnitude too small to see, because the old
///   `trail_saturation` (`0.85`) left `trail_deadband` (`0.15`) almost no
///   headroom to compute a command from. See
///   [`oag_ai::Tuning::trail_saturation`]'s own doc for the three-point sweep
///   against a real corner and the real seven-opponent field test that picked
///   `0.7`/`0.05`/`3.0`. All three rows move again for the same reason the
///   entry above gives: this scenario calls `Driver::drive`, so a differential
///   that behaves differently is a driver that ends up somewhere different.
/// - **All three rows regenerated 2026-09-30, for the player's physics.**
///   `oag_physics::airbrake::evaluate`'s forward `drag` term ran 100x weak,
///   reading the raw `steerX` on `-1..=1` where the original holds it on
///   `+/-100`; see `oag-physics`' own determinism gate for the entry. No
///   driver code changed: every row moves because the AI flies the player's
///   physics and airbrakes into corners, so the same inputs now carry it
///   somewhere else.
/// - **All three rows regenerated 2026-09-30, for `Line::curvature`'s split of
///   a pitch change** (`bend_angle`: yaw plus the *convex* pitch, a concave one
///   reading straight). This scenario's line is flat, so the curvature spread
///   it reports is unchanged (`0.000255..0.020490` and `0.000030..0.026295`) and
///   no corner moved: the rows move because `bend_angle` computes the yaw from
///   the chords flattened onto the ground plane, which differs from the plain
///   chord angle in the last bit, and one bit in a speed target is one bit in
///   the hash. No transcendental was added - `oag_core::math::acos` is the
///   only one it calls.
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

/// **The guard on the guard.** Hashes agree perfectly over a scenario that
/// tests nothing, so this asserts the scenario is still worth hashing.
///
/// Two ways it could quietly stop being: the circuit could flatten into one
/// radius, so every `acos` sees one angle and any error monotone in it moves
/// nothing; or the craft could stop driving - spun off, or placed beside the
/// line - leaving a run that is reproducible because almost nothing happens in
/// it.
///
/// The bounds are deliberately loose. They are not a regression on the
/// controller (that is `closed_loop.rs`); they are a tripwire on the fixture.
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
