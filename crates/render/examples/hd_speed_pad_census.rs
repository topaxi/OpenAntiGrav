//! Where the four original HD circuits author their speed pads: every
//! material in the model whose sampler paths name a speed-up or `_ne` file,
//! the chunks drawn with it, and every node whose class name says pad.
//! `cargo run -p oag-render --example hd_speed_pad_census`

use oag_rcs::rcsmodel;
use oag_render::mesh;
use oag_vex::vex;

fn main() -> anyhow::Result<()> {
    let image = "data/images/hdfury-ps3-eu-dec.iso";
    for circuit in ["talons_junction", "amphiseum", "modesto_heights", "tech_de_ra"] {
        let spec = format!("{image}:PS3_GAME/USRDIR/DATA00.PSARC");
        let name = format!("/data/environments/{circuit}/track.vex");
        let data = mesh::read_blob(&spec, &name)?;
        let blob = mesh::read_blob(&spec, &mesh::rcs::sibling_name(&name).unwrap())?;
        let model = rcsmodel::Model::parse(&blob).map_err(|e| anyhow::anyhow!("{e}"))?;
        println!("== {circuit}: {} materials, {} chunks", model.materials.len(), model.meshes.len());
        let classes = vex::classes_of(&data)?;
        let nodes = vex::nodes(&data)?;
        let order = vex::byte_order(&data);
        for (label, id) in [("Speedup Pad", classes.speedup_pad), ("Weapon Pad", classes.weapon_pad)] {
            let n: Vec<_> = nodes.iter().filter(|n| Some(n.class_id) == id).collect();
            let resolved = n.iter().filter(|nd| {
                let p = &data[nd.payload()];
                p.len() >= 0x34 && model.mesh(order.u32(p, 0x30)).is_some()
            }).count();
            println!("  {label}: {} nodes, {resolved} address a chunk", n.len());
            for nd in n.iter().take(3) {
                let p = &data[nd.payload()];
                if p.len() >= 0x34 { println!("      {:?} len {} hash@30 {:#010x}", nd.name, p.len(), order.u32(p, 0x30)); }
            }
        }
        for (slot, m) in model.materials.iter().enumerate() {
            let hit = m.samplers.iter().any(|(_, p)| p.as_deref().is_some_and(|p| {
                let l = p.to_lowercase(); l.contains("speed") || l.contains("_ne") || l.contains("pad")
            })) || m.name.contains("pad") || m.name.contains("speed");
            if !hit { continue; }
            let users: Vec<_> = model.meshes.iter().filter(|c| c.surfaces().any(|s| s.material as usize == slot)).map(|c| c.hash).collect();
            let tex = m.samplers.iter().filter_map(|(_, p)| p.as_deref()).map(|p| p.rsplit('/').next().unwrap_or(p)).collect::<Vec<_>>().join(",");
            println!("  slot {slot}: {} chunks {} [{tex}]", m.name.rsplit('/').next().unwrap(), users.len());
        }
    }
    Ok(())
}
