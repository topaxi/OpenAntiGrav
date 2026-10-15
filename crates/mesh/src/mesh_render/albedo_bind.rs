//! A material bind group around a texture the caller renders into, for the
//! billboard adverts: a slot of a model whose picture is drawn each frame
//! instead of decoded once.

use super::bindless::Material;
use super::texture;

/// Bind group 1 for a material whose albedo is `albedo`, built against
/// `layout` - [`super::material_bind_group_layout`]. [`card_material`]'s group.
#[must_use]
pub fn albedo_bind_group(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
    albedo: &wgpu::TextureView,
    label: &str,
) -> wgpu::BindGroup {
    card_material(device, queue, albedo).bind_group(device, layout, label)
}

/// The material of a slot whose albedo is `albedo`.
///
/// Everything besides the albedo is what a slot with no lightmap, no pad mask
/// and no wave map binds in [`super::build`]: black with alpha 1, a flat tangent
/// normal with zero alpha, and white. The sampler clamps, because the picture is
/// one whole frame and not a tiling surface, and filters linearly with no mip
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
) -> Material {
    let flat = |rgba: [u8; 4], what: &str| {
        texture::upload_rgba(device, queue, 1, 1, &rgba, what, None)
            .expect("a 1x1 texture fits every device")
            .view
    };
    Material {
        albedo: albedo.clone(),
        sampler: device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("card albedo"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        }),
        lightmap: flat([0, 0, 0, 255], "card: no lightmap"),
        pad_mask: flat([128, 128, 255, 0], "card: no pad mask"),
        wave: flat([255, 255, 255, 255], "card: no wave map"),
    }
}
