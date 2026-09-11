//! Prints the determinism probe hashes for this machine.
//!
//! CI runs this on every supported platform. When the determinism test fails,
//! the CI log for each platform already contains this output, so the divergent
//! target is visible without re-running anything.
//!
//! ```sh
//! cargo run -p oag-core --example core_determinism_report
//! ```

use oag_core::probe;

fn main() {
    println!("target : {}", std::env::consts::ARCH);
    println!("os     : {}", std::env::consts::OS);
    println!();

    for &(ticks, seed) in &[(1_000u32, 1u64), (10_000, 42), (100_000, 0xDEAD_BEEF)] {
        let r = probe::run(ticks, seed);
        println!(
            "ticks={ticks:<7} seed={seed:#018x}  final={:#018x}  trajectory={:#018x}",
            r.final_hash, r.trajectory_hash
        );
    }
}
