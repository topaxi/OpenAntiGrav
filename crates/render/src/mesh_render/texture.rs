//! Getting a [`ModelTexture`] onto the GPU: its mip chain, and its upload.
//!
//! Split out of `mesh_render.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`.

use crate::mesh::{ModelTexture, Texels};

/// Downsamples `rgba` into a full mip chain by repeated 2x2 box filtering,
/// down to a 1x1 level.
///
/// Track and ship textures are seen at every distance and grazing angle a
/// chase camera produces; without mips, minification aliases into shimmer
/// that a single sample can't fix. Filtering happens on `Rgba8UnormSrgb`
/// sample data, matching what the sampler itself blends between levels.
pub(super) fn mip_chain(width: u32, height: u32, rgba: &[u8]) -> Vec<(u32, u32, Vec<u8>)> {
    let mut levels: Vec<(u32, u32, Vec<u8>)> = vec![(width, height, rgba.to_vec())];
    loop {
        let (w, h, data) = levels.last().expect("levels is never empty");
        let (w, h) = (*w, *h);
        if w == 1 && h == 1 {
            break;
        }
        let next_width = (w / 2).max(1);
        let next_height = (h / 2).max(1);
        let mut next = vec![0u8; (next_width * next_height * 4) as usize];
        for y in 0..next_height {
            let y0 = (y * 2).min(h - 1);
            let y1 = (y * 2 + 1).min(h - 1);
            for x in 0..next_width {
                let x0 = (x * 2).min(w - 1);
                let x1 = (x * 2 + 1).min(w - 1);
                for c in 0..4usize {
                    let texel = |sx: u32, sy: u32| data[((sy * w + sx) * 4) as usize + c] as u32;
                    let sum = texel(x0, y0) + texel(x1, y0) + texel(x0, y1) + texel(x1, y1);
                    next[((y * next_width + x) * 4) as usize + c] = ((sum + 2) / 4) as u8;
                }
            }
        }
        levels.push((next_width, next_height, next));
    }
    levels
}

/// **Raw, so the sampler hands the shader the disc's own bytes.**
///
/// The GE blends stored bytes, so every stage of this pipeline works in gamma
/// space and nothing linearises - see
/// [ADR-0020](../../../docs/architecture/adr/0020-gamma-authoritative-colour-space.md).
///
/// This was `Rgba8UnormSrgb`. Vertex colour was never sRGB-decoded (`mesh.rs`
/// builds it as `byte / 255.0`), so under that format the shader multiplied a
/// *linear* texel by a *gamma* vertex colour - two spaces in one expression -
/// and `race.rs` carried a load-time re-encode of the boost plume's texels
/// purely to cancel it. That re-encode is gone with this; putting the sRGB
/// format back without restoring it would darken the plume by the same ~30% the
/// re-encode was compensating for.
pub(super) const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

/// Uploads straight RGBA8 with a box-filtered mip chain, and returns its view.
pub(super) fn upload_rgba(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    width: u32,
    height: u32,
    rgba: &[u8],
    label: &str,
) -> wgpu::TextureView {
    let mips = mip_chain(width, height, rgba);
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: mips.len() as u32,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    for (level, (mip_width, mip_height, mip_rgba)) in mips.iter().enumerate() {
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: level as u32,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            mip_rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(mip_width * 4),
                rows_per_image: Some(*mip_height),
            },
            wgpu::Extent3d {
                width: *mip_width,
                height: *mip_height,
                depth_or_array_layers: 1,
            },
        );
    }
    texture.create_view(&wgpu::TextureViewDescriptor::default())
}

/// Uploads one [`ModelTexture`], in the form it came in.
///
/// `blocks` is whether the device has `TEXTURE_COMPRESSION_BC`. Without it a
/// block-compressed texture is decoded back to RGBA8 here rather than left
/// undrawn - one memory win traded away on an adapter that cannot take it,
/// which is the GL backend and nothing this project ships by default.
pub(super) fn upload(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &ModelTexture,
    blocks: bool,
) -> wgpu::TextureView {
    let Texels::Blocks { format, levels } = &texture.texels else {
        let rgba = texture.rgba().expect("the other arm is Texels::Blocks");
        return upload_rgba(
            device,
            queue,
            texture.width,
            texture.height,
            rgba,
            &texture.label,
        );
    };
    if !blocks {
        // `first()` rather than `[0]`: `Texels::Blocks` is a public variant, and
        // an empty chain built elsewhere should decode to nothing and bind a
        // blank rather than panic here. `skin::blocks` never produces one.
        let decoded = levels.first().and_then(|base| {
            oag_formats::gtf::decode_level(format.as_gtf(), base, texture.width, texture.height, 0)
        });
        let rgba: Vec<u8> = decoded.into_iter().flatten().flatten().collect();
        // A chain that decoded to nothing binds a blank rather than sending
        // `mip_chain` off the end of an empty buffer.
        let full = u64::from(texture.width) * u64::from(texture.height) * 4;
        if rgba.len() as u64 != full {
            return upload_rgba(device, queue, 1, 1, &[255, 255, 255, 255], &texture.label);
        }
        return upload_rgba(
            device,
            queue,
            texture.width,
            texture.height,
            &rgba,
            &texture.label,
        );
    }
    let gpu = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(&texture.label),
        size: wgpu::Extent3d {
            width: texture.width,
            height: texture.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: levels.len() as u32,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: format.wgpu(),
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    for (level, texels) in levels.iter().enumerate() {
        // **The block-aligned size, not the level's own.** A 2048x2048 chain
        // ends 2x2 and 1x1, and a compressed copy is validated in whole blocks
        // against the level's *physical* extent - which WebGPU defines as the
        // logical one rounded up to the block grid. Passing the logical size
        // there is "Copy height is not a multiple of block height", and the
        // last two levels of every mipped texture hit it.
        let width = (texture.width >> level).max(1).div_ceil(4) * 4;
        let height = (texture.height >> level).max(1).div_ceil(4) * 4;
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &gpu,
                mip_level: level as u32,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            texels,
            // Block rows, not texel rows: a compressed level is
            // `ceil(w/4) x ceil(h/4)` blocks of `block_len()` bytes, which is
            // what `oag_formats::gtf::Texture::level_len` sliced out.
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width / 4 * format.block_len()),
                rows_per_image: Some(height / 4),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
    }
    gpu.create_view(&wgpu::TextureViewDescriptor::default())
}
