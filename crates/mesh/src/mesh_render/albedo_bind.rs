//! A material bind group around a texture the caller renders into, for the
//! billboard adverts: a slot of a model whose picture is drawn each frame
//! instead of decoded once.

use super::texture;

/// Bind group 1 for a material whose albedo is `albedo`, built against
/// `layout` - the pipeline's own `get_bind_group_layout(1)`.
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
pub fn albedo_bind_group(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
    albedo: &wgpu::TextureView,
    label: &str,
) -> wgpu::BindGroup {
    let flat = |rgba: [u8; 4], what: &str| {
        texture::upload_rgba(device, queue, 1, 1, &rgba, what, None)
            .expect("a 1x1 texture fits every device")
            .view
    };
    let lightmap = flat([0, 0, 0, 255], "card: no lightmap");
    let pad_mask = flat([128, 128, 255, 0], "card: no pad mask");
    let wave = flat([255, 255, 255, 255], "card: no wave map");
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("card albedo"),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    });
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some(label),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(albedo),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&sampler),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::TextureView(&lightmap),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: wgpu::BindingResource::TextureView(&pad_mask),
            },
            wgpu::BindGroupEntry {
                binding: 4,
                resource: wgpu::BindingResource::TextureView(&wave),
            },
        ],
    })
}
