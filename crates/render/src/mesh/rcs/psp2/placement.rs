//! Where a 2048 node-bound submesh is baked and which table slot moves it.
//!
//! The 2048 counterpart of `mesh::anim_node::placement`, on a
//! `.rcsskeleton` instead of a `.vex`: a node the clip moves - or that
//! hangs under one it moves - keeps its vertices in its own space and takes
//! a slot of the shader's node table, and every other node bakes its
//! vertices through its static world matrix once at load. The three answers
//! move together here for the same reason they do there: anchor-space
//! vertices with a slot of 0 draw at the origin, and world-space vertices
//! with a slot set are moved twice.
//!
//! **A model with no skeleton beside it still places its nodes.** A craft's
//! airbrakes are node-bound with the node's matrix at the tail and the
//! vertices about their own hinge, so drawing them as-is puts both flaps
//! under the cockpit. The model's own bind matrices place those; the
//! skeleton's pivots are only reachable when there is a skeleton.

use oag_rcs::rcsmodel::psp2;
use oag_rcs::rcsskeleton::{IDENTITY, Skeleton, multiply};
use oag_rcs::rig::NodeMotion;
use oag_rcs::{rcsanimclip, rcsskeleton};

use oag_vex::vex;

use crate::mesh::anim_node::{self, AnimNode, Motion, NODE_ANIM_LIMIT};

/// The two files that animate a 2048 model, parsed.
#[derive(Debug, Clone, PartialEq)]
pub struct Animation {
    /// The node hierarchy and bind pose.
    pub skeleton: Skeleton,
    /// The keys, or `None` for a skeleton with nothing moving it.
    pub clip: Option<rcsanimclip::Clip>,
}

impl Animation {
    /// Parses the two sibling files.
    ///
    /// # Errors
    ///
    /// Either file's own parse error.
    pub fn parse(skeleton: &[u8], clip: Option<&[u8]>) -> psp2::Result<Self> {
        Ok(Self {
            skeleton: rcsskeleton::parse(skeleton)?,
            clip: clip.map(rcsanimclip::parse).transpose()?,
        })
    }
}

/// How one node's geometry is baked and drawn.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placement {
    /// The matrix to bake the node's vertices with: its static world matrix,
    /// or the identity for a node the table moves.
    pub to_world: [f32; 16],
    /// The `GpuVertex::xform` slot, `0` for a node baked in world space.
    pub xform: u32,
    /// The node's world matrix at time zero, for a moving node's bounds.
    pub world_at_zero: [f32; 16],
    /// Whether the node is hidden and never shown - its geometry is not
    /// emitted at all.
    pub hidden: bool,
    /// Whether the file gives this node no transform at all - no written bind
    /// matrix and no skeleton entry - so nothing authored says where its
    /// geometry goes. Such a node is [`Self::hidden`] too: drawn nowhere and
    /// counted, rather than left at its node-local position, which is where
    /// the file does *not* put it.
    pub unplaced: bool,
}

impl Placement {
    /// Geometry already in world space that moves for nobody.
    pub const STATIC: Self = Self {
        to_world: IDENTITY,
        xform: 0,
        world_at_zero: IDENTITY,
        hidden: false,
        unplaced: false,
    };
}

/// The plan for one model: a placement per model node, and the table.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Plan {
    /// One per entry of the model's own node table, in that order.
    pub placements: Vec<Placement>,
    /// The nodes the shader's table moves, parents before children.
    pub anim_nodes: Vec<AnimNode>,
    /// Model nodes the skeleton did not name, placed by the model's bind
    /// matrix instead.
    pub unmatched: usize,
    /// Nodes that wanted a slot past [`NODE_ANIM_LIMIT`] and were frozen at
    /// time zero instead.
    pub frozen: usize,
}

/// Where a node bakes by its own bind matrix alone.
///
/// A node past the model's written bind count has no matrix of its own, and
/// with no skeleton entry either nothing authored says where its vertices -
/// which are in the node's space - belong. Not drawn, and counted as unplaced:
/// an identity there would draw the geometry at the node's own origin, which is
/// the one place the file does not put it (on Omega's first frame that was a
/// prop under the camera).
fn by_bind(n: &psp2::nodes::Node) -> Placement {
    match n.bind {
        Some(bind) => Placement {
            to_world: bind,
            xform: 0,
            world_at_zero: bind,
            hidden: false,
            unplaced: false,
        },
        None => Placement {
            hidden: true,
            unplaced: true,
            ..Placement::STATIC
        },
    }
}

/// Plans every node of `scene`.
///
/// Without `animation`, every node-bound mesh bakes through the model's
/// bind matrix and nothing takes a slot. With one, each model node is found
/// in the skeleton by id; a node the clip moves, or under one it moves,
/// takes a slot, and the rest bake through the skeleton's own composition
/// (pivots included), which is where the model's bind matrix and the
/// skeleton disagree - see `oag_rcs::rcsskeleton::Node::local`.
#[must_use]
pub fn plan(scene: &psp2::nodes::Scene, animation: Option<&Animation>) -> Plan {
    let Some(animation) = animation else {
        return Plan {
            placements: scene.nodes.iter().map(by_bind).collect(),
            ..Plan::default()
        };
    };
    let skeleton = &animation.skeleton;
    let motions: Vec<NodeMotion> = skeleton
        .nodes
        .iter()
        .map(|n| NodeMotion {
            node: n.clone(),
            track: animation.clip.as_ref().and_then(|c| c.track(n.id)).cloned(),
        })
        .collect();
    let order = skeleton.order();
    // A node moves if it or any ancestor has keys, and a static node is
    // hidden for good if it or any ancestor is authored invisible with no
    // keys to ever show it - visibility inherits down a Maya hierarchy.
    let mut moving = vec![false; skeleton.nodes.len()];
    let mut hidden = vec![false; skeleton.nodes.len()];
    for &i in &order {
        let parent = skeleton.nodes[i].parent;
        moving[i] = parent.is_some_and(|p| moving[p]) || motions[i].is_animated();
        hidden[i] =
            !moving[i] && (parent.is_some_and(|p| hidden[p]) || !motions[i].visible_at(0.0));
    }
    // Static world matrices, composed with the pivots; a moving node's is
    // its time-zero world, which is what its bounds are lifted by.
    let mut world = vec![IDENTITY; skeleton.nodes.len()];
    for &i in &order {
        let node = &skeleton.nodes[i];
        let parent = node.parent.map_or(node.above, |p| world[p]);
        world[i] = multiply(&motions[i].sample(0.0), &parent);
    }
    // Slots for the moving nodes, parents first so a child's parent slot is
    // already assigned - the same invariant `Model::sample_anim_nodes` leans
    // on for a `.vex` chain.
    let mut slot = vec![None; skeleton.nodes.len()];
    let mut anim_nodes = Vec::new();
    let mut frozen = 0;
    for &i in &order {
        if !moving[i] {
            continue;
        }
        if anim_nodes.len() + 1 >= NODE_ANIM_LIMIT {
            frozen += 1;
            continue;
        }
        let node = &skeleton.nodes[i];
        let (static_above, parent) = match node.parent {
            None => (node.above, None),
            Some(p) if moving[p] => match slot[p] {
                Some(s) => (IDENTITY, Some(s)),
                // The parent was frozen past the limit: freeze this one too,
                // rather than moving it under a parent that does not move.
                None => {
                    frozen += 1;
                    continue;
                }
            },
            Some(p) => (world[p], None),
        };
        slot[i] = Some(anim_nodes.len());
        anim_nodes.push(AnimNode {
            transform: Motion::Rig(Box::new(motions[i].clone())),
            static_above,
            parent,
        });
    }
    let mut unmatched = 0;
    let placements = scene
        .nodes
        .iter()
        .map(|model_node| {
            let Some(i) = skeleton.node_by_id(model_node.id) else {
                unmatched += 1;
                return by_bind(model_node);
            };
            match slot[i] {
                Some(s) => Placement {
                    to_world: IDENTITY,
                    xform: u32::try_from(s + 1).unwrap_or(0),
                    world_at_zero: world[i],
                    hidden: false,
                    unplaced: false,
                },
                None => Placement {
                    to_world: world[i],
                    xform: 0,
                    world_at_zero: world[i],
                    hidden: hidden[i],
                    unplaced: false,
                },
            }
        })
        .collect();
    Plan {
        placements,
        anim_nodes,
        unmatched,
        frozen,
    }
}

/// Plans `scene` off the animation a `.vex` authors beside it.
///
/// **Omega's front-end scene ships its motion in the `.vex`, not in a
/// `.rcsskeleton`/`.rcsanimclip` pair** - the same file HD ships, re-exported
/// with a PS4 `.rcsmodel` for its geometry - so the `Anim Transform` nodes are
/// what moves it. Each of the model's mesh objects is found in the `.vex` by
/// its shape name, and placed the way the PS3 path places a chunk
/// (`mesh::anim_node::placement`): baked through the static chain between its
/// node and the nearest `Anim Transform` above it, and moved by that
/// transform's slot of the shader's table, or baked through the whole chain
/// where nothing above it moves.
///
/// A shape is found by its name, and failing that by its name without a
/// `namespace:` prefix. A model node no mesh object names, or whose shape the
/// `.vex` does not, falls back to its own bind matrix and is counted in [`Plan::unmatched`]. A `.vex`
/// that will not parse plans nothing at all, every node baking by its bind.
#[must_use]
pub fn plan_from_vex(scene: &psp2::nodes::Scene, vex_data: &[u8]) -> Plan {
    let by_bind_only = || Plan {
        placements: scene.nodes.iter().map(by_bind).collect(),
        ..Plan::default()
    };
    let Ok(nodes) = vex::nodes(vex_data) else {
        return by_bind_only();
    };
    let Ok(classes) = vex::classes_of(vex_data) else {
        return by_bind_only();
    };
    let anchors = vex::anim_anchors(vex_data, &nodes);
    let anchor_world = vex::anchor_world(vex_data, &nodes, 0.0);
    let (anim_nodes, slots) = anim_node::collect(vex_data, &nodes, &anchors, classes);
    let mut unmatched = 0;
    let placements = scene
        .nodes
        .iter()
        .enumerate()
        .map(|(index, model_node)| {
            let found = scene
                .meshes
                .iter()
                .find(|mesh| mesh.node == Some(index))
                .and_then(|mesh| {
                    // The exporter spells a namespaced shape (`goteki:canopy..`)
                    // with its namespace in the model and without it in the
                    // `.vex`: three of Omega's scene's five misses.
                    let bare = mesh.name.rsplit(':').next().unwrap_or(&mesh.name);
                    let named =
                        |want: &str| nodes.iter().position(|n| n.name.as_deref() == Some(want));
                    named(&mesh.name).or_else(|| named(bare))
                });
            let Some(found) = found else {
                unmatched += 1;
                return by_bind(model_node);
            };
            let placed = anim_node::placement(&anchors, &anchor_world, &slots, found);
            let at_zero = match (placed.xform, placed.bounds_matrix) {
                (0, _) => placed.to_world,
                (_, Some(anchor)) => vex::multiply(&placed.to_world, &anchor),
                (_, None) => placed.to_world,
            };
            Placement {
                to_world: placed.to_world,
                xform: placed.xform,
                world_at_zero: at_zero,
                hidden: false,
                unplaced: false,
            }
        })
        .collect();
    Plan {
        placements,
        anim_nodes,
        unmatched,
        frozen: 0,
    }
}

#[cfg(test)]
mod tests;
