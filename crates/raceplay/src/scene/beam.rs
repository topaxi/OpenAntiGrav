//! Builds the LeachBeam ribbon's pipeline, pulled out of `Scene::new` under
//! the 1,000-line rule in `scripts/check-file-size.py` - a move, with no
//! behaviour change.

use oag_fx::exhaust::FlareTexture;

const POOL_SIZE: usize = oag_weapons::projectile::leach_beam::POOL_SIZE;

/// `None` when `texture` did not decode - `load`'s report already says why,
/// and the ribbon draws nothing rather than a stand-in.

pub(super) fn build(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    format: wgpu::TextureFormat,
    store: oag_gpu::formats::SceneStore,
    texture: Option<&FlareTexture>,
    sample_count: u32,
) -> Option<std::cell::RefCell<oag_fx::beam::Pipeline>> {
    texture.map(|texture| {
        std::cell::RefCell::new(oag_fx::beam::Pipeline::with_style(
            device,
            queue,
            format,
            store,
            texture,
            sample_count,
            super::mesh_render::Velocity::Write,
            // Room for every pool entry's ribbon, each bridged to the next by two
            // degenerate vertices (`Race::leach_beam_ribbon_vertices`).
            oag_fx::beam::pipeline::Style {
                capacity: POOL_SIZE * (oag_fx::beam::MAX_VERTICES + 2),
                ..oag_fx::beam::pipeline::Style::BEAM
            },
        ))
    })
}
