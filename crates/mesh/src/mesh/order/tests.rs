use super::*;
use crate::mesh::{Bounds, REFERENCE_FOV_DEGREES};

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
        writes_depth: false,
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
        order_blended(&draws, LOOKING_DOWN_NEG_Z).map(|sorted| sorted.order),
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
        order_blended(&draws, LOOKING_DOWN_NEG_Z).map(|sorted| sorted.order),
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
        order_blended(&draws, LOOKING_DOWN_NEG_Z).map(|sorted| sorted.order),
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
        order_blended(&draws, LOOKING_DOWN_NEG_Z).map(|sorted| sorted.order),
        Some(vec![1, 2, 0])
    );
}

fn blended_at(distances: &[f32]) -> BlendedOrder {
    let draws: Vec<DrawCall> = distances
        .iter()
        .map(|d| draw(LIST_B_LAYER, Some([0.0, 0.0, -d])))
        .collect();
    order_blended(&draws, LOOKING_DOWN_NEG_Z).expect("list-B draws carry a depth point")
}

/// A flare is a list-B entry: a mesh nearer than the flare sorts after it, a farther one before
/// it. Dropping the flare's place in the list leaves every position at the end.
#[test]
fn a_nearer_list_b_mesh_sorts_after_a_farther_flare() {
    let sorted = blended_at(&[400.0, 120.0, 30.0]);
    let flare = flare_key(LOOKING_DOWN_NEG_Z, [0.0, 0.0, -100.0]);
    assert_eq!(sorted.order, vec![0, 1, 2]);
    assert_eq!(
        sorted.position_of(flare),
        2,
        "the 400 and 120 unit meshes are farther than the flare, the 30 unit mesh is nearer"
    );
}

#[test]
fn a_flare_nearer_than_every_mesh_draws_last_and_a_farther_one_first() {
    let sorted = blended_at(&[400.0, 120.0, 30.0]);
    assert_eq!(
        sorted.position_of(flare_key(LOOKING_DOWN_NEG_Z, [0.0, 0.0, -6.0])),
        3
    );
    assert_eq!(
        sorted.position_of(flare_key(LOOKING_DOWN_NEG_Z, [0.0, 0.0, -900.0])),
        0
    );
}

/// The flare's `z` is not scaled and a list-B mesh's is (`fov / 65`): a mesh 80 units out at a
/// 90 degree fov ranks as if 111 units out, so it is farther than a flare 100 units out and
/// draws before it, where the unscaled plane would put it nearer and after.
#[test]
fn the_list_b_plane_is_scaled_by_the_fov_and_the_flare_is_not() {
    let draws = [draw(LIST_B_LAYER, Some([0.0, 0.0, -80.0]))];
    let flare = flare_key(LOOKING_DOWN_NEG_Z, [0.0, 0.0, -100.0]);
    let scale = 90.0 / REFERENCE_FOV_DEGREES;
    let scaled = order_blended(&draws, LOOKING_DOWN_NEG_Z.map(|c| c * scale)).expect("list B");
    assert_eq!(
        scaled.position_of(flare),
        1,
        "the mesh draws before the flare"
    );
    let unscaled = order_blended(&draws, LOOKING_DOWN_NEG_Z).expect("list B");
    assert_eq!(
        unscaled.position_of(flare),
        0,
        "unscaled, the mesh is nearer"
    );
}

/// A point behind the eye is keyed zero, so a flare there draws before every list-B mesh.
#[test]
fn a_flare_behind_the_eye_draws_before_the_list() {
    let sorted = blended_at(&[400.0, 30.0]);
    assert_eq!(
        sorted.position_of(flare_key(LOOKING_DOWN_NEG_Z, [0.0, 0.0, 5.0])),
        0
    );
}
