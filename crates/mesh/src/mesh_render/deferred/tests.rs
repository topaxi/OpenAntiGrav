//! The race path's geometry deferral against the real `build()`: a build under
//! a scope leaves the buffers unwritten and [`defer_geometry`] is what fills
//! them, so dropping that call fails here. That the bytes then land whole is
//! `oag_gpu::deferred_upload`'s own readback test: a vertex buffer has no
//! `COPY_SRC` to read it back through.

use std::sync::Arc;

use oag_gpu::deferred_upload::{self, Scope};

use super::{defer_geometry, geometry_deferred};
use crate::mesh_render::pipeline_cache::tests::{build_once, device, triangle_model};

/// A model whose geometry is past the deferral threshold.
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
    assert!(!geometry_deferred(), "no scope, no deferral");
    let scope = Scope::open(&queue);
    assert!(geometry_deferred());
    let built = build_once(&device, &queue, &model);
    // The build parks its small constants itself and leaves the geometry to
    // `defer_geometry`: nothing of the model's is waiting yet.
    let constants = deferred_upload::pending_bytes();
    assert!(constants < 64 * 1024);
    let model = Arc::new(model);
    defer_geometry(&built.vertex_buffer, &built.index_buffer, &model);
    let waiting = deferred_upload::pending_bytes();
    assert_eq!(
        waiting - constants,
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
