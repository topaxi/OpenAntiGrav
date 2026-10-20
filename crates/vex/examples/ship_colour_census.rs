//! Scratch probe: vertex colour and normal census of a ship's hull batches.
//!
//! ```sh
//! cargo run -q -p oag-vex --example ship_colour_census -- data/images/pulse-ps2-eu.chd Assegai
//! ```
use oag_vex::vex;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let image = args.next().ok_or("usage: IMAGE TEAM")?;
    let team = args.next().unwrap_or_else(|| "Assegai".into());
    let only: Option<usize> = args.next().and_then(|a| a.parse().ok());
    let mut archives = oag_pulse::open(&image)?;
    let blob = archives.read_name(&format!(r"Data\Ships\{team}\Ship.vex"))?;
    let nodes = vex::nodes(&blob)?;
    let classes = vex::classes_of(&blob)?;
    for (i, node) in nodes.iter().enumerate() {
        if Some(node.class_id) != classes.mesh {
            continue;
        }
        let payload = &blob[node.payload()];
        for list in 0..2u8 {
            let Ok(batches) = vex::mesh_batches(payload, list) else {
                continue;
            };
            for (b, batch) in batches.iter().enumerate() {
                let n = batch.vertices.len();
                if only.is_some_and(|w| n != w && n + 2 != w) {
                    continue;
                }
                let with_c = batch.vertices.iter().filter(|v| v.colour.is_some()).count();
                let with_n = batch.vertices.iter().filter(|v| v.normal.is_some()).count();
                let (mut lo, mut hi) = ([255u8; 3], [0u8; 3]);
                let mut sum = [0u32; 3];
                for c in batch.vertices.iter().filter_map(|v| v.colour) {
                    for k in 0..3 {
                        lo[k] = lo[k].min(c[k]);
                        hi[k] = hi[k].max(c[k]);
                        sum[k] += u32::from(c[k]);
                    }
                }
                let avg = sum.map(|s| s / with_c.max(1) as u32);
                let (mut lo3, mut hi3) = ([f32::MAX; 3], [f32::MIN; 3]);
                for v in &batch.vertices {
                    for k in 0..3 {
                        lo3[k] = lo3[k].min(v.position[k]);
                        hi3[k] = hi3[k].max(v.position[k]);
                    }
                }
                println!("bbox {lo3:?} {hi3:?}");
                println!(
                    "node {i} {:?} list {list} batch {b} vtype {:#x} verts {n} colour {with_c} normal {with_n} rgb lo {lo:?} hi {hi:?} avg {avg:?} pass {:#x}",
                    node.name, batch.vertex_type, batch.pass_mask
                );
            }
        }
    }
    Ok(())
}
