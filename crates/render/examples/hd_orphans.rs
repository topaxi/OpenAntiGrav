//! Scratch probe: chunks in an HD track's `.rcsmodel` that no `.vex` mesh
//! node addresses, with material names and rough extents from position data.

use oag_formats::rcsmodel;

use oag_render::mesh;
use oag_vex::vex;

fn main() -> anyhow::Result<()> {
    let spec = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "data/images/hdfury-ps3-eu-dec.iso:PS3_GAME/USRDIR/DATA00.PSARC".into());
    let name = std::env::args()
        .nth(2)
        .unwrap_or_else(|| "/data/environments/talons_junction/track.vex".into());
    let data = mesh::read_blob(&spec, &name)?;
    let sibling = mesh::rcs::sibling_name(&name).unwrap();
    let model_blob = mesh::read_blob(&spec, &sibling)?;
    let model = rcsmodel::Model::parse(&model_blob).map_err(|e| anyhow::anyhow!("{e}"))?;
    let classes = vex::classes_of(&data)?;
    let class_id = classes.mesh.unwrap();
    let nodes = vex::nodes(&data)?;
    let order = vex::byte_order(&data);
    let mut node_hashes = std::collections::BTreeSet::new();
    for node in &nodes {
        if node.class_id != class_id {
            continue;
        }
        let payload = &data[node.payload()];
        if payload.len() >= 0x34 {
            node_hashes.insert(order.u32(payload, 0x30));
        }
    }
    println!(
        "{} chunks in the model, {} mesh nodes in the vex",
        model.meshes.len(),
        node_hashes.len()
    );
    let mut orphans = 0;
    for chunk in &model.meshes {
        if node_hashes.contains(&chunk.hash) {
            continue;
        }
        orphans += 1;
        let material = model
            .material_of(chunk)
            .map(|m| m.name.as_str())
            .unwrap_or("?");
        let verts: usize = chunk.submeshes.iter().map(|s| s.vertex_count).sum();
        let (mut min, mut max) = ([f32::MAX; 3], [f32::MIN; 3]);
        for submesh in &chunk.submeshes {
            let Some(stride) = chunk.declared_stride() else {
                continue;
            };
            if let Ok(positions) = chunk.positions(&model_blob, submesh, stride) {
                for p in positions {
                    for i in 0..3 {
                        min[i] = min[i].min(p[i]);
                        max[i] = max[i].max(p[i]);
                    }
                }
            }
        }
        println!(
            "{:#010x}  material {:<40}  verts {:6}  size [{:7.1} {:7.1} {:7.1}]",
            chunk.hash,
            material,
            verts,
            max[0] - min[0],
            max[1] - min[1],
            max[2] - min[2],
        );
    }
    println!("{orphans} orphan chunk(s)");
    Ok(())
}
