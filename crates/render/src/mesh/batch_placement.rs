//! Where each GE batch of a `.vex` model landed in the [`Model`](super::Model)
//! built from it, and the matrix its vertices were baked with.
//!
//! A [`Model`](super::Model) flattens every batch of every node of one class
//! into one vertex buffer, one batch after another, in node order and list A
//! before list B - see `build_class`. Something that names a batch by its place
//! in the file, the way a Quake road span does
//! (`oag_vex::quake::Span::batch`), needs that walk's answer: which vertices of
//! the buffer are that batch's, and which matrix moved them there. This is the
//! same walk, reporting instead of building.
//! `batch_placements_tile_each_node` in
//! `crates/game/tests/quake_ripple_ground_truth.rs` holds it to
//! `build_class`'s own per-node ranges.

use anyhow::{Context, Result};
use oag_vex::{quake, vex};

use super::anim_node;

/// One batch's place in a built model.
#[derive(Debug, Clone, PartialEq)]
pub struct BatchPlacement {
    /// File offset of the batch's header.
    pub header: usize,
    /// Its vertices, as a range into [`super::Model::vertices`].
    pub vertices: std::ops::Range<u32>,
    /// The matrix `build_class` baked them with: the node's world matrix, or
    /// the matrix to its `Anim Transform` anchor when it moves.
    pub to_world: [f32; 16],
}

/// Every batch of every node of the class `pick` names, in the order the model
/// built from the same `data` and `pick` holds their vertices.
///
/// # Errors
///
/// A file whose class table or node tree will not read, or a batch that will
/// not decode - the same failures the build itself reports.
pub fn batch_placements(
    data: &[u8],
    pick: fn(vex::classes::Classes) -> Option<u32>,
) -> Result<Vec<BatchPlacement>> {
    let classes = vex::classes_of(data).context("class table")?;
    let Some(class_id) = pick(classes) else {
        return Ok(Vec::new());
    };
    let nodes = vex::nodes(data).context("walking the node tree")?;
    let anchors = vex::anim_anchors(data, &nodes);
    let (_, anim_slot) = anim_node::collect(data, &nodes, &anchors, classes);
    let anchor_world = vex::anchor_world(data, &nodes, 0.0);

    let mut out = Vec::new();
    let mut next = 0u32;
    for (index, node) in nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| n.class_id == class_id)
    {
        let payload = &data[node.payload()];
        let to_world = anim_node::placement(&anchors, &anchor_world, &anim_slot, index).to_world;
        for list in [0u8, 1] {
            let batches = vex::mesh_batches(payload, list).context("decoding batches")?;
            let headers = quake::batch_offsets(payload, list);
            for (batch, header) in batches.iter().zip(headers) {
                let count = u32::try_from(batch.vertices.len()).unwrap_or(u32::MAX);
                out.push(BatchPlacement {
                    header: node.payload().start + header,
                    vertices: next..next + count,
                    to_world,
                });
                next += count;
            }
        }
    }
    Ok(out)
}
