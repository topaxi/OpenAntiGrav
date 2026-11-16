//! A cutout batch and a transparent batch each reach the frame.
//!
//! Runs on a real device and needs no game content. `capture_pixels_from`
//! draws `Model::draws`, then `Model::alpha_tested_draws`, then
//! `Model::transparent_draws`; one quad in each of the last two lists lands in
//! its own half of the frame, so a loop that stops drawing either list leaves
//! that half at the clear colour. Skips when there is no adapter.

use oag_mesh::mesh::{Bounds, DrawCall, GpuVertex, Model};
use oag_mesh::mesh_render::{Anisotropy, capture_pixels_from};

const SIZE: u32 = 64;

/// A quad in the X=0 plane, which the capture camera (on +X for yaw = pitch = 0)
/// faces; `z` runs across the frame.
fn quad(z0: f32, z1: f32, first: u32) -> ([GpuVertex; 4], [u32; 6]) {
    let corner = |y: f32, z: f32| GpuVertex {
        quake: 0.0,
        position: [0.0, y, z],
        normal: [1.0, 0.0, 0.0],
        colour: [1.0, 1.0, 1.0, 1.0],
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
        depth_bias: 0.0,
    };
    let vertices = [
        corner(-1.0, z0),
        corner(1.0, z0),
        corner(1.0, z1),
        corner(-1.0, z1),
    ];
    // Both windings, so the check does not depend on which way the quad faces.
    let indices = [0, 1, 2, 0, 2, 3].map(|i| first + i);
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

/// Two quads, one in each half of the frame; `cutout` and `transparent` say
/// which of them are submitted through their list.
fn model(cutout: bool, transparent: bool) -> Model {
    let mut model = Model::none("cutout then transparent");
    let (a, ia) = quad(-3.0, -0.5, 0);
    let (b, ib) = quad(0.5, 3.0, 4);
    model.vertices = a.into_iter().chain(b).collect();
    model.indices = ia.into_iter().chain(ib).collect();
    model.centre = [0.0; 3];
    model.radius = 6.0;
    model.mesh_count = 1;
    if cutout {
        model.alpha_tested_draws = vec![draw(0..6)];
    }
    if transparent {
        model.transparent_draws = vec![draw(6..12)];
    }
    model
}

fn capture(model: &Model) -> Option<Vec<u8>> {
    capture_pixels_from(model, SIZE, SIZE, 0.0, 0.0, Anisotropy::Off, 0.0).ok()
}

/// Pixels that differ from the empty frame, left and right of the middle column.
fn painted(pixels: &[u8], empty: &[u8]) -> (usize, usize) {
    let (mut left, mut right) = (0, 0);
    for (i, (a, b)) in pixels
        .as_chunks::<4>()
        .0
        .iter()
        .zip(empty.as_chunks::<4>().0.iter())
        .enumerate()
    {
        if a != b {
            if (i % SIZE as usize) < SIZE as usize / 2 {
                left += 1;
            } else {
                right += 1;
            }
        }
    }
    (left, right)
}

#[test]
fn each_of_the_cutout_and_transparent_lists_paints_its_own_half_of_the_frame() {
    let Some(empty) = capture(&model(false, false)) else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    let both = capture(&model(true, true)).expect("capture");
    let cutout_only = capture(&model(true, false)).expect("capture");
    let transparent_only = capture(&model(false, true)).expect("capture");

    let (cutout_left, cutout_right) = painted(&cutout_only, &empty);
    let (transparent_left, transparent_right) = painted(&transparent_only, &empty);
    // Which half is which depends on the camera's handedness; the two quads
    // are on opposite sides, so one list paints one side and the other list
    // the other.
    let (cutout_side, cutout_other) =
        (cutout_left.max(cutout_right), cutout_left.min(cutout_right));
    let (transparent_side, transparent_other) = (
        transparent_left.max(transparent_right),
        transparent_left.min(transparent_right),
    );
    assert!(
        cutout_side > 20,
        "the cutout quad painted {cutout_side} pixels"
    );
    assert!(
        transparent_side > 20,
        "the transparent quad painted {transparent_side} pixels"
    );
    assert_eq!(cutout_other, 0, "the cutout quad stays on its own side");
    assert_eq!(
        transparent_other, 0,
        "the transparent quad stays on its own side"
    );
    assert!(
        (cutout_left > 0) != (transparent_left > 0),
        "the two quads are on opposite sides"
    );

    // Both lists in one pass paint both quads, and nothing else.
    let (both_left, both_right) = painted(&both, &empty);
    assert_eq!(both_left, cutout_left + transparent_left);
    assert_eq!(both_right, cutout_right + transparent_right);
}
