//! Shared fixtures for the two `.rcsmodel` ground-truth binaries.
//!
//! `tests/<dir>/mod.rs` is not itself a test target, which is why this is a
//! directory: the helpers are needed by both
//! `rcsmodel_ground_truth.rs` (the container) and
//! `rcsmodel_vertex_ground_truth.rs` (what a vertex holds), and duplicating
//! them would let the two drift.

#![allow(dead_code)]

use std::path::{Path, PathBuf};

use oag_formats::vex;

/// The decrypted PS3 image.
pub const PS3_IMAGE: &str = "hdfury-ps3-eu-dec.iso";

/// A `.vex` and the `.rcsmodel` beside it, with the archive holding both.
pub const PAIRS: &[(&str, &str, &str)] = &[
    (
        "DATA02.PSARC",
        "/data/ships/assegai/ship.vex",
        "/data/ships/assegai/ship.rcsmodel",
    ),
    (
        "DATA02.PSARC",
        "/data/ships/assegai/ship_lod1.vex",
        "/data/ships/assegai/ship_lod1.rcsmodel",
    ),
    (
        "DATA00.PSARC",
        "/data/environments/talons_junction/track.vex",
        "/data/environments/talons_junction/track.rcsmodel",
    ),
];

/// Two quantisation steps at the 1/128 scale every file on the disc uses.
pub const TOLERANCE: f32 = 2.0 / 128.0;

pub fn image() -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/images")
        .join(PS3_IMAGE);

    if path.exists() {
        return Some(path);
    }
    assert!(
        std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
        "OAG_REQUIRE_GAME_DATA is set but {} is missing",
        path.display()
    );
    println!("skipping: {} not present", path.display());
    None
}

/// Both halves of one pair, read straight out of the archive.
pub fn pair(archive: &str, vex_path: &str, model_path: &str) -> Option<(Vec<u8>, Vec<u8>)> {
    let image = image()?;
    let spec = format!("{}:PS3_GAME/USRDIR/{archive}", image.display());
    let mut open = oag_assets::psarc::Archive::open(&spec).expect("the archive opens");
    Some((
        open.read_path(vex_path).expect("the .vex reads"),
        open.read_path(model_path).expect("the .rcsmodel reads"),
    ))
}

/// Every `Mesh` node in a `.vex`, as (name, hash, min, max).
pub fn mesh_nodes(blob: &[u8]) -> Vec<(String, u32, [f32; 3], [f32; 3])> {
    let classes = vex::classes_of(blob).expect("a class table");
    let mesh = classes.mesh.expect("version 6 numbers Mesh");
    let order = vex::byte_order(blob);
    vex::nodes(blob)
        .expect("the node tree walks")
        .into_iter()
        .filter(|node| node.class_id == mesh)
        .filter_map(|node| {
            let payload = &blob[node.payload()];
            // The box pair at +0x10/+0x20 and the chunk hash at +0x30, which is
            // as much of a PS3 `Mesh` payload as anything has recovered.
            if payload.len() < 0x34 {
                return None;
            }
            let read3 = |at: usize| std::array::from_fn(|i| order.f32(payload, at + i * 4));
            Some((
                node.name.clone().unwrap_or_default(),
                order.u32(payload, 0x30),
                read3(0x10),
                read3(0x20),
            ))
        })
        .collect()
}

/// A `Mesh` node that found no chunk: its name, its hash, and the box it
/// authors - which is the only thing that can say whether a chunk found
/// elsewhere is the same mesh.
pub type Orphan = (String, u32, ([f32; 3], [f32; 3]));

/// The widest axis of a point set.
pub fn span(points: &[[f32; 3]]) -> f32 {
    (0..3)
        .map(|i| {
            let lo = points.iter().fold(f32::MAX, |a, p| a.min(p[i]));
            let hi = points.iter().fold(f32::MIN, |a, p| a.max(p[i]));
            hi - lo
        })
        .fold(0.0, f32::max)
}

pub fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

pub fn unit3(v: [f32; 3]) -> Option<[f32; 3]> {
    let l = dot(v, v).sqrt();
    (l > 1e-9).then(|| [v[0] / l, v[1] / l, v[2] / l])
}

/// Area-weighted vertex normals, which is what the `.rcsmodel` is checked
/// against and what it does not itself state.
///
/// **Area-weighted on purpose**: summing raw cross products rather than
/// normalising each face first is what an exporter writes, and unweighted
/// averaging diverges wherever a mesh mixes large and small triangles - which a
/// circuit does everywhere.
pub fn smooth_normals(points: &[[f32; 3]], indices: &[u16]) -> Vec<Option<[f32; 3]>> {
    let mut acc = vec![[0.0f32; 3]; points.len()];
    for t in indices.chunks_exact(3) {
        let [a, b, c] = [t[0], t[1], t[2]].map(|i| points[i as usize]);
        let (u, v) = (
            [b[0] - a[0], b[1] - a[1], b[2] - a[2]],
            [c[0] - a[0], c[1] - a[1], c[2] - a[2]],
        );
        let n = [
            u[1] * v[2] - u[2] * v[1],
            u[2] * v[0] - u[0] * v[2],
            u[0] * v[1] - u[1] * v[0],
        ];
        for &i in t {
            for k in 0..3 {
                acc[i as usize][k] += n[k];
            }
        }
    }
    acc.into_iter().map(unit3).collect()
}
