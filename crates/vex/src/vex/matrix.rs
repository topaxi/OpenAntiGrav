//! Node matrices, and composing them into world space.
//!
//! Split out of `vex.rs` for size alone. Everything here is row-major with
//! the translation in row 3 - the row-vector convention, `v' = v * M`.

use oag_formats::ByteOrder;

use super::{Node, byte_order, classes_of};

/// Model-space matrices of every node of `class_id` that carries a 4x4 payload.
///
/// The locator classes (`Engine Flare`, `Ship Muzzle`, `Ship Collision Fx`,
/// `cannon_flash`, `Start Position`) store a **64-byte payload in exactly the
/// layout [`transform`] reads**. Not assumed: [`crate::track::start_position`]
/// decodes `Start Position` that way, corroborated against the running game to
/// within 1.12 degrees of heading (`docs/formats/track.md`).
///
/// [`world_transforms`] treats every non-`Transform` class as the identity, so a
/// track's assembly does not depend on guessing at payloads; this is the opposite
/// trade, taken explicitly for one class at a time, composing with the parent
/// chain the same way.
///
/// Nodes whose payload is too short to be a matrix are skipped, not defaulted to
/// the identity: a locator at the origin and one that failed to decode must not
/// look the same.
pub fn class_world_transforms(data: &[u8], nodes: &[Node], class_id: u32) -> Vec<[f32; 16]> {
    named_class_world_transforms(data, nodes, class_id)
        .into_iter()
        .map(|(_, matrix)| matrix)
        .collect()
}

/// [`class_world_transforms`], each matrix beside its node's name, for a class
/// whose nodes the original tells apart by name (HD's two `cannon_flash`
/// locators).
pub fn named_class_world_transforms<'a>(
    data: &[u8],
    nodes: &'a [Node],
    class_id: u32,
) -> Vec<(Option<&'a str>, [f32; 16])> {
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
            Some((node.name.as_deref(), multiply(&local, &parent)))
        })
        .collect()
}

/// One step of a node's ancestry, node first.
#[derive(Debug, Clone, PartialEq)]
enum Link {
    /// A fixed local matrix: the node's own, or a `Transform`'s.
    Fixed([f32; 16]),
    /// An `Anim Transform`, sampled on the scenery clock.
    Anim(Box<super::AnimTransform>),
}

/// One node's world matrix as a function of time, with nothing else of the file.
///
/// What [`world_transforms_at`] computes for the whole tree, kept for the one node
/// that is asked about every frame: a placed effect or a light a craft passes. The
/// chain owns its matrices and tracks, so it outlives the file's bytes.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeChain {
    links: Vec<Link>,
}

impl NodeChain {
    /// The chain of `nodes[index]`: `own` as its local matrix, then each ancestor's
    /// `Transform` matrix or `Anim Transform` track, every other class the identity,
    /// root last. The same product [`world_transforms_at`] builds for that node.
    ///
    /// `own` is the node's payload matrix for a class that carries one (a
    /// particle system) and [`IDENTITY`] for a class that does not (a light),
    /// which is what [`world_transforms_at`] gives every class but those two.
    #[must_use]
    pub fn of(data: &[u8], nodes: &[Node], index: usize, own: [f32; 16]) -> Self {
        let classes = classes_of(data).ok();
        let transform_class = classes.and_then(|classes| classes.transform);
        let anim_class = classes.and_then(|classes| classes.anim_transform);
        let order = byte_order(data);
        let mut links = vec![Link::Fixed(own)];
        let mut at = nodes.get(index).and_then(|node| node.parent);
        while let Some(p) = at {
            let ancestor = &nodes[p];
            if Some(ancestor.class_id) == transform_class {
                links.push(Link::Fixed(
                    data.get(ancestor.payload())
                        .and_then(|payload| transform(payload, order))
                        .unwrap_or(IDENTITY),
                ));
            } else if Some(ancestor.class_id) == anim_class {
                links.push(
                    super::anim_transform_of(data, ancestor)
                        .map_or(Link::Fixed(IDENTITY), |anim| Link::Anim(Box::new(anim))),
                );
            }
            at = ancestor.parent;
        }
        Self { links }
    }

    /// The node's world matrix at `seconds` into the scenery clock, row-major
    /// with the translation in row 3.
    #[must_use]
    pub fn world_at(&self, seconds: f32) -> [f32; 16] {
        self.links.iter().fold(IDENTITY, |world, link| {
            let local = match link {
                Link::Fixed(m) => *m,
                Link::Anim(anim) => anim.sample(seconds),
            };
            multiply(&world, &local)
        })
    }

    /// Whether any ancestor moves, so [`Self::world_at`] depends on time.
    #[must_use]
    pub fn moves(&self) -> bool {
        self.links.iter().any(|link| matches!(link, Link::Anim(_)))
    }
}

/// Where every node sits **relative to the `Anim Transform` above it**, and which
/// one that is.
///
/// One entry per node. `anchor` is the nearest `Anim Transform` on the ancestor
/// chain (the node itself if it is one); `local` is the product of the matrices
/// between anchor and node, the anchor's own contributing the identity.
///
/// The split a renderer needs and [`world_transforms_at`] cannot give: vertices
/// are baked once at load, so anything moving per frame stays out of the bake.
/// Bake `local`, multiply by the anchor's world matrix ([`anchor_world`]) in the
/// shader.
///
/// A node with no `Anim Transform` above it gets `anchor: None` and its full
/// world matrix in `local` ([`world_transforms`]'s answer), so one array serves
/// both cases.
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
        // An `Anim Transform` anchors *itself*: it and everything below, its own
        // local matrix included, is evaluated per frame rather than baked.
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

/// The world matrix of every `Anim Transform`, evaluated at `seconds`; `None` for
/// every other node. Composes through nested anchors (14 of Pulse's 393 sit under
/// another), whose parent chain can itself be moving.
///
/// Pair with [`anim_anchors`]: a vertex baked with `local` and drawn through
/// `anchor_world[anchor]` lands where [`world_transforms_at`] would put it and
/// moves with the node.
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
        // The chain above this node: static matrices up to the next anchor, then
        // that anchor's world matrix, already resolved (nodes precede their
        // descendants).
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
/// **Row-major, translation in row 3**, the row-vector convention `v' = v * M`.
/// Rows 0 to 2 are an orthonormal basis in every node checked, row 3 ends in
/// `1.0`. An empty payload is the identity (57 of `01_Track`'s 715 transforms).
///
/// Corroborated outside this format: the `Start Position` bind forces **row 1**
/// to `(0, 1, 0)` when re-orthonormalising a grid slot, so row 1 is up and `+y`
/// is world up (`docs/formats/track.md`).
///
/// `order` is the containing file's ([`byte_order`]): a payload carries no magic
/// (a PS3 track authors 28,545 `Transform` nodes, so this decoder has to be told).
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

/// Applies the rotation part of a row-major matrix to a direction (no
/// translation, no `w`): `n * M3x3`, the product the GE takes a vertex normal
/// through, by the world matrix and by nothing else.
///
/// Not renormalised. The GE does, so a caller drawing through a matrix that
/// scales has to; every node matrix on the discs is orthonormal (the largest
/// deviation over the rotated `Mesh` nodes is 1.2e-7,
/// `vex_node_normals_ground_truth.rs`), where this and the inverse transpose
/// are the same thing.
#[must_use]
pub fn transform_direction(m: &[f32; 16], n: [f32; 3]) -> [f32; 3] {
    [
        n[0] * m[0] + n[1] * m[4] + n[2] * m[8],
        n[0] * m[1] + n[1] * m[5] + n[2] * m[9],
        n[0] * m[2] + n[1] * m[6] + n[2] * m[10],
    ]
}

/// Whether the rotation part of `m` is exactly the identity, so a direction
/// carried through it comes out bit-for-bit as it went in. The 3x3 only: a node
/// that is nothing but a translation (most of a circuit's) qualifies.
#[must_use]
pub fn leaves_directions(m: &[f32; 16]) -> bool {
    (0..3).all(|row| (0..3).all(|col| m[row * 4 + col] == IDENTITY[row * 4 + col]))
}

/// World matrix of every node, composed down the tree, at time zero.
///
/// [`world_transforms_at`] at `0.0`: what callers placing static geometry want.
pub fn world_transforms(data: &[u8], nodes: &[Node]) -> Vec<[f32; 16]> {
    world_transforms_at(data, nodes, 0.0)
}

/// World matrix of every node, composed down the tree, with every
/// [`AnimTransform`](super::AnimTransform) evaluated at `seconds`.
///
/// One entry per node of [`nodes`], in order: the product of every `Transform`
/// matrix on its ancestor chain, itself included, so a `Mesh` is placed with one
/// lookup. This is what makes a track assemblable: mesh vertices are in the local
/// space of the enclosing transform, and on `01_Track` a mesh sits under up to 25
/// nested transforms.
///
/// **`Anim Transform` contributes its evaluated matrix, not the identity**:
/// treating it as the identity dropped its placement with its animation and left
/// 245 of the 474 meshes below one at the world origin. Every other class
/// contributes the identity, so a track's assembly does not depend on guessing at
/// undecoded payloads. `LoopEnd` and `FixedFrames` node attributes are applied
/// ([`super::anim_transform_of`]); see `docs/rendering/scenery-animation.md`.
pub fn world_transforms_at(data: &[u8], nodes: &[Node], seconds: f32) -> Vec<[f32; 16]> {
    // **From the file's own version word.** `CLASS_TRANSFORM` is version 6's
    // `0x6e`; version 4 numbers it `0x6d`, so composing against the constant
    // would leave every matrix at identity and pile a track into one space.
    // `None` for a version whose id is unrecovered gives the same result, but only
    // where nobody has measured otherwise.
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
            // identity, wrong visibly but better than a half-read payload placing
            // the mesh somewhere arbitrary.
            //
            // `anim_transform_of` rather than `anim_transform`: `LoopEnd` and
            // `FixedFrames` are node attributes, and without them a looping object
            // runs past its authored end and a stepped one smears between key pairs.
            super::anim_transform_of(data, node).map_or(IDENTITY, |anim| anim.sample(seconds))
        } else {
            IDENTITY
        };

        out.push(multiply(&local, &parent));
    }
    out
}
