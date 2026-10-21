//! Disc-wide census of the vertex programs `1 - v` reading: for every
//! `.rcsmodel`, every material's resolved lit-race vertex block, whether the
//! flip is written straight into an interpolator (the old reading) or through a
//! temporary (added 2026-10-21), and which materials only the second catches.
//!
//! ```sh
//! cargo run -p oag-render --example hd_flip_disc_census
//! ```
use oag_assets::psarc;
use oag_rcs::{rcsmaterial, rcsmodel};
use std::collections::BTreeMap;

const ARCHIVES: &[&str] = &[
    "PS3_GAME/USRDIR/DATA00.PSARC",
    "PS3_GAME/USRDIR/DATA01.PSARC",
    "PS3_GAME/USRDIR/DATA02.PSARC",
    "PS3_GAME/USRDIR/DATA03.PSARC",
    "PS3_GAME/USRDIR/DATA04.PSARC",
    "PS3_GAME/USRDIR/DATA05.PSARC",
    "PS3_GAME/USRDIR/DATA06.PSARC",
];

fn direct_flip(p: &rcsmaterial::vertex::Program, slot: u32) -> bool {
    p.instructions.iter().any(|i| {
        i.dest_is_output
            && i.dest >= 7
            && u32::from(i.input) == slot
            && match i.vec_op {
                0x01 | 0x0d | 0x0e | 0x0f | 0x16 | 0x17 => &[0usize][..],
                0x03 => &[0, 2][..],
                0x04 => &[0, 1, 2][..],
                _ => &[0, 1][..],
            }
            .iter()
            .any(|&n| i.sources[n] & 3 == 2 && i.sources[n] & (1 << 16) != 0)
    })
}

fn main() -> anyhow::Result<()> {
    let image = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "data/images/hdfury-ps3-eu-dec.iso".into());
    // (material path, vertex offset) -> (direct, new, models using it)
    let mut seen: BTreeMap<(String, usize), (bool, bool, Vec<String>)> = BTreeMap::new();
    let mut models = 0usize;
    for archive in ARCHIVES {
        let spec = format!("{image}:{archive}");
        let Ok(mut psarc) = psarc::Archive::open(&spec) else {
            continue;
        };
        let paths: Vec<String> = psarc
            .paths()
            .iter()
            .filter(|p| p.ends_with(".rcsmodel"))
            .cloned()
            .collect();
        let mut blobs: BTreeMap<String, Option<Vec<u8>>> = BTreeMap::new();
        for path in paths {
            let Ok(data) = psarc.read_path(&path) else {
                continue;
            };
            let Ok(model) = rcsmodel::Model::parse(&data) else {
                continue;
            };
            models += 1;
            let mut decl_of = BTreeMap::new();
            for mesh in model.meshes.iter().flat_map(rcsmodel::Mesh::surfaces) {
                if let Some(d) = mesh.decl.as_ref() {
                    decl_of.entry(mesh.material).or_insert(d);
                }
            }
            for (slot, material) in model.materials.iter().enumerate() {
                let Some(decl) = decl_of.get(&u32::try_from(slot)?) else {
                    continue;
                };
                let Some(hash) = decl.diffuse_texcoord().map(|a| a.name_hash) else {
                    continue;
                };
                let blob = blobs
                    .entry(material.name.clone())
                    .or_insert_with(|| psarc.read_path(&format!("/{}", material.name)).ok())
                    .clone();
                let Some(blob) = blob else { continue };
                let Ok(parsed) = rcsmaterial::RcsMaterial::parse(&blob) else {
                    continue;
                };
                let word =
                    rcsmaterial::Features::chunk_word(rcsmaterial::LIT_RACE_PASS, Some(decl));
                let key = rcsmaterial::Features::from_pass_word(word);
                let Some(variant) = parsed.variant(rcsmaterial::Class::Static, key) else {
                    continue;
                };
                let Some(program) = rcsmaterial::vertex::Program::of(&blob, variant.vertex) else {
                    continue;
                };
                let Some(reg) = program.attribute_slot(hash) else {
                    continue;
                };
                let entry = seen
                    .entry((material.name.clone(), variant.vertex.offset))
                    .or_insert_with(|| (direct_flip(&program, reg), program.flips(hash), vec![]));
                if entry.2.len() < 3 {
                    entry.2.push(path.clone());
                }
            }
        }
    }
    println!(
        "{models} model(s), {} distinct (material, vertex block)",
        seen.len()
    );
    let direct = seen.values().filter(|v| v.0).count();
    let new = seen.values().filter(|v| v.1).count();
    println!("flips: direct {direct}, with temporaries {new}");
    for ((name, off), (d, n, users)) in &seen {
        if d != n {
            println!("ONLY-TEMP {name} @{off:#x} used by {users:?}");
        }
        if *d && !*n {
            println!("LOST {name} @{off:#x}");
        }
    }
    Ok(())
}
