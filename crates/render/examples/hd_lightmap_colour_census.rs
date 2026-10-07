//! Census: across HD circuits, which lightmapped materials bind a different
//! albedo entry when the picture must also be one the program samples as a
//! colour (`Program::samples_colour`) than under "first declared picture". Mirrors `skin::NOT_A_PICTURE` and the variant resolution of
//! `hd_sampler_bind`.
//!
//! ```sh
//! cargo run -p oag-render --example hd_lightmap_colour_census -- <image> /data/environments/01_vineta_k/track.vex ...
//! ```

use oag_assets::Container;
use oag_mesh::mesh;
use oag_rcs::{rcsmaterial, rcsmodel};

const NOT_A_PICTURE: &[u32] = &[
    rcsmaterial::LIGHTMAP_SAMPLER,
    0x3528_1c78,
    0x994b_bcf1,
    0x94b2_b285,
    0x739a_786e,
    0x20c3_e476,
    0x48f3_7f5a,
    0xfe9b_d1f3,
    0xeddf_202a,
    0xb1f2_a176,
];

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let image = args.next().unwrap();
    let (mut total, mut changed) = (0usize, 0usize);
    let mut seen = std::collections::BTreeSet::new();
    for name in args {
        let Some((spec, data)) = (0..4).find_map(|n| {
            let spec = format!("{image}:PS3_GAME/USRDIR/DATA0{n}.PSARC");
            mesh::read_blob(&spec, &name).ok().map(|d| (spec, d))
        }) else {
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
            if material.lightmap_entry().is_none() {
                continue;
            }
            total += 1;
            let picture = |(hash, path): &(u32, Option<String>)| {
                path.is_some() && !NOT_A_PICTURE.contains(hash)
            };
            let _plain = material.samplers.iter().position(picture);
            let declared = container
                .read_entry(&format!("/{}", material.name))
                .ok()
                .and_then(|blob| {
                    let parsed = rcsmaterial::RcsMaterial::parse(&blob).ok()?;
                    let word =
                        rcsmaterial::Features::chunk_word(rcsmaterial::LIT_RACE_PASS, Some(*decl));
                    let key = rcsmaterial::Features::from_pass_word(word);
                    let variant = parsed.variant(rcsmaterial::Class::Static, key)?;
                    rcsmaterial::Declared::parse(&blob, variant.fragment.offset)
                });
            let Some(declared) = declared else { continue };
            let old_declared = material
                .samplers
                .iter()
                .position(|e| picture(e) && declared.samplers.iter().any(|&(h, _)| h == e.0));
            let program = container
                .read_entry(&format!("/{}", material.name))
                .ok()
                .and_then(|blob| {
                    let parsed = rcsmaterial::RcsMaterial::parse(&blob).ok()?;
                    let word =
                        rcsmaterial::Features::chunk_word(rcsmaterial::LIT_RACE_PASS, Some(*decl));
                    let key = rcsmaterial::Features::from_pass_word(word);
                    let variant = parsed.variant(rcsmaterial::Class::Static, key)?;
                    rcsmaterial::fragment::Program::parse(&blob, variant.fragment.offset)
                });
            let Some(program) = program else { continue };
            let new = material
                .samplers
                .iter()
                .position(|e| {
                    picture(e)
                        && declared
                            .samplers
                            .iter()
                            .any(|&(h, u)| h == e.0 && program.samples_colour(u as u8))
                })
                .or(old_declared);
            let old = old_declared;
            if new.is_some() && new != old {
                changed += 1;
                let leaf = |i: Option<usize>| {
                    i.and_then(|i| material.samplers[i].1.clone())
                        .unwrap_or_default()
                };
                let line = format!(
                    "{}: {} -> {}",
                    material.name.rsplit('/').next().unwrap_or(""),
                    leaf(old).rsplit('/').next().unwrap_or(""),
                    leaf(new).rsplit('/').next().unwrap_or("")
                );
                if seen.insert(line.clone()) {
                    println!("{name}: {line}");
                }
            }
        }
    }
    println!("lightmapped (resolved decl) materials {total}, albedo entry changed {changed}");
    Ok(())
}
