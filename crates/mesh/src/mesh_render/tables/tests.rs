//! The node table's head upload leaves the GPU's table what a full upload leaves.

use super::*;
use crate::mesh::{AnimNode, Motion};
use oag_vex::vex::{AnimChannel, AnimTransform};

/// A node that slides along x and rolls about z, so its matrix moves with the clock.
fn moving(seed: f32) -> Motion {
    let key = |x: f32, y: f32, z: f32| [x, y, z];
    Motion::Vex(Box::new(AnimTransform {
        translation: AnimChannel::new(
            vec![0, 60, 180],
            vec![
                key(0.0, seed, 0.0),
                key(100.0, 3.0, seed),
                key(-40.0, 0.0, 9.0),
            ],
            Vec::new(),
            false,
        ),
        translation_quantum: [0.1, 0.2, 0.3],
        translation_base: [seed, 1.0, 2.0],
        rotation: AnimChannel::new(
            vec![0, 90],
            vec![key(0.0, 0.0, 0.0), key(0.0, 0.0, 12000.0)],
            Vec::new(),
            false,
        ),
        scale: AnimChannel::default(),
        seconds_per_key: 1.0 / 60.0,
        loop_seconds: 4.0,
        ..AnimTransform::default()
    }))
}

fn model_with(nodes: usize) -> Model {
    let mut model = crate::mesh_render::pipeline_cache::tests::triangle_model();
    model.anim_nodes = (0..nodes)
        .map(|n| AnimNode {
            transform: moving(n as f32),
            static_above: [
                1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, n as f32, 0.5, 0.0, 1.0,
            ],
            // Every third node hangs under the one before it.
            parent: (n % 3 == 2).then(|| n - 1),
        })
        .collect();
    model
}

/// The buffer after the head is written over the identity table it was built with is the
/// table a full upload writes, over a clock sweep and for a model past the slot ceiling.
#[test]
fn the_head_over_the_identity_table_is_the_full_table() {
    for nodes in [1, 2, 3, 7, 74, 259, crate::mesh::NODE_ANIM_LIMIT - 1, 400] {
        let model = model_with(nodes);
        for tick in 0..900 {
            let seconds = tick as f32 / 60.0 * 1.37 - 2.0;
            let full = NodeAnims::sample_started(&model, seconds, &|_| None);
            let head = NodeAnims::head_started(&model, seconds, &|_| None);
            let mut gpu = NodeAnims::default();
            gpu.transform[..head.len()].copy_from_slice(&head);
            assert_eq!(
                bytemuck::bytes_of(&gpu),
                bytemuck::bytes_of(&full),
                "{nodes} nodes at {seconds}"
            );
            assert_eq!(head.len(), nodes.min(crate::mesh::NODE_ANIM_LIMIT - 1) + 1);
        }
    }
}
