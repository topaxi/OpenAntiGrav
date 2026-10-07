//! The headless capture's target, its readback and its PNG - moved out of
//! `capture.rs` under the 1,000-line rule in `scripts/check-file-size.py`,
//! with no behaviour change.

use anyhow::{Context, Result};

/// The texture a headless capture draws into.
///
/// `Rgba8Unorm` at both call sites rather than the surface's sRGB format: the
/// readback is written straight into a PNG, so a second gamma encode would
/// double-correct.
pub(super) fn offscreen(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    width: u32,
    height: u32,
) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("capture"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    })
}

/// Submits `encoder`, then copies the drawn texture back as tightly packed RGBA.
///
/// Copies out of a texture must have rows aligned to 256 bytes, so the readback
/// buffer is usually wider than the image and needs unpadding.
pub(super) fn read_back(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    mut encoder: wgpu::CommandEncoder,
    target: &wgpu::Texture,
    width: u32,
    height: u32,
) -> Result<Vec<u8>> {
    let unpadded = width as usize * 4;
    let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize;
    let padded = unpadded.div_ceil(align) * align;

    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("readback"),
        size: (padded * height as usize) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    encoder.copy_texture_to_buffer(
        target.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded as u32),
                rows_per_image: Some(height),
            },
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    queue.submit(Some(encoder.finish()));

    let slice = readback.slice(..);
    slice.map_async(wgpu::MapMode::Read, |_| {});
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .context("waiting for the GPU")?;

    let mapped = slice
        .get_mapped_range()
        .context("mapping the readback buffer")?;
    let mut pixels = Vec::with_capacity(unpadded * height as usize);
    for row in mapped.chunks(padded).take(height as usize) {
        pixels.extend_from_slice(&row[..unpadded]);
    }
    drop(mapped);
    readback.unmap();
    Ok(pixels)
}

pub(super) fn write_png(
    path: &std::path::Path,
    width: u32,
    height: u32,
    pixels: &[u8],
) -> Result<()> {
    // A screenshot is opaque. The frame's alpha channel is the bloom mask and
    // not coverage - see `oag_mesh::capture::make_opaque` - so encoding it
    // straight from the readback writes a fully transparent PNG.
    let mut pixels = pixels.to_vec();
    oag_mesh::capture::make_opaque(&mut pixels);
    let png = oag_texture::png::encode_rgba(width, height, &pixels);
    std::fs::write(path, png).with_context(|| format!("writing {}", path.display()))?;
    println!("wrote {} ({width}x{height})", path.display());
    Ok(())
}

/// The TOUCH CONTROLS page's preview over the finished frame, when that is
/// the page being captured; the window draws it in its overlay pass.
pub(super) fn touch_preview(
    options: &super::Options,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    encoder: &mut wgpu::CommandEncoder,
    target: &wgpu::Texture,
) -> Result<()> {
    if options.menu_page.as_deref() != Some(crate::touch_controls::preview::PAGE) {
        return Ok(());
    }
    crate::touch_controls::draw_preview(
        &options.settings.controls,
        (device, queue, target.format()),
        encoder,
        &target.create_view(&wgpu::TextureViewDescriptor::default()),
        (target.width(), target.height()),
    )
}
