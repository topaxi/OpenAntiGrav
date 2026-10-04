//! Scratch probe: every PSP `.pob` emitter's flag `0x200000`, shape, and animated attributes.
//!
//! ```sh
//! cargo run -q -p oag-render --example pob_ring_census -- data/images/pulse-psp-usa.chd [NAME]
//! ```

use oag_assets::Archive;
use oag_vex::pob;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let image = args.next().ok_or("usage: IMAGE [NAME]")?;
    let only = args.next();
    let mut archive = Archive::open(&format!("{image}:PSP_GAME/USRDIR/Data.wad"))?;
    for index in 0..archive.directory().entries.len() {
        let Ok(head) = archive.peek(index, 4) else {
            continue;
        };
        if !pob::looks_like_particle_system(&head) {
            continue;
        }
        let blob = archive.read(index)?;
        let system = pob::ParticleSystem::parse(&blob)?;
        if only.as_ref().is_some_and(|n| *n != system.name) {
            continue;
        }
        for e in system.emitters(&blob)? {
            let interesting = e.flags & 2 != 0
                || e.playback_rate != 1.0
                || e.flags & 0x0020_0000 != 0
                || e.flags & 0x20 != 0
                || !e.attribute_animations.is_empty();
            if only.is_some() {
                println!("{}: {e:#?}", system.name);
            } else if interesting {
                println!(
                    "{} {:?} rate {} flags {:#x} shape {} rmode {} vmode {} extent {} per {:?} interval {:?} life {:?} attrs {:?}",
                    system.name,
                    e.name,
                    e.playback_rate,
                    e.flags,
                    e.shape,
                    e.radius_mode,
                    e.velocity_mode,
                    e.extent,
                    e.per_emission,
                    e.interval_ticks,
                    e.lifetime_ticks,
                    e.attribute_animations
                        .iter()
                        .map(|a| (
                            a.selector,
                            a.channel.lo,
                            a.channel.hi,
                            a.channel.mode,
                            a.channel.keys.clone()
                        ))
                        .collect::<Vec<_>>()
                );
            }
        }
    }
    Ok(())
}
