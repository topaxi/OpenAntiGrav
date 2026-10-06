//! Prints the simulation determinism hashes for this machine: the twin of `oag-core`'s
//! `core_determinism_report` over the real force law. CI runs it on every supported platform, so
//! when `tests/determinism.rs` fails each platform's log already carries this output.
//!
//! It prints a **row every hundred ticks** as well as the totals, so a cross-platform failure is
//! localised: diffing two CI logs dates the divergence to a hundred-tick window.
//!
//! ```sh
//! cargo run -q -p oag-physics --example physics_determinism_report
//! ```

use oag_core::hash::StateHasher;
use oag_physics::probe::{self, Script};
use oag_physics::{Environment, step};

/// Our own fixed timestep. ADR-0007.
const TICK: f32 = 1.0 / 60.0;

/// How often a running hash is printed.
const EVERY: u32 = 100;

fn main() {
    println!("target : {}", std::env::consts::ARCH);
    println!("os     : {}", std::env::consts::OS);
    println!();

    for &(ticks, script) in &[
        (600u32, Script::Corridor),
        (3_600, Script::Corridor),
        (3_600, Script::Aerobatic),
    ] {
        let result = probe::run(script, ticks);
        println!(
            "ticks={ticks:<7} script={script:<12}  final={:#018x}  trajectory={:#018x}",
            result.final_hash,
            result.trajectory_hash,
            script = format!("{script:?}"),
        );
    }

    println!();
    println!("running trajectory hash, every {EVERY} tick(s):");
    for &(ticks, script) in &[(3_600u32, Script::Corridor), (3_600, Script::Aerobatic)] {
        println!("  {script:?}");
        let handling = probe::handling();
        let world = probe::corridor();
        let environment = Environment::default();
        let mut state = probe::start(&handling);
        let mut trajectory = StateHasher::new();

        for tick in 0..ticks {
            step(
                &mut state,
                &probe::controls(script, tick),
                &handling,
                &environment,
                &world,
                TICK,
            );
            probe::hash_state(&mut trajectory, &state);
            if (tick + 1) % EVERY == 0 {
                println!("    tick {:<6} {:#018x}", tick + 1, trajectory.finish());
            }
        }
    }
}
