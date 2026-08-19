//! Scratch probe: every place a `.vex` node payload mentions a chunk hash of
//! the sibling `.rcsmodel` - which node class, which offset, and whether the
//! mesh pass would draw it (mesh node, offset 0x30) or nothing does.

use oag_formats::{rcsmodel, vex};
use oag_render::mesh;

fn main() -> anyhow::Result<()> {
    let spec = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "data/images/hdfury-ps3-eu-dec.iso:PS3_GAME/USRDIR/DATA00.PSARC".into());
    let name = std::env::args()
        .nth(2)
        .unwrap_or_else(|| "/data/environments/talons_junction/track.vex".into());
    let data = mesh::read_blob(&spec, &name)?;
    let model_blob = mesh::read_blob(&spec, &mesh::rcs::sibling_name(&name).unwrap())?;
    let model = rcsmodel::Model::parse(&model_blob).map_err(|e| anyhow::anyhow!("{e}"))?;
    let known: std::collections::HashMap<u32, usize> = model
        .meshes
        .iter()
        .enumerate()
        .map(|(i, m)| (m.hash, i))
        .collect();
    let classes = vex::classes_of(&data)?;
    let mesh_class = classes.mesh.unwrap();
    let nodes = vex::nodes(&data)?;
    let order = vex::byte_order(&data);
    let mut drawn = std::collections::BTreeSet::new();
    let mut mentioned: std::collections::BTreeMap<u32, Vec<String>> = Default::default();
    for node in &nodes {
        let payload = &data[node.payload()];
        for at in (0..payload.len().saturating_sub(3)).step_by(4) {
            let word = order.u32(payload, at);
            if !known.contains_key(&word) {
                continue;
            }
            let draws = node.class_id == mesh_class && at == 0x30;
            if draws {
                drawn.insert(word);
            }
            mentioned.entry(word).or_default().push(format!(
                "class {:#06x} at +{:#04x} node {:?}{}",
                node.class_id,
                at,
                node.name.as_deref().unwrap_or("?"),
                if draws { " [drawn]" } else { "" },
            ));
        }
    }
    let mut lost = 0;
    for (hash, sites) in &mentioned {
        if drawn.contains(hash) {
            continue;
        }
        lost += 1;
        let chunk = &model.meshes[known[hash]];
        let material = model
            .material_of(chunk)
            .map(|m| m.name.as_str())
            .unwrap_or("?");
        println!("{hash:#010x}  {material}");
        for site in sites {
            println!("    {site}");
        }
    }
    println!(
        "{} chunk hashes mentioned, {} drawn by the mesh pass, {lost} mentioned-but-not-drawn",
        mentioned.len(),
        drawn.len(),
    );
    Ok(())
}
