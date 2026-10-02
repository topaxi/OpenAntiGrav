//! Scratch probe: how `oag_render::psys` reads one `Data\Psys` effect.
//!
//! ```sh
//! cargo run -q -p oag-render --example psys_effect_dump -- data/images/pulse-psp-usa.chd WO_BLUE_WELDER
//! ```

use oag_render::psys;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let image = args.next().ok_or("usage: IMAGE NAME")?;
    let name = args.next().ok_or("usage: IMAGE NAME")?;
    let mut archives = oag_pulse::open(&image)?;
    if name == "--jumps" {
        return jumps(&mut archives, args.collect());
    }
    let blob = archives.read_name(&psys::effect_path(&name))?;
    let effect = psys::Effect::parse(&blob, psys::ColourScale::Full)?;
    for (i, e) in effect.emitters.iter().enumerate() {
        println!(
            "#{i} {:?} looping {} dur {} interval {:?} per {:?} life {:?} speed {:?} dir {:?} grav {} render {:?} streak {:?} blend {:?} sprite {} template {} world {} child {:?}/{:?} size {:?} alpha {:?} pal0 {:?} pal128 {:?}",
            e.name,
            e.looping,
            e.duration_ticks,
            e.interval_ticks,
            e.per_emission,
            e.lifetime_ticks,
            e.speed_per_tick,
            e.direction,
            e.gravity_per_tick2,
            e.render,
            e.streak,
            e.blend,
            e.sprite.is_some(),
            e.template,
            e.world_space,
            e.particle_child,
            e.death_child,
            e.size,
            e.alpha,
            e.palette[0],
            e.palette[128]
        );
    }
    let system = oag_vex::pob::ParticleSystem::parse(&blob)?;
    for e in system.emitters(&blob)? {
        for t in &e.initial_particles {
            println!(
                "template {:?} on {:?}: life {:?} size {:?} alpha {:?}",
                t.name, e.name, t.lifetime_ticks, t.size, t.alpha
            );
        }
    }
    Ok(())
}

/// Every periodic keyframed channel, on an emitter or a template, holding two
/// keys at one time with different values: the channels `psys::template::unroll`
/// keeps a jump in since 2026-10-02.
fn jumps(
    archives: &mut oag_assets::Archives,
    names: Vec<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    use oag_vex::pob::{Channel, ChannelMode};
    let jumpy = |c: &Channel| {
        c.period > 0.0
            && c.mode == ChannelMode::Keyframed
            && c.keys
                .windows(2)
                .any(|w| w[0].0 == w[1].0 && w[0].1 != w[1].1)
    };
    for name in names {
        let blob = archives.read_name(&psys::effect_path(&name))?;
        let system = oag_vex::pob::ParticleSystem::parse(&blob)?;
        for e in system.emitters(&blob)? {
            let mut records = vec![(false, &e)];
            records.extend(e.initial_particles.iter().map(|t| (true, t)));
            for (template, r) in records {
                for (label, c) in [
                    ("size", &r.size),
                    ("alpha", &r.alpha),
                    ("rotation", &r.rotation_speed),
                ] {
                    if jumpy(c) {
                        println!(
                            "{name} {} {:?} {label} period {}",
                            if template { "template" } else { "emitter" },
                            r.name,
                            c.period
                        );
                    }
                }
            }
        }
    }
    Ok(())
}
