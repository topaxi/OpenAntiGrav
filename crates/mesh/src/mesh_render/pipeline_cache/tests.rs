//! What [`super::Scope`] actually caches, checked against the real `build()`
//! path rather than a synthetic descriptor - a hand-written key that happened
//! to agree with itself would prove nothing about the one `build` computes.
//!
//! Its own file rather than an inline `#[cfg(test)] mod`, per the 200-line
//! rule in `scripts/check-file-size.py`.

use super::{Scope, shared_shader_module};
use crate::mesh::{Bounds, DrawCall, GpuVertex, Model};
use crate::mesh_render::{self, Anisotropy};

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

/// `None` when there is no adapter, the same guard `mesh_render::tests` and
/// the `tests/*.rs` integration suite already use.
fn device() -> Option<(wgpu::Device, wgpu::Queue)> {
    let instance = wgpu::Instance::default();
    let adapter =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
            .ok()?;
    pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).ok()
}

fn vertex(position: [f32; 3]) -> GpuVertex {
    GpuVertex {
        position,
        normal: [0.0, 0.0, 1.0],
        colour: [1.0, 1.0, 1.0, 1.0],
        texcoord: [0.0, 0.0],
        lightmap_texcoord: [0.0, 0.0],
        lit: 0.0,
        anim: 0,
        xform: 0,
        sun_mask: 1.0,
        slots: crate::mesh::slots::DEFAULT,
        specular_exponent: crate::mesh::DEFAULT_SPECULAR_EXPONENT,
        glow: 0.0,
    }
}

/// A one-triangle model - enough to build a real `Built` without a `.vex`.
fn triangle_model() -> Model {
    Model {
        airbrakes: [None, None],
        node_vertex_ranges: Vec::new(),
        lod_groups: Default::default(),
        label: "pipeline_cache test triangle".into(),
        indices: vec![0, 1, 2],
        draws: vec![DrawCall {
            moving: false,
            blend: None,
            blend_state: None,
            layer: oag_vex::vex::LAYER_DEFAULT,
            culled: false,
            range: 0..3,
            texture: None,
            bounds: Bounds {
                centre: [0.0, 0.0, 0.5],
                radius: 1.5,
            },
            node: None,
            chunk: None,
            alpha_test_ref: None,
        }],
        alpha_tested_draws: Vec::new(),
        transparent_draws: Vec::new(),
        textures: Vec::new(),
        lightmaps: Vec::new(),
        pad_masks: Vec::new(),
        wave_maps: Vec::new(),
        material_slots: Vec::new(),
        material_specular_exponent: Vec::new(),
        material_variants: Vec::new(),
        material_anim: Vec::new(),
        shine_draws: Vec::new(),
        vertex_colour_is_light: false,
        stamps_glow: false,
        glow_by_texel: false,
        flame: None,
        absorb_shell: false,
        alpha_test_ref: None,
        centre: [0.0, 0.0, 0.5],
        radius: 2.0,
        mesh_count: 1,
        anim_tracks: Vec::new(),
        anim_nodes: Vec::new(),
        emissive: Vec::new(),
        vertices: vec![
            vertex([-0.8, -0.8, 0.5]),
            vertex([0.8, -0.8, 0.5]),
            vertex([0.0, 0.8, 0.5]),
        ],
    }
}

fn build_once(device: &wgpu::Device, queue: &wgpu::Queue, model: &Model) -> mesh_render::Built {
    mesh_render::build(
        device,
        queue,
        model,
        FORMAT,
        Anisotropy::Off,
        1,
        mesh_render::Depth::Scene,
        mesh_render::TRANSPARENT_BLEND,
        mesh_render::GlowMask::Protected,
        mesh_render::Velocity::Write,
        &mesh_render::zone::StageArt::NONE,
        mesh_render::ShadowMaps::NONE,
        mesh_render::ShadowReceiver::Never,
    )
    .expect("building the mesh pipeline")
}

/// Two `build()` calls for the same kind of drawable, inside one open
/// [`Scope`], must hand back the identical shader module and the identical
/// opaque pipeline - `wgpu::RenderPipeline` and `wgpu::ShaderModule` both
/// compare by the handle they wrap, not their contents, so this is the same
/// check a stale-cache bug would fail.
#[test]
fn two_identical_builds_share_the_shader_and_the_pipeline_inside_one_scope() {
    let Some((device, queue)) = device() else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    let model = triangle_model();

    let scope = Scope::open();
    let first = build_once(&device, &queue, &model);
    let second = build_once(&device, &queue, &model);
    drop(scope);

    assert_eq!(
        first.pipeline, second.pipeline,
        "the second call should have reused the first's opaque pipeline"
    );
    assert_eq!(
        first.alpha_test_pipeline, second.alpha_test_pipeline,
        "the second call should have reused the first's cutout pipeline"
    );
}

/// The counts a scope reports back what actually happened: two `build()`
/// calls for the same drawable ask for the shared shader module many times
/// (once up front, once per pipeline built from it) and every one past the
/// first is a hit, and the two calls' pipelines collapse to as many distinct
/// ones as one `build()` call alone creates - the second call reuses every
/// single one of them.
#[test]
fn scope_counts_a_hit_for_every_call_past_the_first() {
    let Some((device, queue)) = device() else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    let model = triangle_model();

    let scope = Scope::open();
    build_once(&device, &queue, &model);
    let (shader_calls_after_one, shader_hits_after_one) = scope.shader_counts();
    let (pipeline_calls_after_one, pipeline_hits_after_one, pipelines_after_one) =
        scope.pipeline_counts();
    assert_eq!(pipeline_hits_after_one, 0, "the first build hits nothing");

    build_once(&device, &queue, &model);
    let (shader_calls, shader_hits) = scope.shader_counts();
    let (pipeline_calls, pipeline_hits, pipelines) = scope.pipeline_counts();

    assert!(
        shader_calls > shader_calls_after_one,
        "a second build must ask for the shared module again"
    );
    assert!(
        shader_hits > shader_hits_after_one,
        "and every one of those asks past the very first must be a hit"
    );
    // The second build asks for exactly the set of pipelines the first one
    // did, and every one of them is now cached, so it doubles the call count
    // and turns the first build's own pipeline count into hits, without
    // creating anything new.
    assert_eq!(pipeline_calls, 2 * pipeline_calls_after_one);
    assert_eq!(pipeline_hits as usize, pipelines_after_one);
    assert_eq!(
        pipelines, pipelines_after_one,
        "the second build must not have created any pipeline the first did not"
    );
}

/// Outside a [`Scope`], `build()` keeps its old behaviour exactly: a fresh
/// shader module and fresh pipelines every call, so a caller that builds one
/// model at a time - the viewer, every other test in this crate - is
/// unaffected by this cache existing at all.
#[test]
fn with_no_scope_open_every_build_creates_its_own_shader_and_pipelines() {
    let Some((device, queue)) = device() else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    let model = triangle_model();

    let first = build_once(&device, &queue, &model);
    let second = build_once(&device, &queue, &model);

    assert_ne!(
        first.pipeline, second.pipeline,
        "with no scope open, nothing should be shared"
    );

    let a = shared_shader_module(&device);
    let b = shared_shader_module(&device);
    assert_ne!(a, b, "with no scope open, the shader module is not cached");
}

/// Two models naming one decoded texture - a team's craft - upload it once
/// inside a scope, and a model naming a *different* texture uploads its own.
#[test]
fn a_texture_shared_by_two_models_is_uploaded_once_per_scope() {
    let Some((device, queue)) = device() else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    let shared = std::sync::Arc::new(crate::mesh::ModelTexture::rgba8(
        "shared".into(),
        1,
        1,
        vec![255; 4],
        None,
    ));
    let other = std::sync::Arc::new(crate::mesh::ModelTexture::rgba8(
        "other".into(),
        1,
        1,
        vec![0; 4],
        None,
    ));
    let mut a = triangle_model();
    a.textures = vec![Some(shared.clone())];
    let mut b = triangle_model();
    b.textures = vec![Some(shared.clone())];
    let mut c = triangle_model();
    c.textures = vec![Some(other)];

    let scope = Scope::open();
    build_once(&device, &queue, &a);
    build_once(&device, &queue, &b);
    build_once(&device, &queue, &c);
    assert_eq!(scope.texture_counts(), (3, 1), "3 asked for, 1 reused");
}

/// An adapter without `TEXTURE_COMPRESSION_BC` gets a decoded picture from a
/// BC7 chain rather than nothing - the default device asks for no features.
#[test]
fn a_bc7_chain_uploads_on_a_device_without_block_compression() {
    let Some((device, queue)) = device() else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    assert!(
        !device
            .features()
            .contains(wgpu::Features::TEXTURE_COMPRESSION_BC)
    );
    let texture = crate::mesh::ModelTexture {
        label: "bc7".into(),
        width: 4,
        height: 4,
        texels: crate::mesh::Texels::Blocks {
            format: crate::mesh::BlockFormat::Bc7,
            levels: vec![vec![1; 16]],
        },
        mip_count: None,
    };
    let view = crate::mesh_render::texture::upload(&device, &queue, &texture, false);
    drop(view);
}

/// A scene lit by any rig but Omega's compiles the nova curve out of every
/// pipeline it builds, and one lit by Omega's - or no scene at all - keeps
/// `mesh.wesl`'s live default.
#[test]
fn lit_by_compiles_the_nova_curve_out_only_under_a_rig_that_never_takes_it() {
    assert_eq!(
        super::scope_constants(),
        Vec::new(),
        "no scope, no constants"
    );

    let scope = Scope::open();
    scope.lit_by(&mesh_render::Light::stand_in().with_nova_prelit(1.4, 0.2, 1.5));
    assert_eq!(super::scope_constants(), Vec::new());
    drop(scope);

    let scope = Scope::open();
    scope.lit_by(&mesh_render::Light::stand_in());
    assert_eq!(super::scope_constants(), vec![("nova_prelit", 0.0)]);
    drop(scope);
    assert_eq!(
        super::scope_constants(),
        Vec::new(),
        "closed with its scope"
    );
}
