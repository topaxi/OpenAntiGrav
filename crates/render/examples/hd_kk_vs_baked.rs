//! Scratch probe: the `+0x07` byte against `rcs::is_world_baked`'s answer.
//!
//! The heuristic is this project's own - a node's authored box carried through
//! its transform, within 1.0 world unit of the chunk's bias. If the byte says
//! the same thing, the invented tolerance can go.

use oag_core::math::{Mat4, Vec3};
use oag_formats::{rcsmodel, vex};
use oag_render::mesh;

const TOLERANCE: f32 = 1.0;

fn main() -> anyhow::Result<()> {
    let image = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "data/images/hdfury-ps3-eu-dec.iso".into());
    // [byte says node-local][heuristic says node-local]
    let mut agree = [[0usize; 2]; 2];
    let mut by_bias = [[0usize; 2]; 2];
    let mut disagreed: Vec<(f32, u8, bool, String)> = Vec::new();
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
            let world = vex::world_transforms(&data, &nodes);
            let order = vex::byte_order(&data);
            let be32 = |at: usize| -> u32 {
                blob.get(at..at + 4)
                    .map_or(0, |s| u32::from_be_bytes(s.try_into().unwrap()))
            };
            let table = be32(0x20) as usize;
            let mut local = [[0usize; 2]; 2];
            for (index, node) in nodes.iter().enumerate() {
                if node.class_id != class {
                    continue;
                }
                let payload = &data[node.payload()];
                if payload.len() < 0x34 {
                    continue;
                }
                let read3 = |at: usize| -> [f32; 3] {
                    std::array::from_fn(|i| order.f32(payload, at + i * 4))
                };
                let (hash, min, max) = (order.u32(payload, 0x30), read3(0x10), read3(0x20));
                let (Some(chunk_index), Some(chunk)) = (model.mesh_index(hash), model.mesh(hash))
                else {
                    continue;
                };
                let at = be32(table + chunk_index * 4) as usize;
                let Some(&kk) = blob.get(at + 0x07) else {
                    continue;
                };
                let centre = Vec3::new(
                    (min[0] + max[0]) / 2.0,
                    (min[1] + max[1]) / 2.0,
                    (min[2] + max[2]) / 2.0,
                );
                let to_world = Mat4::from_cols_array(&world[index]);
                let baked = to_world
                    .transform_point3(centre)
                    .distance(Vec3::from_array(chunk.bias))
                    < TOLERANCE;
                // The bias is the third opinion, and an independent one: a
                // chunk baked in world space carries a world position there,
                // one authored in its own space carries something near zero.
                let bias = Vec3::from_array(chunk.bias).length();
                local[usize::from(kk == 2)][usize::from(!baked)] += 1;
                by_bias[usize::from(kk == 2)][usize::from(bias < 50.0)] += 1;
                if (kk == 2) != !baked {
                    disagreed.push((bias, kk, baked, path.clone()));
                }
            }
            for r in 0..2 {
                for c in 0..2 {
                    agree[r][c] += local[r][c];
                }
            }
            let wrong = local[0][1] + local[1][0];
            if wrong > 0 && local.iter().flatten().sum::<usize>() > 40 {
                println!(
                    "{path}: byte-baked/heuristic-baked {} byte-baked/heuristic-local {} byte-local/heuristic-baked {} byte-local/heuristic-local {}",
                    local[0][0], local[0][1], local[1][0], local[1][1],
                );
            }
        }
    }
    println!("\nevery Mesh node on the disc whose chunk is in its own model:");
    println!("                    heuristic: world-baked   node-local");
    println!("  byte says baked  {:>16} {:>14}", agree[0][0], agree[0][1]);
    println!("  byte says local  {:>16} {:>14}", agree[1][0], agree[1][1]);
    let total: usize = agree.iter().flatten().sum();
    println!("\nthe byte against the bias, which is independent of both:");
    println!("                    bias is a world position   bias is near zero");
    println!(
        "  byte says baked  {:>16} {:>22}",
        by_bias[0][0], by_bias[0][1]
    );
    println!(
        "  byte says local  {:>16} {:>22}",
        by_bias[1][0], by_bias[1][1]
    );
    let t: usize = by_bias.iter().flatten().sum();
    println!(
        "  the byte and the bias agree on {} of {} ({:.2}%)",
        by_bias[0][0] + by_bias[1][1],
        t,
        (by_bias[0][0] + by_bias[1][1]) as f64 / t as f64 * 100.0,
    );
    let mut baked_wrong = disagreed
        .iter()
        .filter(|d| d.1 == 2)
        .map(|d| d.0)
        .collect::<Vec<_>>();
    let mut local_wrong = disagreed
        .iter()
        .filter(|d| d.1 == 1)
        .map(|d| d.0)
        .collect::<Vec<_>>();
    baked_wrong.sort_by(|a, b| a.partial_cmp(b).unwrap());
    local_wrong.sort_by(|a, b| a.partial_cmp(b).unwrap());
    for (label, v) in [
        ("byte local, heuristic baked", &baked_wrong),
        ("byte baked, heuristic local", &local_wrong),
    ] {
        if v.is_empty() {
            continue;
        }
        println!(
            "  {label}: {} case(s), |bias| median {:.1}, {} under 50 units",
            v.len(),
            v[v.len() / 2],
            v.iter().filter(|b| **b < 50.0).count(),
        );
    }
    println!(
        "  they agree on {} of {} ({:.2}%)",
        agree[0][0] + agree[1][1],
        total,
        (agree[0][0] + agree[1][1]) as f64 / total as f64 * 100.0,
    );
    Ok(())
}
