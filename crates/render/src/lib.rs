//! The wgpu renderer.
//!
//! Shared by `oag-view` and `oag-game` so there is exactly one mesh pipeline,
//! one track-ribbon builder and one camera convention in the repository. Nothing
//! in the simulation may depend on this crate; see
//! `docs/architecture/workspace-layout.md`.
//!
//! The crate owns no window and no event loop, on purpose. Every entry point
//! takes a `wgpu::Device`, a `wgpu::Queue` and the target `TextureFormat` its
//! caller already has, which is what lets the viewer draw into its own surface
//! and the game into its own without either learning about the other's
//! windowing. That is also why there is no `winit` dependency here.
//!
//! Modules:
//!
//! - [`mesh`] decodes a `.vex` model into one portable vertex and index buffer.
//! - [`exhaust`] is the ship's engine flare: the recovered state machine, and the
//!   one blended pipeline in the crate.
//! - [`track`] builds the driveable ribbon from a decoded track spline.
//! - [`collision`] builds a debug view of the collision soup the physics world
//!   is made of.
//! - [`mesh_render`] is the pipeline that draws either of those, offscreen or
//!   into a surface.
//! - [`camera`] is the camera maths: orbit, free and the chase spring.
//! - [`pvs`] places draw calls into the track's authored visibility sections,
//!   and is the first tier of the two-tier cull the draw loop runs.

pub mod camera;
pub mod collision;
pub mod exhaust;
pub mod mesh;
pub mod mesh_render;
pub mod pvs;
pub mod track;
