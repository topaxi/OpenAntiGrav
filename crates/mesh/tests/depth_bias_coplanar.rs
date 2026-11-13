//! A vertex's authored depth bias wins a depth tie, whichever quad is drawn first.
//!
//! Runs on a real device and needs no game content. Two quads lie in the same plane, so every
//! pixel of their overlap ties on depth and the `Less` test keeps the first one drawn. A quad
//! whose vertices carry `depth_bias` is pulled toward the camera by `6 * bias` of the original's
//! depth units (`depth_bias.wesl`) and so wins the tie from either place in the draw order.
//! Drop the bias from the vertex shader and the second test fails. Skips when there is no
//! adapter.

use oag_mesh::mesh::{Bounds, DrawCall, GpuVertex, Model};
use oag_mesh::mesh_render::{Anisotropy, capture_pixels_from};

const SIZE: u32 = 64;

const RED: [f32; 4] = [1.0, 0.0, 0.0, 1.0];
const GREEN: [f32; 4] = [0.0, 1.0, 0.0, 1.0];

/// A quad in the X=0 plane, facing the capture camera on +X, in `colour` with `bias`.
fn quad(colour: [f32; 4], bias: f32, first: u32) -> ([GpuVertex; 4], [u32; 12]) {
    let corner = |y: f32, z: f32| GpuVertex {
        quake: 0.0,
        position: [0.0, y, z],
        normal: [1.0, 0.0, 0.0],
        colour,
        texcoord: [0.0, 0.0],
        lightmap_texcoord: [0.0, 0.0],
        // Prelit, so the result does not depend on a light rig.
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
    // Both windings, so the check does not depend on which way the quad faces.
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

/// `first` is drawn before `second`; each is `(colour, bias)`.
fn model(first: ([f32; 4], f32), second: ([f32; 4], f32)) -> Model {
    let mut model = Model::none("depth bias");
    let (a, ia) = quad(first.0, first.1, 0);
    let (b, ib) = quad(second.0, second.1, 4);
    model.vertices = a.into_iter().chain(b).collect();
    model.indices = ia.into_iter().chain(ib).collect();
    model.draws = vec![draw(0..12), draw(12..24)];
    model.centre = [0.0; 3];
    model.radius = 2.0;
    model.mesh_count = 1;
    model
}

/// Which of red or green holds the middle of the frame: `Some(true)` for green.
fn winner(model: &Model) -> Option<bool> {
    let pixels = capture_pixels_from(model, SIZE, SIZE, 0.0, 0.0, Anisotropy::Off, 0.0).ok()?;
    let at = ((SIZE / 2 * SIZE + SIZE / 2) * 4) as usize;
    let (r, g) = (pixels[at], pixels[at + 1]);
    assert!(
        r.abs_diff(g) > 64,
        "the middle is neither red nor green: {r} {g}"
    );
    Some(g > r)
}

#[test]
fn without_a_bias_the_first_quad_drawn_wins_the_tie() {
    let Some(green_first) = winner(&model((GREEN, 0.0), (RED, 0.0))) else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    let red_first = winner(&model((RED, 0.0), (GREEN, 0.0))).expect("capture");
    assert!(green_first, "an unbiased tie goes to the first quad");
    assert!(!red_first, "an unbiased tie goes to the first quad");
}

#[test]
fn a_vertex_depth_bias_wins_the_tie_from_either_place_in_the_draw_order() {
    let Some(biased_second) = winner(&model((RED, 0.0), (GREEN, 3.0))) else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    let biased_first = winner(&model((GREEN, 3.0), (RED, 0.0))).expect("capture");
    assert!(biased_second, "the biased quad drawn second wins");
    assert!(biased_first, "the biased quad drawn first still wins");
}
