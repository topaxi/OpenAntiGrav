//! Scratch probe: the material state word, bit by bit, against the material
//! names that carry it.
//!
//! Only the low two bits of `Material::state` are read today
//! (`oag_formats::rcsmodel::Transparency`). This tabulates the rest, so a bit
//! that separates the cutout materials from the rest can be found by what
//! carries it rather than guessed.

use oag_render::mesh;
use std::collections::BTreeMap;

/// The circuit directories, copied from `oag_hd::names::ENVIRONMENTS` because
/// `oag-render` does not depend on `oag-hd` and an example may not add one.
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
    let want = std::env::args()
        .nth(2)
        .unwrap_or_default()
        .to_ascii_lowercase();

    let mut by_state: BTreeMap<u32, (usize, Vec<String>)> = BTreeMap::new();
    let mut by_bit: BTreeMap<u32, Vec<String>> = BTreeMap::new();
    let mut total = 0usize;

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
            let Ok(model) = oag_formats::rcsmodel::Model::parse(&geometry) else {
                continue;
            };
            for material in &model.materials {
                total += 1;
                let stem = material
                    .name
                    .rsplit('/')
                    .next()
                    .unwrap_or(&material.name)
                    .to_string();
                let row = by_state.entry(material.state).or_default();
                row.0 += 1;
                if row.1.len() < 6 && !row.1.contains(&stem) {
                    row.1.push(stem.clone());
                }
                for bit in 2..32 {
                    if material.state & (1 << bit) != 0 {
                        let names = by_bit.entry(bit).or_default();
                        if names.len() < 8 && !names.contains(&stem) {
                            names.push(stem.clone());
                        }
                    }
                }
            }
        }
    }

    println!("{total} material record(s) across every circuit read");
    println!("\ndistinct state words:");
    for (state, (count, names)) in &by_state {
        println!(
            "  {state:#010x} {:#034b} {count:>6}  {}",
            state,
            names.join(", ")
        );
    }
    println!("\nbits above the transparency field, and what carries them:");
    for (bit, names) in &by_bit {
        println!(
            "  bit {bit:>2} ({:#010x}): {}",
            1u32 << bit,
            names.join(", ")
        );
    }
    if !want.is_empty() {
        println!("\nstate words of materials matching {want:?}: see hd_slot_check");
    }
    Ok(())
}
