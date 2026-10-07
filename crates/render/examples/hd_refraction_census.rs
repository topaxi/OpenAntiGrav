//! Census: every drawn material slot of a circuit whose name contains a needle,
//! with its resolved lit-race variant's declared samplers and parameters, the
//! texture each sampler record names, and the fragment program's output
//! texels. The question it answers: "which engine-bound inputs does this
//! material read".
//!
//! The needle is a material-name substring, or `@0x<hash>` to list every slot
//! whose resolved variant declares that sampler hash (`--summary` on the end
//! prints one line per slot instead of the full dump).
//!
//! ```sh
//! cargo run --release -p oag-render --example hd_refraction_census -- \
//!     data/images/hdfury-ps3-eu-dec.iso @0x88a0df95 \
//!     /data/environments/01_vineta_k/track.vex ...
//! ```

use oag_assets::Container;
use oag_rcs::rcsmaterial::{self, fragment::Program};
use oag_rcs::rcsmodel;

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let image = args.next().unwrap();
    let needle = args.next().unwrap_or_default();
    let hash = needle
        .strip_prefix("@0x")
        .map(|h| u32::from_str_radix(h, 16))
        .transpose()?;
    for name in args {
        let Some((spec, data)) = (0..4).find_map(|n| {
            let spec = format!("{image}:PS3_GAME/USRDIR/DATA0{n}.PSARC");
            oag_mesh::mesh::read_blob(&spec, &name)
                .ok()
                .map(|d| (spec, d))
        }) else {
            continue;
        };
        let Some(geometry) = oag_mesh::mesh::rcs::sibling_geometry(&spec, &name, &data) else {
            continue;
        };
        census(&spec, &name, &geometry, &needle, hash)?;
    }
    Ok(())
}

fn census(
    spec: &str,
    name: &str,
    geometry: &[u8],
    needle: &str,
    hash: Option<u32>,
) -> anyhow::Result<()> {
    let model = rcsmodel::Model::parse(geometry)?;
    let mut container = Container::open(spec)?;
    let mut decl_of = std::collections::HashMap::new();
    for mesh in model.meshes.iter().flat_map(rcsmodel::Mesh::surfaces) {
        decl_of.entry(mesh.material).or_insert(mesh.decl.as_ref());
    }
    for (slot, material) in model.materials.iter().enumerate() {
        if hash.is_none() && !material.name.contains(needle) {
            continue;
        }
        let Some(decl) = decl_of.get(&u32::try_from(slot)?) else {
            continue;
        };
        let Ok(blob) = container.read_entry(&format!("/{}", material.name)) else {
            continue;
        };
        let parsed = rcsmaterial::RcsMaterial::parse(&blob)?;
        let word = rcsmaterial::Features::chunk_word(rcsmaterial::LIT_RACE_PASS, *decl);
        let key = rcsmaterial::Features::from_pass_word(word);
        let Some(variant) = parsed.variant(rcsmaterial::Class::Static, key) else {
            continue;
        };
        let declared = rcsmaterial::Declared::parse(&blob, variant.fragment.offset);
        if let Some(h) = hash {
            if declared
                .as_ref()
                .is_some_and(|d| d.samplers.iter().any(|&(s, _)| s == h))
            {
                println!("{name} slot {slot} {}", material.name);
            }
            continue;
        }
        println!(
            "slot {slot} {} fragment@{:#x} feature {:#x}",
            material.name, variant.fragment.offset, variant.feature_hash
        );
        for (hash, path) in &material.samplers {
            println!(
                "  record sampler {hash:#010x} {} -> {path:?}",
                rcsmaterial::names::sampler_name(*hash).unwrap_or("?")
            );
        }
        for p in &material.parameters {
            println!(
                "  record parameter {:#010x} = [{}, {}, {}, {}]",
                p.hash, p.value[0], p.value[1], p.value[2], p.value[3]
            );
        }
        if let Some(d) = declared {
            for (h, unit) in &d.samplers {
                println!(
                    "  declares {h:#010x} {} unit {unit}",
                    rcsmaterial::names::sampler_name(*h).unwrap_or("?")
                );
            }
        }
        if let Some(p) = Program::parse(&blob, variant.fragment.offset) {
            println!("  output texels {:?}", p.output_texels());
        }
    }
    Ok(())
}
