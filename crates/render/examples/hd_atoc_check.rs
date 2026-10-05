//! Scratch probe: does every material carrying `Material::state` bit 7 sample
//! a texture named with the disc's own `_atoc` suffix? Follows on from
//! `hd_state_census`, which found bit 7 carried by exactly eight material
//! names and nothing more precise than the name coincidence.

use oag_mesh::mesh;
use std::collections::BTreeMap;

const ENVIRONMENTS: &[&str] = &[
    "amphiseum",
    "modesto_heights",
    "talons_junction",
    "tech_de_ra",
    "zone_1",
    "zone_2",
    "zone_3",
    "zone_4",
    "01_vineta_k",
    "02_track",
    "03_track",
    "04_chenghou_project",
    "05_ubermall",
    "10_sebenco_climb",
    "12_sol_2",
    "15_anulpha_pass",
];

fn main() -> anyhow::Result<()> {
    let image = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "data/images/hdfury-ps3-eu-dec.iso".into());

    let mut with_atoc = 0usize;
    let mut without_atoc: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut total = 0usize;
    let mut atoc_textured_total = 0usize;
    let mut atoc_textured_bit7 = 0usize;
    let mut atoc_textured_no_bit7: BTreeMap<String, u32> = BTreeMap::new();

    for archive in ["DATA00", "DATA01", "DATA02"] {
        let spec = format!("{image}:PS3_GAME/USRDIR/{archive}.PSARC");
        for environment in ENVIRONMENTS {
            let name = format!("/data/environments/{environment}/track.vex");
            let Ok(data) = mesh::read_blob(&spec, &name) else {
                continue;
            };
            let Some(geometry) = mesh::rcs::sibling_geometry(&spec, &name, &data) else {
                continue;
            };
            let Ok(model) = oag_rcs::rcsmodel::Model::parse(&geometry) else {
                continue;
            };
            for material in &model.materials {
                let stem = material
                    .name
                    .rsplit('/')
                    .next()
                    .unwrap_or(&material.name)
                    .to_string();
                let has_atoc = material
                    .samplers
                    .iter()
                    .any(|(_, path)| path.as_deref().is_some_and(|p| p.contains("_atoc.")));
                let bit7 = material.state & 0x80 != 0;

                if has_atoc {
                    atoc_textured_total += 1;
                    if bit7 {
                        atoc_textured_bit7 += 1;
                    } else {
                        *atoc_textured_no_bit7
                            .entry(format!("{environment}/{stem}"))
                            .or_default() += 1;
                    }
                }

                if !bit7 {
                    continue;
                }
                total += 1;
                if has_atoc {
                    with_atoc += 1;
                } else {
                    let paths: Vec<String> = material
                        .samplers
                        .iter()
                        .map(|(hash, path)| {
                            path.clone()
                                .unwrap_or_else(|| format!("{hash:#010x} (unresolved)"))
                        })
                        .collect();
                    without_atoc
                        .entry(format!("{environment}/{stem}"))
                        .or_default()
                        .extend(paths);
                }
            }
        }
    }

    println!("{total} material record(s) with bit 7 set");
    println!("{with_atoc} sample at least one '_atoc.' texture");
    println!(
        "{} do not - by environment/material, with their sampler paths:",
        without_atoc.len()
    );
    for (key, paths) in &without_atoc {
        println!("  {key}: {paths:?}");
    }

    println!("\n{atoc_textured_total} material record(s) sample an '_atoc.' texture at all");
    println!("{atoc_textured_bit7} of those carry bit 7");
    println!(
        "{} do not - by environment/material, with counts:",
        atoc_textured_no_bit7.len()
    );
    for (key, count) in &atoc_textured_no_bit7 {
        println!("  {key}: {count}");
    }
    Ok(())
}
