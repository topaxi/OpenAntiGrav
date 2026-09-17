//! Building [`oag_render::weapon_quads::Pipeline`] for [`super::Scene`].
//!
//! The state and the maths - the bolt streak's two crossed quads, the
//! muzzle flash's rotated one - are all `oag_render::weapon_quads`'; see
//! that module's own doc and
//! `docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md` for the
//! evidence. This is only the seam: the placeholder-on-absence fallback and
//! the one construction call, split out for the same reason
//! [`super::clouds`] is - see `scripts/check-file-size.py`.

use super::*;

/// Builds the pipeline, falling back to [`FlareTexture::placeholder`] for
/// either texture that did not decode - the same terms `flare`/`noise`
/// already use in [`super::Scene::new`]. Wrapped in the `RefCell`
/// [`super::Scene`] wants, so the call at the one site this has stays on
/// one line.
pub(super) fn build(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    format: wgpu::TextureFormat,
    textures: (Option<FlareTexture>, Option<FlareTexture>),
    sample_count: u32,
) -> std::cell::RefCell<oag_render::weapon_quads::Pipeline> {
    let (bolt, flash) = textures;
    let bolt = bolt.unwrap_or_else(|| FlareTexture::placeholder(64));
    let flash = flash.unwrap_or_else(|| FlareTexture::placeholder(64));
    std::cell::RefCell::new(oag_render::weapon_quads::Pipeline::new(
        device,
        queue,
        format,
        &bolt,
        &flash,
        sample_count,
        // The race pass carries the velocity attachment - see
        // `mesh_render::Velocity` and `oag_render::weapon_quads`'s own doc
        // comment for why this pipeline still masks its own write.
        mesh_render::Velocity::Write,
    ))
}
