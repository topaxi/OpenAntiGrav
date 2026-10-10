//! A model's vertex and index writes, parked for a frame under a
//! [`oag_gpu::deferred_upload::Scope`].
//!
//! **The contract.** A caller of [`super::build_with`] that passes
//! `defer_uploads` must hand the model's geometry to [`defer_geometry`] once it
//! holds the `Arc` the bytes will be read from, or its buffers stay zeroed
//! while a scope is open. `race::Drawable` is the only such caller.

use crate::mesh::Model;

/// Parks the geometry writes [`super::build_with`] skipped, reading from `model`
/// itself rather than a copy of it.
pub fn defer_geometry(
    vertex_buffer: &wgpu::Buffer,
    index_buffer: &wgpu::Buffer,
    model: &std::sync::Arc<Model>,
) {
    struct Vertices(std::sync::Arc<Model>);
    struct Indices(std::sync::Arc<Model>);
    impl oag_gpu::deferred_upload::Bytes for Vertices {
        fn bytes(&self) -> &[u8] {
            bytemuck::cast_slice(&self.0.vertices)
        }
    }
    impl oag_gpu::deferred_upload::Bytes for Indices {
        fn bytes(&self) -> &[u8] {
            bytemuck::cast_slice(&self.0.indices)
        }
    }
    let vertices = std::mem::size_of_val(&model.vertices[..]);
    let indices = std::mem::size_of_val(&model.indices[..]);
    oag_gpu::deferred_upload::defer_buffer(
        vertex_buffer,
        0,
        std::sync::Arc::new(Vertices(model.clone())),
        0..vertices,
    );
    oag_gpu::deferred_upload::defer_buffer(
        index_buffer,
        0,
        std::sync::Arc::new(Indices(model.clone())),
        0..indices,
    );
}

#[cfg(test)]
mod tests;
