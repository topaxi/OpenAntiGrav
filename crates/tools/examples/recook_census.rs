//! Census of re-cooked content: one TSV row per `.rcsmodel` and per texture
//! under every archive path that contains `<needle>` (case-insensitive), so a
//! circuit or hull can be set beside its copy on another title.
//!
//! Rows, tab-separated:
//! - `model <archive> <path> <meshes> <submeshes> <vertices> <triangles> <materials>`
//!   where `<meshes>` is the `.rcsmodel` section count on Vita and PS4 and the
//!   mesh count (extra surfaces included) on HD.
//! - `texture <archive> <path> <count> <width> <height> <mips> <format>`, from
//!   the first texture of a `.gxt`, `.gtf` or `.gnf` container.
//! - `refused <archive> <path> <error>`
//!
//! The model reader is picked by the file's own magic: `0xca5caded` is the
//! Vita/PS4 `RCSMODEL`, anything else goes to HD's reader. `<platform>` is a
//! label kept for the caller's own bookkeeping and selects nothing.
//!
//! `--submeshes` adds one `sub` row per submesh of every Vita/PS4 model, and
//! `--cells <size>` one `cell` row per occupied `size`-metre square of the x/z
//! plane, so two models of the same circuit can be compared per object and per
//! region. Both are read off the decoded positions, so both need the same
//! archives as the census itself.
//! - `sub <archive> <path> <submesh> <mesh> <node> <material> <vertices> <triangles> <area> <min x,y,z> <max x,y,z> <skipped> <placed> <material name>`
//! - `cell <archive> <path> <cell x> <cell z> <triangles> <area>`
//!
//! Positions are carried to world space by each node-bound submesh's node
//! bind matrix; `placed` is 0 when that node has none.
//!
//! `cargo run -p oag-tools --example recook_census -- [--submeshes] [--cells <size>] <platform> <needle> <archive>...`

use oag_rcs::rcsmodel;
use oag_rcs::rcsmodel::psp2;
use oag_texture::{gnf, gtf, gxt};
use std::collections::BTreeMap;

/// What the extra `sub` and `cell` rows are asked for, from the flags.
#[derive(Default)]
struct Detail {
    submeshes: bool,
    cells: Option<f32>,
}

fn main() {
    let mut raw: Vec<String> = std::env::args().skip(1).collect();
    let mut detail = Detail::default();
    while raw.first().is_some_and(|a| a.starts_with("--")) {
        match raw.remove(0).as_str() {
            "--submeshes" => detail.submeshes = true,
            "--cells" => {
                let size = raw
                    .remove(0)
                    .parse::<f32>()
                    .expect("--cells takes a size in metres");
                detail.cells = Some(size);
            }
            other => panic!("unknown flag {other}"),
        }
    }
    let mut args = raw.into_iter();
    let platform = args
        .next()
        .expect("usage: recook_census [flags] <platform> <needle> <archive>...");
    let needle = args
        .next()
        .expect("usage: recook_census [flags] <platform> <needle> <archive>...")
        .to_lowercase();
    for archive_path in args {
        let mut archive = match oag_assets::psarc::Archive::open(&archive_path) {
            Ok(archive) => archive,
            Err(e) => {
                println!("refused\t{platform}:{archive_path}\t-\t{e}");
                continue;
            }
        };
        let label = std::path::Path::new(&archive_path)
            .file_name()
            .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
        let paths: Vec<String> = archive
            .paths()
            .iter()
            .filter(|p| p.to_lowercase().contains(&needle))
            .cloned()
            .collect();
        for path in paths {
            let lower = path.to_lowercase();
            let blob = match archive.read_path(&path) {
                Ok(blob) => blob,
                Err(e) => {
                    println!("refused\t{label}\t{path}\t{e}");
                    continue;
                }
            };
            if lower.ends_with(".rcsmodel") {
                census_model(&label, &path, &blob, &detail);
            } else if lower.ends_with(".gxt") {
                match gxt::Gxt::parse(&blob) {
                    Ok(g) => census_texture(
                        &label,
                        &path,
                        g.textures.len(),
                        g.textures.first().map(|t| {
                            (
                                u32::from(t.width),
                                u32::from(t.height),
                                t.mip_levels,
                                format!("{:?}", t.format()),
                            )
                        }),
                    ),
                    Err(e) => println!("refused\t{label}\t{path}\t{e}"),
                }
            } else if lower.ends_with(".gtf") {
                match gtf::Gtf::parse(&blob) {
                    Ok(g) => census_texture(
                        &label,
                        &path,
                        g.textures.len(),
                        g.textures.first().map(|t| {
                            (
                                u32::from(t.width),
                                u32::from(t.height),
                                t.mip_levels,
                                format!("{:?}", t.format),
                            )
                        }),
                    ),
                    Err(e) => println!("refused\t{label}\t{path}\t{e}"),
                }
            } else if lower.ends_with(".gnf") {
                match gnf::Texture::parse(&blob) {
                    Ok(t) => {
                        let mips = t.last_mip_level.saturating_add(1);
                        census_texture(
                            &label,
                            &path,
                            1,
                            Some((t.width, t.height, mips, format!("{:?}", t.surface_format))),
                        );
                    }
                    Err(e) => println!("refused\t{label}\t{path}\t{e}"),
                }
            }
        }
    }
}

fn census_model(label: &str, path: &str, blob: &[u8], detail: &Detail) {
    if blob.len() >= 4 && blob[..4] == psp2::MAGIC.to_le_bytes() {
        match psp2::parse(blob) {
            Ok(model) => {
                let vertices: usize = model.submeshes.iter().map(|s| s.positions.len()).sum();
                let triangles: usize = model.submeshes.iter().map(|s| s.indices.len() / 3).sum();
                println!(
                    "model\t{label}\t{path}\t{}\t{}\t{vertices}\t{triangles}\t{}",
                    model.sections.len(),
                    model.submeshes.len(),
                    model.materials.len()
                );
                if detail.submeshes {
                    for (i, s) in model.submeshes.iter().enumerate() {
                        census_submesh(label, path, i, s, &model);
                    }
                }
                if let Some(size) = detail.cells {
                    census_cells(label, path, &model, size);
                }
            }
            Err(e) => println!("refused\t{label}\t{path}\t{e}"),
        }
        return;
    }
    match rcsmodel::Model::parse(blob) {
        Ok(model) => {
            let (mut meshes, mut submeshes, mut vertices, mut triangles) = (0, 0, 0, 0);
            let mut stack: Vec<&rcsmodel::Mesh> = model.meshes.iter().collect();
            while let Some(mesh) = stack.pop() {
                meshes += 1;
                for s in &mesh.submeshes {
                    submeshes += 1;
                    vertices += s.vertex_count;
                    triangles += s.index_count / 3;
                }
                stack.extend(mesh.extra_surfaces.iter());
            }
            println!(
                "model\t{label}\t{path}\t{meshes}\t{submeshes}\t{vertices}\t{triangles}\t{}",
                model.materials.len()
            );
        }
        Err(e) => println!("refused\t{label}\t{path}\t{e}"),
    }
}

/// Positions of one submesh in the circuit's world space. A node-bound submesh
/// is authored in its node's space, so it is carried by the node's bind matrix
/// (row-major, translation in row 3, as `psp2::nodes::Node::bind` documents);
/// a submesh with no node is already in world space. The flag is false when the
/// node has no matrix, and the positions are then left in the node's own space.
fn world_positions(s: &psp2::SubMesh, nodes: &[psp2::nodes::Node]) -> (Vec<[f32; 3]>, bool) {
    let bind = s.node.and_then(|n| nodes.get(n)).and_then(|node| node.bind);
    match (s.node, bind) {
        (None, _) => (s.positions.clone(), true),
        (Some(_), None) => (s.positions.clone(), false),
        (Some(_), Some(m)) => {
            let out = s
                .positions
                .iter()
                .map(|p| {
                    [0, 1, 2].map(|k| p[0] * m[k] + p[1] * m[4 + k] + p[2] * m[8 + k] + m[12 + k])
                })
                .collect();
            (out, true)
        }
    }
}

/// Triangles of one submesh over `positions`, a triangle whose index runs past
/// the vertex buffer skipped and counted, as `(triangles, skipped)`.
fn triangles(s: &psp2::SubMesh, positions: &[[f32; 3]]) -> (Vec<[[f32; 3]; 3]>, usize) {
    let mut out = Vec::with_capacity(s.indices.len() / 3);
    let mut skipped = 0;
    for tri in s.indices.as_chunks::<3>().0 {
        match (
            positions.get(usize::from(tri[0])),
            positions.get(usize::from(tri[1])),
            positions.get(usize::from(tri[2])),
        ) {
            (Some(a), Some(b), Some(c)) => out.push([*a, *b, *c]),
            _ => skipped += 1,
        }
    }
    (out, skipped)
}

fn area(t: &[[f32; 3]; 3]) -> f64 {
    let u = [0, 1, 2].map(|k| f64::from(t[1][k]) - f64::from(t[0][k]));
    let v = [0, 1, 2].map(|k| f64::from(t[2][k]) - f64::from(t[0][k]));
    let cross = [
        u[1] * v[2] - u[2] * v[1],
        u[2] * v[0] - u[0] * v[2],
        u[0] * v[1] - u[1] * v[0],
    ];
    0.5 * cross.iter().map(|c| c * c).sum::<f64>().sqrt()
}

fn census_submesh(label: &str, path: &str, index: usize, s: &psp2::SubMesh, model: &psp2::Model) {
    let (positions, placed) = world_positions(s, &model.scene.nodes);
    let (tris, skipped) = triangles(s, &positions);
    let total: f64 = tris.iter().map(area).sum();
    let mut lo = [f32::INFINITY; 3];
    let mut hi = [f32::NEG_INFINITY; 3];
    for p in &positions {
        for k in 0..3 {
            lo[k] = lo[k].min(p[k]);
            hi[k] = hi[k].max(p[k]);
        }
    }
    let opt = |v: Option<usize>| v.map_or_else(|| "-".to_string(), |v| v.to_string());
    println!(
        "sub\t{label}\t{path}\t{index}\t{}\t{}\t{}\t{}\t{}\t{total:.3}\t{},{},{}\t{},{},{}\t{skipped}\t{}\t{}",
        opt(s.mesh),
        opt(s.node),
        opt(s.material),
        s.positions.len(),
        tris.len(),
        lo[0],
        lo[1],
        lo[2],
        hi[0],
        hi[1],
        hi[2],
        u8::from(placed),
        s.material
            .and_then(|m| model.materials.get(m))
            .map_or_else(|| "-".to_string(), |m| m.name.replace(['\t', '\n'], " ")),
    );
}

/// One `cell` row per occupied square of the x/z plane, over world positions
/// (see [`world_positions`]). Keys are `BTreeMap` so the rows come out in a
/// fixed order.
fn census_cells(label: &str, path: &str, model: &psp2::Model, size: f32) {
    let mut cells: BTreeMap<(i64, i64), (usize, f64)> = BTreeMap::new();
    for s in &model.submeshes {
        let (positions, _) = world_positions(s, &model.scene.nodes);
        for t in triangles(s, &positions).0 {
            let c = [0, 1, 2].map(|k| (t[0][k] + t[1][k] + t[2][k]) / 3.0);
            let key = ((c[0] / size).floor() as i64, (c[2] / size).floor() as i64);
            let entry = cells.entry(key).or_insert((0, 0.0));
            entry.0 += 1;
            entry.1 += area(&t);
        }
    }
    for ((x, z), (tris, total)) in cells {
        println!("cell\t{label}\t{path}\t{x}\t{z}\t{tris}\t{total:.3}");
    }
}

fn census_texture(label: &str, path: &str, count: usize, first: Option<(u32, u32, u8, String)>) {
    match first {
        Some((w, h, mips, format)) => {
            println!("texture\t{label}\t{path}\t{count}\t{w}\t{h}\t{mips}\t{format}");
        }
        None => println!("refused\t{label}\t{path}\tno texture in container"),
    }
}
