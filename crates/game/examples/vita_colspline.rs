//! Scratch probe: does the racing line run over the triangles a given surface
//! byte marks?
//!
//! The `WO Track` spline is authored *on the track surface* - see
//! `oag_formats::track`'s module docs - so a drivable floor sits directly under
//! it, within the half-width the spline itself carries. Scenery does not. This
//! is the measurement that separates the three surface bytes only 2048's own
//! circuits use from the six the ported ones settle.
//!
//! ```sh
//! cargo run -q -p oag-game --example vita_colspline
//! ```

use std::collections::BTreeMap;

use oag_formats::{track, vex};

const BASE: &str = "data/extracted/vita/PCSF00007/base/PSP2/data.psarc";

fn u16_at(b: &[u8], at: usize) -> usize {
    usize::from(u16::from_le_bytes([b[at], b[at + 1]]))
}
fn u32_at(b: &[u8], at: usize) -> usize {
    u32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]]) as usize
}
fn f32_at(b: &[u8], at: usize) -> f32 {
    f32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
}

/// Every collision triangle's centroid and surface byte.
fn triangles(b: &[u8]) -> Vec<([f32; 3], u8)> {
    let mut at = 12;
    let node_stride = u32_at(b, at);
    let node_count = u32_at(b, at + 4);
    at += 8 + node_stride * node_count + 4;
    let leaf_count = u32_at(b, at);
    at += 4 + leaf_count * 2 + 4 + 24 + 4;
    let vertex_count = u16_at(b, at);
    at += 2;
    let vertices_at = at;
    at += vertex_count * 12;
    let index_stride = u32_at(b, at);
    at += 4;
    let triangle_count = u16_at(b, at);
    at += 2;
    let triangles_at = at;
    let extra_at = triangles_at + index_stride * triangle_count + 2;
    let vertex = |i: usize| {
        let at = vertices_at + i * 12;
        [f32_at(b, at), f32_at(b, at + 4), f32_at(b, at + 8)]
    };
    (0..triangle_count)
        .map(|t| {
            let mut centroid = [0.0f32; 3];
            for k in 0..3 {
                let v = vertex(u16_at(b, triangles_at + t * index_stride + k * 2));
                for a in 0..3 {
                    centroid[a] += v[a] / 3.0;
                }
            }
            (centroid, b[extra_at + t])
        })
        .collect()
}

/// Row: triangles, how many sit under the racing line, and the summed drop
/// from the spline down to them.
type Row = (usize, usize, f64);

fn main() -> anyhow::Result<()> {
    let mut archive = oag_assets::psarc::Archive::open(BASE)?;
    let mut names: Vec<String> = archive
        .paths()
        .iter()
        .filter(|p| p.to_ascii_lowercase().ends_with("/track_col.col"))
        .cloned()
        .collect();
    names.sort();
    let mut all: BTreeMap<u8, Row> = BTreeMap::new();
    for name in names {
        let Ok(col) = archive.read_path(&name) else {
            continue;
        };
        let Ok(vex_blob) = archive.read_path(&name.replace("track_col.col", "track.vex")) else {
            continue;
        };
        let Ok(nodes) = vex::nodes(&vex_blob) else {
            continue;
        };
        let Some(node) = track::find_node(&vex_blob, &nodes) else {
            continue;
        };
        let Ok(ai) = track::parse(&vex_blob[node.payload()]) else {
            continue;
        };
        // Every control point, with the half-width it authors.
        let points: Vec<([f32; 3], [f32; 3], f32)> = ai
            .paths
            .iter()
            .flat_map(|p| &p.points)
            .map(|p| (p.pos, p.down, p.half_width_left.max(p.half_width_right)))
            .collect();
        let mut per: BTreeMap<u8, Row> = BTreeMap::new();
        for (centroid, byte) in triangles(&col) {
            // Nearest control point in the horizontal plane, then how far the
            // triangle sits below it along the spline's own down axis.
            let mut best = (f32::MAX, 0.0f32, 0.0f32);
            for (pos, down, width) in &points {
                let d = (pos[0] - centroid[0]).powi(2) + (pos[2] - centroid[2]).powi(2);
                if d < best.0 {
                    let drop = (0..3)
                        .map(|a| (centroid[a] - pos[a]) * down[a])
                        .sum::<f32>();
                    best = (d, drop, *width);
                }
            }
            let row = per.entry(byte).or_default();
            row.0 += 1;
            // Under the racing line: inside the authored half-width horizontally
            // and within 2 units of the surface the spline was authored on.
            if best.0.sqrt() <= best.2 && best.1.abs() <= 2.0 {
                row.1 += 1;
            }
            row.2 += f64::from(best.1);
        }
        for (byte, row) in per {
            let e = all.entry(byte).or_default();
            e.0 += row.0;
            e.1 += row.1;
            e.2 += row.2;
        }
    }
    println!(
        "  {:>5} {:>9} {:>16} {:>14}",
        "byte", "tris", "under the line", "mean drop"
    );
    for (byte, (n, under, drop)) in &all {
        println!(
            "  {byte:>5} {n:>9} {:>15.1}% {:>14.1}",
            100.0 * *under as f32 / *n as f32,
            drop / *n as f64
        );
    }
    coverage()
}

/// The check that decides it: how much of the racing line has ground under it,
/// counting only the surface bytes named in `keep`?
///
/// A control point with no floor beneath it is a hole a craft falls through, so
/// this asks the question the race asks.
fn coverage() -> anyhow::Result<()> {
    let mut archive = oag_assets::psarc::Archive::open(BASE)?;
    let mut names: Vec<String> = archive
        .paths()
        .iter()
        .filter(|p| p.to_ascii_lowercase().ends_with("/track_col.col"))
        .cloned()
        .collect();
    names.sort();
    println!("\n  control points with a floor triangle within 4 units below them");
    println!(
        "  {:<18} {:>14} {:>18}",
        "circuit", "byte 2 only", "plus 10, 11, 12"
    );
    for name in names {
        let Ok(col) = archive.read_path(&name) else {
            continue;
        };
        let Ok(vex_blob) = archive.read_path(&name.replace("track_col.col", "track.vex")) else {
            continue;
        };
        let Ok(nodes) = vex::nodes(&vex_blob) else {
            continue;
        };
        let Some(node) = track::find_node(&vex_blob, &nodes) else {
            continue;
        };
        let Ok(ai) = track::parse(&vex_blob[node.payload()]) else {
            continue;
        };
        let tris = triangles(&col);
        let mut covered = [0usize; 2];
        let mut total = 0usize;
        for point in ai.paths.iter().flat_map(|p| &p.points) {
            total += 1;
            let mut hit = [false; 2];
            for (centroid, byte) in &tris {
                let floor = match byte {
                    2 => 0,
                    10..=12 => 1,
                    _ => continue,
                };
                let d = (point.pos[0] - centroid[0]).powi(2) + (point.pos[2] - centroid[2]).powi(2);
                let drop: f32 = (0..3)
                    .map(|a| (centroid[a] - point.pos[a]) * point.down[a])
                    .sum();
                if d <= 36.0 && (-1.0..4.0).contains(&drop) {
                    if floor == 0 {
                        hit[0] = true;
                    }
                    hit[1] = true;
                }
            }
            if hit[0] {
                covered[0] += 1;
            }
            if hit[1] {
                covered[1] += 1;
            }
        }
        let short = name.rsplit('/').nth(1).unwrap_or(&name).to_string();
        println!(
            "  {short:<18} {:>13.1}% {:>17.1}%",
            100.0 * covered[0] as f32 / total as f32,
            100.0 * covered[1] as f32 / total as f32
        );
    }
    Ok(())
}
