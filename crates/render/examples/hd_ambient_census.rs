//! Scratch probe: which drawn materials actually declare
//! `constantAmbientColour`, against the renderer applying it to all of them.
//!
//! `mesh.wgsl` adds `scene.light.ambient` to every surface. A material's
//! resolved fragment block declares the parameters it is fed, and
//! `rcsmaterial::Declared::takes_constant_ambient` reads that list - so the
//! difference between the two is measurable rather than arguable.

use oag_mesh::mesh;

/// The engine parameter names this probe needs, from `EBOOT.elf`'s own table
/// at `0x008b7f08` - see `docs/ghidra/functions/ps3-hdfury-eu/renderer.md`.
const NAMES: &[(u32, &str)] = &[
    (0x81db_67ea, "constantAmbientColour"),
    (0x02df_31e5, "directionalLight0DirectionWorldSpace"),
    (0x2dba_643d, "directionalLight0Colour"),
    (0x3dc3_1258, "fogColour"),
    (0x370a_63cb, "prelitScale?"),
    (0x81e0_e773, "specPower?"),
    (0x37b5_db58, "lightmap"),
    (0x9edd_3243, "paraboloidReflectionTex"),
];

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let spec = args
        .next()
        .unwrap_or_else(|| "data/images/hdfury-ps3-eu-dec.iso:PS3_GAME/USRDIR/DATA02.PSARC".into());
    let name = args
        .next()
        .unwrap_or_else(|| "/data/environments/15_anulpha_pass/track.vex".into());

    let data = mesh::read_blob(&spec, &name)?;
    let (model, _) = mesh::rcs::scene_from(&spec, &name, &data)?
        .ok_or_else(|| anyhow::anyhow!("{name}: not a PS3 model"))?;
    let geometry = mesh::rcs::sibling_geometry(&spec, &name, &data)
        .ok_or_else(|| anyhow::anyhow!("no sibling"))?;
    let source = oag_rcs::rcsmodel::Model::parse(&geometry)?;

    let mut chunks_of: std::collections::HashMap<u32, usize> = Default::default();
    for mesh in &source.meshes {
        *chunks_of.entry(mesh.material).or_default() += 1;
    }

    let (mut takes, mut not, mut unread) = (0usize, 0usize, 0usize);
    let (mut takes_chunks, mut not_chunks) = (0usize, 0usize);
    let mut examples: Vec<String> = Vec::new();
    // The three-way split the per-material lighting branch would key on:
    // an ambient, a directional light, both, or neither.
    let mut family: std::collections::BTreeMap<&str, (usize, usize, Vec<String>)> =
        Default::default();
    for (slot, variant) in model.material_variants.iter().enumerate() {
        let Some(material) = source.materials.get(slot) else {
            continue;
        };
        let chunks = chunks_of
            .get(&u32::try_from(slot).unwrap_or(u32::MAX))
            .copied()
            .unwrap_or(0);
        if chunks == 0 {
            continue;
        }
        let declared = variant.and_then(|variant| {
            mesh::read_blob(&spec, &format!("/{}", material.name))
                .ok()
                .and_then(|blob| {
                    oag_rcs::rcsmaterial::Declared::parse(&blob, variant.fragment.offset)
                })
        });
        if let Some(d) = declared.as_ref() {
            const SUN_COLOUR: u32 = 0x2dba_643d;
            const SUN_DIRECTION: u32 = 0x02df_31e5;
            let ambient = d.takes_constant_ambient();
            let sun = d
                .parameters
                .iter()
                .any(|h| *h == SUN_COLOUR || *h == SUN_DIRECTION);
            let key = match (ambient, sun) {
                (true, true) => "ambient + sun",
                (true, false) => "ambient only",
                (false, true) => "sun only",
                (false, false) => "neither",
            };
            let row = family.entry(key).or_default();
            row.0 += 1;
            row.1 += chunks;
            let stem = material
                .name
                .rsplit('/')
                .next()
                .unwrap_or_default()
                .to_string();
            if row.2.len() < 10 && !row.2.contains(&stem) {
                row.2.push(stem);
            }
        }
        match declared {
            Some(declared) if declared.takes_constant_ambient() => {
                takes += 1;
                takes_chunks += chunks;
            }
            Some(declared) => {
                not += 1;
                not_chunks += chunks;
                if examples.len() < 6 {
                    let stem = material.name.rsplit('/').next().unwrap_or_default();
                    if !examples.iter().any(|e| e.starts_with(stem)) {
                        let names: Vec<String> = declared
                            .parameters
                            .iter()
                            .map(|h| {
                                NAMES
                                    .iter()
                                    .find(|(hash, _)| hash == h)
                                    .map_or_else(|| format!("{h:#010x}"), |(_, n)| (*n).to_string())
                            })
                            .collect();
                        examples.push(format!("{stem} [{}]", names.join(" ")));
                    }
                }
            }
            None => unread += 1,
        }
    }
    println!(
        "{takes} drawn material(s) declare constantAmbientColour ({takes_chunks} chunk(s)), \
         {not} do not ({not_chunks} chunk(s)), {unread} unread"
    );
    // What actually reaches the shader, as opposed to what the declaration
    // says: the bits `mesh::rcs::skin::roles` packed.
    let mut emissive_slots = 0usize;
    for (slot, packed) in model.material_slots.iter().enumerate() {
        if packed & oag_mesh::mesh::slots::EMISSIVE == oag_mesh::mesh::slots::EMISSIVE {
            emissive_slots += 1;
            if emissive_slots <= 6 {
                println!(
                    "  slot {slot} packs EMISSIVE: {}",
                    source.materials.get(slot).map_or("?", |m| m.name.as_str())
                );
            }
        }
    }
    println!("{emissive_slots} slot(s) pack slots::EMISSIVE");

    println!("\nby what the program is fed:");
    for (key, (slots, chunks, names)) in &family {
        println!(
            "  {key:<14} {slots:>4} material(s) {chunks:>5} chunk(s)  {}",
            names.join(", ")
        );
    }
    println!("\n  not declaring an ambient:");
    for line in &examples {
        println!("    {line}");
    }
    Ok(())
}
