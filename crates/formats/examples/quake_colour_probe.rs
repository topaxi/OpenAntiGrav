//! Scratch probe: dump `WO_QUAKE.POB`'s emitter tree and 256-entry colour
//! tables, for the "is the Quake's wave the right colour" investigation.
//!
//! ```sh
//! cargo run -q -p oag-formats --example quake_colour_probe -- /tmp/wo_quake.pob
//! ```

use oag_formats::pob::ParticleSystem;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    for path in std::env::args().skip(1) {
        let blob = std::fs::read(&path)?;
        let system = ParticleSystem::parse(&blob)?;
        println!("== {path} :: {} ==", system.name);
        let emitters = system.emitters(&blob)?;
        for (i, e) in emitters.iter().enumerate() {
            println!(
                "-- emitter[{i}] {:?} flags={:#010x} shape={} render_mode={:#x} draw_class={:?} blend_class={} colour_mode={}",
                e.name,
                e.flags,
                e.shape,
                e.render_mode,
                e.draw_class(),
                e.blend_class,
                e.colour_mode
            );
            println!(
                "   lifetime_ticks={:?} interval_ticks={:?} per_emission={:?} speed_per_tick={:?}",
                e.lifetime_ticks, e.interval_ticks, e.per_emission, e.speed_per_tick
            );
            println!(
                "   size: mode={:?} lo={} hi={} keys={:?}",
                e.size.mode, e.size.lo, e.size.hi, e.size.keys
            );
            // Print the distinct colour table entries, in order of first
            // appearance, to see the ramp without 256 near-duplicate lines.
            let mut seen = Vec::new();
            for c in e.colours.iter() {
                if seen.last() != Some(c) {
                    seen.push(*c);
                }
            }
            println!("   colour ramp ({} distinct runs):", seen.len());
            for c in &seen {
                println!("     {c:?}");
            }
            println!(
                "   death_child={:?} particle_child={:?}",
                e.death_child, e.particle_child
            );
        }
    }
    Ok(())
}
