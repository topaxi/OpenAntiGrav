//! Scratch probe: build one front-end flyer through the game's own
//! `oag_assets::Archives` and print what each of its materials resolves to.
//!
//! ```sh
//! cargo run -p oag-game --example hd_flyer_probe -- data/images/hdfury-ps3-eu-dec.iso \
//!     /data/fe/flyers/01_uplift/flyer.vex
//! ```

use oag_render::mesh;

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let image = args.next().expect("an image");
    let path = args.next().expect("a .vex path");
    let mut archives = oag_assets::Archives::open(&image, oag_hd::TITLE)?;
    let data = archives.read_name(&path)?;
    let sibling = mesh::rcs::sibling_name(&path).expect("a .vex name");
    let geometry = archives.read_name(&sibling)?;
    let source = oag_rcs::rcsmodel::Model::parse(&geometry)?;
    for material in &source.materials {
        let blob = archives.read_name(&material.name).ok();
        println!(
            "material {} -> {} bytes",
            material.name,
            blob.as_ref().map_or(0, Vec::len)
        );
    }
    let (model, report) = mesh::rcs::build_scene(&path, &data, &geometry, &mut |name| {
        archives.read_name(name).ok()
    })?;
    println!("{}", report.describe());
    for (slot, variant) in model.material_variants.iter().enumerate() {
        let Some(variant) = variant else {
            println!("slot {slot}: no variant");
            continue;
        };
        let name = &source.materials[slot].name;
        let blob = archives.read_name(name)?;
        let program =
            oag_rcs::rcsmaterial::fragment::Program::parse(&blob, variant.fragment.offset);
        let declared = oag_rcs::rcsmaterial::Declared::parse(&blob, variant.fragment.offset);
        let names: Vec<&str> = program
            .as_ref()
            .map(|p| {
                p.instructions
                    .iter()
                    .map(|i| i.name().unwrap_or("?"))
                    .collect()
            })
            .unwrap_or_default();
        println!(
            "slot {slot}: class {:?} feature {:08x} frag@{:#x} len {} hash {:08x}\n    mnemonics {names:?}\n    params {:08x?}\n    samplers {:08x?}",
            variant.class,
            variant.feature_hash,
            variant.fragment.offset,
            variant.fragment.len,
            variant.fragment.program_hash,
            declared.as_ref().map(|d| d.parameters.clone()),
            declared.as_ref().map(|d| d.samplers.clone()),
        );
    }
    let roles: Vec<u32> = model
        .transparent_draws
        .iter()
        .chain(&model.draws)
        .map(|d| model.vertices[model.indices[d.range.start as usize] as usize].slots)
        .collect();
    println!("roles {roles:x?}");
    for d in model.draws.iter().chain(&model.transparent_draws) {
        let mut lo = [f32::MAX; 3];
        let mut hi = [f32::MIN; 3];
        for &i in &model.indices[d.range.start as usize..d.range.end as usize] {
            let p = model.vertices[i as usize].position;
            for k in 0..3 {
                lo[k] = lo[k].min(p[k]);
                hi[k] = hi[k].max(p[k]);
            }
        }
        println!(
            "draw node {:?} x {:.1}..{:.1} y {:.1}..{:.1} z {:.1}..{:.1}",
            d.node, lo[0], hi[0], lo[1], hi[1], lo[2], hi[2]
        );
    }
    println!("anim nodes {}", model.anim_nodes.len());
    for (i, node) in model.anim_nodes.iter().enumerate().take(4) {
        println!(
            "  anim {i} at 5.9s: {:?}",
            node.transform
                .sample(5.9)
                .map(|v| (v * 100.0).round() / 100.0)
        );
        if let mesh::Motion::Vex(t) = &node.transform {
            println!(
                "    tr {:?} base {:?} quantum {:?} sc {:?}",
                t.translation.values, t.translation_base, t.translation_quantum, t.scale.values
            );
        }
    }
    for (i, node) in model.anim_nodes.iter().enumerate().take(12) {
        if let mesh::Motion::Vex(t) = &node.transform {
            println!(
                "  anim {i}: loop {} unit {} tr {:?} rot {:?} sc {:?}",
                t.loop_seconds,
                t.seconds_per_key,
                t.translation.times,
                t.rotation.times,
                t.scale.times
            );
        }
    }
    for (index, texture) in model.textures.iter().enumerate() {
        let Some(texture) = texture else {
            println!("texture {index}: none");
            continue;
        };
        let first = match &texture.texels {
            mesh::Texels::Rgba8(bytes) => format!("rgba8 {:?}", &bytes[..8.min(bytes.len())]),
            other => format!("{:?}", std::mem::discriminant(other)),
        };
        println!(
            "texture {index}: {} {}x{} {first}",
            texture.label, texture.width, texture.height
        );
    }
    Ok(())
}
