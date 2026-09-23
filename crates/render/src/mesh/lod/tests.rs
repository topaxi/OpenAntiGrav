use super::*;
use crate::mesh::Bounds;
use oag_core::math::camera;

/// A two-child group at the model origin switching at `distance`, with
/// node 1 the group, node 2 and node 3 its children and node 4 a mesh under
/// the second.
fn two_tiers(distance: f32) -> LodGroups {
    LodGroups {
        groups: vec![Group {
            position: [0.0; 3],
            distances: vec![distance],
            children: 2,
        }],
        member: vec![
            None,
            None,
            Some(Member { group: 0, child: 0 }),
            Some(Member { group: 0, child: 1 }),
            Some(Member { group: 0, child: 1 }),
        ],
    }
}

fn draw(node: Option<u32>) -> DrawCall {
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
        layer: 0,
        alpha_test_ref: None,
        node,
        chunk: None,
    }
}

/// A camera on the `+z` axis looking back at the origin, `depth` away -
/// right-handed, so the origin sits at view-space `z = -depth`.
fn eye_at(depth: f32) -> Mat4 {
    camera::look_at(Vec3::new(0.0, 0.0, depth), Vec3::ZERO, Vec3::Y)
}

fn eye(depth: f32) -> LodEye {
    LodEye {
        view: eye_at(depth),
        fov_degrees: REFERENCE_FOV_DEGREES,
        detail: ModelDetail::Original,
    }
}

#[test]
fn a_point_in_front_of_the_eye_has_negative_view_depth() {
    let z = eye_at(40.0).transform_point3(Vec3::ZERO).z;
    assert!((z + 40.0).abs() < 1e-4, "{z}");
}

#[test]
fn the_child_advances_once_the_scaled_depth_reaches_the_distance() {
    let groups = two_tiers(30.0);
    let at = |depth: f32, fov: f32| groups.child_at(0, eye_at(depth), fov, ModelDetail::Original);
    // At the reference field the depth is the distance itself.
    assert_eq!(at(29.9, 65.0), 0);
    assert_eq!(at(30.0, 65.0), 1, "`d >= distance` advances");
    assert_eq!(at(31.0, 65.0), 1);
    // `d = 40 * 48.75 / 65 = 30`: a narrower field reaches it later.
    assert_eq!(at(39.9, 48.75), 0);
    assert_eq!(at(40.0, 48.75), 1);
    // Behind the eye: negative depth, the finest child.
    let behind = Mat4::from_translation(Vec3::new(0.0, 0.0, 50.0));
    assert_eq!(groups.child_at(0, behind, 65.0, ModelDetail::Original), 0);
}

#[test]
fn a_preset_scales_the_authored_distance() {
    let groups = two_tiers(30.0);
    let at = |depth: f32, detail| groups.child_at(0, eye_at(depth), 65.0, detail);
    assert_eq!(at(59.0, ModelDetail::High), 0);
    assert_eq!(at(60.0, ModelDetail::High), 1);
    assert_eq!(at(1.0e30, ModelDetail::Maximum), 0);
}

#[test]
fn the_group_position_is_taken_through_the_model_matrix() {
    let groups = two_tiers(30.0);
    // The model is pushed 20 units towards the eye, so a 40-unit eye sees
    // the group at 20 - inside the switch.
    let model = Mat4::from_translation(Vec3::new(0.0, 0.0, 20.0));
    let switch = LodSwitch::new(&groups);
    switch.select(&groups, model, eye(40.0));
    assert_eq!(switch.chosen(0), Some(0));
    switch.select(&groups, Mat4::IDENTITY, eye(40.0));
    assert_eq!(switch.chosen(0), Some(1));
}

#[test]
fn only_the_chosen_child_shows() {
    let groups = two_tiers(30.0);
    let switch = LodSwitch::new(&groups);
    // Fresh: the finest tier, which is also what a caller with no camera gets.
    assert!(switch.shows(&groups, &draw(Some(2))));
    assert!(!switch.shows(&groups, &draw(Some(3))));
    assert!(!switch.shows(&groups, &draw(Some(4))));
    assert!(groups.shows_nearest(&draw(Some(2))));
    assert!(!groups.shows_nearest(&draw(Some(4))));
    switch.select(&groups, Mat4::IDENTITY, eye(100.0));
    assert!(groups.shows_at(&draw(Some(4)), Mat4::IDENTITY, eye(100.0)));
    assert!(!groups.shows_at(&draw(Some(4)), Mat4::IDENTITY, eye(10.0)));
    assert!(!switch.shows(&groups, &draw(Some(2))));
    assert!(switch.shows(&groups, &draw(Some(4))));
    // Under no group, or synthetic: always.
    assert!(switch.shows(&groups, &draw(Some(0))));
    assert!(switch.shows(&groups, &draw(None)));
    assert!(switch.shows(&groups, &draw(Some(99))));
    switch.reset();
    assert!(switch.shows(&groups, &draw(Some(2))));
}

#[test]
fn an_empty_table_shows_everything() {
    let groups = LodGroups::default();
    let switch = LodSwitch::new(&groups);
    switch.select(&groups, Mat4::IDENTITY, eye(1000.0));
    assert!(switch.shows(&groups, &draw(Some(3))));
    assert!(groups.shows_nearest(&draw(Some(3))));
}

#[test]
fn the_child_index_never_passes_the_last_child() {
    let mut groups = two_tiers(30.0);
    groups.groups[0].distances = vec![10.0, 20.0, 30.0];
    assert_eq!(
        groups.child_at(0, eye_at(100.0), 65.0, ModelDetail::Original),
        1
    );
}

#[test]
fn every_name_round_trips() {
    for detail in ModelDetail::ALL {
        assert_eq!(detail.name().parse::<ModelDetail>(), Ok(detail));
        assert_eq!(Lod::from(detail).detail(), Some(detail));
    }
    for lod in Lod::ALL {
        assert_eq!(lod.name().parse::<Lod>(), Ok(lod));
    }
    assert_eq!(Lod::Both.detail(), None);
    assert_eq!(Lod::default().detail(), Some(ModelDetail::default()));
}
