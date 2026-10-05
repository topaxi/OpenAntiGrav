//! Which **sampler entry** of a material the shader's colour and alpha lanes
//! actually come from, once each entry's own name hash is resolved to a
//! texture unit through the resolved variant's declaration.
//!
//! The measurement that says what binding more than two textures would buy:
//! an entry index of 2 or more is a texture `oag-render` cannot bind today.

use oag_assets::Container;
use oag_mesh::mesh;
use oag_rcs::{rcsmaterial, rcsmodel};

const ARCHIVES: &[&str] = &[
    "PS3_GAME/USRDIR/DATA00.PSARC",
    "PS3_GAME/USRDIR/DATA01.PSARC",
    "PS3_GAME/USRDIR/DATA02.PSARC",
    "PS3_GAME/USRDIR/DATA03.PSARC",
];

fn main() -> anyhow::Result<()> {
    let image = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "data/images/hdfury-ps3-eu-dec.iso".into());
    let circuits: Vec<String> = std::env::args().skip(2).collect();
    let circuits = if circuits.is_empty() {
        vec!["/data/environments/talons_junction/track.vex".to_string()]
    } else {
        circuits
    };

    let mut colour: std::collections::BTreeMap<String, usize> = Default::default();
    let mut alpha: std::collections::BTreeMap<String, usize> = Default::default();
    let mut chunks_beyond = 0usize;
    let mut lightmap_first = 0usize;

    for name in &circuits {
        let Some((spec, data)) = ARCHIVES.iter().find_map(|archive| {
            let spec = format!("{image}:{archive}");
            mesh::read_blob(&spec, name).ok().map(|d| (spec, d))
        }) else {
            continue;
        };
        let Some(geometry) = mesh::rcs::sibling_geometry(&spec, name, &data) else {
            continue;
        };
        let model = rcsmodel::Model::parse(&geometry)?;
        let mut container = Container::open(&spec)?;
        let mut decl_of: std::collections::HashMap<u32, Option<&rcsmodel::VertexDecl>> =
            Default::default();
        let mut chunks_of: std::collections::HashMap<u32, usize> = Default::default();
        for mesh in &model.meshes {
            decl_of.entry(mesh.material).or_insert(mesh.decl.as_ref());
            *chunks_of.entry(mesh.material).or_default() += 1;
        }
        let mut cache: std::collections::HashMap<String, Option<Vec<u8>>> = Default::default();
        for (slot, material) in model.materials.iter().enumerate() {
            let ordinal = u32::try_from(slot).unwrap_or(u32::MAX);
            let Some(decl) = decl_of.get(&ordinal) else {
                continue;
            };
            if material
                .samplers
                .first()
                .is_some_and(|&(h, _)| h == rcsmaterial::LIGHTMAP_SAMPLER)
            {
                lightmap_first += 1;
            }
            let blob = cache
                .entry(material.name.clone())
                .or_insert_with(|| container.read_entry(&format!("/{}", material.name)).ok())
                .clone();
            let Some(blob) = blob else { continue };
            let Some(parsed) = rcsmaterial::RcsMaterial::parse(&blob).ok() else {
                continue;
            };
            let word = rcsmaterial::Features::chunk_word(rcsmaterial::LIT_RACE_PASS, *decl);
            let key = rcsmaterial::Features::from_pass_word(word);
            let Some(variant) = parsed.variant(rcsmaterial::Class::Static, key) else {
                continue;
            };
            let Some(declared) = rcsmaterial::Declared::parse(&blob, variant.fragment.offset)
            else {
                continue;
            };
            let Some(program) =
                rcsmaterial::fragment::Program::parse(&blob, variant.fragment.offset)
            else {
                continue;
            };
            // entry index -> unit, for the entries that supply a texture
            let entry_unit = |index: usize| -> Option<u32> {
                let (hash, path) = material.samplers.get(index)?;
                path.as_ref()?;
                declared
                    .samplers
                    .iter()
                    .find(|(h, _)| h == hash)
                    .map(|&(_, unit)| unit)
            };
            let index_of = |unit: u32| -> Option<usize> {
                (0..material.samplers.len()).find(|&i| entry_unit(i) == Some(unit))
            };
            let texels = program.output_texels();
            let merged = texels[0].merge(texels[1]).merge(texels[2]);
            let describe = |unit: Option<u8>| match unit.and_then(|u| index_of(u32::from(u))) {
                Some(i) if i < 2 => format!("entry {i} (bindable today)"),
                Some(i) => format!("entry {i} (NOT bindable)"),
                None => "no entry supplies that unit".to_string(),
            };
            let c = describe(merged.unit());
            if c.contains("NOT") {
                chunks_beyond += chunks_of.get(&ordinal).copied().unwrap_or(0);
            }
            *colour.entry(c).or_default() += 1;
            *alpha
                .entry(describe(match texels[3] {
                    rcsmaterial::fragment::Texel::Unit { unit, .. } => Some(unit),
                    _ => None,
                }))
                .or_default() += 1;
        }
    }
    println!("colour lane:");
    for (k, v) in &colour {
        println!("  {v:5}  {k}");
    }
    println!("alpha lane:");
    for (k, v) in &alpha {
        println!("  {v:5}  {k}");
    }
    println!("chunks whose colour needs an unbindable entry: {chunks_beyond}");
    println!("materials whose FIRST entry is the lightmap: {lightmap_first}");
    Ok(())
}
