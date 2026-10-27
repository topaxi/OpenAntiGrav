//! Draws one engine-flame surface through the mesh pipeline on a fixture
//! needing no game content, and pins where the flame's colour tap is decoded.
//!
//! On HD's race target (linear scene format, [`SceneStore::Encoded`]) the
//! console reads `flame_01` with gamma nibble 7 on unit 0 (RGB sRGB-decoded on
//! fetch, alpha raw) and encodes the program's output before the blend, so a
//! texel `t` stores `enc(dec(t) * colour_scale * output_scale)`
//! (`docs/ghidra/functions/ps3-hdfury-eu/engine-trail.md`, "The flame's colour
//! tap is decoded on fetch"). Everywhere else the flame draws as it did. Skips
//! when there is no adapter. Runs in `just test`.

use oag_gpu::formats::{SCENE_FORMAT, SceneStore};
use oag_mesh::mesh::{Bounds, DrawCall, Flame, GpuVertex, Model, ModelTexture, Texels, slots};
use oag_mesh::mesh_render::Scene;
use std::sync::Arc;

use crate::hd_holo_billboard_lit_path::draw_into;

/// The numbers every craft's `flame_test` material authors.
const FLAME: Flame = Flame {
    rim_power: 10.0,
    rim_scale: 0.3,
    rim_min: 0.45,
    alpha_scale: 2.0,
    colour_scale: 1.0,
    scroll_speed: 2.0,
};

fn vertex(position: [f32; 3]) -> GpuVertex {
    GpuVertex {
        quake: 0.0,
        position,
        normal: [0.0, 0.0, 1.0],
        // Face-on the rim term is 0, so the program's alpha is
        // `2 * (1 - 0.45) = 1.1` times this: 1.0 in all.
        colour: [1.0, 1.0, 1.0, 1.0 / 1.1],
        texcoord: [0.0, 0.0],
        lightmap_texcoord: [0.0, 0.0],
        lit: 0.0,
        anim: 0,
        xform: 0,
        sun_mask: 1.0,
        slots: slots::DEFAULT,
        specular_exponent: oag_mesh::mesh::DEFAULT_SPECULAR_EXPONENT,
        glow: 0.0,
        texcoord2: [0.0, 0.0],
        tangent: 0,
    }
}

/// A flame triangle whose texture is one texel of grey `grey` with alpha 0, so
/// the noise tap displaces nothing.
fn model(grey: u8) -> Model {
    Model {
        airbrakes: [None, None],
        node_vertex_ranges: Vec::new(),
        lod_groups: Default::default(),
        label: "hd flame triangle".into(),
        indices: vec![0, 1, 2],
        draws: Vec::new(),
        transparent_draws: vec![DrawCall {
            writes_depth: false,
            moving: false,
            blend: None,
            blend_state: None,
            depth_point: None,
            layer: oag_vex::vex::LAYER_DEFAULT,
            culled: false,
            range: 0..3,
            texture: Some(0),
            bounds: Bounds {
                centre: [0.0, 0.0, 0.5],
                radius: 1.5,
            },
            node: None,
            chunk: None,
            alpha_test_ref: None,
        }],
        alpha_tested_draws: Vec::new(),
        textures: vec![Some(Arc::new(ModelTexture {
            label: "flame_01".into(),
            width: 1,
            height: 1,
            texels: Texels::Rgba8(vec![grey, grey, grey, 0]),
            mip_count: None,
        }))],
        lightmaps: Vec::new(),
        pad_masks: Vec::new(),
        wave_maps: Vec::new(),
        fifth_maps: Vec::new(),
        sixth_maps: Vec::new(),
        material_slots: Vec::new(),
        material_specular_exponent: Vec::new(),
        material_glow: Vec::new(),
        material_colour_factor: Vec::new(),
        material_quake: Vec::new(),
        material_tangent: Vec::new(),
        material_variants: Vec::new(),
        material_anim: Vec::new(),
        shine_draws: Vec::new(),
        vertex_colour_is_light: false,
        stamps_glow: false,
        glow_by_texel: false,
        glow_from_program: false,
        flame: Some(FLAME),
        absorb_shell: false,
        material_lod_bias: 0.0,
        material_state: Vec::new(),
        alpha_test_ref: None,
        centre: [0.0, 0.0, 0.5],
        radius: 2.0,
        mesh_count: 1,
        anim_tracks: Vec::new(),
        anim_nodes: Vec::new(),
        emissive: Vec::new(),
        vertices: vec![
            vertex([-0.9, -0.9, 0.5]),
            vertex([0.9, -0.9, 0.5]),
            vertex([0.0, 0.9, 0.5]),
        ],
    }
}

/// Grey `128` decodes to `0.2158`, which the encoded store keeps as `0.502`;
/// the raw texel used as light would store `0.502^(1/2.2) = 0.731`, the white
/// rest flame this pins against. On a linear target that stores what it
/// computes (Omega's) and on a gamma target the flame is unchanged: `0.502`.
#[test]
fn on_hds_encoded_store_the_colour_tap_is_decoded_and_elsewhere_it_is_not() {
    let instance = wgpu::Instance::default();
    let Some(adapter) = oag_gpu::test_adapter::request(&instance) else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("requesting the device");
    let mut scene = Scene::off();
    scene.fog.camera = [0.0, 0.0, 100.5];
    let flame = model(128);
    let at = |format, store| draw_into(&device, &queue, &flame, scene, format, store)[0];

    let encoded = at(SCENE_FORMAT, SceneStore::Encoded);
    assert!((encoded - 0.502).abs() < 0.01, "HD's store: {encoded}");
    let computed = at(SCENE_FORMAT, SceneStore::Computed);
    assert!(
        (computed - 0.502).abs() < 0.01,
        "a computed store: {computed}"
    );
    let gamma = at(wgpu::TextureFormat::Rgba8Unorm, SceneStore::Computed);
    assert!((gamma - 0.502).abs() < 0.01, "a gamma target: {gamma}");
}
