//! What [`super::plan`] decides, on a skeleton and clip built in memory.

use oag_rcs::rcsanimclip::{Channel, Clip, Track};
use oag_rcs::rcsmodel::psp2::nodes::{Node as ModelNode, Scene};
use oag_rcs::rcsskeleton::{IDENTITY, Kind, Node, SLOTS, Skeleton};

use super::*;

fn node(id: u32, parent: Option<usize>, x: f32, visible: bool) -> Node {
    Node {
        id,
        parent,
        scale: [1.0; 3],
        rotation: [0.0, 0.0, 0.0, 1.0],
        translation: [x, 0.0, 0.0],
        visible,
        pivot: [0.0; 3],
        pivot_translate: [0.0; 3],
        above: IDENTITY,
        kinds: [None; SLOTS],
    }
}

fn scene(ids: &[u32]) -> Scene {
    Scene {
        nodes: ids
            .iter()
            .map(|&id| ModelNode {
                name_hash: 0,
                id,
                bind: Some({
                    let mut m = IDENTITY;
                    m[12] = 1000.0;
                    m
                }),
            })
            .collect(),
        meshes: Vec::new(),
    }
}

fn track(id: u32) -> Track {
    let mut channels: [Option<Channel>; SLOTS] = Default::default();
    channels[2] = Some(Channel {
        kind: Kind::Vec3,
        keys: vec![0.0, 0.0, 0.0, 5.0, 0.0, 0.0],
        seconds_per_key: 0.2,
        count: 2,
    });
    Track {
        id,
        duration: 0.4,
        channels,
    }
}

#[test]
fn without_a_skeleton_every_node_bakes_through_its_bind_matrix() {
    let plan = plan(&scene(&[1, 2]), None);
    assert_eq!(plan.placements.len(), 2);
    assert!(plan.anim_nodes.is_empty());
    assert_eq!(plan.placements[0].xform, 0);
    assert_eq!(plan.placements[0].to_world[12], 1000.0);
}

/// A node with no written matrix, in a model with no skeleton (or one that
/// does not name it), has nothing authored to place it by: not drawn, and
/// counted apart from a node the file authors invisible.
#[test]
fn a_node_with_no_matrix_and_no_skeleton_entry_is_unplaced_not_left_at_its_origin() {
    let mut scene = scene(&[1, 2]);
    scene.nodes[1].bind = None;

    let bare = plan(&scene, None);
    assert!(!bare.placements[0].unplaced);
    assert_eq!(bare.placements[0].to_world[12], 1000.0);
    assert!(bare.placements[1].unplaced && bare.placements[1].hidden);

    // A skeleton that names neither node changes nothing about the second.
    let animation = Animation {
        skeleton: Skeleton {
            nodes: vec![node(50, None, 0.0, true)],
        },
        clip: None,
    };
    let unmatched = plan(&scene, Some(&animation));
    assert_eq!(unmatched.unmatched, 2);
    assert!(!unmatched.placements[0].unplaced);
    assert!(unmatched.placements[1].unplaced && unmatched.placements[1].hidden);

    // Named by the skeleton, the same node is placed by it, bind or not.
    let named = Animation {
        skeleton: Skeleton {
            nodes: vec![node(2, None, 3.0, true)],
        },
        clip: None,
    };
    let placed = plan(&scene, Some(&named));
    assert!(!placed.placements[1].unplaced && !placed.placements[1].hidden);
    assert_eq!(placed.placements[1].to_world[12], 3.0);
}

#[test]
fn moving_nodes_take_slots_and_static_ones_bake_through_the_skeleton() {
    // 0: a tracked root; 1: its untracked child (moves with it); 2: a static
    // root at x = 7; 3: a static root authored invisible; 4: 3's child.
    let skeleton = Skeleton {
        nodes: vec![
            node(10, None, 0.0, true),
            node(11, Some(0), 1.0, true),
            node(12, None, 7.0, true),
            node(13, None, 0.0, false),
            node(14, Some(3), 0.0, true),
        ],
    };
    let clip = Clip {
        duration: 0.4,
        bound: vec![10],
        tracks: vec![track(10)],
    };
    let animation = Animation {
        skeleton,
        clip: Some(clip),
    };
    // The model's table lists them in another order, and one id the
    // skeleton does not know.
    let plan = plan(&scene(&[12, 11, 10, 99, 13, 14]), Some(&animation));
    assert_eq!(plan.anim_nodes.len(), 2, "the tracked root and its child");
    assert_eq!(plan.anim_nodes[0].parent, None);
    assert_eq!(plan.anim_nodes[1].parent, Some(0));
    assert_eq!(plan.frozen, 0);
    assert_eq!(plan.unmatched, 1);

    let by_id = |id: u32| {
        plan.placements[[12u32, 11, 10, 99, 13, 14]
            .iter()
            .position(|&i| i == id)
            .unwrap()]
    };
    let static_root = by_id(12);
    assert_eq!(static_root.xform, 0);
    assert_eq!(
        static_root.to_world[12], 7.0,
        "the skeleton's own composition, not the model's"
    );
    assert!(!static_root.hidden);

    let tracked = by_id(10);
    assert_eq!(tracked.xform, 1);
    assert_eq!(
        tracked.to_world, IDENTITY,
        "left in node space for the table"
    );
    let child = by_id(11);
    assert_eq!(child.xform, 2);
    assert_eq!(child.world_at_zero[12], 1.0);

    let unknown = by_id(99);
    assert_eq!(unknown.xform, 0);
    assert_eq!(unknown.to_world[12], 1000.0, "the model's bind matrix");

    assert!(by_id(13).hidden);
    assert!(by_id(14).hidden, "visibility inherits down the hierarchy");
}
