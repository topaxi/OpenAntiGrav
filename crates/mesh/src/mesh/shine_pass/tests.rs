use super::*;
use crate::mesh::{Bounds, DrawCall, GpuVertex, slots};

fn vertex(normal: [f32; 3]) -> GpuVertex {
    GpuVertex {
        quake: 0.0,
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
        texcoord2: [0.0, 0.0],
        tangent: 0,
    }
}

fn draw(range: std::ops::Range<u32>, texture: usize) -> DrawCall {
    DrawCall {
        writes_depth: false,
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
        depth_point: None,
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

fn texture(label: &str) -> Option<std::sync::Arc<crate::mesh::ModelTexture>> {
    Some(std::sync::Arc::new(crate::mesh::ModelTexture::rgba8(
        label.to_string(),
        1,
        1,
        vec![255, 255, 255, 128],
        None,
    )))
}

/// Pure's hull: the extra pass of a batch whose ordinary pass blends by its own
/// texture's alpha draws through a slot pairing the environment map with that
/// texture, and the shader is told to weight by the pair's second alpha.
#[test]
fn an_alpha_weighted_batch_binds_its_own_texture_second_at_its_authored_coordinate() {
    let mut hull = hull();
    hull.textures = vec![texture("base"), texture("other"), texture("env")];
    let mut lightmaps = Vec::new();
    let slot = masked_slot(&mut hull.textures, &mut lightmaps, 2, 1);
    assert_eq!(slot, 3, "a new slot after the model's own");
    assert_eq!(
        masked_slot(&mut hull.textures, &mut lightmaps, 2, 1),
        3,
        "one slot per pair"
    );
    assert_eq!(masked_slot(&mut hull.textures, &mut lightmaps, 2, 0), 4);
    hull.lightmaps = lightmaps;
    hull.shine_draws = vec![draw(3..6, slot), draw(0..3, 2)];
    let model = build(&hull).unwrap();
    assert!(std::sync::Arc::ptr_eq(
        model.textures[3].as_ref().unwrap(),
        model.textures[2].as_ref().unwrap()
    ));
    assert!(std::sync::Arc::ptr_eq(
        model.lightmaps[3].as_ref().unwrap(),
        model.textures[1].as_ref().unwrap()
    ));
    assert!(is_masked(&model, &model.draws[0]) && !is_masked(&model, &model.draws[1]));
    assert_eq!(model.emissive.len(), 1);
    assert_eq!(
        model.emissive[0].program[0],
        crate::mesh::PROGRAM_SHINE_MASK
    );
    for v in &model.vertices[3..6] {
        assert_eq!(
            v.lightmap_texcoord,
            [0.25, 0.75],
            "the batch's own coordinate"
        );
        assert_eq!(
            slots::material_index(v.slots),
            1,
            "the glow table's mask entry"
        );
    }
    for v in &model.vertices[0..3] {
        assert_eq!(
            v.slots,
            slots::DEFAULT,
            "an unweighted batch keeps its word"
        );
    }
}

#[test]
fn a_hull_with_no_weighted_batch_carries_no_mask_entry() {
    let model = build(&hull()).unwrap();
    assert!(model.emissive.is_empty() && model.lightmaps.is_empty());
}
