//! Scratch probe: the role word a material slot resolved to, decoded bit by
//! bit, with its glow-table entry - what `mesh.wesl` will actually do with
//! the slot, as opposed to what the slot binds (`hd_slot_inspect`).
//!
//! ```sh
//! cargo run -p oag-render --example hd_slot_roles -- <image>:<archive> /data/environments/15_anulpha_pass/track.vex 476,485
//! ```

use oag_mesh::mesh::{self, slots};

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let spec = args.next().expect("<image>:<archive>");
    let name = args.next().expect("track .vex path");
    let wanted: Vec<u32> = args
        .next()
        .expect("slots")
        .split(',')
        .filter_map(|s| s.trim().parse().ok())
        .collect();
    let data = mesh::read_blob(&spec, &name)?;
    let (model, report) = mesh::rcs::scene_from(&spec, &name, &data)?
        .ok_or_else(|| anyhow::anyhow!("{name}: not a PS3 model"))?;
    let names: &[(u32, &str)] = &[
        (slots::SECOND_IS_LIGHTMAP, "SECOND_IS_LIGHTMAP"),
        (slots::ALBEDO_FROM_SECOND, "ALBEDO_FROM_SECOND"),
        (slots::ALPHA_FROM_SECOND, "ALPHA_FROM_SECOND"),
        (slots::FLIP_V, "FLIP_V"),
        (slots::NO_AMBIENT, "NO_AMBIENT"),
        (slots::NO_SUN, "NO_SUN"),
        (slots::ADD_SECOND, "ADD_SECOND"),
        (slots::ZONE_TRACK, "ZONE_TRACK"),
        (slots::FACING_RAMP_SHEEN, "FACING_RAMP_SHEEN"),
        (slots::RIM_GLOW, "RIM_GLOW"),
        (slots::RIM_EDGE, "RIM_EDGE"),
        (slots::GLOW_BATCH, "GLOW_BATCH"),
        (slots::PAD_NE, "PAD_NE"),
        (slots::CLOCK_SCROLL_RING, "CLOCK_SCROLL_RING"),
        (slots::CLOCK_SCROLL_HALO, "CLOCK_SCROLL_HALO"),
        (slots::MAG_WAVE, "MAG_WAVE"),
        (slots::MAG_LOOP, "MAG_LOOP"),
        (slots::LIGHT_CONE, "LIGHT_CONE"),
        (slots::REFRACTION, "REFRACTION"),
        (slots::BOMB_HALO, "BOMB_HALO"),
        (slots::ICE, "ICE"),
        (slots::HOLO_BILLBOARD, "HOLO_BILLBOARD"),
        (slots::LIT_ALPHA, "LIT_ALPHA"),
    ];
    for slot in wanted {
        let Some(&word) = model.material_slots.get(slot as usize) else {
            println!("slot {slot}: none");
            continue;
        };
        let bits: Vec<&str> = names
            .iter()
            .filter(|(bit, _)| word & bit != 0)
            .map(|(_, n)| *n)
            .collect();
        let index = slots::material_index(word);
        let glow = index
            .checked_sub(1)
            .and_then(|i| model.emissive.get(i as usize))
            .map(|e| format!("{e:?}"));
        println!(
            "slot {slot} word {word:#010x} alpha ch {} index {index} spec {:?} colour_factor {:?}: {bits:?} glow {glow:?}",
            (word >> 3) & 3,
            model.material_specular_exponent.get(slot as usize),
            model.material_colour_factor.get(slot as usize),
        );
        let label = |slots: &mesh::TextureSlots| {
            slots
                .get(slot as usize)
                .and_then(Option::as_ref)
                .map_or("-", |t| t.label.as_str())
                .to_string()
        };
        println!(
            "  albedo {} | lightmap {} | third {} | wave {}",
            label(&model.textures),
            label(&model.lightmaps),
            label(&model.pad_masks),
            label(&model.wave_maps)
        );
    }
    println!("{}", report.describe());
    Ok(())
}
