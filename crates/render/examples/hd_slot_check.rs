//! Scratch probe: what `mesh::rcs::skin::roles` decided for each material of a
//! built HD model - which unit the albedo and the alpha come from, and which
//! channel - beside the material's own two texture paths.
//!
//! Built through `mesh::rcs::scene_from`, the real production path.

use oag_render::mesh;
use oag_render::mesh::slots;

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let spec = args
        .next()
        .unwrap_or_else(|| "data/images/hdfury-ps3-eu-dec.iso:PS3_GAME/USRDIR/DATA00.PSARC".into());
    let name = args
        .next()
        .unwrap_or_else(|| "/data/environments/talons_junction/track.vex".into());
    let want = args.next().unwrap_or_default();

    let data = mesh::read_blob(&spec, &name)?;
    let geometry = mesh::rcs::sibling_geometry(&spec, &name, &data)
        .ok_or_else(|| anyhow::anyhow!("{name}: no sibling .rcsmodel"))?;
    let raw = oag_rcs::rcsmodel::Model::parse(&geometry)?;
    let (model, _) = mesh::rcs::scene_from(&spec, &name, &data)?
        .ok_or_else(|| anyhow::anyhow!("{name}: not a PS3 model"))?;

    let mut chunks: std::collections::BTreeMap<u32, usize> = Default::default();
    for mesh in &raw.meshes {
        *chunks.entry(mesh.material).or_default() += 1;
    }
    for (slot, material) in raw.materials.iter().enumerate() {
        if !material.name.contains(&want) {
            continue;
        }
        let packed = model.material_slots.get(slot).copied().unwrap_or(0);
        let mut roles = Vec::new();
        if packed & slots::SECOND_IS_LIGHTMAP != 0 {
            roles.push("SECOND_IS_LIGHTMAP".to_string());
        }
        if packed & slots::ALBEDO_FROM_SECOND != 0 {
            roles.push("ALBEDO_FROM_SECOND".to_string());
        }
        if packed & slots::ALPHA_FROM_SECOND != 0 {
            roles.push("ALPHA_FROM_SECOND".to_string());
        }
        roles.push(format!("alpha_channel={}", (packed >> 3) & 3));
        let variant = model
            .material_variants
            .get(slot)
            .and_then(|v| v.as_ref())
            .map(|v| {
                format!(
                    "fragment@{:#x} vertex@{:#x}",
                    v.fragment.offset, v.vertex.offset
                )
            })
            .unwrap_or_else(|| "no resolved variant".into());
        if std::env::var("OAG_VARIANTS").is_ok()
            && let Ok(blob) = oag_assets::Container::open(&spec)
                .and_then(|mut c| c.read_entry(&format!("/{}", material.name)))
            && let Ok(parsed) = oag_rcs::rcsmaterial::RcsMaterial::parse(&blob)
        {
            let picked = model
                .material_variants
                .get(slot)
                .and_then(|v| v.as_ref())
                .map(|v| v.fragment.offset);
            for v in &parsed.variants {
                let samplers = oag_rcs::rcsmaterial::Declared::parse(&blob, v.fragment.offset)
                    .map(|d| {
                        let mut s: Vec<String> = d
                            .samplers
                            .iter()
                            .map(|(h, u)| format!("{h:#010x}@{u}"))
                            .collect();
                        s.sort();
                        s.join(" ")
                    })
                    .unwrap_or_default();
                println!(
                    "     variant class {:?} features {:#010x} fragment@{:#x}{}\n        {samplers}",
                    v.class,
                    v.feature_hash,
                    v.fragment.offset,
                    if picked == Some(v.fragment.offset) {
                        "  <- RESOLVED"
                    } else {
                        ""
                    },
                );
            }
        }
        let traced = model
            .material_variants
            .get(slot)
            .and_then(|v| v.as_ref())
            .and_then(|v| {
                let blob = oag_assets::Container::open(&spec)
                    .and_then(|mut c| c.read_entry(&format!("/{}", material.name)))
                    .ok()?;
                let program =
                    oag_rcs::rcsmaterial::fragment::Program::parse(&blob, v.fragment.offset)?;
                let t = program.output_texels();
                let colour = t[0].merge(t[1]).merge(t[2]);
                Some(format!("colour {colour:?} | alpha {:?}", t[3]))
            })
            .unwrap_or_else(|| "no trace".into());
        println!("     traced {traced}");
        let blend = format!(
            "{:?} src {:#06x} dst {:#06x} state {:#010x}",
            material.blend(),
            material.src_factor,
            material.dst_factor,
            material.state
        );
        // The vertex declaration one chunk of this slot carries, because
        // which attribute a texture is sampled through is the other half of
        // "which unit is which texture" - see the resolved variant's own
        // attribute list from `scripts/ps3-microcode.py vp-file`.
        if let Some(mesh) = raw.meshes.iter().find(|m| m.material as usize == slot)
            && let Some(decl) = mesh.decl.as_ref()
        {
            let diffuse = decl.diffuse_texcoord().map(|a| a.name_hash);
            for a in &decl.attributes {
                println!(
                    "     attr {:#010x} {:16} {} component(s) at +{:#04x}{}",
                    a.name_hash,
                    a.name().unwrap_or("-"),
                    a.components,
                    a.offset,
                    if Some(a.name_hash) == diffuse {
                        "  <- diffuse_texcoord()"
                    } else {
                        ""
                    },
                );
            }
        }
        println!(
            "slot {slot:4} {:3} chunk(s) {:#06x} [{}]\n     {}\n     tex    {}\n     second {}\n     {variant}\n     {blend}",
            chunks.get(&(slot as u32)).copied().unwrap_or(0),
            packed,
            roles.join(" "),
            material.name,
            material.texture,
            material.second_texture.as_deref().unwrap_or("-"),
        );
    }
    Ok(())
}
