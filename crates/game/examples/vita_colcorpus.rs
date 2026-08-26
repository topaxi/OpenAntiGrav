//! Scratch probe: the surface byte across every `track_col.col` the three
//! packages carry, with the facing statistic per value.
//!
//! ```sh
//! cargo run -q -p oag-game --example vita_colcorpus
//! ```

use std::collections::BTreeMap;

const PACKAGES: [&str; 3] = [
    "data/extracted/vita/PCSF00007/base/PSP2/data.psarc",
    "data/extracted/vita/PCSF00007/dlc1/PSP2/dlc1.psarc",
    "data/extracted/vita/PCSF00007/dlc2/PSP2/dlc2.psarc",
];

fn u16_at(b: &[u8], at: usize) -> usize {
    usize::from(u16::from_le_bytes([b[at], b[at + 1]]))
}
fn u32_at(b: &[u8], at: usize) -> usize {
    u32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]]) as usize
}
fn f32_at(b: &[u8], at: usize) -> f32 {
    f32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
}

/// Per surface byte: triangle count, and the summed |normal.y| behind it.
type Facing = BTreeMap<u8, (usize, f64, usize, usize)>;

fn tally(b: &[u8], into: &mut Facing) -> Option<(usize, usize, usize)> {
    if b.len() < 16 || &b[0..4] != b"kdtr" {
        return None;
    }
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
    at += index_stride * triangle_count;
    let extra_count = u16_at(b, at);
    at += 2 + extra_count + 24 + 4;
    if at != b.len() || extra_count != triangle_count {
        println!(
            "    ! does not close: ends at {at} of {}, {extra_count} extra byte(s) for {triangle_count} triangle(s)",
            b.len()
        );
    }
    let vertex = |i: usize| {
        let at = vertices_at + i * 12;
        [f32_at(b, at), f32_at(b, at + 4), f32_at(b, at + 8)]
    };
    for t in 0..triangle_count {
        let v: Vec<[f32; 3]> = (0..3)
            .map(|k| vertex(u16_at(b, triangles_at + t * index_stride + k * 2)))
            .collect();
        let e0 = [v[1][0] - v[0][0], v[1][1] - v[0][1], v[1][2] - v[0][2]];
        let e1 = [v[2][0] - v[0][0], v[2][1] - v[0][1], v[2][2] - v[0][2]];
        let n = [
            e0[1] * e1[2] - e0[2] * e1[1],
            e0[2] * e1[0] - e0[0] * e1[2],
            e0[0] * e1[1] - e0[1] * e1[0],
        ];
        let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
        let up = if len > 1e-9 { (n[1] / len).abs() } else { 0.0 };
        let row = into
            .entry(b[triangles_at + index_stride * triangle_count + 2 + t])
            .or_default();
        row.0 += 1;
        row.1 += f64::from(up);
        if up > 0.866 {
            row.2 += 1;
        }
        if up < 0.5 {
            row.3 += 1;
        }
    }
    Some((vertex_count, triangle_count, node_count))
}

fn main() -> anyhow::Result<()> {
    let mut all = Facing::new();
    let mut files = 0usize;
    for package in PACKAGES {
        let mut archive = oag_assets::psarc::Archive::open(package)?;
        let mut names: Vec<String> = archive
            .paths()
            .iter()
            .filter(|p| p.to_ascii_lowercase().ends_with("/track_col.col"))
            .cloned()
            .collect();
        names.sort();
        for name in names {
            let Ok(blob) = archive.read_path(&name) else {
                continue;
            };
            let mut per = Facing::new();
            let short = name.rsplit('/').nth(1).unwrap_or(&name).to_string();
            match tally(&blob, &mut per) {
                None => println!("  {short:<18} not a kdtr file"),
                Some((v, t, n)) => {
                    files += 1;
                    let bytes: Vec<String> = per
                        .iter()
                        .map(|(k, row)| format!("{k}:{}", row.0))
                        .collect();
                    println!(
                        "  {short:<18} {v:>6} verts {t:>6} tris {n:>6} nodes | {}",
                        bytes.join(" ")
                    );
                }
            }
            for (k, row) in per {
                let e = all.entry(k).or_default();
                e.0 += row.0;
                e.1 += row.1;
                e.2 += row.2;
                e.3 += row.3;
            }
        }
    }
    println!("\n=== {files} file(s), by surface byte ===");
    println!(
        "  {:>5} {:>9} {:>9} {:>9} {:>10}",
        "byte", "tris", "horiz%", "vert%", "mean|n.y|"
    );
    for (k, (n, sum, horiz, vert)) in &all {
        println!(
            "  {k:>5} {n:>9} {:>8.1}% {:>8.1}% {:>10.3}",
            100.0 * *horiz as f32 / *n as f32,
            100.0 * *vert as f32 / *n as f32,
            sum / *n as f64
        );
    }
    Ok(())
}
