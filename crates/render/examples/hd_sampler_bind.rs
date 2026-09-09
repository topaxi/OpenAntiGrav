//! Sweeps every HD circuit and asks whether a `.rcsmodel` material's
//! per-texture **sampler name hash** (`Material::texture_sampler`) is declared
//! by the shader variant that material resolves to, and at which unit.
//!
//! The direct test of "a texture slot's ordinal is not its texture unit". Built
//! through `mesh::rcs::scene_from`'s own variant resolution, so what it counts
//! is what the renderer would bind.

use oag_assets::Container;
use oag_formats::{rcsmaterial, rcsmodel};
use oag_render::mesh;

/// Where each circuit's `track.vex` lives, archive by archive.
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

    // `OAG_RAMPS=1`: what each sampler hash actually binds, with the texture's
    // own dimensions, so a "ramp addressed by a scalar" can be told from a
    // picture addressed by a UV.
    if std::env::var("OAG_RAMPS").is_ok() {
        let mut by_hash: std::collections::BTreeMap<u32, Vec<(String, u32, u32)>> =
            Default::default();
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
            let mut size: std::collections::HashMap<String, Option<(u32, u32)>> =
                Default::default();
            for material in &model.materials {
                for (hash, path) in &material.samplers {
                    let Some(path) = path else { continue };
                    let wh = size
                        .entry(path.clone())
                        .or_insert_with(|| {
                            let blob = container.read_entry(path).ok()?;
                            let parsed = oag_texture::gtf::Gtf::parse(&blob).ok()?;
                            parsed.only().map(|t| t.level_size(0))
                        })
                        .to_owned();
                    let Some((w, h)) = wh else { continue };
                    let leaf = path.rsplit('/').next().unwrap_or(path).to_string();
                    by_hash.entry(*hash).or_default().push((leaf, w, h));
                }
            }
        }
        for (hash, uses) in &by_hash {
            let ramps = uses.iter().filter(|&&(_, w, h)| h <= 32 || w <= 32).count();
            let mut leaves: Vec<&str> = uses.iter().map(|(l, _, _)| l.as_str()).collect();
            leaves.sort_unstable();
            leaves.dedup();
            println!(
                "{hash:#010x}  {:4} use(s), {ramps} of them <=32 in a dimension; {} distinct file(s): {}",
                uses.len(),
                leaves.len(),
                leaves
                    .iter()
                    .take(4)
                    .copied()
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
        return Ok(());
    }

    // How the slot's own hash relates to the unit the variant gives it.
    let (mut at_unit, mut undeclared, mut no_variant, mut ordinal_agrees) = (0, 0, 0, 0);
    let mut units: std::collections::BTreeMap<(usize, u32), usize> = Default::default();

    for name in &circuits {
        let Some((spec, data)) = ARCHIVES.iter().find_map(|archive| {
            let spec = format!("{image}:{archive}");
            mesh::read_blob(&spec, name).ok().map(|d| (spec, d))
        }) else {
            println!("{name}: in none of the archives swept");
            continue;
        };
        let Some(geometry) = mesh::rcs::sibling_geometry(&spec, name, &data) else {
            continue;
        };
        let model = rcsmodel::Model::parse(&geometry)?;
        let mut container = Container::open(&spec)?;
        let mut cache: std::collections::HashMap<String, Option<Vec<u8>>> = Default::default();
        let mut decl_of: std::collections::HashMap<u32, Option<&rcsmodel::VertexDecl>> =
            Default::default();
        for mesh in &model.meshes {
            decl_of.entry(mesh.material).or_insert(mesh.decl.as_ref());
        }
        for (slot, material) in model.materials.iter().enumerate() {
            let ordinal = u32::try_from(slot).unwrap_or(u32::MAX);
            let Some(decl) = decl_of.get(&ordinal) else {
                continue;
            };
            let blob = cache
                .entry(material.name.clone())
                .or_insert_with(|| container.read_entry(&format!("/{}", material.name)).ok())
                .clone();
            let resolved = blob.as_deref().and_then(|blob| {
                let parsed = rcsmaterial::RcsMaterial::parse(blob).ok()?;
                let word = rcsmaterial::Features::chunk_word(rcsmaterial::LIT_RACE_PASS, *decl);
                let key = rcsmaterial::Features::from_pass_word(word);
                let variant = parsed.variant(rcsmaterial::Class::Static, key)?;
                let declared = rcsmaterial::Declared::parse(blob, variant.fragment.offset)?;
                Some(declared)
            });
            let Some(declared) = resolved else {
                no_variant += 1;
                continue;
            };
            for (which, hash) in [
                (
                    0usize,
                    (!material.texture.is_empty()).then_some(material.texture_sampler),
                ),
                (1usize, material.second_texture_sampler),
            ] {
                let Some(hash) = hash else { continue };
                match declared.samplers.iter().find(|(h, _)| *h == hash) {
                    Some(&(_, unit)) => {
                        at_unit += 1;
                        ordinal_agrees += usize::from(unit as usize == which);
                        *units.entry((which, unit)).or_default() += 1;
                    }
                    None => undeclared += 1,
                }
            }
        }
        println!("{name}: swept");
    }
    println!(
        "slot hashes the resolved variant declares: {at_unit} \
         (of which {ordinal_agrees} land on the slot's own ordinal), \
         {undeclared} declared by no sampler of that variant, \
         {no_variant} slot(s) with no resolved variant"
    );
    for ((which, unit), n) in &units {
        println!("  texture slot {which} -> unit {unit}: {n}");
    }
    Ok(())
}
