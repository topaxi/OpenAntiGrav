//! The material of a slot whose albedo is a texture the caller renders into, for the
//! billboard adverts: a slot of a model whose picture is drawn each frame
//! instead of decoded once.

use super::bindless::Material;
use super::texture;

/// The material of a slot whose albedo is `albedo`. A caller replacing several slots
/// builds this once and swaps the albedo into a copy for each, so the slots share one
/// sampler and one of each 1x1 (one element each of the bindless arrays).
///
/// Everything besides the albedo is what a slot with no lightmap, no pad mask
/// and no wave map binds in [`super::build`]: black with alpha 1, a flat tangent
/// normal with zero alpha, and white for the wave and the fifth and sixth pictures. The sampler repeats where `wrap` says the
/// title's own does (HD, measured), else clamps, and filters linearly with no mip
/// chain to choose from.
///
/// # Panics
///
/// If a 1x1 texture will not upload, which no device refuses.
#[must_use]
pub fn card_material(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    albedo: &wgpu::TextureView,
    wrap: bool,
) -> Material {
    let address = if wrap {
        wgpu::AddressMode::Repeat
    } else {
        wgpu::AddressMode::ClampToEdge
    };
    let flat = |rgba: [u8; 4], what: &str| {
        texture::upload_rgba(device, queue, 1, 1, &rgba, what, None)
            .expect("a 1x1 texture fits every device")
            .view
    };
    let white = flat([255, 255, 255, 255], "card: white");
    Material {
        albedo: albedo.clone(),
        sampler: device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("card albedo"),
            address_mode_u: address,
            address_mode_v: address,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        }),
        lightmap: flat([0, 0, 0, 255], "card: no lightmap"),
        pad_mask: flat([128, 128, 255, 0], "card: no pad mask"),
        wave: white.clone(),
        fifth: white.clone(),
        sixth: white,
    }
}
