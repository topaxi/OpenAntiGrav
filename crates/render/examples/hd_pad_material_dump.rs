//! Scratch probe behind [`pads.md`](../../../docs/rendering/pads.md)'s "What
//! binds the `_ne` file": dump a pad chunk's own material record out of `track.rcsmodel`,
//! in full - every sampler entry, not just the two this project currently
//! reads - and say whether it is an inline record or names a separate
//! `.rcsmaterial` file the chunk's own samplers do not carry.

use oag_formats::{rcsmaterial, rcsmodel};

use oag_render::mesh;
use oag_vex::vex;

/// The sampler-name hash both pad `_ne` files bind to, measured off
/// `12_sol_2`'s own material records (see the dump this prints).
const NE_SAMPLER_HASH: u32 = 0xa2d5_55b9;

fn main() -> anyhow::Result<()> {
    let spec = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "data/images/hdfury-ps3-eu-dec.iso:PS3_GAME/USRDIR/DATA02.PSARC".into());
    let name = std::env::args()
        .nth(2)
        .unwrap_or_else(|| "/data/environments/12_sol_2/track.vex".into());
    let data = mesh::read_blob(&spec, &name)?;
    let model_blob = mesh::read_blob(&spec, &mesh::rcs::sibling_name(&name).unwrap())?;
    let model = rcsmodel::Model::parse(&model_blob).map_err(|e| anyhow::anyhow!("{e}"))?;
    let classes = vex::classes_of(&data)?;
    let nodes = vex::nodes(&data)?;
    let order = vex::byte_order(&data);
    let read = |path: &str| mesh::read_blob(&spec, path).ok();

    for (label, class_id) in [
        ("Speedup Pad", classes.speedup_pad),
        ("Weapon Pad", classes.weapon_pad),
    ] {
        let Some(class_id) = class_id else {
            println!("{label}: no class id for this .vex version");
            continue;
        };
        println!("== {label} (class {class_id:#x}) ==");
        for node in nodes.iter().filter(|n| n.class_id == class_id) {
            let payload = &data[node.payload()];
            if payload.len() < 0x34 {
                continue;
            }
            let hash = order.u32(payload, 0x30);
            let Some(chunk) = model.mesh(hash) else {
                println!(
                    "  {:?}: chunk {hash:#010x} not in the model",
                    node.name.as_deref().unwrap_or("?")
                );
                continue;
            };
            for (surface_index, surface) in chunk.surfaces().enumerate() {
                let slot = surface.material as usize;
                let material = model.materials.get(slot);
                print!(
                    "  {:<28} surface {surface_index} slot {slot:>4}  ",
                    node.name.as_deref().unwrap_or("?"),
                );
                let Some(material) = material else {
                    println!(
                        "(slot out of range, {} materials total)",
                        model.materials.len()
                    );
                    continue;
                };
                println!("name={:?}", material.name);
                println!(
                    "      texture={:?} second_texture={:?}",
                    material.texture, material.second_texture
                );
                println!(
                    "      texture_sampler={:#010x?} second_texture_sampler={:#010x?}",
                    material.texture_sampler, material.second_texture_sampler
                );
                println!("      lightmap_entry={:?}", material.lightmap_entry());
                println!("      samplers ({} total):", material.samplers.len());
                for (i, (hash, path)) in material.samplers.iter().enumerate() {
                    println!("        [{i}] hash={hash:#010x} path={path:?}");
                }

                // Resolve the shader variant the way `skin::variants` does,
                // for the ordinary lit-race pass, and ask whether its
                // fragment program accumulates the unit the `_ne` sampler's
                // name hash declares - the check that decides whether
                // wiring it would change the picture at all.
                let Some(blob) = read(&format!("/{}", material.name)) else {
                    println!("      (material file did not load)");
                    continue;
                };
                let word = rcsmaterial::Features::chunk_word(
                    rcsmaterial::LIT_RACE_PASS,
                    surface.decl.as_ref(),
                );
                let key = rcsmaterial::Features::from_pass_word(word);
                let Some(variant) = rcsmaterial::RcsMaterial::parse(&blob)
                    .ok()
                    .and_then(|m| m.variant(rcsmaterial::Class::Static, key).copied())
                else {
                    println!("      (no Static variant for this chunk's own feature word)");
                    continue;
                };
                let declared = rcsmaterial::Declared::parse(&blob, variant.fragment.offset);
                let program = rcsmaterial::fragment::Program::parse(&blob, variant.fragment.offset);
                let ne_unit = declared
                    .as_ref()
                    .and_then(|d| d.samplers.iter().find(|(h, _)| *h == NE_SAMPLER_HASH))
                    .map(|&(_, unit)| unit);
                println!(
                    "      variant resolved: declared={} program={} _ne unit={ne_unit:?} accumulates(_ne unit)={:?}",
                    declared.is_some(),
                    program.is_some(),
                    ne_unit
                        .zip(program.as_ref())
                        .map(|(u, p)| p.accumulates(u as u8))
                );
                println!("      parameters ({} total):", material.parameters.len());
                for p in &material.parameters {
                    println!("        hash={:#010x} value={:?}", p.hash, p.value);
                }
            }
        }
    }

    println!("== is materials/speedup_material.rcsmaterial named by any material record? ==");
    let named = model
        .materials
        .iter()
        .filter(|m| m.name.ends_with("speedup_material.rcsmaterial"))
        .count();
    println!(
        "  {named} material record(s) name it, out of {}",
        model.materials.len()
    );

    for suffix in [
        "materials/weapon_pads.rcsmaterial",
        "materials/diffuse_normal_specular_emmissive.rcsmaterial",
    ] {
        let count = model
            .materials
            .iter()
            .filter(|m| m.name.ends_with(suffix))
            .count();
        println!("  {suffix}: named by {count} material record(s)");
    }

    Ok(())
}
