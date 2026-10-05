//! Disc-wide sweep for the "glass sheen" combine `etched_glass_tech.rcsmaterial`
//! block #7 traces in full: a `dot(V, N)`-addressed facing ramp mixed with a
//! surface-UV grid texture, no lightmap, no normal or specular map -
//! `docs/formats/rcsmaterial.md`, "Three sampler roles identified by what
//! they bind, and the glass floor stops painting a ramp".
//!
//! # The classifier, and why it is fact-based rather than a material name
//!
//! A resolved lit-race variant qualifies when **all** hold:
//!
//! 1. Its declared sampler set is *exactly* three hashes: [`TEXTURE1`], one of
//!    the three facing-ramp hashes ([`RAMP_HASHES`]), and
//!    [`PARABOLOID_REFLECTION`] - no lightmap, no normal map, no specular map
//!    mixed in.
//! 2. `Program::output_texels`'s colour lane is [`Texel::Mixed`] - more than
//!    one unit reaches it, which a flat single-texture picture never is.
//! 3. The alpha lane traces to the unit the ramp hash's own entry sits at -
//!    `etched_glass_tech`'s own alpha is unit 0's red, and unit 0 is
//!    `Texture1`, not the ramp.
//!
//! Built as the direct test of "does this shape occur once or many times",
//! before `crates/mesh/src/mesh/rcs/skin.rs`'s `picks`/`roles` act on it -
//! the same reason `hd_emissive_reach.rs` exists for its own mechanism.
//!
//! ```sh
//! cargo run --release -p oag-render --example hd_glass_sheen_census
//! ```
//!
//! A second mode, `--params <material substring>`, prints one material's own
//! parameter table (hash, value) against `<image>:<archive>` and `<track>` -
//! how this page's own `c` value (`0x512f8e65`) was read off the disc rather
//! than guessed.

use oag_mesh::mesh;
use oag_rcs::rcsmaterial::{self, fragment::Texel};

const ARCHIVES: &[&str] = &[
    "PS3_GAME/USRDIR/DATA00.PSARC",
    "PS3_GAME/USRDIR/DATA01.PSARC",
    "PS3_GAME/USRDIR/DATA02.PSARC",
    "PS3_GAME/USRDIR/DATA03.PSARC",
];

/// `~crc32("Texture1")`.
const TEXTURE1: u32 = 0x3bdc_0403;
/// `~crc32("paraboloidReflectionTex")`.
const PARABOLOID_REFLECTION: u32 = 0x9edd_3243;
/// The three facing-ramp hashes `skin::NOT_A_PICTURE` also carries - every
/// use disc-wide binds a texture 32 texels or less in one dimension. See
/// `docs/formats/rcsmaterial.md`, "Three sampler roles identified by what
/// they bind".
const RAMP_HASHES: [u32; 3] = [0x3528_1c78, 0x994b_bcf1, 0x94b2_b285];

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let image = "data/images/hdfury-ps3-eu-dec.iso".to_string();

    if let Some(flag) = args.next()
        && flag == "--params"
    {
        let want = args.next().unwrap_or_default();
        let spec = format!("{image}:{}", ARCHIVES[0]);
        let name = "/data/environments/talons_junction/track.vex";
        let data = mesh::read_blob(&spec, name)?;
        let geometry = mesh::rcs::sibling_geometry(&spec, name, &data)
            .ok_or_else(|| anyhow::anyhow!("no sibling .rcsmodel"))?;
        let model = oag_rcs::rcsmodel::Model::parse(&geometry)?;
        for material in &model.materials {
            if !material.name.contains(&want) {
                continue;
            }
            println!("{}", material.name);
            for (hash, path) in &material.samplers {
                println!("  sampler   {hash:#010x} -> {path:?}");
            }
            for p in &material.parameters {
                println!(
                    "  parameter {:#010x} = [{}, {}, {}, {}] ({} quad(s))",
                    p.hash, p.value[0], p.value[1], p.value[2], p.value[3], p.quads
                );
            }
        }
        return Ok(());
    }

    let mut total_materials = 0usize;
    let mut total_chunks = 0usize;
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
            let Ok((model, _report)) =
                mesh::rcs::build_scene(&path, &data, &geometry, &mut |name| {
                    mesh::read_blob(&spec, name).ok()
                })
            else {
                continue;
            };
            let Ok(geometry_model) = oag_rcs::rcsmodel::Model::parse(&geometry) else {
                continue;
            };
            let mut chunks: std::collections::BTreeMap<u32, usize> = Default::default();
            for mesh in geometry_model
                .meshes
                .iter()
                .flat_map(oag_rcs::rcsmodel::Mesh::surfaces)
            {
                *chunks.entry(mesh.material).or_default() += 1;
            }
            let mut cache: std::collections::HashMap<String, Option<Vec<u8>>> = Default::default();
            for (slot, material) in geometry_model.materials.iter().enumerate() {
                let Some(variant) = model.material_variants.get(slot).copied().flatten() else {
                    continue;
                };
                let blob = cache
                    .entry(material.name.clone())
                    .or_insert_with(|| mesh::read_blob(&spec, &format!("/{}", material.name)).ok())
                    .clone();
                let Some(blob) = blob else { continue };
                let Some(declared) = rcsmaterial::Declared::parse(&blob, variant.fragment.offset)
                else {
                    continue;
                };
                let ramp_hash = declared
                    .samplers
                    .iter()
                    .find(|&&(h, _)| RAMP_HASHES.contains(&h))
                    .map(|&(h, _)| h);
                let has_texture1 = declared.samplers.iter().any(|&(h, _)| h == TEXTURE1);
                let has_paraboloid = declared
                    .samplers
                    .iter()
                    .any(|&(h, _)| h == PARABOLOID_REFLECTION);
                let Some(ramp_hash) = ramp_hash else {
                    continue;
                };
                if !(has_texture1 && has_paraboloid && declared.samplers.len() == 3) {
                    continue;
                }
                let Some(program) =
                    rcsmaterial::fragment::Program::parse(&blob, variant.fragment.offset)
                else {
                    continue;
                };
                let texels = program.output_texels();
                let colour = texels[0].merge(texels[1]).merge(texels[2]);
                if colour != Texel::Mixed {
                    continue;
                }
                let ramp_unit = declared
                    .samplers
                    .iter()
                    .find(|&&(h, _)| h == ramp_hash)
                    .map(|&(_, u)| u);
                let alpha_at_ramp = matches!(
                    texels[3],
                    Texel::Unit { unit, .. } if ramp_unit == Some(u32::from(unit))
                );
                if alpha_at_ramp {
                    // Alpha traces to the ramp's own unit rather than
                    // `Texture1`'s - a different combine from the one traced,
                    // so this is reported and not counted as a match.
                    println!(
                        "  {path} slot {slot} {}: alpha traces to the RAMP's own unit, not Texture1's - not this combine",
                        material.name
                    );
                    continue;
                }
                let n = chunks
                    .get(&u32::try_from(slot).unwrap_or(u32::MAX))
                    .copied()
                    .unwrap_or(0);
                total_materials += 1;
                total_chunks += n;
                println!(
                    "{path} slot {slot} {:4} chunk(s) {} (ramp hash {ramp_hash:#010x})",
                    n, material.name
                );
            }
        }
    }
    println!("\ntotal: {total_materials} material(s), {total_chunks} chunk(s)");
    Ok(())
}
