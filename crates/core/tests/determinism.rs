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
//! `determinism_report` example output already in each CI log, and fix the
//! cause. Usual suspects, in order of likelihood:
//!
//! 1. A `mul_add` crept into the simulation path.
//! 2. A build flag enabled fast-math or FMA contraction.
//! 3. `glam`'s `scalar-math` feature got dropped, re-enabling SIMD.
//! 4. A transcendental (`sin`, `cos`, `exp`) is resolving to a different libm.
//!
//! Point 4 is the one to watch: IEEE-754 requires correct rounding for `sqrt`
//! but not for transcendentals, so those are genuinely platform-dependent.
//! The probe uses `sin` deliberately, to catch this rather than hide it. If it
//! turns out that platform libm differences are unavoidable, the fix is to
//! bring our own implementations into `oag-core`, not to weaken the test.
//!
//! Regenerate deliberately, only after establishing that a change of behaviour
//! is intended:
//!
//! ```sh
//! cargo run -p oag-core --example determinism_report
//! ```

use oag_core::probe;

/// `(ticks, seed, final_hash, trajectory_hash)`.
///
/// Recorded on x86_64 Linux, and verified identical between debug and release.
const REFERENCE: &[(u32, u64, u64, u64)] = &[
    (1_000, 1, 0x8fae_bc50_d5b8_5a51, 0xbb79_231b_9491_a914),
    (10_000, 42, 0x12f4_44d7_2e72_7d29, 0x75d8_415d_d6e5_f4c8),
    (
        100_000,
        0xDEAD_BEEF,
        0xf74b_01b2_3bf5_6ea5,
        0x7ee2_d19b_a2b6_8815,
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
