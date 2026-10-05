//! Scratch probe: every PSP `.pob` emitter (and template) with shape 8 or draw
//! class 6, plus its flags, aspect, extents and size.
//!
//! ```sh
//! cargo run -q -p oag-render --example pob_shape8_census -- data/images/pulse-psp-usa.chd [NAME]
//! ```

use oag_assets::Archive;
use oag_pob as pob;

fn line(system: &str, kind: &str, e: &pob::Emitter) {
    println!(
        "{system} {kind} {:?} shape {} class {:?} flags {:#010x} aspect {} extent {} unread {:?} rmode {} vmode {} speed {:?} size {:?}/{:?} {:?} life {:?} per {:?} blend {} grid {:?} col0 {:?}",
        e.name,
        e.shape,
        e.draw_class(),
        e.flags,
        e.aspect,
        e.extent,
        e.extent_unread,
        e.radius_mode,
        e.velocity_mode,
        e.speed_per_tick,
        e.size.lo,
        e.size.hi,
        e.size.keys,
        e.lifetime_ticks,
        e.per_emission,
        e.blend_class,
        e.atlas_grid,
        e.colours[0],
    );
}

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
            if only.is_some() || e.shape == 8 || e.draw_class() == Some(6) {
                line(&system.name, "emitter", &e);
            }
            for t in &e.initial_particles {
                if only.is_some() || t.shape == 8 || t.draw_class() == Some(6) {
                    line(&system.name, "template", t);
                }
            }
        }
    }
    Ok(())
}
