//! Census behind `mesh::rcs::light_cone`: every drawn HD material whose
//! resolved fragment variant declares exactly `Texture1` and the cone's ramp
//! (`0xa2d555b9`), with what each lane of its output traces to and the two
//! parameters it is multiplied by.
//!
//! ```sh
//! cargo run --release -p oag-render --example hd_light_cone_census
//! ```

use oag_mesh::mesh;
use oag_rcs::rcsmaterial::{self, fragment::Texel};

const ARCHIVES: &[&str] = &[
    "PS3_GAME/USRDIR/DATA00.PSARC",
    "PS3_GAME/USRDIR/DATA01.PSARC",
    "PS3_GAME/USRDIR/DATA02.PSARC",
    "PS3_GAME/USRDIR/DATA03.PSARC",
];
const TEXTURE1: u32 = 0x3bdc_0403;
const RAMP: u32 = 0xa2d5_55b9;
const INTENSITY: u32 = 0x60ea_f40d;
const SCALE: u32 = 0x7611_a2d8;

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
            .filter(|p| p.starts_with("/data/environments/") && p.ends_with("/track.vex"))
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
                let mut want = vec![TEXTURE1, RAMP];
                want.sort_unstable();
                if hashes != want {
                    continue;
                }
                let Some(program) =
                    rcsmaterial::fragment::Program::parse(&blob, variant.fragment.offset)
                else {
                    continue;
                };
                let t = program.output_texels();
                let show = |t: Texel| format!("{t:?}");
                let value = |h: u32| {
                    material
                        .parameters
                        .iter()
                        .find(|p| p.hash == h)
                        .map(|p| p.value)
                };
                println!(
                    "{path} slot {slot} {} frag@{:#x} samplers {:?} rgba [{} {} {} {}] intensity {:?} scale {:?}",
                    material.name,
                    variant.fragment.offset,
                    declared.samplers,
                    show(t[0]),
                    show(t[1]),
                    show(t[2]),
                    show(t[3]),
                    value(INTENSITY),
                    value(SCALE),
                );
            }
        }
    }
    Ok(())
}
