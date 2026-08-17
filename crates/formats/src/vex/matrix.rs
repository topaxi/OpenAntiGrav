//! Node matrices, and composing them into world space.
//!
//! Split out of `vex.rs` for size alone. Everything here is row-major with
//! the translation in row 3 - the row-vector convention, `v' = v * M`.

use crate::ByteOrder;

use super::{Node, byte_order, classes_of};

/// Model-space matrices of every node of `class_id` that carries a 4x4 payload.
///
/// The locator classes - `Engine Flare`, `Ship Muzzle`, `Ship Collision Fx`,
/// `cannon_flash`, `Start Position` - all store a **64-byte payload in exactly
/// the layout [`transform`] reads**: row-major, translation in row 3. That is not
/// assumed here; [`crate::track::start_position`] already decodes `Start
/// Position` that way, and its reading is corroborated against the running game
/// to within 1.12 degrees of heading (see `docs/formats/track.md`).
///
/// [`world_transforms`] deliberately treats every non-`Transform` class as the
/// identity, because a track's assembly must not depend on guessing at payloads
/// it does not decode. This function is the opposite trade, taken explicitly for
/// one class at a time, and it composes with the parent chain the same way.
///
/// Nodes whose payload is too short to be a matrix are skipped rather than
/// defaulted to the identity: a locator at the origin and a locator that failed
/// to decode should not look the same to a caller.
pub fn class_world_transforms(data: &[u8], nodes: &[Node], class_id: u32) -> Vec<[f32; 16]> {
    let chain = world_transforms(data, nodes);
    nodes
        .iter()
        .filter(|node| node.class_id == class_id)
        .filter_map(|node| {
            let local = transform(data.get(node.payload())?, byte_order(data))?;
            let parent = node
                .parent
                .and_then(|p| chain.get(p).copied())
                .unwrap_or(IDENTITY);
            Some(multiply(&local, &parent))
        })
        .collect()
}

/// A `Transform` node's matrix, or `None` if the payload is not one.
///
/// **Row-major, translation in row 3**, which is the row-vector convention:
/// `v' = v * M`. Rows 0 to 2 are an orthonormal basis in every node checked, and
/// row 3 ends in `1.0`.
///
/// A `Transform` with an empty payload is the identity, which is how 57 of
/// `01_Track`'s 715 transforms are stored.
///
/// The convention is corroborated outside this format: the `Start Position` bind
/// forces **row 1** to `(0, 1, 0)` when it re-orthonormalises a grid slot, so row
/// 1 is the up axis and `+y` is world up. See `docs/formats/track.md`.
///
/// `order` is the containing file's, from [`byte_order`]. A payload carries no
/// magic of its own, so it cannot say - and a PS3 track authors 28,545
/// `Transform` nodes, so this is one of the decoders that has to be told.
#[must_use]
pub fn transform(payload: &[u8], order: ByteOrder) -> Option<[f32; 16]> {
    if payload.is_empty() {
        return Some(IDENTITY);
    }
    if payload.len() < 64 {
        return None;
    }
    let mut m = [0.0f32; 16];
    for (i, cell) in m.iter_mut().enumerate() {
        *cell = order.f32(payload, i * 4);
    }
    Some(m)
}

/// The identity, in the same row-major layout as [`transform`].
pub const IDENTITY: [f32; 16] = [
    1.0, 0.0, 0.0, 0.0, //
    0.0, 1.0, 0.0, 0.0, //
    0.0, 0.0, 1.0, 0.0, //
    0.0, 0.0, 0.0, 1.0, //
];

/// Multiplies two row-major matrices: the result applies `a` then `b`.
#[must_use]
pub fn multiply(a: &[f32; 16], b: &[f32; 16]) -> [f32; 16] {
    let mut out = [0.0f32; 16];
    for row in 0..4 {
        for col in 0..4 {
            let mut sum = 0.0;
            for k in 0..4 {
                sum += a[row * 4 + k] * b[k * 4 + col];
            }
            out[row * 4 + col] = sum;
        }
    }
    out
}

/// Applies a row-major matrix to a point, with an implicit `w` of 1.
#[must_use]
pub fn transform_point(m: &[f32; 16], p: [f32; 3]) -> [f32; 3] {
    [
        p[0] * m[0] + p[1] * m[4] + p[2] * m[8] + m[12],
        p[0] * m[1] + p[1] * m[5] + p[2] * m[9] + m[13],
        p[0] * m[2] + p[1] * m[6] + p[2] * m[10] + m[14],
    ]
}

/// World matrix of every node, composed down the tree.
///
/// One entry per node of [`nodes`], in the same order. A node's matrix is the
/// product of every `Transform` matrix on its ancestor chain, itself included, so
/// a `Mesh` can be placed with a single lookup.
///
/// This is what makes a whole track assemblable: mesh vertices are in the local
/// space of whichever transform encloses them, and on `01_Track` a mesh sits
/// under up to 25 nested transforms.
pub fn world_transforms(data: &[u8], nodes: &[Node]) -> Vec<[f32; 16]> {
    // **From the file's own version word.** `CLASS_TRANSFORM` is version 6's
    // `0x6e`; a version-4 file numbers `Transform` `0x6d`, so composing against
    // the constant would leave every matrix at identity and pile a whole track
    // into one space. `None` for a version whose id is unrecovered, which gives
    // the same identity-everywhere result - but only where nobody has measured
    // otherwise, rather than on a title that has been.
    let transform_class = classes_of(data).ok().and_then(|classes| classes.transform);
    let order = byte_order(data);
    let mut out: Vec<[f32; 16]> = Vec::with_capacity(nodes.len());
    for node in nodes {
        let parent = node
            .parent
            .and_then(|p| out.get(p).copied())
            .unwrap_or(IDENTITY);

        let local = if Some(node.class_id) == transform_class {
            data.get(node.payload())
                .and_then(|payload| transform(payload, order))
                .unwrap_or(IDENTITY)
        } else {
            IDENTITY
        };

        out.push(multiply(&local, &parent));
    }
    out
}
