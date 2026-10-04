//! Node `+0x30` hashes against the chunk hashes of the speed-pad material.
use oag_rcs::rcsmodel;
use oag_render::mesh;
use oag_vex::vex;
fn main() -> anyhow::Result<()> {
    let spec = "data/images/hdfury-ps3-eu-dec.iso:PS3_GAME/USRDIR/DATA00.PSARC";
    let circuit = std::env::args().nth(1).unwrap_or("talons_junction".into());
    let name = format!("/data/environments/{circuit}/track.vex");
    let data = mesh::read_blob(spec, &name)?;
    let blob = mesh::read_blob(spec, &mesh::rcs::sibling_name(&name).unwrap())?;
    let model = rcsmodel::Model::parse(&blob).map_err(|e| anyhow::anyhow!("{e}"))?;
    let classes = vex::classes_of(&data)?;
    let nodes = vex::nodes(&data)?;
    let order = vex::byte_order(&data);
    for nd in nodes.iter().filter(|n| Some(n.class_id) == classes.speedup_pad) {
        let p = &data[nd.payload()];
        let h = order.u32(p, 0x30);
        println!("node {:?} {h:#010x} words {:08x?}", nd.name, (0x20..0x44).step_by(4).map(|o| order.u32(p, o)).collect::<Vec<_>>());
    }
    for (i, c) in model.meshes.iter().enumerate() {
        let m = &model.materials[c.material as usize];
        if m.samplers.iter().any(|(_, p)| p.as_deref().is_some_and(|p| p.contains("speedup"))) {
            println!("chunk #{i} {:#010x} slot {} verts {:?} bias {:?}", c.hash, c.material, c.submeshes.len(), c.bias);
        }
    }
    Ok(())
}
