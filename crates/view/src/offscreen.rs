//! Renders a single asset to a PNG without a window.
//!
//! Useful in three ways: it works over SSH and in CI, it gives the render path
//! something a test can assert on, and it is the basis for visual regression
//! checks once there is geometry to get wrong.
//!
//! The pipeline here is deliberately *not* shared with the windowed renderer.
//! It renders the texture straight, with no checkerboard and no letterboxing,
//! so the output is exactly the decoded image and a byte comparison means
//! something.

use anyhow::{Context, Result};
use std::path::Path;

use crate::assets::Asset;

/// Renders `asset` to a PNG at `path`.
pub fn capture(asset: &Asset, path: &Path) -> Result<()> {
    let instance = wgpu::Instance::default();
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        compatible_surface: None,
        ..Default::default()
    }))
    .context("no GPU adapter available")?;

    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("oag-view offscreen"),
        ..Default::default()
    }))
    .context("requesting the device")?;

    let size = wgpu::Extent3d {
        width: asset.width,
        height: asset.height,
        depth_or_array_layers: 1,
    };

    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("asset"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });

    queue.write_texture(
        texture.as_image_copy(),
        &asset.rgba,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(asset.width * 4),
            rows_per_image: Some(asset.height),
        },
        size,
    );

    // Copies out of a texture must have rows aligned to 256 bytes, so the
    // readback buffer is usually wider than the image and needs unpadding.
    let unpadded = asset.width as usize * 4;
    let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize;
    let padded = unpadded.div_ceil(align) * align;

    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("readback"),
        size: (padded * asset.height as usize) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("capture"),
    });
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded as u32),
                rows_per_image: Some(asset.height),
            },
        },
        size,
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
    let mut pixels = Vec::with_capacity(unpadded * asset.height as usize);
    for row in mapped.chunks(padded).take(asset.height as usize) {
        pixels.extend_from_slice(&row[..unpadded]);
    }
    drop(mapped);
    readback.unmap();

    let png = oag_formats::png::encode_rgba(asset.width, asset.height, &pixels);
    std::fs::write(path, png).with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}
