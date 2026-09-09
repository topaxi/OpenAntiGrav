//! Scratch probe: list the mesh nodes of an HD track whose chunk hash the
//! `.rcsmodel` beside it does not carry, with names and world-space boxes.

use oag_rcs::rcsmodel;

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
    let world = vex::world_transforms(&data, &nodes);
    let order = vex::byte_order(&data);
    for (index, node) in nodes.iter().enumerate() {
        if node.class_id != class_id {
            continue;
        }
        let payload = &data[node.payload()];
        if payload.len() < 0x34 {
            continue;
        }
        let hash = order.u32(payload, 0x30);
        let min: [f32; 3] = std::array::from_fn(|i| order.f32(payload, 0x10 + i * 4));
        let max: [f32; 3] = std::array::from_fn(|i| order.f32(payload, 0x20 + i * 4));
        let found = model.mesh(hash).is_some();
        if found {
            continue;
        }
        let m = &world[index];
        let centre = [
            (min[0] + max[0]) / 2.0,
            (min[1] + max[1]) / 2.0,
            (min[2] + max[2]) / 2.0,
        ];
        let world_centre = [
            centre[0] * m[0] + centre[1] * m[4] + centre[2] * m[8] + m[12],
            centre[0] * m[1] + centre[1] * m[5] + centre[2] * m[9] + m[13],
            centre[0] * m[2] + centre[1] * m[6] + centre[2] * m[10] + m[14],
        ];
        let size = [max[0] - min[0], max[1] - min[1], max[2] - min[2]];
        println!(
            "{hash:#010x}  {:<28}  at [{:8.1} {:8.1} {:8.1}]  size [{:7.1} {:7.1} {:7.1}]",
            node.name.as_deref().unwrap_or("?"),
            world_centre[0],
            world_centre[1],
            world_centre[2],
            size[0],
            size[1],
            size[2],
        );
    }
    Ok(())
}
