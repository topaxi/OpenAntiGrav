//! Prints the driver determinism hashes for this machine.
//!
//! The third of its kind, after `oag-core`'s `core_determinism_report` and
//! `oag-physics`'s `physics_determinism_report`, and here for the same reason:
//! CI runs it on every supported platform, so when `tests/determinism.rs`
//! fails, each platform's log already carries the numbers and the divergent
//! target is visible without re-running anything.
//!
//! It prints a row every hundred ticks as well as the totals, which is what
//! makes a cross-platform failure localisable: diffing two CI logs dates the
//! divergence to a hundred-tick window instead of saying only that the run
//! ended differently. The curvature spread is printed too, because a hash that
//! agrees over a scenario with no corners in it agrees about nothing.
//!
//! ```sh
//! cargo run -q -p oag-ai --example ai_determinism_report
//! ```

use oag_ai::probe::{self, Scenario};

/// How often a running hash is printed.
const EVERY: u32 = 100;

fn main() {
    println!("target : {}", std::env::consts::ARCH);
    println!("os     : {}", std::env::consts::OS);
    println!();

    for &(ticks, scenario) in &[
        (600u32, Scenario::Solo),
        (600, Scenario::Field),
        (1_800, Scenario::Field),
    ] {
        for at in (EVERY..ticks).step_by(EVERY as usize) {
            let partial = probe::run(scenario, at);
            println!(
                "  ticks={at:<5} scenario={scenario:<6} final={:#018x}  trajectory={:#018x}",
                partial.final_hash, partial.trajectory_hash
            );
        }
        let result = probe::run(scenario, ticks);
        println!(
            "ticks={ticks:<7} scenario={scenario:<6} final={:#018x}  trajectory={:#018x}  curvature={:.6}..{:.6}  travelled={:.0}",
            result.final_hash,
            result.trajectory_hash,
            result.curvature.0,
            result.curvature.1,
            result.travelled
        );
        println!();
    }
}
