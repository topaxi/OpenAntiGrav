//! Scratch probe: list a `.vex`'s `LodGroup` nodes - payload position,
//! authored child count and switch distance, and the `Mesh` nodes under each
//! child with their triangle counts - so a track `LodGroup` can be found on
//! screen and its two tiers compared against the original.
//!
//! ```sh
//! cargo run -q -p oag-vex --example lod_group_probe -- data/images/pulse-psp-usa.chd 16_Track
//! ```

use oag_vex::vex;

fn u32_at(data: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(data[at..at + 4].try_into().expect("four bytes"))
}

fn f32_at(data: &[u8], at: usize) -> f32 {
    f32::from_bits(u32_at(data, at))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let image = args
        .next()
        .ok_or("usage: lod_group_probe IMAGE TRACK_DIR|ENTRY")?;
    let track = args.next().unwrap_or_else(|| "16_Track".to_string());
    let mut archives = oag_pulse::open(&image)?;
    let name = if track.contains('\\') {
        track.clone()
    } else {
        format!(r"Data\Environments\{track}\track.vex")
    };
    let blob = archives.read_name(&name)?;
    let nodes = vex::nodes(&blob)?;
    for (index, node) in nodes.iter().enumerate() {
        if node.class_id != vex::CLASS_LOD_GROUP {
            continue;
        }
        let p = &blob[node.payload()];
        let pos = [f32_at(p, 0x40), f32_at(p, 0x44), f32_at(p, 0x48)];
        let count = u32_at(p, 0x50);
        let switch = (count == 2).then(|| f32_at(p, 0x60));
        println!(
            "LodGroup #{index} {:?} pos {pos:?} child_count {count} switch {switch:?}",
            node.name
        );
        let children: Vec<usize> = (0..nodes.len())
            .filter(|&i| nodes[i].parent == Some(index))
            .collect();
        for (tier, &child) in children.iter().enumerate() {
            let mut tris = 0usize;
            let mut meshes = Vec::new();
            for (i, n) in nodes.iter().enumerate() {
                if n.class_id != vex::CLASS_MESH || !descends_from(&nodes, i, child) {
                    continue;
                }
                let t: usize = vex::mesh_batches(&blob[n.payload()], 0)
                    .map(|b| b.iter().map(|b| b.triangles().len()).sum())
                    .unwrap_or(0);
                tris += t;
                meshes.push(format!("{}:{t}", n.name.as_deref().unwrap_or("?")));
            }
            println!(
                "  tier {tier} {:?}: {tris} list-0 triangles in {}",
                nodes[child].name,
                meshes.join(" ")
            );
        }
    }
    Ok(())
}

fn descends_from(nodes: &[vex::Node], mut i: usize, ancestor: usize) -> bool {
    while let Some(parent) = nodes[i].parent {
        if parent == ancestor {
            return true;
        }
        i = parent;
    }
    false
}
