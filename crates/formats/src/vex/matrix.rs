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

/// Where every node sits **relative to the `Anim Transform` above it**, and
/// which one that is.
///
/// One entry per node of `nodes`. `anchor` is the index of the nearest
/// `Anim Transform` on the ancestor chain (the node itself if it is one), and
/// `local` is the product of the matrices between that anchor and this node,
/// with the anchor's own contributing the identity.
///
/// This is the split a renderer needs and [`world_transforms_at`] cannot give
/// it. Vertices are baked into a buffer once, at load, so anything that moves
/// per frame has to stay *out* of the bake: bake `local`, and multiply by the
/// anchor's own world matrix - which [`anchor_world`] evaluates - in the shader.
///
/// A node with no `Anim Transform` above it gets `anchor: None` and its full
/// world matrix in `local`, which is exactly [`world_transforms`]'s answer, so
/// a caller can use one array for both cases.
#[must_use]
pub fn anim_anchors(data: &[u8], nodes: &[Node]) -> Vec<Anchored> {
    let classes = classes_of(data).ok();
    let transform_class = classes.and_then(|classes| classes.transform);
    let anim_class = classes.and_then(|classes| classes.anim_transform);
    let order = byte_order(data);
    let mut out: Vec<Anchored> = Vec::with_capacity(nodes.len());
    for (index, node) in nodes.iter().enumerate() {
        let parent = node.parent.and_then(|p| out.get(p).copied());
        let is_anim = Some(node.class_id) == anim_class;
        // An `Anim Transform` anchors *itself*: everything below it, including
        // its own local matrix, is evaluated per frame rather than baked.
        if is_anim {
            out.push(Anchored {
                anchor: Some(index),
                local: IDENTITY,
            });
            continue;
        }
        let local = if Some(node.class_id) == transform_class {
            data.get(node.payload())
                .and_then(|payload| transform(payload, order))
                .unwrap_or(IDENTITY)
        } else {
            IDENTITY
        };
        let above = parent.unwrap_or(Anchored {
            anchor: None,
            local: IDENTITY,
        });
        out.push(Anchored {
            anchor: above.anchor,
            local: multiply(&local, &above.local),
        });
    }
    out
}

/// One node's placement relative to the `Anim Transform` above it. See
/// [`anim_anchors`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Anchored {
    /// Index of the nearest `Anim Transform` ancestor, itself included, or
    /// `None` where the node is placed statically.
    pub anchor: Option<usize>,
    /// The matrix between that anchor and this node - or the full world matrix
    /// when there is no anchor.
    pub local: [f32; 16],
}

/// The world matrix of every `Anim Transform`, evaluated at `seconds`.
///
/// `None` for every other node. Composes through nested anchors: 14 of Pulse's
/// 393 `Anim Transform` nodes sit under another one, so an anchor's own parent
/// chain can itself be moving and cannot be folded into the bake either.
///
/// Pair with [`anim_anchors`]: a vertex baked with `local` and drawn through
/// `anchor_world[anchor]` lands exactly where [`world_transforms_at`] would put
/// it, and moves when the node does.
#[must_use]
pub fn anchor_world(data: &[u8], nodes: &[Node], seconds: f32) -> Vec<Option<[f32; 16]>> {
    let anim_class = classes_of(data)
        .ok()
        .and_then(|classes| classes.anim_transform);
    let anchors = anim_anchors(data, nodes);
    let mut out: Vec<Option<[f32; 16]>> = vec![None; nodes.len()];
    for (index, node) in nodes.iter().enumerate() {
        if Some(node.class_id) != anim_class {
            continue;
        }
        let local = super::anim_transform_of(data, node).map_or(IDENTITY, |a| a.sample(seconds));
        // The chain above this node: static matrices up to the next anchor,
        // then that anchor's own world matrix, which is already resolved
        // because nodes precede their descendants in the tree.
        let above = node.parent.and_then(|p| anchors.get(p).copied());
        let (static_above, parent_anchor) = above.map_or((IDENTITY, None), |a| (a.local, a.anchor));
        let parent_world = parent_anchor
            .and_then(|p| out.get(p).copied().flatten())
            .unwrap_or(IDENTITY);
        out[index] = Some(multiply(&multiply(&local, &static_above), &parent_world));
    }
    out
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

/// World matrix of every node, composed down the tree, at time zero.
///
/// [`world_transforms_at`] with `seconds` of `0.0`. Every caller that places
/// static geometry wants this one; a caller that animates wants the other and
/// has to say when.
pub fn world_transforms(data: &[u8], nodes: &[Node]) -> Vec<[f32; 16]> {
    world_transforms_at(data, nodes, 0.0)
}

/// World matrix of every node, composed down the tree, with every
/// [`AnimTransform`](super::AnimTransform) evaluated at `seconds`.
///
/// One entry per node of [`nodes`], in the same order. A node's matrix is the
/// product of every `Transform` matrix on its ancestor chain, itself included, so
/// a `Mesh` can be placed with a single lookup.
///
/// This is what makes a whole track assemblable: mesh vertices are in the local
/// space of whichever transform encloses them, and on `01_Track` a mesh sits
/// under up to 25 nested transforms.
///
/// **`Anim Transform` contributes its evaluated matrix, not the identity.**
/// Treating it as the identity - which this function did until 2026-08-18 -
/// drops the node's placement along with its animation, and left 245 of the 474
/// meshes below one at the world origin. Every other class still contributes
/// the identity, because a track's assembly must not depend on guessing at
/// payloads this crate does not decode.
///
/// `step` is `FixedFrames`, which lives on the node's attribute list rather
/// than in its payload; this crate does not decode that list, so it is passed
/// as `false`. Between two 60 Hz keys the difference is a blend the original
/// would have snapped, which moves a stepped object early by at most one key
/// rather than putting it anywhere else. Recorded in
/// `docs/rendering/scenery-animation.md` as the one part of the reproduction
/// that is knowingly approximate.
pub fn world_transforms_at(data: &[u8], nodes: &[Node], seconds: f32) -> Vec<[f32; 16]> {
    // **From the file's own version word.** `CLASS_TRANSFORM` is version 6's
    // `0x6e`; a version-4 file numbers `Transform` `0x6d`, so composing against
    // the constant would leave every matrix at identity and pile a whole track
    // into one space. `None` for a version whose id is unrecovered, which gives
    // the same identity-everywhere result - but only where nobody has measured
    // otherwise, rather than on a title that has been.
    let classes = classes_of(data).ok();
    let transform_class = classes.and_then(|classes| classes.transform);
    let anim_class = classes.and_then(|classes| classes.anim_transform);
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
        } else if Some(node.class_id) == anim_class {
            // An `Anim Transform` this crate cannot decode falls back to the
            // identity, which is the old behaviour and is wrong in a visible
            // way - but a half-read payload would place the mesh somewhere
            // arbitrary, which is worse and harder to spot.
            //
            // `anim_transform_of` rather than `anim_transform`: `LoopEnd` and
            // `FixedFrames` are node attributes, and without them a looping
            // object runs on past its authored end and a stepped one smears
            // between its key pairs.
            super::anim_transform_of(data, node).map_or(IDENTITY, |anim| anim.sample(seconds))
        } else {
            IDENTITY
        };

        out.push(multiply(&local, &parent));
    }
    out
}
