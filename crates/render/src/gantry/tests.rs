//! Unit tests for the mount's plane fit, on synthetic panels.
//!
//! The real measurement is `crates/render/tests/gantry_mount_ground_truth.rs`,
//! which needs a disc. These pin the arithmetic: a panel whose normal, up and
//! extents are known by construction, so a wrong eigenvector ordering or a
//! flipped basis fails here rather than as a gantry facing the wrong way.

use super::*;

/// A rectangle in the plane `z = z0`, `width` across x and `height` up y.
fn panel(centre: Vec3, width: f32, height: f32) -> Vec<Vec3> {
    let mut points = Vec::new();
    for i in 0..5 {
        for j in 0..5 {
            let u = (i as f32 / 4.0 - 0.5) * width;
            let v = (j as f32 / 4.0 - 0.5) * height;
            points.push(centre + Vec3::new(u, v, 0.0));
        }
    }
    points
}

#[test]
fn a_flat_panel_gives_its_own_normal_and_extents() {
    let mount = plane(Some(7), &panel(Vec3::new(34.0, -36.0, -185.0), 30.0, 12.0))
        .expect("a panel has a plane");
    assert_eq!(mount.node, Some(7));
    assert!((mount.centre - Vec3::new(34.0, -36.0, -185.0)).length() < 1e-3);
    assert!(
        mount.normal.dot(Vec3::Z).abs() > 0.999,
        "{:?}",
        mount.normal
    );
    assert!(mount.up.dot(Vec3::Y) > 0.999, "{:?}", mount.up);
    assert!((mount.width - 30.0).abs() < 1e-3);
    assert!((mount.height - 12.0).abs() < 1e-3);
    assert!(mount.thickness < 1e-3);
}

#[test]
fn a_taller_than_wide_panel_still_calls_the_vertical_axis_up() {
    // The long axis is the vertical one here, which is the case that breaks a
    // fit that assumes a board is always wider than it is tall.
    let mount = plane(None, &panel(Vec3::ZERO, 6.0, 40.0)).expect("a panel has a plane");
    assert!(mount.up.dot(Vec3::Y) > 0.999, "{:?}", mount.up);
    assert!((mount.height - 40.0).abs() < 1e-3);
    assert!((mount.width - 6.0).abs() < 1e-3);
}

#[test]
fn the_normals_sign_comes_from_the_caller_not_the_fit() {
    let mount = plane(None, &panel(Vec3::ZERO, 30.0, 12.0)).expect("a panel has a plane");
    assert!(mount.facing(Vec3::Z).dot(Vec3::Z) > 0.0);
    assert!(mount.facing(-Vec3::Z).dot(Vec3::Z) < 0.0);
}

#[test]
fn the_matrix_stands_the_model_on_the_mount_facing_the_race() {
    let centre = Vec3::new(34.0, -36.0, -185.0);
    let mount = plane(None, &panel(centre, 30.0, 12.0)).expect("a panel has a plane");
    // A model whose board faces its own -Z, placed on a mount whose race
    // direction is +Z: the board must end up facing +Z in world space.
    let matrix = mount.matrix(Vec3::Z, -Vec3::Z, 1.0);
    let placed = matrix.transform_point3(Vec3::ZERO);
    assert!((placed - centre).length() < 1e-3, "{placed:?}");
    let faced = matrix.transform_vector3(-Vec3::Z).normalize();
    assert!(faced.dot(Vec3::Z) > 0.99, "{faced:?}");
    // Up stays up: a gantry standing on its side would pass a facing test.
    let up = matrix.transform_vector3(Vec3::Y).normalize();
    assert!(up.dot(Vec3::Y) > 0.99, "{up:?}");
}

#[test]
fn a_yawed_panel_reports_a_yawed_normal() {
    let angle = 0.7f32;
    let (s, c) = angle.sin_cos();
    let points: Vec<Vec3> = panel(Vec3::ZERO, 30.0, 12.0)
        .into_iter()
        .map(|p| Vec3::new(c * p.x + s * p.z, p.y, -s * p.x + c * p.z))
        .collect();
    let mount = plane(None, &points).expect("a panel has a plane");
    let expected = Vec3::new(s, 0.0, c);
    assert!(
        mount.facing(expected).dot(expected) > 0.999,
        "{:?}",
        mount.normal
    );
    assert!((mount.width - 30.0).abs() < 1e-2);
}

#[test]
fn too_few_points_is_no_plane() {
    assert!(plane(None, &[Vec3::ZERO, Vec3::X]).is_none());
}

/// A model of two one-triangle draws in the blend list, at the given x offsets.
///
/// No `Anim Transform`, so `clip_to_panel` reads the vertex positions as they
/// are - which is the case the real asset reduces to once its node table is
/// sampled.
fn two_draws(a: f32, b: f32) -> Model {
    use crate::mesh::{Bounds, DrawCall, GpuVertex};

    let vertex = |x: f32| GpuVertex {
        position: [x, 0.0, 0.0],
        normal: [0.0, 0.0, 1.0],
        colour: [1.0; 4],
        texcoord: [0.0; 2],
        lit: 0.0,
        anim: 0,
        lightmap_texcoord: [0.0; 2],
        xform: 0,
        sun_mask: 1.0,
        slots: 0,
        specular_exponent: crate::mesh::DEFAULT_SPECULAR_EXPONENT,
    };
    let mut model = Model::none("two draws");
    model.vertices = vec![
        vertex(a),
        vertex(a),
        vertex(a),
        vertex(b),
        vertex(b),
        vertex(b),
    ];
    model.indices = vec![0, 1, 2, 3, 4, 5];
    let draw = |range: std::ops::Range<u32>| DrawCall {
        range,
        texture: None,
        bounds: Bounds {
            centre: [0.0; 3],
            radius: 0.0,
        },
        moving: false,
        culled: false,
        blend: None,
        blend_state: None,
        layer: 0,
        node: None,
        chunk: None,
    };
    model.transparent_draws = vec![draw(0..3), draw(3..6)];
    model
}

#[test]
fn a_draw_parked_beyond_the_panel_is_clipped_and_the_one_on_it_is_kept() {
    let mut model = two_draws(0.0, 40.0);
    assert_eq!(clip_to_panel(&mut model, 22.75, 0.0), 1);
    assert_eq!(model.transparent_draws.len(), 1);
    // The survivor is the one at the panel's centre, not whichever came first.
    let kept = &model.transparent_draws[0];
    assert_eq!(
        model.vertices[model.indices[kept.range.start as usize] as usize].position[0],
        0.0
    );
}

#[test]
fn clipping_everything_clips_nothing() {
    // The guard: a model whose board plane is not its own XY would put every
    // draw outside, and deleting the whole object reads exactly like a circuit
    // that authors no gantry at all. See `clip_to_panel`.
    let mut model = two_draws(-40.0, 40.0);
    assert_eq!(clip_to_panel(&mut model, 22.75, 0.0), 0);
    assert_eq!(model.transparent_draws.len(), 2);
}
