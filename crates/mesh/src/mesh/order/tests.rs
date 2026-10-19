use super::*;
use crate::mesh::Bounds;

fn draw(layer: u32, depth_point: Option<[f32; 3]>) -> DrawCall {
    DrawCall {
        range: 0..3,
        texture: None,
        bounds: Bounds {
            centre: [0.0; 3],
            radius: 1.0,
        },
        moving: false,
        culled: false,
        blend: None,
        blend_state: None,
        depth_point,
        layer,
        alpha_test_ref: None,
        node: None,
        chunk: None,
    }
}

/// A camera at the origin looking down `-z`: `clip.w = -z`.
const LOOKING_DOWN_NEG_Z: [f32; 4] = [0.0, 0.0, -1.0, 0.0];

#[test]
fn a_list_with_no_depth_point_keeps_its_own_order() {
    let draws = vec![draw(0x4500_0000, None), draw(0x4a00_0000, None)];
    assert_eq!(order_blended(&draws, LOOKING_DOWN_NEG_Z), None);
}

#[test]
fn list_b_meshes_draw_far_to_near_whatever_their_scene_layer() {
    let layer = 0x4d00_0000;
    let draws = vec![
        draw(layer, Some([0.0, 0.0, -40.0])),
        draw(layer, Some([0.0, 0.0, -400.0])),
        draw(layer, Some([0.0, 0.0, -200.0])),
    ];
    assert_eq!(
        order_blended(&draws, LOOKING_DOWN_NEG_Z),
        Some(vec![1, 2, 0])
    );
}

#[test]
fn list_a_stays_ahead_of_every_list_b_mesh() {
    let draws = vec![
        draw(0x4d00_0000, Some([0.0, 0.0, -900.0])),
        draw(0x4a00_0000, None),
        draw(0x4500_0000, None),
    ];
    assert_eq!(
        order_blended(&draws, LOOKING_DOWN_NEG_Z),
        Some(vec![2, 1, 0])
    );
}

#[test]
fn the_batches_of_one_mesh_stay_together_in_list_order() {
    let layer = 0x4d00_0000;
    let near = Some([0.0, 0.0, -10.0]);
    let far = Some([0.0, 0.0, -300.0]);
    let draws = vec![draw(layer, near), draw(layer, near), draw(layer, far)];
    assert_eq!(
        order_blended(&draws, LOOKING_DOWN_NEG_Z),
        Some(vec![2, 0, 1])
    );
}

#[test]
fn a_point_behind_the_eye_or_past_the_clamp_draws_first() {
    let layer = 0x4d00_0000;
    let draws = vec![
        draw(layer, Some([0.0, 0.0, -5.0])),
        draw(layer, Some([0.0, 0.0, 50.0])),
        draw(layer, Some([0.0, 0.0, -5000.0])),
    ];
    assert_eq!(
        order_blended(&draws, LOOKING_DOWN_NEG_Z),
        Some(vec![1, 2, 0])
    );
}
