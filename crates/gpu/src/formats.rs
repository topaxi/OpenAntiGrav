//! The formats of the offscreen targets the scene pass draws into and the
//! post chain reads.
//!
//! Defined here rather than beside the pass that owns each, because the other
//! side of the pair needs the value in real code (`oag-mesh` gates its bloom
//! output on [`SCENE_FORMAT`]; `oag-post`'s FSR 3 readback samples
//! [`VELOCITY_FORMAT`]) and neither crate may depend on the other.

/// The screen-space velocity target's format. `Rg16Float` is enough for a uv
/// delta and is multisample-renderable at 4x - pinned by
/// `rg16float_is_multisample_renderable_at_4x_on_this_adapter` in
/// `oag_post::motion_blur`'s tests, with `Rgba16Float` as the documented
/// fallback should an adapter ever fail that pin.
pub const VELOCITY_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rg16Float;

/// The offset target blend class 8 draws into and the Omega composite reads.
///
/// **Chosen, not measured.** The executable's target is `R8G8_SNORM` (Gnm word
/// `0x22c103`, `docs/ghidra/functions/ps4-omega-eu/heat-haze.md`, confidence
/// 75) and WebGPU cannot render to `rg8snorm`, so the offsets are summed in
/// half floats and the reader clamps what it reads to the `[-1, 1]` an SNORM
/// write would have left.
pub const DISTORTION_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rg16Float;

/// The scene target's format: linear, and wide enough for the bloom gate to
/// see luminance above 1.0. Drawing into it is what tells every mesh pipeline
/// to output linear light - see `oag_mesh::mesh_render::is_linear_target`.
pub const SCENE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;
