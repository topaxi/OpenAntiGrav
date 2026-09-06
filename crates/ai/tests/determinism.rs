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
//! cargo run -q -p oag-ai --example determinism_report
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
const REFERENCE: [(u32, Scenario, u64, u64); 3] = [
    (
        600,
        Scenario::Solo,
        0x97c4_ca8b_5fe5_d5bf,
        0xecac_6673_29ca_a1e0,
    ),
    (
        600,
        Scenario::Field,
        0x8d30_b677_11bf_3dfc,
        0x999e_bb23_475b_ae46,
    ),
    (
        1_800,
        Scenario::Field,
        0xa50a_d4d9_556c_3992,
        0x30bf_cc62_6468_dbaa,
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
