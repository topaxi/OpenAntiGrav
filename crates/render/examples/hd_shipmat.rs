//! Scratch probe: which materials a model's chunks name, and what each chunk
//! declares - the question being what lights an HD ship.
use oag_formats::rcsmodel;
use oag_render::mesh;
fn main() -> anyhow::Result<()> {
    let spec = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "data/images/hdfury-ps3-eu-dec.iso:PS3_GAME/USRDIR/DATA00.PSARC".into());
    let name = std::env::args()
        .nth(2)
        .unwrap_or_else(|| "/data/ships/detonator/ship.rcsmodel".into());
    let blob = mesh::read_blob(&spec, &name)?;
    let model = rcsmodel::Model::parse(&blob).map_err(|e| anyhow::anyhow!("{e}"))?;
    println!(
        "{} material(s), {} mesh(es)",
        model.materials.len(),
        model.meshes.len()
    );
    for (i, m) in model.materials.iter().enumerate() {
        println!("  [{i}] {}", m.name);
    }
    let mut colour = 0usize;
    let mut lmap = 0usize;
    for chunk in &model.meshes {
        let Some(decl) = chunk.decl.as_ref() else {
            continue;
        };
        if decl.vertex_colour().is_some() {
            colour += 1
        }
        if decl.lightmap_texcoord().is_some() {
            lmap += 1
        }
    }
    println!("chunks: {colour} with a colour set, {lmap} with a lightmapUV");
    Ok(())
}
