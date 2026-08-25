//! Scratch probe: does a chunk's `+0x07` byte say whether a `.vex` node places
//! it?
//!
//! `oag_render::mesh::rcs::is_world_baked` decides that with a 1.0-unit
//! tolerance this project invented. If the byte answers it outright, the
//! heuristic can go.

use oag_formats::{rcsmodel, vex};
use oag_render::mesh;

fn main() -> anyhow::Result<()> {
    let image = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "data/images/hdfury-ps3-eu-dec.iso".into());
    let mut totals = [[0usize; 2]; 2];
    for archive_index in 0..7 {
        let spec = format!("{image}:PS3_GAME/USRDIR/DATA{archive_index:02}.PSARC");
        let Ok(mut psarc) = oag_assets::psarc::Archive::open(&spec) else {
            continue;
        };
        let vexes: Vec<String> = psarc
            .paths()
            .iter()
            .filter(|p| p.ends_with(".vex"))
            .cloned()
            .collect();
        for path in vexes {
            let Ok(data) = psarc.read_path(&path) else {
                continue;
            };
            let Some(sibling) = mesh::rcs::sibling_name(&path) else {
                continue;
            };
            let Ok(blob) = psarc.read_path(&sibling) else {
                continue;
            };
            let Ok(model) = rcsmodel::Model::parse(&blob) else {
                continue;
            };
            let Ok(classes) = vex::classes_of(&data) else {
                continue;
            };
            let Some(class) = classes.mesh else { continue };
            let Ok(nodes) = vex::nodes(&data) else {
                continue;
            };
            let order = vex::byte_order(&data);
            let mut addressed = std::collections::BTreeSet::new();
            for node in &nodes {
                if node.class_id != class {
                    continue;
                }
                let payload = &data[node.payload()];
                if payload.len() >= 0x34 {
                    addressed.insert(order.u32(payload, 0x30));
                }
            }
            let be32 = |at: usize| -> u32 {
                blob.get(at..at + 4)
                    .map_or(0, |s| u32::from_be_bytes(s.try_into().unwrap()))
            };
            let table = be32(0x20) as usize;
            let mut local = [[0usize; 2]; 2];
            for (index, chunk) in model.meshes.iter().enumerate() {
                let at = be32(table + index * 4) as usize;
                let Some(&kk) = blob.get(at + 0x07) else {
                    continue;
                };
                let row = usize::from(kk == 2);
                let col = usize::from(addressed.contains(&chunk.hash));
                local[row][col] += 1;
                totals[row][col] += 1;
            }
            if local[0][1] + local[1][0] > 0 && local.iter().flatten().sum::<usize>() > 40 {
                println!(
                    "{path}: kk1/unaddressed {} kk1/addressed {} kk2/unaddressed {} kk2/addressed {}",
                    local[0][0], local[0][1], local[1][0], local[1][1],
                );
            }
        }
    }
    println!("\nover every .vex/.rcsmodel pair on the disc:");
    println!("                 no node   a node addresses it");
    println!("  kk = 1  {:>12} {:>18}", totals[0][0], totals[0][1]);
    println!("  kk = 2  {:>12} {:>18}", totals[1][0], totals[1][1]);
    Ok(())
}
