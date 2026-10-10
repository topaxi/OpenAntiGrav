//! The race path's geometry deferral against the real `build()`: a build under
//! a scope leaves the buffers unwritten and [`defer_geometry`] is what fills
//! them, so dropping that call fails here. That the bytes then land whole is
//! `oag_gpu::deferred_upload`'s own readback test: a vertex buffer has no
//! `COPY_SRC` to read it back through.

use std::sync::Arc;

use oag_gpu::deferred_upload::{self, Scope};

use super::defer_geometry;
use crate::mesh_render::pipeline_cache::tests::{build_once, device, triangle_model};

fn build_deferring(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    model: &crate::mesh::Model,
) -> crate::mesh_render::Built {
    use crate::mesh_render::{self, Anisotropy};
    mesh_render::build_with(
        device,
        queue,
        model,
        wgpu::TextureFormat::Rgba8Unorm,
        Anisotropy::Off,
        1,
        mesh_render::Depth::Scene,
        mesh_render::TRANSPARENT_BLEND,
        mesh_render::GlowMask::Protected,
        mesh_render::Velocity::Write,
        &mesh_render::zone::StageArt::NONE,
        mesh_render::ShadowMaps::NONE,
        mesh_render::ShadowReceiver::Never,
        mesh_render::Texcoords::Interleaved,
        false,
        false,
        true,
    )
    .expect("building the mesh pipeline")
}

/// A model with enough geometry for several drain steps.
fn large_model() -> crate::mesh::Model {
    let mut model = triangle_model();
    let vertex = model.vertices[0];
    model.vertices = (0..8000u32)
        .map(|i| {
            let mut v = vertex;
            v.position = [i as f32, (i % 7) as f32, 0.5];
            v
        })
        .collect();
    model.indices = (0..24000u32).map(|i| i % 8000).collect();
    model
}

#[test]
fn a_build_under_a_scope_leaves_its_geometry_to_the_drain() {
    let Some((device, queue)) = device() else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    let model = large_model();
    let scope = Scope::open(&queue);
    // A caller that does not ask (the viewer, a preview) keeps immediate
    // geometry whatever scope is open: only its small constants are parked.
    let plain = build_once(&device, &queue, &model);
    let constants = deferred_upload::pending_bytes();
    assert!(constants < 64 * 1024);
    drop(plain);
    let built = build_deferring(&device, &queue, &model);
    assert_eq!(
        deferred_upload::pending_bytes(),
        2 * constants,
        "a deferring build parks its constants twice over and no geometry"
    );
    let model = Arc::new(model);
    defer_geometry(&built.vertex_buffer, &built.index_buffer, &model);
    let waiting = deferred_upload::pending_bytes();
    assert_eq!(
        waiting - 2 * constants,
        (size_of_val(&model.vertices[..]) + size_of_val(&model.indices[..])) as u64
    );

    // A frame's worth at a time, never the whole.
    let mut calls = 0;
    while deferred_upload::drain(|| false) > 0 {
        calls += 1;
    }
    assert!(calls > 1, "{waiting} bytes took {calls} drains");
    assert_eq!(deferred_upload::pending_bytes(), 0);
    drop(scope);
}
