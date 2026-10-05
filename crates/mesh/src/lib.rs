//! The mesh pipeline: a `.vex` or `RCSMODEL` model decoded into one portable
//! vertex and index buffer, and the pipeline that draws it.
//!
//! Split out of `oag-render` so the two crates that grew around it - the
//! post-processing chain (`oag-post`) and the renderer's effects - build
//! beside it rather than inside one 63,000-line crate. It owns no window and no
//! event loop, for the reason `oag-render` does not: every entry point takes a
//! `wgpu::Device`, a `wgpu::Queue` and the target format its caller has.
//!
//! - [`mesh`] decodes a model into one portable vertex and index buffer.
//! - [`mesh_render`] is the pipeline that draws it, offscreen or into a surface.
//! - [`orbit`] is the viewer's orbit camera, which `mesh_render` frames a model
//!   with.
//! - [`capture`] renders one frame headless and returns or writes the pixels.

pub mod capture;
pub mod mesh;
pub mod mesh_render;
pub mod orbit;
