//! Builds the LeachBeam ribbon's pipeline, pulled out of `Scene::new` under
//! the 1,000-line rule in `scripts/check-file-size.py` - a move, with no
//! behaviour change.

use oag_render::exhaust::FlareTexture;

/// `None` when `texture` did not decode - `load`'s report already says why,
/// and the ribbon draws nothing rather than a stand-in.
pub(super) fn build(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    format: wgpu::TextureFormat,
    texture: Option<&FlareTexture>,
    sample_count: u32,
) -> Option<std::cell::RefCell<oag_render::beam::Pipeline>> {
    texture.map(|texture| {
        std::cell::RefCell::new(oag_render::beam::Pipeline::new(
            device,
            queue,
            format,
            texture,
            sample_count,
            super::mesh_render::Velocity::Write,
        ))
    })
}
