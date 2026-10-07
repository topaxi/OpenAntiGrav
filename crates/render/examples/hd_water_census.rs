//! Census: every circuit material whose resolved fragment variant declares
//! `paraboloidReflectionTex` (`0x9edd3243`) or whose name says water or ice.
//!
//! ```sh
//! cargo run -p oag-render --example hd_water_census -- <image> /data/environments/01_vineta_k/track.vex ...
//! ```

use oag_assets::Container;
use oag_mesh::mesh;
use oag_rcs::{rcsmaterial, rcsmodel};

const PARABOLOID: u32 = 0x9edd_3243;

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let image = args.next().unwrap();
    for name in args {
        let Some((spec, data)) = (0..4).find_map(|n| {
            let spec = format!("{image}:PS3_GAME/USRDIR/DATA0{n}.PSARC");
            mesh::read_blob(&spec, &name).ok().map(|d| (spec, d))
        }) else {
            println!("{name}: not found");
            continue;
        };
        let Some(geometry) = mesh::rcs::sibling_geometry(&spec, &name, &data) else {
            continue;
        };
        let model = rcsmodel::Model::parse(&geometry)?;
        let mut container = Container::open(&spec)?;
        let mut decl_of = std::collections::HashMap::new();
        for mesh in model.meshes.iter().flat_map(rcsmodel::Mesh::surfaces) {
            decl_of.entry(mesh.material).or_insert(mesh.decl.as_ref());
        }
        for (slot, material) in model.materials.iter().enumerate() {
            let Some(Some(decl)) = decl_of.get(&u32::try_from(slot)?) else {
                continue;
            };
            let blob = container.read_entry(&format!("/{}", material.name)).ok();
            let mut frag = 0usize;
            let declared = blob.as_ref().and_then(|blob| {
                let parsed = rcsmaterial::RcsMaterial::parse(blob).ok()?;
                let word =
                    rcsmaterial::Features::chunk_word(rcsmaterial::LIT_RACE_PASS, Some(*decl));
                let key = rcsmaterial::Features::from_pass_word(word);
                let variant = parsed.variant(rcsmaterial::Class::Static, key)?;
                frag = variant.fragment.offset;
                rcsmaterial::Declared::parse(blob, variant.fragment.offset)
            });
            let para = declared
                .as_ref()
                .is_some_and(|d| d.samplers.iter().any(|&(h, _)| h == PARABOLOID));
            let lower = material.name.to_lowercase();
            let wordy = lower.contains("water") || lower.contains("ice") || lower.contains("sea");
            if !para && !wordy {
                continue;
            }
            let pics: Vec<String> = material
                .samplers
                .iter()
                .map(|(h, p)| {
                    format!(
                        "{h:08x}={}",
                        p.as_deref().unwrap_or("-").rsplit('/').next().unwrap_or("")
                    )
                })
                .collect();
            let decl: Vec<String> = declared
                .iter()
                .flat_map(|d| d.samplers.iter())
                .map(|(h, u)| format!("{h:08x}@{u}"))
                .collect();
            let params: Vec<String> = material
                .parameters
                .iter()
                .map(|p| format!("{:08x}={:?}", p.hash, p.value))
                .collect();
            println!("    params {}", params.join(" "));
            println!(
                "{name} slot {slot} frag@{frag:#x} para={para} {} [{}] decl [{}]",
                material.name.rsplit('/').next().unwrap_or(""),
                pics.join(" "),
                decl.join(" ")
            );
        }
    }
    Ok(())
}
