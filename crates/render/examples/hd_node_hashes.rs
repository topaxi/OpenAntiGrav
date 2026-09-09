//! Scratch probe: how many of a `Mesh` node's payload words are chunk hashes
//! the `.rcsmodel` beside it actually carries.
//!
//! `mesh::rcs::node_geometry` reads **one**, at the payload's own `+0x30`. If
//! a node names a list - an LOD chain, or a multi-part object - everything
//! past the first is geometry no pass draws.

use oag_rcs::rcsmodel;

use oag_render::mesh;
use oag_vex::vex;

fn main() -> anyhow::Result<()> {
    let spec = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "data/images/hdfury-ps3-eu-dec.iso:PS3_GAME/USRDIR/DATA00.PSARC".into());
    let name = std::env::args()
        .nth(2)
        .unwrap_or_else(|| "/data/environments/talons_junction/track.vex".into());
    let data = mesh::read_blob(&spec, &name)?;
    let sibling = mesh::rcs::sibling_name(&name).unwrap();
    let model = rcsmodel::Model::parse(&mesh::read_blob(&spec, &sibling)?)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let order = vex::byte_order(&data);
    let nodes = vex::nodes(&data)?;
    let mesh_class = vex::classes_of(&data)?.mesh.unwrap();

    if std::env::var("OAG_STRINGS").is_ok() {
        // Printable runs inside each class's node payloads: what a node names
        // that is not geometry this build reads.
        let classes = vex::classes_of(&data)?;
        let mut by_class: std::collections::BTreeMap<u32, Vec<String>> = Default::default();
        for node in &nodes {
            if Some(node.class_id) == classes.mesh {
                continue;
            }
            let payload = &data[node.payload()];
            let mut run = Vec::new();
            for &b in payload {
                if (0x20..0x7f).contains(&b) {
                    run.push(b);
                } else {
                    if run.len() >= 6 {
                        by_class
                            .entry(node.class_id)
                            .or_default()
                            .push(String::from_utf8_lossy(&run).into_owned());
                    }
                    run.clear();
                }
            }
        }
        for (id, mut found) in by_class {
            found.sort();
            found.dedup();
            println!("class {id:#06x}: {} distinct string(s)", found.len());
            for f in found.iter().take(5) {
                println!("    {f}");
            }
        }
        return Ok(());
    }

    if std::env::var("OAG_TRIS").is_ok() {
        // Every triangle the .rcsmodel holds, against what the build emits.
        let mut chunks = 0usize;
        let mut tris = 0usize;
        let mut submeshes = 0usize;
        for m in &model.meshes {
            chunks += 1;
            for sm in &m.submeshes {
                submeshes += 1;
                tris += sm.index_count / 3;
            }
        }
        let (built, report) = mesh::rcs::scene_from(&spec, &name, &data)?.unwrap();
        println!(
            "file: {chunks} chunk(s), {submeshes} submesh(es), {tris} triangle(s)\n\
             built: {} triangle(s) reported, {} index/3 in the buffers",
            report.triangles,
            built.indices.len() / 3
        );
        println!("{}", report.describe());
        return Ok(());
    }

    if std::env::var("OAG_CLASSES").is_ok() {
        let classes = vex::classes_of(&data)?;
        let mut by_class: std::collections::BTreeMap<u32, (usize, usize)> = Default::default();
        for node in &nodes {
            let e = by_class.entry(node.class_id).or_default();
            e.0 += 1;
            e.1 = e.1.max(data[node.payload()].len());
        }
        println!("{} node(s) over {} class(es)", nodes.len(), by_class.len());
        println!("known: {classes:?}");
        for (id, (n, longest)) in &by_class {
            let known = if Some(*id) == classes.mesh {
                " <- Mesh (the only class drawn)"
            } else {
                ""
            };
            println!("  class {id:#010x}: {n:5} node(s), longest payload {longest}{known}");
        }
        return Ok(());
    }

    let mut per_node: std::collections::BTreeMap<usize, usize> = Default::default();
    let mut at_offset: std::collections::BTreeMap<usize, usize> = Default::default();
    let mut lengths: std::collections::BTreeMap<usize, usize> = Default::default();
    for node in nodes.iter().filter(|n| n.class_id == mesh_class) {
        let payload = &data[node.payload()];
        *lengths.entry(payload.len()).or_default() += 1;
        let mut hits = 0;
        for at in (0..payload.len().saturating_sub(3)).step_by(4) {
            if model.mesh(order.u32(payload, at)).is_some() {
                hits += 1;
                *at_offset.entry(at).or_default() += 1;
            }
        }
        *per_node.entry(hits).or_default() += 1;
    }
    println!("{name}");
    println!("chunk hashes found per Mesh node: {per_node:?}");
    println!("payload lengths: {lengths:?}");
    println!("offsets that hold one, by hit count:");
    for (at, n) in at_offset.iter() {
        println!("  +{at:#06x}: {n}");
    }
    Ok(())
}
