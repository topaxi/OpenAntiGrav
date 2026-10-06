//! [`AnimNode`]: an `Anim Transform` a model carries, and resolving it to a
//! matrix at a given time.
//!
//! Split out of `mesh.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change.

use super::*;

/// How many `Anim Transform` nodes one model may carry, matching
/// `shaders/types.wesl`'s `NodeAnims` array.
///
/// Slot 0 is the identity, so a model gets `NODE_ANIM_LIMIT - 1` real nodes.
///
/// **Measured, not guessed, and Wipeout HD is what sets it.** Pulse's busiest
/// circuit is `16_Track` with 74 nodes, against 393 over all twelve - 128 was
/// a factor of 1.7 on that. HD authors **5,518** across its seven archives and
/// its busiest file is `modesto_heights/track.vex` with **259**, so 128 would
/// have frozen 131 of that circuit's moving objects while drawing them in the
/// right place. 384 is a factor of 1.48 on the worst file measured, and 384
/// mat4s is 24 KiB of uniform - still well inside the 64 KiB binding every
/// backend guarantees. `scenery_animation_ground_truth.rs` re-measures Pulse's
/// side and `crates/render/examples/hd_anim_census.rs` sweeps HD's.
///
/// Exceeding it is graceful: the nodes past the ceiling draw at their
/// time-zero placement rather than the build failing - which is a frozen
/// object, not a misplaced one.
pub const NODE_ANIM_LIMIT: usize = 384;

/// What moves a node: the two authored forms a local matrix at a time comes
/// from.
///
/// Both answer the same question - "this node's local matrix at `seconds`" -
/// and [`Model::sample_anim_nodes`] composes either through the same chain,
/// so the shader's table and the per-frame upload never learn which title
/// authored the node.
///
/// Both variants are boxed: a `.vex` transform is 288 bytes of key vectors
/// and the rig's nine channel slots twice that, and a table of a few
/// hundred is walked once a frame.
#[derive(Debug, Clone, PartialEq)]
pub enum Motion {
    /// A `.vex` `Anim Transform` node: Pulse, Pure and Wipeout HD.
    Vex(Box<vex::AnimTransform>),
    /// A `.rcsskeleton` node under its `.rcsanimclip` track: Wipeout 2048,
    /// whose `track.vex` authors no `Anim Transform` at all.
    Rig(Box<oag_rcs::rig::NodeMotion>),
}

impl Motion {
    /// The local matrix at `seconds`, row-major with the translation in row
    /// 3 - [`vex::AnimTransform::sample`]'s convention, which
    /// [`oag_rcs::rig::NodeMotion::sample`] shares.
    #[must_use]
    pub fn sample(&self, seconds: f32) -> [f32; 16] {
        match self {
            Self::Vex(transform) => transform.sample(seconds),
            Self::Rig(motion) => motion.sample(seconds),
        }
    }
}

/// One animated node a model carries, with everything needed to place it
/// again at another time.
///
/// [`Model`] does not keep the file it was built from, so the parent chain is
/// resolved here at load: [`Self::static_above`] is the product of the static
/// matrices between this node and the next animated node above it, and
/// [`Self::parent`] indexes that node in [`Model::anim_nodes`]. 14 of Pulse's
/// 393 sit under another one, so the chain is not optional.
#[derive(Debug, Clone, PartialEq)]
pub struct AnimNode {
    /// The node's own authored channels.
    pub transform: Motion,
    /// The static matrices between this node and its parent anchor.
    pub static_above: [f32; 16],
    /// The parent anchor's slot in [`Model::anim_nodes`], if it has one.
    pub parent: Option<usize>,
}

impl Model {
    /// Every entry of [`Self::anim_nodes`] resolved to a world matrix at
    /// `seconds`, in the order the shader's table wants them.
    ///
    /// Slot 0 of the table is the identity and is not produced here; the caller
    /// writes these from slot 1. Nodes past [`NODE_ANIM_LIMIT`] `- 1` are
    /// dropped, which freezes them at whatever the table still holds.
    #[must_use]
    pub fn sample_anim_nodes(&self, seconds: f32) -> Vec<[f32; 16]> {
        let mut out: Vec<[f32; 16]> = Vec::with_capacity(self.anim_nodes.len());
        for node in self.anim_nodes.iter().take(NODE_ANIM_LIMIT - 1) {
            let local = node.transform.sample(seconds);
            // A parent always precedes its child in `anim_nodes`, because
            // `vex::nodes` is a pre-order walk and the slots are assigned in
            // that order - so the parent's world matrix is already resolved.
            let parent = node
                .parent
                .and_then(|p| out.get(p).copied())
                .unwrap_or(vex::IDENTITY);
            out.push(vex::multiply(
                &vex::multiply(&local, &node.static_above),
                &parent,
            ));
        }
        out
    }
}

/// Which slot of [`Model::anim_nodes`] each `Anim Transform` node index took.
pub(super) type AnimSlots = std::collections::BTreeMap<usize, usize>;

/// Every `Anim Transform` in the file, in tree order, with its chain resolved.
///
/// Tree order is load-bearing rather than tidy: a parent always lands in a
/// lower slot than its child, which is what lets [`Model::sample_anim_nodes`]
/// resolve a nested chain in one forward pass instead of recursing.
///
/// A node whose payload will not decode is skipped rather than defaulted to the
/// identity, so it takes no slot and the geometry under it keeps `xform` of 0 -
/// drawn at its time-zero placement, frozen rather than moved to nowhere.
pub(super) fn collect(
    data: &[u8],
    nodes: &[vex::Node],
    anchors: &[vex::Anchored],
    classes: vex::classes::Classes,
) -> (Vec<AnimNode>, AnimSlots) {
    let mut out: Vec<AnimNode> = Vec::new();
    let mut slots = AnimSlots::new();
    let Some(anim_class) = classes.anim_transform else {
        return (out, slots);
    };
    for (index, node) in nodes.iter().enumerate() {
        if node.class_id != anim_class {
            continue;
        }
        let Some(transform) = vex::anim_transform_of(data, node) else {
            continue;
        };
        let above = node.parent.and_then(|p| anchors.get(p).copied());
        let (static_above, parent_node) =
            above.map_or((vex::IDENTITY, None), |a| (a.local, a.anchor));
        let slot = out.len();
        out.push(AnimNode {
            transform: Motion::Vex(Box::new(transform)),
            static_above,
            parent: parent_node.and_then(|p| slots.get(&p).copied()),
        });
        slots.insert(index, slot);
    }
    (out, slots)
}

/// How one node's geometry is baked and drawn: see [`placement`].
#[derive(Debug, Clone, Copy)]
pub(super) struct Placement {
    /// The matrix to bake this node's vertices with.
    pub(super) to_world: [f32; 16],
    /// The [`GpuVertex::xform`] slot those vertices carry.
    pub(super) xform: u32,
    /// The matrix that lifts an anchor-space bounding sphere into world space,
    /// or `None` when the vertices are already in world space.
    pub(super) bounds_matrix: Option<[f32; 16]>,
}

impl Placement {
    /// Geometry that is already in world space and moves for nobody.
    ///
    /// Wipeout HD's second pass, which draws the 913 chunks of a circuit that
    /// no `Mesh` node addresses: they carry a world-space bias and there is no
    /// node above them to be anchored to. See `mesh::rcs::build_with_options`.
    pub(super) const STATIC: Self = Self {
        to_world: vex::IDENTITY,
        xform: 0,
        bounds_matrix: None,
    };
}

/// Decides whether node `index` is baked in world space or in its anchor's.
///
/// The three answers move together and getting one of them out of step is the
/// bug this exists to make impossible: geometry baked in anchor space with an
/// `xform` of 0 draws at the origin, and geometry baked in world space with an
/// `xform` set draws with the anchor's matrix applied twice.
///
/// **Past [`NODE_ANIM_LIMIT`] the node falls back to the world-space bake**, so
/// it lands where the file puts it and simply does not move - frozen rather than
/// misplaced, which is the same graceful direction the texture-transform
/// ceiling takes.
pub(super) fn placement(
    anchors: &[vex::Anchored],
    anchor_world: &[Option<[f32; 16]>],
    slots: &AnimSlots,
    index: usize,
) -> Placement {
    let anchored = anchors.get(index).copied().unwrap_or(vex::Anchored {
        anchor: None,
        local: vex::IDENTITY,
    });
    let xform = anchored
        .anchor
        .and_then(|a| slots.get(&a).copied())
        .filter(|slot| slot + 1 < NODE_ANIM_LIMIT)
        .map_or(0, |slot| u32::try_from(slot + 1).unwrap_or(0));
    let at_zero = anchored
        .anchor
        .and_then(|a| anchor_world.get(a).copied().flatten());
    if xform == 0 {
        return Placement {
            to_world: at_zero.map_or(anchored.local, |a| vex::multiply(&anchored.local, &a)),
            xform,
            bounds_matrix: None,
        };
    }
    Placement {
        to_world: anchored.local,
        xform,
        // The bounds a draw call is culled by are world-space, so an anchored
        // batch's have to be lifted back out of anchor space.
        bounds_matrix: at_zero,
    }
}
