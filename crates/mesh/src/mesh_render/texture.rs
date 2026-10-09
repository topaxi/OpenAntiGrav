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
///
/// `max_levels` caps the synthesised chain at the source asset's own declared
/// depth, where known - see [`crate::mesh::ModelTexture::mip_count`]. `None`
/// keeps the full chain down to 1x1, today's behaviour for every path that
/// has not measured its own asset's depth yet.
///
/// **A base level wider than the device allows is not uploaded**: the chain
/// goes up from the first level that fits (see [`first_fitting_level`]), and
/// the number of levels left out is returned beside the view. `None` when no
/// level fits.
pub(super) fn upload_rgba(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    width: u32,
    height: u32,
    rgba: &[u8],
    label: &str,
    max_levels: Option<u32>,
) -> Option<Placed> {
    let mut mips = mip_chain(width, height, rgba);
    if let Some(max_levels) = max_levels {
        mips.truncate(max_levels as usize);
    }
    let skip = first_fitting_level(
        width,
        height,
        mips.len(),
        device.limits().max_texture_dimension_2d,
        1,
    )?;
    let levels = mips[skip..]
        .iter()
        .map(|(w, h, texels)| (*w, *h, texels.as_slice()));
    let gpu_bytes = levels
        .clone()
        .map(|(_, _, texels)| texels.len() as u64)
        .sum();
    Some(Placed {
        view: upload_levels(device, queue, label, levels),
        gpu_bytes,
        dropped_levels: skip as u32,
    })
}

/// A texture on the GPU: its view, what it occupies, and how many of the
/// source's leading mip levels were left out to fit the device.
pub(super) struct Placed {
    pub(super) view: wgpu::TextureView,
    /// Read by the native texture sink only; the web build has none.
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    pub(super) gpu_bytes: u64,
    /// **Chosen, not measured**: the base level is skipped whole when it is
    /// wider than `max_texture_dimension_2d`, and so is every level after it
    /// until one fits.
    pub(super) dropped_levels: u32,
}

/// The first of `levels` mip levels, counting from the base at `width` by
/// `height`, whose sides both fit in `limit`; `None` when none does.
///
/// **The one rule every `create_texture` here goes through.** A level `n` is
/// `max(width >> n, 1)` by `max(height >> n, 1)`, so a chain whose base is too
/// large for the device still holds a level that is not, and the picture goes
/// up at that level's size rather than panicking in wgpu's validation. **Chosen,
/// not measured**: nothing about how the original handled an oversize texture is
/// known, because no GPU the originals ran on had one.
///
/// `align` is the block size the chosen base must be a whole number of: 1 for an
/// uncompressed texture and 4 for BC, which WebGPU refuses to create at a base
/// that is not (the levels under it round up and need no such check).
pub(super) fn first_fitting_level(
    width: u32,
    height: u32,
    levels: usize,
    limit: u32,
    align: u32,
) -> Option<usize> {
    (0..levels).find(|&level| {
        let shift = u32::try_from(level).unwrap_or(u32::MAX).min(31);
        let (w, h) = ((width >> shift).max(1), (height >> shift).max(1));
        w <= limit && h <= limit && w % align == 0 && h % align == 0
    })
}

/// Uploads a chain the caller already holds, one `(width, height, texels)` per
/// level, base first, and returns its view.
///
/// The shared tail of [`upload_rgba`], which builds its own chain, and of the
/// disc-authored [`Texels::Chain`], which does not: the level count is the
/// iterator's, so both cap and complete chains go through the same writes.
fn upload_levels<'a>(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    label: &str,
    levels: impl ExactSizeIterator<Item = (u32, u32, &'a [u8])> + Clone,
) -> wgpu::TextureView {
    let (width, height, _) = levels.clone().next().expect("a chain has a base level");
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: levels.len() as u32,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    for (level, (mip_width, mip_height, mip_rgba)) in levels.enumerate() {
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
                rows_per_image: Some(mip_height),
            },
            wgpu::Extent3d {
                width: mip_width,
                height: mip_height,
                depth_or_array_layers: 1,
            },
        );
    }
    texture.create_view(&wgpu::TextureViewDescriptor::default())
}

/// [`upload`], once per distinct texture across an open
/// [`super::pipeline_cache::Scope`] - see
/// [`super::pipeline_cache::cached_texture_view`].
pub(super) fn upload_shared(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &std::sync::Arc<ModelTexture>,
    blocks: bool,
) -> Option<wgpu::TextureView> {
    super::pipeline_cache::cached_texture_view(texture, || {
        upload(device, queue, texture, blocks).map(|placed| placed.view)
    })
}

/// Uploads one [`ModelTexture`], in the form it came in.
///
/// A texture already [`Texels::Uploaded`] hands back the view it carries.
///
/// `blocks` is whether the device has `TEXTURE_COMPRESSION_BC`. Without it a
/// block-compressed texture is decoded back to RGBA8 here rather than left
/// undrawn - one memory win traded away on an adapter that cannot take it,
/// which is the GL backend and nothing this project ships by default.
///
/// **`None` is a texture nothing on the device can hold**: no level of its
/// chain fits `max_texture_dimension_2d`. The caller binds nothing for it, as
/// it does for any texture that did not decode, and the loader report names it
/// ([`log_census`]). A chain whose base is too large but a later level is not
/// goes up from that level - see [`first_fitting_level`].
pub(super) fn upload(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &ModelTexture,
    blocks: bool,
) -> Option<Placed> {
    let limit = device.limits().max_texture_dimension_2d;
    // Decoded under a `texture_sink` scope: the picture is already up, and this
    // is the one place that would otherwise put it there a second time.
    #[cfg(not(target_arch = "wasm32"))]
    if let Texels::Uploaded {
        view,
        gpu_bytes,
        dropped_levels,
        ..
    } = &texture.texels
    {
        return Some(Placed {
            view: view.clone(),
            gpu_bytes: *gpu_bytes,
            dropped_levels: *dropped_levels,
        });
    }
    let level_dims = |level: usize| {
        (
            (texture.width >> level.min(31)).max(1),
            (texture.height >> level.min(31)).max(1),
        )
    };
    if let Texels::Chain(levels) = &texture.texels {
        let skip = first_fitting_level(texture.width, texture.height, levels.len(), limit, 1)?;
        let kept = levels[skip..].iter().enumerate().map(|(index, texels)| {
            let (width, height) = level_dims(skip + index);
            (width, height, texels.as_slice())
        });
        return Some(Placed {
            view: upload_levels(device, queue, &texture.label, kept),
            gpu_bytes: levels[skip..].iter().map(|level| level.len() as u64).sum(),
            dropped_levels: skip as u32,
        });
    }
    let Texels::Blocks { format, levels } = &texture.texels else {
        let rgba = texture.rgba().expect("the other arm is Texels::Blocks");
        return upload_rgba(
            device,
            queue,
            texture.width,
            texture.height,
            rgba,
            &texture.label,
            texture.mip_count,
        );
    };
    if !blocks {
        // The first level that fits, not `levels.first()`: an oversize base
        // would otherwise be decoded to RGBA8 in full before anything dropped
        // it. `first()` rather than `[0]` still holds in spirit: `Texels::Blocks`
        // is a public variant, and an empty chain built elsewhere decodes to
        // nothing and binds a blank rather than panicking here. `skin::blocks`
        // never produces one.
        let skip =
            first_fitting_level(texture.width, texture.height, levels.len().max(1), limit, 1)?;
        let (width, height) = level_dims(skip);
        let decoded = levels
            .get(skip)
            .and_then(|level| format.decode_level(level, width, height));
        let rgba: Vec<u8> = decoded.into_iter().flatten().flatten().collect();
        // A chain that decoded to nothing binds a blank rather than sending
        // `mip_chain` off the end of an empty buffer.
        let full = u64::from(width) * u64::from(height) * 4;
        if rgba.len() as u64 != full {
            return upload_rgba(
                device,
                queue,
                1,
                1,
                &[255, 255, 255, 255],
                &texture.label,
                None,
            );
        }
        let mut placed = upload_rgba(
            device,
            queue,
            width,
            height,
            &rgba,
            &texture.label,
            texture
                .mip_count
                .map(|count| count.saturating_sub(skip as u32)),
        )?;
        placed.dropped_levels += skip as u32;
        return Some(placed);
    }
    let skip = first_fitting_level(texture.width, texture.height, levels.len(), limit, 4)?;
    let (base_width, base_height) = level_dims(skip);
    let gpu = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(&texture.label),
        size: wgpu::Extent3d {
            width: base_width,
            height: base_height,
            depth_or_array_layers: 1,
        },
        mip_level_count: (levels.len() - skip) as u32,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: format.wgpu(),
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    for (index, texels) in levels[skip..].iter().enumerate() {
        // **The block-aligned size, not the level's own.** A 2048x2048 chain
        // ends 2x2 and 1x1, and a compressed copy is validated in whole blocks
        // against the level's *physical* extent - which WebGPU defines as the
        // logical one rounded up to the block grid. Passing the logical size
        // there is "Copy height is not a multiple of block height", and the
        // last two levels of every mipped texture hit it.
        let width = (base_width >> index).max(1).div_ceil(4) * 4;
        let height = (base_height >> index).max(1).div_ceil(4) * 4;
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &gpu,
                mip_level: index as u32,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            texels,
            // Block rows, not texel rows: a compressed level is
            // `ceil(w/4) x ceil(h/4)` blocks of `block_len()` bytes, which is
            // what `oag_texture::gtf::Texture::level_len` sliced out.
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
    Some(Placed {
        view: gpu.create_view(&wgpu::TextureViewDescriptor::default()),
        gpu_bytes: levels[skip..].iter().map(|level| level.len() as u64).sum(),
        dropped_levels: skip as u32,
    })
}

/// What [`upload`] will do with a texture on a device whose
/// `max_texture_dimension_2d` is `limit`: how many leading mip levels it leaves
/// out and how many bytes the rest occupy, mip chain included. `None` where
/// [`upload`] returns `None`.
///
/// Arithmetic, not a readback: [`upload`] is the one place a texture reaches
/// the GPU and this mirrors its outcomes - the disc's own blocks and chain,
/// blocks decoded back to RGBA8 on an adapter without `TEXTURE_COMPRESSION_BC`,
/// and RGBA8 with the box-filtered chain, each from the first level that fits.
pub(super) fn plan(texture: &ModelTexture, blocks: bool, limit: u32) -> Option<(u32, u64)> {
    let sum = |levels: &[Vec<u8>]| levels.iter().map(|level| level.len() as u64).sum();
    let synthesised = |width: u32, height: u32, max: Option<u32>| {
        let mut levels = mip_chain_dims(width, height);
        if let Some(max) = max {
            levels.truncate(max as usize);
        }
        let skip = first_fitting_level(width, height, levels.len(), limit, 1)?;
        let bytes = levels[skip..]
            .iter()
            .map(|(w, h)| u64::from(*w) * u64::from(*h) * 4)
            .sum();
        Some((skip as u32, bytes))
    };
    match &texture.texels {
        #[cfg(not(target_arch = "wasm32"))]
        Texels::Uploaded {
            gpu_bytes,
            dropped_levels,
            ..
        } => Some((*dropped_levels, *gpu_bytes)),
        Texels::Chain(levels) => {
            let skip = first_fitting_level(texture.width, texture.height, levels.len(), limit, 1)?;
            Some((skip as u32, sum(&levels[skip..])))
        }
        Texels::Blocks { levels, .. } if blocks => {
            let skip = first_fitting_level(texture.width, texture.height, levels.len(), limit, 4)?;
            Some((skip as u32, sum(&levels[skip..])))
        }
        Texels::Blocks { levels, .. } => {
            let skip =
                first_fitting_level(texture.width, texture.height, levels.len().max(1), limit, 1)?;
            let (_, bytes) = synthesised(
                (texture.width >> skip.min(31)).max(1),
                (texture.height >> skip.min(31)).max(1),
                texture
                    .mip_count
                    .map(|count| count.saturating_sub(skip as u32)),
            )?;
            Some((skip as u32, bytes))
        }
        Texels::Rgba8(_) => synthesised(texture.width, texture.height, texture.mip_count),
    }
}

/// The size of every level [`mip_chain`] builds, without building it.
fn mip_chain_dims(width: u32, height: u32) -> Vec<(u32, u32)> {
    let mut levels = vec![(width, height)];
    while let Some(&(w, h)) = levels.last() {
        if w == 1 && h == 1 {
            break;
        }
        levels.push(((w / 2).max(1), (h / 2).max(1)));
    }
    levels
}

/// Logs what one model's textures cost, on the CPU and on the GPU.
///
/// **One line per model, in the loader report**, so a change to how textures
/// are held or uploaded shows in the same place the rest of a load is
/// reported. Each distinct texture is counted once however many material
/// slots share its `Arc`; `cpu` is what the [`Model`](crate::mesh::Model)
/// keeps alive after the upload and `gpu` what [`upload`] sends.
pub(super) fn log_census(model: &crate::mesh::Model, blocks: bool, limit: u32) {
    let mut seen = std::collections::HashSet::new();
    let mut counts = [0usize; 2];
    let mut cpu = [0u64; 2];
    let mut gpu = [0u64; 2];
    let mut compressed = 0usize;
    let mut trimmed: Vec<String> = Vec::new();
    let mut refused: Vec<&str> = Vec::new();
    for (role, slots) in [(0, &model.textures), (1, &model.lightmaps)] {
        for texture in slots.iter().flatten() {
            if !seen.insert(std::sync::Arc::as_ptr(texture) as usize) {
                continue;
            }
            counts[role] += 1;
            cpu[role] += texture.cpu_bytes();
            match plan(texture, blocks, limit) {
                Some((dropped, bytes)) => {
                    gpu[role] += bytes;
                    if dropped > 0 {
                        trimmed.push(format!("{} (from level {dropped})", texture.label));
                    }
                }
                None => refused.push(&texture.label),
            }
            compressed += usize::from(match &texture.texels {
                Texels::Blocks { .. } => blocks,
                #[cfg(not(target_arch = "wasm32"))]
                Texels::Uploaded {
                    block_compressed, ..
                } => *block_compressed,
                _ => false,
            });
        }
    }
    if counts == [0, 0] {
        return;
    }
    let mib = |bytes: u64| bytes as f64 / (1024.0 * 1024.0);
    log::debug!(
        "{}: texture memory - {} albedo + {} lightmap texture(s), {compressed} block-compressed; \
         CPU kept {:.1} + {:.1} MiB, GPU uploaded {:.1} + {:.1} MiB (albedo + lightmap); \
         geometry {:.1} MiB vertices + {:.1} MiB indices, CPU and GPU each",
        model.label,
        counts[0],
        counts[1],
        mib(cpu[0]),
        mib(cpu[1]),
        mib(gpu[0]),
        mib(gpu[1]),
        mib((model.vertices.len() * std::mem::size_of::<crate::mesh::GpuVertex>()) as u64),
        mib((model.indices.len() * 4) as u64),
    );
    if !trimmed.is_empty() {
        log::warn!(
            "{}: {} texture(s) wider than this device's max_texture_dimension_2d of {limit} \
             uploaded from the first mip level that fits - chosen, not measured: {}",
            model.label,
            trimmed.len(),
            trimmed.join(", "),
        );
    }
    if !refused.is_empty() {
        log::warn!(
            "{}: {} texture(s) have no mip level within this device's max_texture_dimension_2d \
             of {limit} and are not drawn - chosen, not measured: {}",
            model.label,
            refused.len(),
            refused.join(", "),
        );
    }
}

#[cfg(test)]
mod tests;
