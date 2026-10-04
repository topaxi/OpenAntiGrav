//! What `build_scene` now binds `_ne` on: PAD_NE vertex count, its glow
//! colours and the bound/unread tally, per circuit.
//! `cargo run -p oag-render --example hd_speed_pad_scene`
use oag_render::mesh::{self, slots};

fn main() -> anyhow::Result<()> {
    let image = "data/images/hdfury-ps3-eu-dec.iso";
    for (archive, circuit) in [
        ("DATA00", "talons_junction"),
        ("DATA00", "amphiseum"),
        ("DATA00", "modesto_heights"),
        ("DATA00", "tech_de_ra"),
        ("DATA02", "12_sol_2"),
        ("DATA02", "15_anulpha_pass"),
        ("DATA02", "01_vineta_k"),
    ] {
        let spec = format!("{image}:PS3_GAME/USRDIR/{archive}.PSARC");
        let name = format!("/data/environments/{circuit}/track.vex");
        let data = mesh::read_blob(&spec, &name)?;
        let blob = mesh::read_blob(&spec, &mesh::rcs::sibling_name(&name).unwrap())?;
        let mut read = |p: &str| mesh::read_blob(&spec, p).ok();
        let (model, report) = mesh::rcs::build_scene(&name, &data, &blob, &mut read)?;
        let flagged = model.vertices.iter().filter(|v| v.slots & slots::PAD_NE != 0).count();
        let mut colours: Vec<[f32; 3]> = model.vertices.iter().filter(|v| v.slots & slots::PAD_NE != 0)
            .filter_map(|v| model.emissive.get((slots::material_index(v.slots) as usize).checked_sub(1)?).map(|e| e.tint)).collect();
        colours.sort_by(|a, b| a.partial_cmp(b).unwrap());
        colours.dedup();
        if let Some(first) = model.vertices.iter().find(|v| v.slots & slots::PAD_NE != 0) {
            let near: Vec<_> = model.vertices.iter().filter(|v| v.slots & slots::PAD_NE != 0
                && (0..3).map(|k| (v.position[k] - first.position[k]).powi(2)).sum::<f32>() < 144.0).collect();
            let n = near.len() as f32;
            let c: Vec<f32> = (0..3).map(|k| near.iter().map(|v| v.position[k]).sum::<f32>() / n).collect();
            let nm: Vec<f32> = (0..3).map(|k| near.iter().map(|v| v.normal[k]).sum::<f32>() / n).collect();
            println!("  first pad centre {:.3},{:.3},{:.3} normal {:.3},{:.3},{:.3}", c[0], c[1], c[2], nm[0], nm[1], nm[2]);
        }
        println!("{circuit}: scene {} vertices, {flagged} PAD_NE, colours {colours:?}, bound {} unread {}", model.vertices.len(), report.pad_ne_bound, report.pad_ne_unread);
    }
    Ok(())
}
