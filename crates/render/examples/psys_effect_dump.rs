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
    Ok(())
}
