//! Census: which resolved lit-race vertex blocks flip the `lightmapUV` attribute
//! (`1 - v` into an interpolator, straight or through a temporary), the way
//! `hd_flip_disc_census` asks of the diffuse coordinate, and how many chunks and
//! materials each affects. `mesh::rcs::skin::flips` reads only the diffuse set.
//!
//! ```sh
//! cargo run -p oag-render --example hd_lightmap_flip_census [image]
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

/// What one (material, vertex block) pair does with its coordinates.
struct Block {
    lightmap_flips: bool,
    diffuse_flips: bool,
    users: Vec<String>,
    chunks: usize,
}

fn main() -> anyhow::Result<()> {
    let image = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "data/images/hdfury-ps3-eu-dec.iso".into());
    let mut seen: BTreeMap<(String, usize), Block> = BTreeMap::new();
    let (mut models, mut resolved, mut unread) = (0usize, 0usize, 0usize);
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
            for mesh in model.meshes.iter().flat_map(rcsmodel::Mesh::surfaces) {
                let Some(decl) = mesh.decl.as_ref() else {
                    continue;
                };
                let Some(lm) = decl
                    .attributes
                    .iter()
                    .find(|a| a.name() == Some("lightmapUV"))
                else {
                    continue;
                };
                let Some(material) = model.materials.get(mesh.material as usize) else {
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
                    unread += 1;
                    continue;
                };
                let Some(program) = rcsmaterial::vertex::Program::of(&blob, variant.vertex) else {
                    unread += 1;
                    continue;
                };
                resolved += 1;
                let diffuse = decl
                    .diffuse_texcoord()
                    .is_some_and(|a| program.flips(a.name_hash));
                let entry = seen
                    .entry((material.name.clone(), variant.vertex.offset))
                    .or_insert_with(|| Block {
                        lightmap_flips: program.flips(lm.name_hash),
                        diffuse_flips: diffuse,
                        users: vec![],
                        chunks: 0,
                    });
                entry.chunks += 1;
                if entry.users.len() < 2 && !entry.users.contains(&path) {
                    entry.users.push(path.clone());
                }
            }
        }
    }
    println!(
        "{models} model(s); {resolved} lightmapUV chunk(s) resolved, {unread} not; {} distinct (material, vertex block)",
        seen.len()
    );
    let flipping: Vec<_> = seen.iter().filter(|(_, v)| v.lightmap_flips).collect();
    println!(
        "lightmapUV flipped by {} block(s), {} chunk(s)",
        flipping.len(),
        flipping.iter().map(|(_, v)| v.chunks).sum::<usize>()
    );
    for ((name, off), block) in flipping {
        println!(
            "FLIPS lightmapUV {name} @{off:#x} diffuse-flips {} chunks {} in {:?}",
            block.diffuse_flips, block.chunks, block.users
        );
    }
    Ok(())
}
