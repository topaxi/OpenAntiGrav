//! What the `Anim Transform` decode moves on a Wipeout HD circuit.
//!
//! The before/after that matters for the renderer: a `Mesh` node under one used
//! to compose through the identity - `world_transforms` falls back to it when
//! the payload will not decode - so its chunk drew wherever the rest of the
//! chain put it. This reports how far each one moves now, and whether the
//! `is_world_baked` verdict `mesh::rcs::referenced` takes changes with it.

use oag_core::math::{Mat4, Vec3};
use oag_rcs::rcsmodel;
use oag_render::mesh;
use oag_vex::vex;

fn main() -> anyhow::Result<()> {
    let image = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "data/images/hdfury-ps3-eu-dec.iso".into());

    let mut total = 0usize;
    let mut moved = 0usize;
    let mut resolved = 0usize;
    let mut flipped = 0usize;
    let mut worst = 0.0f32;
    let mut spaces = std::collections::BTreeMap::new();

    for archive in ["DATA00.PSARC", "DATA02.PSARC"] {
        let spec = format!("{image}:PS3_GAME/USRDIR/{archive}");
        let Ok(mut open) = oag_assets::psarc::Archive::open(&spec) else {
            continue;
        };
        let tracks: Vec<String> = open
            .paths()
            .iter()
            .filter(|p| p.starts_with("/data/environments/") && p.ends_with("/track.vex"))
            .cloned()
            .collect();
        for path in tracks {
            let Ok(data) = open.read_path(&path) else {
                continue;
            };
            let Some(model_blob) = mesh::rcs::sibling_geometry(&spec, &path, &data) else {
                continue;
            };
            let Ok(model) = rcsmodel::Model::parse(&model_blob) else {
                continue;
            };
            let classes = vex::classes_of(&data)?;
            let (Some(mesh_class), Some(anim_class)) = (classes.mesh, classes.anim_transform)
            else {
                continue;
            };
            let nodes = vex::nodes(&data)?;
            let order = vex::byte_order(&data);
            let anchors = vex::anim_anchors(&data, &nodes);
            let now = vex::world_transforms(&data, &nodes);
            // The old answer: every `Anim Transform` contributing the identity,
            // which is what a failed decode falls back to.
            let before = identity_at_anim(&data, &nodes, anim_class);

            let mut circuit_moved = 0usize;
            for (index, node) in nodes.iter().enumerate() {
                if node.class_id != mesh_class || anchors[index].anchor.is_none() {
                    continue;
                }
                total += 1;
                let payload = &data[node.payload()];
                if payload.len() < 0x34 {
                    continue;
                }
                let centre: [f32; 3] = std::array::from_fn(|k| {
                    (order.f32(payload, 0x10 + k * 4) + order.f32(payload, 0x20 + k * 4)) / 2.0
                });
                let a = Mat4::from_cols_array(&before[index])
                    .transform_point3(Vec3::from_array(centre));
                let b =
                    Mat4::from_cols_array(&now[index]).transform_point3(Vec3::from_array(centre));
                let d = a.distance(b);
                if d > 1e-3 {
                    moved += 1;
                    circuit_moved += 1;
                    worst = worst.max(d);
                }
                let hash = order.u32(payload, 0x30);
                if let Some(chunk) = model.mesh(hash) {
                    resolved += 1;
                    *spaces.entry(format!("{:?}", chunk.space)).or_insert(0usize) += 1;
                    // `is_world_baked` reads the chunk's own `+0x07` byte first
                    // and only falls back to the box test, so a changed
                    // `to_world` can flip the verdict on `Unknown` alone.
                    if matches!(chunk.space, rcsmodel::Space::Unknown(_)) {
                        flipped += 1;
                    }
                }
            }
            if circuit_moved > 0 {
                println!("{path}: {circuit_moved} anchored meshes move");
            }
        }
    }

    println!("\n{total} Mesh nodes anchored to an Anim Transform on a circuit");
    println!("{moved} land somewhere new, worst move {worst:.1} world units");
    println!("{resolved} resolve to a chunk; spaces {spaces:?}");
    println!("{flipped} could flip the is_world_baked verdict (Space::Unknown)");
    Ok(())
}

/// `world_transforms` as it behaved with the class undecodable: every
/// `Anim Transform` the identity.
fn identity_at_anim(data: &[u8], nodes: &[vex::Node], anim_class: u32) -> Vec<[f32; 16]> {
    let transform_class = vex::classes_of(data).ok().and_then(|c| c.transform);
    let order = vex::byte_order(data);
    let mut out: Vec<[f32; 16]> = Vec::with_capacity(nodes.len());
    for node in nodes {
        let parent = node
            .parent
            .and_then(|p| out.get(p).copied())
            .unwrap_or(vex::IDENTITY);
        let local = if Some(node.class_id) == transform_class {
            data.get(node.payload())
                .and_then(|p| vex::transform(p, order))
                .unwrap_or(vex::IDENTITY)
        } else {
            let _ = anim_class;
            vex::IDENTITY
        };
        out.push(vex::multiply(&local, &parent));
    }
    out
}
