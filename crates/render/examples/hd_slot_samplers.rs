//! Scratch probe: for chosen material slots of one circuit, every texture
//! entry with its sampler hash, what the resolved variant declares (hash and
//! unit), the packed role word and the declared-picture pick.
//!
//! ```sh
//! cargo run --release -p oag-render --example hd_slot_samplers -- <image> /data/environments/10_sebenco_climb/track.vex 102,100
//! ```

use oag_assets::Container;
use oag_mesh::mesh;
use oag_rcs::{rcsmaterial, rcsmodel};

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let image = args.next().unwrap();
    let name = args.next().unwrap();
    let slots: Vec<usize> = args
        .next()
        .unwrap()
        .split(',')
        .filter_map(|s| s.parse().ok())
        .collect();
    let (spec, data) = (0..4)
        .find_map(|n| {
            let spec = format!("{image}:PS3_GAME/USRDIR/DATA0{n}.PSARC");
            mesh::read_blob(&spec, &name).ok().map(|d| (spec, d))
        })
        .ok_or_else(|| anyhow::anyhow!("{name} not found"))?;
    let geometry = mesh::rcs::sibling_geometry(&spec, &name, &data)
        .ok_or_else(|| anyhow::anyhow!("no sibling model"))?;
    let model = rcsmodel::Model::parse(&geometry)?;
    let (scene, _) = mesh::rcs::scene_from(&spec, &name, &data)?
        .ok_or_else(|| anyhow::anyhow!("not a PS3 model"))?;
    let mut container = Container::open(&spec)?;
    let mut decl_of = std::collections::HashMap::new();
    for mesh in model.meshes.iter().flat_map(rcsmodel::Mesh::surfaces) {
        decl_of.entry(mesh.material).or_insert(mesh.decl.as_ref());
    }
    for slot in slots {
        let material = &model.materials[slot];
        println!("slot {slot}: {}", material.name);
        println!(
            "  texture {} second {:?} packed {:#x} specexp {:?}",
            material.texture,
            material.second_texture,
            scene.material_slots.get(slot).copied().unwrap_or(0),
            scene.material_specular_exponent.get(slot)
        );
        for (i, (hash, path)) in material.samplers.iter().enumerate() {
            println!("  entry {i}: {hash:#010x} {path:?}");
        }
        let declared = decl_of
            .get(&u32::try_from(slot)?)
            .and_then(|d| d.as_ref())
            .and_then(|decl| {
                let blob = container.read_entry(&format!("/{}", material.name)).ok()?;
                let parsed = rcsmaterial::RcsMaterial::parse(&blob).ok()?;
                let word =
                    rcsmaterial::Features::chunk_word(rcsmaterial::LIT_RACE_PASS, Some(*decl));
                let key = rcsmaterial::Features::from_pass_word(word);
                let variant = parsed.variant(rcsmaterial::Class::Static, key)?;
                rcsmaterial::Declared::parse(&blob, variant.fragment.offset)
            });
        match declared {
            Some(d) => println!("  declared samplers (hash, unit): {:x?}", d.samplers),
            None => println!("  no resolved variant"),
        }
    }
    Ok(())
}
