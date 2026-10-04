//! [`FlareTexture`]: the pixels the exhaust pipeline draws with.
//!
//! Split out of `exhaust.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py` when the ribbon grew a second texture; the
//! struct and its `placeholder` are a move, and only [`FlareTexture::bind`]
//! changed.

/// An RGBA8 image for the flare, with its dimensions.
///
/// A plain struct rather than a borrow of `oag_formats`' type, so this crate
/// keeps depending on nothing but `oag-core` and `wgpu` for the exhaust: the
/// caller decodes the `.mip` and hands the pixels over.
#[derive(Debug, Clone, PartialEq)]
pub struct FlareTexture {
    /// Width in texels.
    pub width: u32,
    /// Height in texels.
    pub height: u32,
    /// `width * height * 4` bytes, RGBA8.
    pub rgba: Vec<u8>,
}

impl FlareTexture {
    /// A soft radial glow, for when the disc's own texture is unavailable.
    ///
    /// Used by tests and by the headless capture path. **Not** a silent
    /// substitute in the game: the caller reports a missing archive entry rather
    /// than quietly drawing this, because a stand-in that looks plausible is how
    /// a decode failure survives review.
    #[must_use]
    pub fn placeholder(size: u32) -> Self {
        let mut rgba = Vec::with_capacity((size * size * 4) as usize);
        let centre = (size as f32 - 1.0) / 2.0;
        for y in 0..size {
            for x in 0..size {
                let dx = (x as f32 - centre) / centre;
                let dy = (y as f32 - centre) / centre;
                // Squared falloff, so the edge reaches zero rather than clipping.
                let d = (dx * dx + dy * dy).sqrt().min(1.0);
                let a = (1.0 - d) * (1.0 - d);
                rgba.extend_from_slice(&[255, 255, 255, (a * 255.0) as u8]);
            }
        }
        Self {
            width: size,
            height: size,
            rgba,
        }
    }

    /// Uploads this image, and a second beside it, as one bind group.
    ///
    /// **Two textures because Wipeout HD's ribbon needs two**, and its material
    /// says which is which by name: `hd_enginetrail_red_alphaisnoise.gtf` in
    /// slot 0, whose alpha the program uses to displace the sample, and
    /// `hd_enginetrail_blue_alphaistrail.gtf` in slot 2, whose alpha *is* the
    /// ribbon's coverage. See
    /// `docs/ghidra/functions/ps3-hdfury-eu/engine-flare.md`, "the ribbon's own
    /// program".
    ///
    /// `shape` is `None` for every title that names one texture, and then slot
    /// 2 is bound to this same image so the layout stays satisfied - the shader
    /// never reads it there, because `trail_shape` is only switched on where a
    /// second was supplied.
    pub(super) fn bind(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layout: &wgpu::BindGroupLayout,
        label: &str,
        wrap_v: bool,
        shape: Option<&Self>,
    ) -> wgpu::BindGroup {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width: self.width.max(1),
                height: self.height.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &self.rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(self.width.max(1) * 4),
                rows_per_image: Some(self.height.max(1)),
            },
            wgpu::Extent3d {
                width: self.width.max(1),
                height: self.height.max(1),
                depth_or_array_layers: 1,
            },
        );
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        // The ribbon's state list sets `sceGuTexWrap(GU_REPEAT, GU_REPEAT)`, and
        // it needs both: `u` tiles `LAYER_TEX_SCALE_U` times along the trail
        // plus a scroll offset, and `v` runs once around the diamond tube plus
        // its own scroll, so either coordinate routinely leaves `[0, 1]`. The
        // flare quad samples exactly 0..1 and keeps `v` clamped so its soft
        // edge cannot bleed the opposite row in under bilinear filtering.
        let v_mode = if wrap_v {
            wgpu::AddressMode::Repeat
        } else {
            wgpu::AddressMode::ClampToEdge
        };
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some(label),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: v_mode,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let shape_texture = shape.map(|shape| shape.upload(device, queue, label));
        let shape_view = shape_texture
            .as_ref()
            .map(|texture| texture.create_view(&wgpu::TextureViewDescriptor::default()));
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(label),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(
                        shape_view.as_ref().unwrap_or(&view),
                    ),
                },
            ],
        })
    }

    /// This image as a texture on the device, with its pixels written.
    ///
    /// Split out of [`Self::bind`] when the ribbon needed a second one; the
    /// body is that function's own upload, unchanged.
    fn upload(&self, device: &wgpu::Device, queue: &wgpu::Queue, label: &str) -> wgpu::Texture {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width: self.width.max(1),
                height: self.height.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &self.rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(self.width.max(1) * 4),
                rows_per_image: Some(self.height.max(1)),
            },
            wgpu::Extent3d {
                width: self.width.max(1),
                height: self.height.max(1),
                depth_or_array_layers: 1,
            },
        );
        texture
    }
}
