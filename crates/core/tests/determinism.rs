//! The cross-platform determinism gate.
//!
//! CI runs this on Linux, Windows and macOS. Every one must produce the hashes
//! below. If they do not, the shared math foundation is not portable and the
//! replay system, golden tests and behavioural verification suite are all
//! unsound, whatever they appear to say.
//!
//! # When this test fails
//!
//! Do not update the constants to make it pass. That converts a real bug into a
//! silent one. Work out which platform diverged, using the
//! `core_determinism_report` example output already in each CI log, and fix the
//! cause. Usual suspects, in order of likelihood:
//!
//! 1. A `mul_add` crept into the simulation path.
//! 2. A build flag enabled fast-math or FMA contraction.
//! 3. `glam`'s `scalar-math` feature got dropped, re-enabling SIMD.
//! 4. A transcendental (`sin`, `cos`, `exp`) is resolving to a different libm.
//!
//! Point 4 is the one to watch: IEEE-754 requires correct rounding for `sqrt`
//! but not for transcendentals, so those are genuinely platform-dependent.
//! The probe takes its `sin` from `oag_core::math` (the `libm` crate, the same
//! code on every target), as all simulation code must; a platform `sin`
//! reaching the probe again would show up here as a cross-OS mismatch.
//!
//! Regenerate deliberately, only after establishing that a change of behaviour
//! is intended:
//!
//! ```sh
//! cargo run -p oag-core --example core_determinism_report
//! ```

use oag_core::probe;

/// `(ticks, seed, final_hash, trajectory_hash)`.
///
/// Recorded on x86_64 Linux, and verified identical between debug and release.
///
/// # History
///
/// - Regenerated 2026-07-26 because [`oag_core::rng`] was not the generator it
///   claimed to be: its state update read four stale words where
///   `xoshiro128**` reads two updated ones, which cost the map its bijectivity.
///   The old constants encoded that generator, so they had to move. This is the
///   deliberate regeneration the module docs above allow, not the reflex one
///   they warn against: the *cause* was found and fixed first, and the fix is
///   pinned by a published reference vector in `rng.rs` so the same class of
///   change cannot happen quietly again.
/// - Regenerated 2026-10-07 because the probe called the platform's own `sin`
///   (a deliberate libm canary) and the constants were recorded with glibc's,
///   so macOS and Windows could never match them - the first CI runs to
///   compare those two OSes in about four weeks failed exactly there. The
///   cause was established first (swapping `sin`/`from_axis_angle` for
///   `oag_core::math` on Linux reproduced Windows' 1000-tick trajectory bit for
///   bit, and `sin` alone macOS' 100000-tick one), then the maintainer chose
///   the fix this module always named: the probe takes `oag_core::math`. Only
///   the trajectory hashes move; the final-state hashes are unchanged.
const REFERENCE: &[(u32, u64, u64, u64)] = &[
    (1_000, 1, 0xc45d_a2ce_49d2_b04b, 0xbcd0_56e3_34f2_b1cb),
    (10_000, 42, 0x422a_d61d_f161_2c01, 0xa774_f231_b9e3_43d0),
    (
        100_000,
        0xDEAD_BEEF,
        0xeabc_0eab_577c_b538,
        0x3133_7c04_f203_2805,
    ),
];

#[test]
fn determinism_matches_the_committed_reference() {
    let mut failures = Vec::new();

    for &(ticks, seed, expected_final, expected_trajectory) in REFERENCE {
        let got = probe::run(ticks, seed);

        if got.final_hash != expected_final || got.trajectory_hash != expected_trajectory {
            failures.push(format!(
                "ticks={ticks} seed={seed:#x}\n  \
                 final:      expected {expected_final:#018x}, got {:#018x}\n  \
                 trajectory: expected {expected_trajectory:#018x}, got {:#018x}",
                got.final_hash, got.trajectory_hash
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "the simulation is not reproducible on this platform ({} / {}):\n\n{}\n\n\
         Do not update the constants to silence this. See the module docs.",
        std::env::consts::OS,
        std::env::consts::ARCH,
        failures.join("\n\n")
    );
}

/// Guards the property the reference constants depend on: that the probe is a
/// pure function of its inputs. If this fails, the reference test above is
/// meaningless even when it passes.
#[test]
fn determinism_is_stable_across_repeated_runs() {
    for &(ticks, seed, _, _) in REFERENCE {
        let first = probe::run(ticks, seed);
        for attempt in 0..4 {
            assert_eq!(
                probe::run(ticks, seed),
                first,
                "run {attempt} diverged for ticks={ticks} seed={seed:#x}"
            );
        }
    }
}
