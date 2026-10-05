//! Scratch probe: how a Pulse model's vertices split across glow-mask stamps,
//! per draw list - see `docs/rendering/glow-mask.md`.
//!
//! ```sh
//! cargo run -q -p oag-render --example glow_mask_probe -- data/images/pulse-psp-usa.chd 'Data\Environments\16_Track\track.vex'
//! ```

use std::collections::BTreeMap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let image = args.next().ok_or("usage: glow_mask_probe IMAGE ENTRY")?;
    let entry = args.next().ok_or("usage: glow_mask_probe IMAGE ENTRY")?;
    let mut archives = oag_pulse::open(&image)?;
    let blob = archives.read_name(&entry)?;
    let model = oag_mesh::mesh::build(&entry, &blob)?;
    for (name, draws) in [
        ("opaque", &model.draws),
        ("cutout", &model.alpha_tested_draws),
        ("transparent", &model.transparent_draws),
    ] {
        let mut histogram = BTreeMap::<u8, usize>::new();
        for draw in draws.iter() {
            for &index in &model.indices[draw.range.start as usize..draw.range.end as usize] {
                let glow = model.vertices[index as usize].glow;
                *histogram.entry((glow * 255.0).round() as u8).or_default() += 1;
            }
        }
        println!(
            "{name}: {} draws, stamp histogram (index count) {histogram:?}",
            draws.len()
        );
    }
    Ok(())
}
