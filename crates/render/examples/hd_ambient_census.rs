//! Scratch probe: which drawn materials actually declare
//! `constantAmbientColour`, against the renderer applying it to all of them.
//!
//! `mesh.wgsl` adds `scene.light.ambient` to every surface. A material's
//! resolved fragment block declares the parameters it is fed, and
//! `rcsmaterial::Declared::takes_constant_ambient` reads that list - so the
//! difference between the two is measurable rather than arguable.

use oag_render::mesh;

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
    let source = oag_formats::rcsmodel::Model::parse(&geometry)?;

    let mut chunks_of: std::collections::HashMap<u32, usize> = Default::default();
    for mesh in &source.meshes {
        *chunks_of.entry(mesh.material).or_default() += 1;
    }

    let (mut takes, mut not, mut unread) = (0usize, 0usize, 0usize);
    let (mut takes_chunks, mut not_chunks) = (0usize, 0usize);
    let mut examples: Vec<String> = Vec::new();
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
                    oag_formats::rcsmaterial::Declared::parse(&blob, variant.fragment.offset)
                })
        });
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
    println!("  not declaring it:");
    for line in &examples {
        println!("    {line}");
    }
    Ok(())
}
