//! Scratch probe: what does `track_col.col`'s per-triangle surface byte mean?
//!
//! Wipeout 2048's DLC re-ships twelve Wipeout HD circuits, and HD keeps its
//! collision in `.vex` nodes whose classes are *named* - `Floor Collision`,
//! `Wall Collision`, `Reset Collision`, `Mag Floor Collision`,
//! `collision_trackwall`. So for a ported circuit each 2048 collision triangle
//! can be matched to the HD triangle at the same place and read off HD's own
//! class, which is ground truth rather than a facing statistic.
//!
//! ```sh
//! cargo run -q -p oag-game --example vita_surface
//! ```

use std::collections::{BTreeMap, HashMap};

use oag_vex::collision::{self, SurfaceKind};

const HD_DIR: &str = "data/extracted/ps3/hdfury-eu/PS3_GAME/USRDIR";
const VITA_DLC: [&str; 2] = [
    "data/extracted/vita/PCSF00007/dlc1/PSP2/dlc1.psarc",
    "data/extracted/vita/PCSF00007/dlc2/PSP2/dlc2.psarc",
];

/// `(HD archive, HD environment, 2048 environment)`.
const PAIRS: &[(&str, &str, &str)] = &[
    ("DATA02", "05_ubermall", "Ubermall"),
    ("DATA02", "10_sebenco_climb", "Sebenco_Climb"),
    ("DATA02", "12_sol_2", "Sol_2"),
    ("DATA00", "amphiseum", "amphiseum"),
    ("DATA00", "modesto_heights", "modesto_heights"),
    ("DATA00", "talons_junction", "talons_junction"),
    ("DATA00", "tech_de_ra", "tech_de_ra"),
    ("DATA00", "zone_1", "zone_1"),
    ("DATA00", "zone_2", "zone_2"),
    ("DATA00", "zone_3", "zone_3"),
    ("DATA00", "zone_4", "zone_4"),
];

/// One collision triangle: its centroid, what marks it, and its face normal.
type Marked<T> = ([f32; 3], T, [f32; 3]);

/// Centroids are hashed onto a grid this many units across, so a triangle is
/// compared only against the ones near it.
const CELL: f32 = 4.0;

fn cell(p: [f32; 3]) -> (i32, i32, i32) {
    (
        (p[0] / CELL).floor() as i32,
        (p[1] / CELL).floor() as i32,
        (p[2] / CELL).floor() as i32,
    )
}

fn u16_at(b: &[u8], at: usize) -> usize {
    usize::from(u16::from_le_bytes([b[at], b[at + 1]]))
}
fn u32_at(b: &[u8], at: usize) -> usize {
    u32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]]) as usize
}
fn f32_at(b: &[u8], at: usize) -> f32 {
    f32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
}

/// 2048's side: every collision triangle's centroid and its surface byte.
fn normal(v: &[[f32; 3]; 3]) -> [f32; 3] {
    let e0 = [v[1][0] - v[0][0], v[1][1] - v[0][1], v[1][2] - v[0][2]];
    let e1 = [v[2][0] - v[0][0], v[2][1] - v[0][1], v[2][2] - v[0][2]];
    [
        e0[1] * e1[2] - e0[2] * e1[1],
        e0[2] * e1[0] - e0[0] * e1[2],
        e0[0] * e1[1] - e0[1] * e1[0],
    ]
}

fn vita_triangles(name: &str) -> Option<Vec<Marked<u8>>> {
    let mut blob = None;
    for psarc in VITA_DLC {
        let Ok(mut archive) = oag_assets::psarc::Archive::open(psarc) else {
            continue;
        };
        let entry = format!("data/art/published/DLC1/environments/{name}/track_col.col");
        if let Ok(read) = archive.read_path(&entry) {
            blob = Some(read);
            break;
        }
    }
    let b = blob?;
    let mut at = 12;
    let node_stride = u32_at(&b, at);
    let node_count = u32_at(&b, at + 4);
    at += 8 + node_stride * node_count + 4;
    let leaf_count = u32_at(&b, at);
    at += 4 + leaf_count * 2 + 4 + 24 + 4;
    let vertex_count = u16_at(&b, at);
    at += 2;
    let vertices_at = at;
    at += vertex_count * 12;
    let index_stride = u32_at(&b, at);
    at += 4;
    let triangle_count = u16_at(&b, at);
    at += 2;
    let triangles_at = at;
    at += index_stride * triangle_count;
    let extra_at = at + 2;

    let vertex = |i: usize| {
        let at = vertices_at + i * 12;
        [f32_at(&b, at), f32_at(&b, at + 4), f32_at(&b, at + 8)]
    };
    assert_eq!(index_stride, 6, "{name}: triangle stride is not 6");
    let mut out = Vec::with_capacity(triangle_count);
    for t in 0..triangle_count {
        let v: [[f32; 3]; 3] =
            std::array::from_fn(|k| vertex(u16_at(&b, triangles_at + t * index_stride + k * 2)));
        let mut centroid = [0.0f32; 3];
        for w in &v {
            for a in 0..3 {
                centroid[a] += w[a] / 3.0;
            }
        }
        out.push((centroid, b[extra_at + t], normal(&v)));
    }
    Some(out)
}

/// HD's side: every collision triangle's centroid and the class it belongs to.
fn hd_triangles(archive: &str, environment: &str) -> anyhow::Result<Vec<Marked<SurfaceKind>>> {
    let mut hd = oag_assets::psarc::Archive::open(&format!("{HD_DIR}/{archive}.PSARC"))?;
    let blob = hd.read_path(&format!("/data/environments/{environment}/track.vex"))?;
    let mut out = Vec::new();
    for node in collision::from_vex(&blob)? {
        for mesh in &node.geometry.meshes {
            for triangle in &mesh.triangles {
                let v: [[f32; 3]; 3] =
                    std::array::from_fn(|k| mesh.vertices[usize::from(triangle[k])]);
                let mut centroid = [0.0f32; 3];
                for w in &v {
                    for a in 0..3 {
                        centroid[a] += w[a] / 3.0;
                    }
                }
                out.push((centroid, node.kind, normal(&v)));
            }
        }
    }
    Ok(out)
}

fn main() -> anyhow::Result<()> {
    let mut total: BTreeMap<(u8, &'static str), usize> = BTreeMap::new();
    let mut unmatched: BTreeMap<u8, usize> = BTreeMap::new();
    let mut winding = (0usize, 0usize);
    for &(archive, environment, name) in PAIRS {
        let Some(vita) = vita_triangles(name) else {
            println!("{name}: no 2048 track_col.col");
            continue;
        };
        let hd = hd_triangles(archive, environment)?;
        let mut grid: HashMap<(i32, i32, i32), Vec<usize>> = HashMap::new();
        for (i, (centroid, _, _)) in hd.iter().enumerate() {
            grid.entry(cell(*centroid)).or_default().push(i);
        }
        let mut matched = 0usize;
        let (mut agree, mut opposed) = (0usize, 0usize);
        let mut per_track: BTreeMap<(u8, &'static str), usize> = BTreeMap::new();
        for (centroid, byte, n) in &vita {
            let (cx, cy, cz) = cell(*centroid);
            let mut best: Option<(f32, SurfaceKind, [f32; 3])> = None;
            for dx in -1..=1 {
                for dy in -1..=1 {
                    for dz in -1..=1 {
                        for &i in grid.get(&(cx + dx, cy + dy, cz + dz)).into_iter().flatten() {
                            let (other, kind, hd_n) = hd[i];
                            let d = (0..3)
                                .map(|a| (other[a] - centroid[a]).powi(2))
                                .sum::<f32>();
                            if best.is_none_or(|(b, _, _)| d < b) {
                                best = Some((d, kind, hd_n));
                            }
                        }
                    }
                }
            }
            match best {
                Some((d, kind, hd_n)) if d < 1.0 => {
                    matched += 1;
                    let dot: f32 = (0..3).map(|a| n[a] * hd_n[a]).sum();
                    if dot > 0.0 {
                        agree += 1;
                    } else if dot < 0.0 {
                        opposed += 1;
                    }
                    *per_track.entry((*byte, name_of(kind))).or_default() += 1;
                    *total.entry((*byte, name_of(kind))).or_default() += 1;
                }
                _ => *unmatched.entry(*byte).or_default() += 1,
            }
        }
        println!(
            "{name}: {} 2048 triangle(s), {} HD triangle(s), {matched} matched within 1 unit \
| winding: {agree} agree, {opposed} opposed",
            vita.len(),
            hd.len()
        );
        winding.0 += agree;
        winding.1 += opposed;
        for ((byte, kind), n) in &per_track {
            println!("    byte {byte:>3} -> {kind:<12} {n:>7}");
        }
    }
    println!("\n=== all eleven circuits ===");
    for ((byte, kind), n) in &total {
        println!("  byte {byte:>3} -> {kind:<12} {n:>8}");
    }
    println!("  unmatched by byte: {unmatched:?}");
    println!(
        "  winding against HD: {} agree, {} opposed",
        winding.0, winding.1
    );
    Ok(())
}

fn name_of(kind: SurfaceKind) -> &'static str {
    match kind {
        SurfaceKind::Wall => "Wall",
        SurfaceKind::Floor => "Floor",
        SurfaceKind::Reset => "Reset",
        SurfaceKind::MagFloor => "MagFloor",
        SurfaceKind::Cage => "Cage",
        SurfaceKind::TrackWall => "TrackWall",
    }
}
