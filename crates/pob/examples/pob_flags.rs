//! Prints each emitter of the `.pob` files named on the command line: name,
//! flags (hex), duration and whether `LOOPING` is set.
//!
//! `cargo run -p oag-pob --example pob_flags -- effect.pob...`

fn main() {
    for path in std::env::args().skip(1) {
        let blob = std::fs::read(&path).expect("the file reads");
        let system = oag_pob::ParticleSystem::parse(&blob).expect("it parses");
        println!("{} ({path})", system.name);
        for emitter in system.emitters(&blob).expect("the emitters walk") {
            println!(
                "  {:<24} flags {:#010x} looping {} duration {}",
                emitter.name,
                emitter.flags,
                emitter.flags & oag_pob::flags::LOOPING != 0,
                emitter.duration_ticks
            );
        }
    }
}
