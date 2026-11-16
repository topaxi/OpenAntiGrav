//! A single-level texture born holding its pixels, and the clamped bilinear sampler a sprite
//! is read through.
//!
//! The effect, shadow and loading pipelines each upload one decoded image as a sampled,
//! one-mip 2D texture and bind it with the same sampler. This is that upload and that
//! sampler, once. The mip-chain upload is `oag_mesh`'s own and the frame's writes through a
//! [`crate::writes::Log`] are not this: the data goes to the queue directly.

#[cfg(test)]
mod tests;

/// A `width` by `height` texture of `format`, usage `TEXTURE_BINDING | COPY_DST`, one mip and
/// one sample, holding `data`.
///
/// **A zero extent becomes one texel**, in the texture, the write's extent and its row
/// stride alike, so a decoder's empty image is a 1x1 texture rather than a validation
/// error; `data` is then expected to hold that texel. `data` is tightly packed rows of
/// an uncompressed colour `format` (its texel size is the row stride).
///
/// # Panics
///
/// `format` has no single-aspect copy size (a depth format) or is block compressed.
pub fn upload_2d(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    label: &str,
    format: wgpu::TextureFormat,
    (width, height): (u32, u32),
    data: &[u8],
) -> wgpu::Texture {
    let (width, height) = (width.max(1), height.max(1));
    assert_eq!(
        format.block_dimensions(),
        (1, 1),
        "{label}: {format:?} is block compressed"
    );
    let texel = format
        .block_copy_size(None)
        .unwrap_or_else(|| panic!("{label}: {format:?} has no copy size"));
    let size = wgpu::Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    };
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        texture.as_image_copy(),
        data,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(width * texel),
            rows_per_image: Some(height),
        },
        size,
    );
    texture
}

/// [`upload_2d`] for an RGBA8 image sampled raw (`Rgba8Unorm`, no sRGB decode).
pub fn upload_rgba8(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    label: &str,
    size: (u32, u32),
    rgba: &[u8],
) -> wgpu::Texture {
    upload_2d(
        device,
        queue,
        label,
        wgpu::TextureFormat::Rgba8Unorm,
        size,
        rgba,
    )
}

/// Bilinear (`Linear` mag and min) and clamped to the edge on every axis: a sample past
/// `0..1` repeats the border texel instead of wrapping round to the opposite side.
pub fn clamp_linear_sampler(device: &wgpu::Device, label: &str) -> wgpu::Sampler {
    device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some(label),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    })
}
