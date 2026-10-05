//! Disc-wide census behind `docs/rendering/hd-unlit-programs.md`: every drawn
//! material slot of every `.rcsmodel` in all seven archives, resolved to its
//! lit-race-pass fragment program the way `mesh::rcs::skin` resolves it, and
//! sorted into
//!
//! - the slots `mesh::rcs::rim_glow_bit` routes (the LeachBall's
//!   `RIM_GLOW`, the Plasma head's `RIM_EDGE`);
//! - slots whose program has either shape's **mnemonic sequence** but was
//!   refused on its literals, parameters or authored alpha - what the path
//!   does not cover, listed rather than widened to;
//! - slots whose program is the bloomring's texture-only shape
//!   (`TEX`, `fogColour`, `globalAlphaScaler` and nothing else), which the
//!   existing `slots::EMISSIVE` path already draws as its texture.
//!
//! ```sh
//! cargo run --release -p oag-render --example hd_unlit_census
//! ```

use std::collections::{BTreeMap, HashMap};

use oag_mesh::mesh;
use oag_rcs::rcsmaterial::{self, Class, Declared, Features, LIT_RACE_PASS, fragment::Program};
use oag_rcs::rcsmodel;

const ARCHIVES: &[&str] = &[
    "DATA00", "DATA01", "DATA02", "DATA03", "DATA04", "DATA05", "DATA06",
];

/// `hd_leachbeam_ball_glow` `@0x19b0`'s and `plasmasphere_subtractive_glow`
/// `@0x18d0`'s opening mnemonics, long enough to tell the two skeletons from
/// anything else: a program that opens this way and is refused is a near
/// miss worth listing.
const LEACH_OPENING: &[&str] = &[
    "MOV", "MOV", "MOV", "MOV", "MAD", "MOV", "TEX", "MOV", "DP3",
];
const PLASMA_OPENING: &[&str] = &[
    "MOV", "MOV", "DP3", "DP3", "MOV", "DP3", "MUL", "DIVSQ", "MOV",
];

/// `hd_leachbeam_bloomring` `@0x16a0`: one fogged texture tap.
const TEXTURE_ONLY: &[&str] = &[
    "MUL", "MUL", "MOV", "MUL", "MOV", "EX2", "TEX", "MAD", "MAD", "FENCB", "MAD",
];

fn main() {
    let image = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "data/images/hdfury-ps3-eu-dec.iso".into());
    let mut handles: Vec<_> = ARCHIVES
        .iter()
        .filter_map(|a| {
            oag_assets::psarc::Archive::open(&format!("{image}:PS3_GAME/USRDIR/{a}.PSARC"))
                .ok()
                .map(|h| (*a, h))
        })
        .collect();
    let mut materials: HashMap<String, Option<Vec<u8>>> = HashMap::new();
    // (category, material name) -> (models, chunks)
    let mut rows: BTreeMap<(&str, String), (usize, usize)> = BTreeMap::new();
    let (mut slots_seen, mut unresolved) = (0usize, 0usize);

    for h in 0..handles.len() {
        let paths: Vec<String> = handles[h]
            .1
            .paths()
            .iter()
            .filter(|p| p.ends_with(".rcsmodel"))
            .cloned()
            .collect();
        for path in paths {
            let Ok(data) = handles[h].1.read_path(&path) else {
                continue;
            };
            let Ok(model) = rcsmodel::Model::parse(&data) else {
                continue;
            };
            let mut chunks: HashMap<u32, (usize, Option<&rcsmodel::VertexDecl>)> = HashMap::new();
            for m in &model.meshes {
                let entry = chunks.entry(m.material).or_insert((0, m.decl.as_ref()));
                entry.0 += 1;
            }
            for (slot, material) in model.materials.iter().enumerate() {
                let Some(&(count, decl)) = chunks.get(&u32::try_from(slot).unwrap_or(u32::MAX))
                else {
                    continue;
                };
                slots_seen += 1;
                let name = format!("/{}", material.name);
                let blob = materials
                    .entry(name.clone())
                    .or_insert_with(|| {
                        handles
                            .iter_mut()
                            .find_map(|(_, a)| a.read_path(&name).ok())
                    })
                    .clone();
                let resolved = blob.as_deref().and_then(|blob| {
                    let parsed = rcsmaterial::RcsMaterial::parse(blob).ok()?;
                    let key = Features::from_pass_word(Features::chunk_word(LIT_RACE_PASS, decl));
                    let variant = Class::ALL
                        .into_iter()
                        .find_map(|c| parsed.variant(c, key))?;
                    Some((
                        Declared::parse(blob, variant.fragment.offset)?,
                        Program::parse(blob, variant.fragment.offset)?,
                    ))
                });
                let Some((declared, program)) = resolved else {
                    unresolved += 1;
                    continue;
                };
                let alpha = material
                    .parameters
                    .iter()
                    .find(|p| p.hash == mesh::rcs::RIM_EDGE_ALPHA)
                    .map(|p| p.value[0]);
                let bit = mesh::rcs::rim_glow_bit(&declared, &program, alpha);
                let names: Vec<&str> = program
                    .instructions
                    .iter()
                    .map(|i| i.name().unwrap_or("?"))
                    .collect();
                let category = if bit == mesh::slots::RIM_GLOW {
                    "RIM_GLOW routed"
                } else if bit == mesh::slots::RIM_EDGE {
                    "RIM_EDGE routed"
                } else if names.starts_with(LEACH_OPENING) || names.starts_with(PLASMA_OPENING) {
                    "rim skeleton, refused"
                } else if names == TEXTURE_ONLY {
                    "texture-only (EMISSIVE path)"
                } else {
                    continue;
                };
                let row = rows
                    .entry((
                        category,
                        format!(
                            "{} [{}{}]",
                            material.name,
                            handles[h].0,
                            if bit != 0 {
                                format!(" {path}")
                            } else {
                                String::new()
                            }
                        ),
                    ))
                    .or_default();
                row.0 += 1;
                row.1 += count;
            }
        }
    }

    println!(
        "{slots_seen} drawn material slot(s), {unresolved} with no resolvable lit-race program"
    );
    let mut last = "";
    for ((category, name), (models, chunks)) in &rows {
        if *category != last {
            println!("\n{category}:");
            last = category;
        }
        println!("  {name}: {models} model slot(s), {chunks} chunk(s)");
    }
}
