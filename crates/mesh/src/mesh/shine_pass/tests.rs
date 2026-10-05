use super::*;
use crate::mesh::{Bounds, DrawCall, GpuVertex, slots};

fn vertex(normal: [f32; 3]) -> GpuVertex {
    GpuVertex {
        position: [0.0; 3],
        normal,
        colour: [0.3, 0.4, 0.5, 0.6],
        texcoord: [0.25, 0.75],
        lightmap_texcoord: [0.0, 0.0],
        lit: 1.0,
        anim: 2,
        xform: 0,
        sun_mask: 1.0,
        slots: slots::DEFAULT,
        specular_exponent: crate::mesh::DEFAULT_SPECULAR_EXPONENT,
        glow: 1.0,
    }
}

fn draw(range: std::ops::Range<u32>, texture: usize) -> DrawCall {
    DrawCall {
        range,
        texture: Some(texture),
        bounds: Bounds {
            centre: [0.0; 3],
            radius: 1.0,
        },
        moving: false,
        culled: true,
        blend: None,
        blend_state: None,
        alpha_test_ref: None,
        layer: oag_vex::vex::LAYER_DEFAULT,
        node: Some(3),
        chunk: None,
    }
}

fn hull() -> Model {
    let mut hull = Model::none("hull");
    hull.vertices = vec![vertex([0.0, 0.0, 1.0]); 6];
    hull.indices = vec![0, 1, 2, 3, 4, 5];
    hull.draws = vec![draw(0..3, 0), draw(3..6, 1)];
    hull.transparent_draws = vec![draw(3..6, 5)];
    hull.shine_draws = vec![draw(3..6, 2)];
    hull
}

#[test]
fn a_hull_with_no_extra_pass_batch_builds_no_shine_model() {
    let mut bare = hull();
    bare.shine_draws.clear();
    assert!(build(&bare).is_none());
}

#[test]
fn the_shine_model_draws_only_the_extra_pass_batches_under_their_second_texture() {
    let model = build(&hull()).expect("a hull with an extra pass");
    assert_eq!(model.draws.len(), 1);
    assert_eq!(model.draws[0].range, 3..6, "the batch's own vertices");
    assert_eq!(model.draws[0].texture, Some(2), "the second texture");
    assert!(model.alpha_tested_draws.is_empty() && model.transparent_draws.is_empty());
    assert!(
        model.shine_draws.is_empty(),
        "moved into `draws`, not kept twice"
    );
}

#[test]
fn the_pass_is_unlit_white_and_writes_no_glow() {
    let model = build(&hull()).unwrap();
    for v in &model.vertices {
        assert_eq!(v.colour, [1.0; 4]);
        assert_eq!((v.lit, v.anim, v.glow), (0.0, 0, 0.0));
    }
    assert!(!model.stamps_glow);
}
