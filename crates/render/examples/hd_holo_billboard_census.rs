//! Census behind `mesh::rcs::holo_billboard`: every drawn HD material slot
//! whose resolved fragment variant declares the three samplers of
//! `nr_billboardholographicscanlines` (advert, scanline layer, second layer),
//! with the block it resolved to, the inline literals its program multiplies
//! by, and the three paths its sampler table binds.
//!
//! ```sh
//! cargo run -p oag-render --example hd_holo_billboard_census
//! ```

use oag_mesh::mesh;
use oag_rcs::rcsmaterial::{self, fragment::Program};

const ARCHIVES: &[&str] = &[
    "PS3_GAME/USRDIR/DATA00.PSARC",
    "PS3_GAME/USRDIR/DATA01.PSARC",
    "PS3_GAME/USRDIR/DATA02.PSARC",
    "PS3_GAME/USRDIR/DATA03.PSARC",
];
const ADVERT: u32 = 0x6f46_9b89;
const SCANLINES: u32 = 0xfad8_b460;
const SECOND: u32 = 0xdd7e_c609;

fn main() -> anyhow::Result<()> {
    let image = "data/images/hdfury-ps3-eu-dec.iso";
    for archive in ARCHIVES {
        let spec = format!("{image}:{archive}");
        let Ok(open) = oag_assets::psarc::Archive::open(&spec) else {
            continue;
        };
        let tracks: Vec<String> = open
            .paths()
            .iter()
            .filter(|p| p.starts_with("/data/environments/") && p.ends_with(".vex"))
            .filter(|p| !p.contains("/fe/") && !p.contains("start_grid") && !p.contains("padrepl"))
            .cloned()
            .collect();
        for path in tracks {
            let Ok(data) = mesh::read_blob(&spec, &path) else {
                continue;
            };
            let Some(geometry) = mesh::rcs::sibling_geometry(&spec, &path, &data) else {
                continue;
            };
            let Ok((model, _)) = mesh::rcs::build_scene(&path, &data, &geometry, &mut |name| {
                mesh::read_blob(&spec, name).ok()
            }) else {
                continue;
            };
            let Ok(source) = oag_rcs::rcsmodel::Model::parse(&geometry) else {
                continue;
            };
            for (slot, material) in source.materials.iter().enumerate() {
                let Some(variant) = model.material_variants.get(slot).copied().flatten() else {
                    continue;
                };
                let Ok(blob) = mesh::read_blob(&spec, &format!("/{}", material.name)) else {
                    continue;
                };
                let Some(declared) = rcsmaterial::Declared::parse(&blob, variant.fragment.offset)
                else {
                    continue;
                };
                let mut hashes: Vec<u32> = declared.samplers.iter().map(|s| s.0).collect();
                hashes.sort_unstable();
                let mut want = vec![ADVERT, SCANLINES, SECOND];
                want.sort_unstable();
                if hashes != want {
                    continue;
                }
                let Some(program) = Program::parse(&blob, variant.fragment.offset) else {
                    continue;
                };
                let literals: Vec<String> = program
                    .instructions
                    .iter()
                    .filter(|i| matches!(i.name(), Some("MUL" | "MAD")))
                    .filter_map(|i| {
                        let (c, s) = (i.constant?, i.const_slot?);
                        let patched = program.patches(0x906b_67ba).any(|p| p == s);
                        Some(format!(
                            "{}@{s:#x}{}={}",
                            i.name()?,
                            if patched { "p" } else { "" },
                            c[0]
                        ))
                    })
                    .collect();
                let path_of = |hash: u32| {
                    material
                        .samplers
                        .iter()
                        .find(|(h, _)| *h == hash)
                        .and_then(|(_, p)| p.clone())
                };
                let params: Vec<String> = [0x088b_2c6a_u32, 0x4f2a_ef81, 0xef6e_4697]
                    .iter()
                    .map(|h| {
                        material
                            .parameters
                            .iter()
                            .find(|p| p.hash == *h)
                            .map_or("-".to_string(), |p| format!("{}", p.value[0]))
                    })
                    .collect();
                println!(
                    "{path} slot {slot} {} frag@{:#x} n={} interp={:#x} lit [{}] params [{}] advert {:?} scan {:?} second {:?} bound-albedo {:?}",
                    material.name.rsplit('/').next().unwrap_or(""),
                    variant.fragment.offset,
                    program.instructions.len(),
                    program.interpolators(),
                    literals.join(" "),
                    params.join(" "),
                    path_of(ADVERT),
                    path_of(SCANLINES),
                    path_of(SECOND),
                    material.texture,
                );
            }
        }
    }
    Ok(())
}
