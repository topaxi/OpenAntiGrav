//! Scratch probe: for each HD weapon-pad node, the node's world translation
//! against its chunk's own position extent - which answers whether the chunk
//! is authored in node space (extent near the origin, transform needed) or
//! baked in world space (extent already at the node's position).

use oag_rcs::rcsmodel;

use oag_mesh::mesh;
use oag_vex::vex;

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
    let classes = vex::classes_of(&data)?;
    let pad_class = if std::env::args().nth(3).as_deref() == Some("speedup") {
        classes.speedup_pad.unwrap()
    } else {
        classes.weapon_pad.unwrap()
    };
    let nodes = vex::nodes(&data)?;
    let world = vex::world_transforms(&data, &nodes);
    let order = vex::byte_order(&data);
    for (index, node) in nodes.iter().enumerate() {
        if node.class_id != pad_class {
            continue;
        }
        let payload = &data[node.payload()];
        if payload.len() < 0x34 {
            println!(
                "{:?}: payload only {} bytes",
                node.name.as_deref().unwrap_or("?"),
                payload.len()
            );
            continue;
        }
        let hash = order.u32(payload, 0x30);
        let m = &world[index];
        let Some(chunk) = model.mesh(hash) else {
            println!(
                "{:?}: {hash:#010x} not in the model",
                node.name.as_deref().unwrap_or("?")
            );
            continue;
        };
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
            "{:<44}  node at [{:8.1} {:8.1} {:8.1}]  chunk centre [{:8.1} {:8.1} {:8.1}]  size [{:5.1} {:5.1} {:5.1}]",
            node.name.as_deref().unwrap_or("?"),
            m[12],
            m[13],
            m[14],
            (min[0] + max[0]) / 2.0,
            (min[1] + max[1]) / 2.0,
            (min[2] + max[2]) / 2.0,
            max[0] - min[0],
            max[1] - min[1],
            max[2] - min[2],
        );
    }
    Ok(())
}
