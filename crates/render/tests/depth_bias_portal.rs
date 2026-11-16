//! A batch's authored depth bias pulls by the same amount in `1 / w` inside a portal view as in
//! the main view (`docs/rendering/depth-bias.md`, "Under a portal's oblique near plane").
//!
//! Two coplanar quads face a camera; the unbiased one lies at distance 6, the biased one
//! (bias 9, `6 * 9 / 76779` in `1 / w`) behind it. It wins the tie while it lies within the
//! predicted pull and loses past it. Run under the plain projection and under
//! `portal::oblique_near` with a plane tilted well off the view axis, whose depth row is no
//! function of `w`. Skips when there is no adapter.

use oag_core::math::{Mat4, Vec3, camera};
use oag_mesh::mesh::{Bounds, DrawCall, GpuVertex, Model};
use oag_mesh::mesh_render::{Anisotropy, capture_pixels_through};
use oag_render::camera::portal::oblique_near;

const SIZE: u32 = 64;
const DISTANCE: f32 = 6.0;
const BIAS: f32 = 9.0;
/// Where the scene stands in world space. The eye is off the origin along the quads' plane, as
/// it is on a circuit, so the oblique depth row's tilt (square to the view axis) meets it.
const SCENE: Vec3 = Vec3::new(0.0, 0.0, 50.0);

const RED: [f32; 4] = [1.0, 0.0, 0.0, 1.0];
const GREEN: [f32; 4] = [0.0, 1.0, 0.0, 1.0];

/// A quad in the plane at `x`, facing +X, in `colour` with `bias`.
fn quad(colour: [f32; 4], bias: f32, x: f32, first: u32) -> ([GpuVertex; 4], [u32; 12]) {
    let corner = |y: f32, z: f32| GpuVertex {
        quake: 0.0,
        position: [x, y, z],
        normal: [1.0, 0.0, 0.0],
        colour,
        texcoord: [0.0, 0.0],
        lightmap_texcoord: [0.0, 0.0],
        lit: 0.0,
        anim: 0,
        xform: 0,
        sun_mask: 1.0,
        slots: oag_mesh::mesh::slots::DEFAULT,
        specular_exponent: oag_mesh::mesh::DEFAULT_SPECULAR_EXPONENT,
        glow: 0.0,
        texcoord2: [0.0, 0.0],
        tangent: 0,
        depth_bias: bias,
    };
    let vertices = [
        corner(-1.0, -1.0),
        corner(1.0, -1.0),
        corner(1.0, 1.0),
        corner(-1.0, 1.0),
    ];
    let indices = [0, 1, 2, 0, 2, 3, 0, 2, 1, 0, 3, 2].map(|i| first + i);
    (vertices, indices)
}

fn draw(range: std::ops::Range<u32>) -> DrawCall {
    DrawCall {
        writes_depth: false,
        range,
        texture: None,
        bounds: Bounds {
            centre: [0.0; 3],
            radius: 4.0,
        },
        moving: false,
        culled: false,
        blend: None,
        blend_state: None,
        depth_point: None,
        layer: oag_vex::vex::LAYER_DEFAULT,
        node: None,
        chunk: None,
        alpha_test_ref: None,
    }
}

/// A red quad at the scene's centre, drawn first, then a green biased one `behind` it.
fn model(behind: f32) -> Model {
    let mut model = Model::none("portal depth bias");
    let (a, ia) = quad(RED, 0.0, 0.0, 0);
    let (b, ib) = quad(GREEN, BIAS, -behind, 4);
    model.vertices = a.into_iter().chain(b).collect();
    model.indices = ia.into_iter().chain(ib).collect();
    model.draws = vec![draw(0..12), draw(12..24)];
    model.centre = (-SCENE).to_array();
    model.radius = 2.0;
    model.mesh_count = 1;
    model
}

fn view() -> Mat4 {
    camera::look_at(SCENE + Vec3::new(DISTANCE, 0.0, 0.0), SCENE, Vec3::Y)
}

fn projection() -> Mat4 {
    camera::perspective(45f32.to_radians(), 1.0, 1.0, 100.0)
}

/// The plain projection, or the same one with its near plane swung onto a plane through the
/// point halfway to the quads, tilted 40 degrees off the view axis. The quads lie in front of it.
fn view_projection(oblique: bool) -> Mat4 {
    let plain = projection();
    if !oblique {
        return plain * view();
    }
    let tilt = 40f32.to_radians();
    let normal = Vec3::new(-tilt.cos(), 0.0, tilt.sin());
    let point = SCENE + Vec3::new(DISTANCE / 2.0, 0.0, 0.0);
    let world = normal.extend(-normal.dot(point));
    let plane = view().inverse().transpose() * world;
    oblique_near(plain, plane).expect("the eye is behind the plane") * view()
}

/// The same hyperbola in `1 / w` the shader solves, on the CPU: `1 / w' + k = 1 / w` for `w'`.
fn pull() -> f32 {
    let k = 6.0 * BIAS / 76_779.0;
    DISTANCE * DISTANCE * k / (1.0 - DISTANCE * k)
}

/// `Some(true)` when the biased green quad holds the middle of the frame.
fn biased_wins(oblique: bool, factor: f32) -> Option<bool> {
    let pixels = capture_pixels_through(
        &model(pull() * factor),
        SIZE,
        SIZE,
        view_projection(oblique),
        Anisotropy::Off,
        0.0,
    )
    .ok()?;
    let at = ((SIZE / 2 * SIZE + SIZE / 2) * 4) as usize;
    let (r, g) = (pixels[at], pixels[at + 1]);
    assert!(
        r.abs_diff(g) > 64,
        "the middle is neither red nor green: {r} {g}"
    );
    Some(g > r)
}

fn bracket(oblique: bool) {
    let Some(inside) = biased_wins(oblique, 0.7) else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    let outside = biased_wins(oblique, 1.3).expect("capture");
    assert!(
        inside,
        "a bias of {BIAS} reaches {} behind the plane at {DISTANCE} (oblique: {oblique})",
        pull()
    );
    assert!(
        !outside,
        "and no further than {} (it won at 1.3 times, oblique: {oblique})",
        pull()
    );
}

/// The fixture is not vacuous: the oblique depth row is no multiple of the `w` row, so the
/// coefficient read from the depth and `w` rows alone is not the one along a ray.
#[test]
fn the_oblique_fixture_rewrites_the_depth_row() {
    let tilted = view_projection(true);
    let (depth, w) = (tilted.row(2), tilted.row(3));
    let alpha = depth.truncate().dot(w.truncate()) / w.truncate().length_squared();
    let from_rows = depth.w - alpha * w.w;
    let at_the_eye = depth.truncate().dot(SCENE + Vec3::new(DISTANCE, 0.0, 0.0)) + depth.w;
    assert!(
        (from_rows / at_the_eye - 1.0).abs() > 0.4,
        "{from_rows} against {at_the_eye}"
    );
}

#[test]
fn the_pull_is_the_same_under_the_plain_projection() {
    bracket(false);
}

#[test]
fn the_pull_is_the_same_under_an_oblique_near_plane() {
    bracket(true);
}
