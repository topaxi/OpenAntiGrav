//! Which speed-pad node owns which chunk, on circuits whose nodes name no
//! chunk: each `Speedup Pad` node's world volume against every pad-material
//! chunk's decoded vertex extent (a chunk is world-baked), by the distance
//! from the node's box to the chunk's centroid, and whether the nearest-chunk
//! assignment is one to one.
//!
//! `cargo run -p oag-render --example hd_speed_pad_owner -- <circuit> [archive] [weapon]`
use oag_mesh::mesh;
use oag_rcs::rcsmodel;
use oag_vex::{pads, vex};

fn main() -> anyhow::Result<()> {
    let circuit = std::env::args().nth(1).unwrap_or("talons_junction".into());
    let archive = std::env::args().nth(2).unwrap_or("DATA00".into());
    let spec = format!("data/images/hdfury-ps3-eu-dec.iso:PS3_GAME/USRDIR/{archive}.PSARC");
    let name = format!("/data/environments/{circuit}/track.vex");
    let data = mesh::read_blob(&spec, &name)?;
    let blob = mesh::read_blob(&spec, &mesh::rcs::sibling_name(&name).unwrap())?;
    let model = rcsmodel::Model::parse(&blob).map_err(|e| anyhow::anyhow!("{e}"))?;
    let classes = vex::classes_of(&data)?;
    let nodes = vex::nodes(&data)?;
    let weapon = std::env::args().nth(3).as_deref() == Some("weapon");
    let class = if weapon {
        classes.weapon_pad
    } else {
        classes.speedup_pad
    }
    .expect("pad class");
    let named_all: Vec<u32> = nodes
        .iter()
        .filter(|n| {
            Some(n.class_id) == classes.speedup_pad || Some(n.class_id) == classes.weapon_pad
        })
        .map(|n| vex::byte_order(&data).u32(&data[n.payload()], 0x30))
        .collect();
    let volumes = pads::volumes(&data, &nodes, class);
    let names: Vec<_> = nodes
        .iter()
        .filter(|n| n.class_id == class)
        .map(|n| n.name.clone().unwrap_or_default())
        .collect();
    let order = vex::byte_order(&data);
    let named: Vec<u32> = nodes
        .iter()
        .filter(|n| n.class_id == class)
        .map(|n| order.u32(&data[n.payload()], 0x30))
        .collect();
    println!(
        "{circuit}: {} nodes, {} resolve to a chunk",
        volumes.len(),
        named
            .iter()
            .filter(|h| model.mesh_index(**h).is_some())
            .count()
    );
    let mut chunks = Vec::new();
    for (i, c) in model.meshes.iter().enumerate() {
        if named_all.contains(&c.hash) {
            continue;
        }
        let Some(m) = model.materials.get(c.material as usize) else {
            continue;
        };
        if !m.name.contains("weapon_pads") && !m.name.contains("speedup_material") {
            continue;
        }
        let Some(stride) = c
            .declared_stride()
            .or_else(|| c.solve_stride_without_a_box(&blob))
        else {
            println!("chunk #{i}: no stride");
            continue;
        };
        let mut pts = Vec::new();
        for s in &c.submeshes {
            pts.extend(c.positions(&blob, s, stride).unwrap_or_default());
        }
        let n = pts.len() as f32;
        let centre: [f32; 3] = std::array::from_fn(|k| pts.iter().map(|p| p[k]).sum::<f32>() / n);
        chunks.push((i, c.hash, centre));
    }
    println!("{} unnamed pad-material chunks", chunks.len());
    let mut taken = vec![0usize; chunks.len()];
    for (v, nm) in volumes.iter().zip(&names) {
        let mut best: Vec<(f32, usize)> = chunks
            .iter()
            .enumerate()
            .map(|(k, (_, _, c))| (v.distance(*c), k))
            .collect();
        best.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        taken[best[0].1] += 1;
        println!(
            "{nm}: centre {:?} nearest chunk #{} dist {:.2}, next {:.2}",
            v.centre().map(|x| (x * 10.0).round() / 10.0),
            chunks[best[0].1].0,
            best[0].0,
            best[1].0
        );
    }
    println!(
        "chunks claimed twice: {}, claimed none: {}",
        taken.iter().filter(|n| **n > 1).count(),
        taken.iter().filter(|n| **n == 0).count()
    );
    Ok(())
}
