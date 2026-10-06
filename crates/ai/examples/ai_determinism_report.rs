//! Prints the driver determinism hashes for this machine.
//!
//! The third after `oag-core`'s `core_determinism_report` and `oag-physics`'s
//! `physics_determinism_report`: CI runs it on every platform, so when
//! `tests/determinism.rs` fails each log already carries the numbers.
//!
//! A row every hundred ticks as well as the totals dates a cross-platform
//! divergence to a hundred-tick window. The curvature spread is printed too: a
//! hash agreeing over a scenario with no corners agrees about nothing.
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
