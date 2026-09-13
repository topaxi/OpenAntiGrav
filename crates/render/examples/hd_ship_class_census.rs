//! Measures, for every ship `.vex` on the disc, which [`rcsmaterial::Class`]
//! each drawn material's own variant table declares, and whether
//! `Class::Static` - what `skin::variants` hardcodes today - resolves the
//! ordinary lit-race key versus `Class::RigidBody`/`Class::StaticQuake`.
//!
//! A handover thread named `variants()`'s `Class::Static` hardcode as the
//! lead for the ship hull's unresolved materials. **Measured and refuted for
//! that population**: across all 37 `ship.vex` files, 178 drawn materials,
//! trying every `Class` moves **zero** of the 78 materials `Class::Static`
//! misses - every ship material's table carries an identical `Static` and
//! `RigidBody` row at every feature hash it ships (confirmed at the program
//! level too: `diffuse_vcol.rcsmaterial`'s `Static` and `RigidBody` fragment
//! blocks at the shared `Ambient` key are the same offset and the same
//! content hash, `0xe3dae875 @ 0x32e0` - only the *vertex* program differs,
//! which this project's own position/normal decode does not run anyway).
//! The 78 misses are one single cause instead: every one wants the
//! `IleVertex` feature and no class of the material ships it, because
//! `Features::chunk_word` reads "the chunk has a colour set" off
//! `VertexDecl::vertex_colour()`, which shape-matches (4 components,
//! `RSX_UBYTE_NORM`, not named `tangent`) rather than checking a hash - and a
//! ship chunk's matching attribute is `VertexColour1` (`0x7493d450`), a
//! **named, distinct** attribute from the `colorSet1`/`0x1aaf7631` pair
//! `vertex_colour()`'s own doc comment measures. See
//! `hd_ship_vcol_dump.rs` for the per-attribute dump that found this.
//!
//! The class hardcode is still a real, separate bug for a table that ships
//! *only* `RigidBody` rows and no `Static` row at all -
//! `data/fe/frontendscene/frontendscene_hd_atg.vex`'s
//! `basic_vertexemissive.rcsmaterial` is exactly that case, and this example
//! reports it as `MOVED`. It is just not what darkens a ship hull.
//!
//! ```sh
//! cargo run -p oag-render --example hd_ship_class_census
//! cargo run -p oag-render --example hd_ship_class_census -- data/images/hdfury-ps3-eu-dec.iso /data/ships/feisar_c1/ship.vex
//! ```

use oag_rcs::{rcsmaterial, rcsmodel};
use oag_render::mesh;

// Copied from `oag_hd::archives::ALL` - `oag-render` does not depend on
// `oag-hd` and an example may not add one (see `hd_mode2_alpha.rs`'s own
// doc comment for the same rule).
const ARCHIVES: &[&str] = &[
    "PS3_GAME/USRDIR/DATA00.PSARC",
    "PS3_GAME/USRDIR/DATA01.PSARC",
    "PS3_GAME/USRDIR/DATA02.PSARC",
    "PS3_GAME/USRDIR/DATA03.PSARC",
    "PS3_GAME/USRDIR/DATA04.PSARC",
    "PS3_GAME/USRDIR/DATA05.PSARC",
    "PS3_GAME/USRDIR/DATA06.PSARC",
];

/// Every `/data/ships/<team>/ship.vex` on the disc - the 12 base teams, their
/// `_c1`/`_n1` Fury variants, `detonator` and `zone` - found by listing each
/// archive's own manifest rather than a hardcoded roster.
fn every_ship_vex(image: &str) -> anyhow::Result<Vec<String>> {
    let mut found = std::collections::BTreeSet::new();
    for archive in ARCHIVES {
        let spec = format!("{image}:{archive}");
        let Ok(psarc) = oag_assets::psarc::Archive::open(&spec) else {
            continue;
        };
        for path in psarc.paths() {
            if path.starts_with("/data/ships/")
                && path.ends_with("/ship.vex")
                && path.matches('/').count() == 4
            {
                found.insert(path.clone());
            }
        }
    }
    Ok(found.into_iter().collect())
}

struct MaterialCase {
    resolves_static_only: bool,
    resolves_any_class: bool,
}

fn main() -> anyhow::Result<()> {
    let image = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "data/images/hdfury-ps3-eu-dec.iso".into());
    let paths: Vec<String> = {
        let rest: Vec<String> = std::env::args().skip(2).collect();
        if rest.is_empty() {
            every_ship_vex(&image)?
        } else {
            rest
        }
    };

    let mut total_materials = 0usize;
    let mut resolved_static_only = 0usize;
    let mut resolved_any_class = 0usize;
    let mut chunks_total = 0usize;
    let mut chunks_static_only = 0usize;
    let mut chunks_any_class = 0usize;
    let mut moved_by_class = 0usize; // resolves under RigidBody/StaticQuake but not Static
    let mut still_unresolved_by_key = 0usize; // no class resolves at all

    for name in &paths {
        let Some((spec, data)) = ARCHIVES.iter().find_map(|archive| {
            let spec = format!("{image}:{archive}");
            mesh::read_blob(&spec, name).ok().map(|d| (spec, d))
        }) else {
            println!("{name}: not found in any archive");
            continue;
        };
        let Some(geometry) = mesh::rcs::sibling_geometry(&spec, name, &data) else {
            println!("{name}: no sibling .rcsmodel");
            continue;
        };
        let model = rcsmodel::Model::parse(&geometry).map_err(|e| anyhow::anyhow!("{e}"))?;

        let mut decl_of: std::collections::HashMap<u32, Option<&rcsmodel::VertexDecl>> =
            Default::default();
        let mut chunks_of: std::collections::HashMap<u32, usize> = Default::default();
        for mesh in &model.meshes {
            decl_of.entry(mesh.material).or_insert(mesh.decl.as_ref());
            *chunks_of.entry(mesh.material).or_default() += 1;
        }

        for (slot, material) in model.materials.iter().enumerate() {
            let ordinal = u32::try_from(slot).unwrap_or(u32::MAX);
            let chunks = chunks_of.get(&ordinal).copied().unwrap_or(0);
            if chunks == 0 {
                continue;
            }
            let Ok(blob) = mesh::read_blob(&spec, &format!("/{}", material.name)) else {
                continue;
            };
            let Ok(parsed) = rcsmaterial::RcsMaterial::parse(&blob) else {
                continue;
            };
            let decl = decl_of.get(&ordinal).copied().flatten();
            let word = rcsmaterial::Features::chunk_word(rcsmaterial::LIT_RACE_PASS, decl);
            let key = rcsmaterial::Features::from_pass_word(word);

            let resolves = |class: rcsmaterial::Class| parsed.variant(class, key).is_some();
            let case = MaterialCase {
                resolves_static_only: resolves(rcsmaterial::Class::Static),
                resolves_any_class: rcsmaterial::Class::ALL.into_iter().any(resolves),
            };

            total_materials += 1;
            chunks_total += chunks;
            if case.resolves_static_only {
                resolved_static_only += 1;
                chunks_static_only += chunks;
            }
            if case.resolves_any_class {
                resolved_any_class += 1;
                chunks_any_class += chunks;
            }
            if case.resolves_any_class && !case.resolves_static_only {
                moved_by_class += 1;
                let mut classes: Vec<u32> = parsed.variants.iter().map(|v| v.class_hash).collect();
                classes.sort_unstable();
                classes.dedup();
                let resolving: Vec<String> = rcsmaterial::Class::ALL
                    .into_iter()
                    .filter(|&c| resolves(c))
                    .map(|c| format!("{c:?}"))
                    .collect();
                println!(
                    "MOVED  {name} [{slot}] {} ({chunks} chunk(s)) - table carries only {:?}, \
                     Static misses, resolves via {resolving:?}",
                    material.name,
                    classes
                        .iter()
                        .map(|&h| rcsmaterial::Class::from_hash(h)
                            .map_or_else(|| format!("{h:#010x}"), |c| format!("{c:?}")))
                        .collect::<Vec<_>>(),
                );
            } else if !case.resolves_any_class {
                still_unresolved_by_key += 1;
                let mut classes: Vec<u32> = parsed.variants.iter().map(|v| v.class_hash).collect();
                classes.sort_unstable();
                classes.dedup();
                println!(
                    "MISS   {name} [{slot}] {} ({chunks} chunk(s)) - table carries {:?}, \
                     none ships the wanted key {:?} ({:#010x}) under any class - a key problem, \
                     not a class problem",
                    material.name,
                    classes
                        .iter()
                        .map(|&h| rcsmaterial::Class::from_hash(h)
                            .map_or_else(|| format!("{h:#010x}"), |c| format!("{c:?}")))
                        .collect::<Vec<_>>(),
                    key.name(),
                    key.hash(),
                );
            }
        }
    }

    println!();
    println!(
        "materials: {resolved_static_only} of {total_materials} resolve today (Class::Static only)"
    );
    println!(
        "materials: {resolved_any_class} of {total_materials} would resolve trying every Class \
         ({moved_by_class} moved by the class fix, {still_unresolved_by_key} still miss - a key \
         problem, not a class one)"
    );
    println!(
        "chunks: {chunks_static_only} of {chunks_total} drawn today, {chunks_any_class} of \
         {chunks_total} with every Class tried"
    );
    Ok(())
}
