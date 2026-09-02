//! The original's bloom: a bright pass masked by destination alpha, a
//! separable 11-tap blur, and an additive composite.
//!
//! Recovered end to end from `BOOT.BIN` - see
//! [`bloom.md`](../../../../docs/ghidra/functions/psp-pulse-usa/bloom.md) for
//! the decode, the addresses and the evidence. **Every constant here is read
//! out of the executable; none is fitted to a screenshot**, which is why they
//! are `pub` and named after the globals they come from.
//!
//! # It is not exhaust-specific
//!
//! Nothing in this module knows what drew the frame. It blooms **whatever is
//! in the scene target's alpha channel**, so every surface that opts into the
//! glow mask is handled by the same three passes at the same cost. That is the
//! original's own design: `Mesh_SetBatchDrawState` protects alpha for ordinary
//! geometry and only three code paths open it, so the mask is a shared
//! resource rather than one effect's private buffer.
//!
//! The writers this project reproduces are listed on `bloom.md`; the exhaust
//! is merely the first.
//!
//! # Why the buffers are a fixed 240 x 136
//!
//! The original's scratch buffers are exactly half of the PSP's 480 x 272, and
//! its blur taps are **+/-5 texels of that buffer** - about 2 % of the screen
//! width. Sizing our buffers to half of *our* scene instead would keep the tap
//! count and lose the radius: at a 1920-wide render, five texels of a 960-wide
//! buffer is a quarter of the spread the original has, and the bloom would
//! tighten as resolution rose.
//!
//! Fixing the buffers at the original's own size keeps every recovered number
//! literal - the kernel, the tap offsets and the 2x upscale are all exactly
//! what the GE does - and makes the effect resolution-independent. A bloom is
//! low-frequency by construction, so the upscale to a larger scene costs
//! nothing visible; `bloom.md` records that the original leans on that upscale
//! as part of the spread.

use anyhow::Result;

/// A scratch colour buffer: sampled by the next pass, drawn into by this one.
#[derive(Debug)]
struct Target {
    #[expect(dead_code, reason = "held so the view stays valid")]
    texture: wgpu::Texture,
    view: wgpu::TextureView,
}

impl Target {
    fn new(
        device: &wgpu::Device,
        label: &str,
        format: wgpu::TextureFormat,
        (width, height): (u32, u32),
    ) -> Result<Self> {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        Ok(Self { texture, view })
    }
}

/// Width of the original's bloom scratch buffers, `g_bloom_buffer_width`
/// (`DAT_08a84cf0`), exactly half of the PSP's 480.
pub const BLOOM_WIDTH: u32 = 240;

/// Height of the same, `g_bloom_buffer_height` (`DAT_08a84cf4`), half of 272.
pub const BLOOM_HEIGHT: u32 = 136;

/// The blur kernel, `g_bloom_blur_weights` (`DAT_08ab2348`), eleven bytes
/// verbatim.
///
/// **Deliberately not normalised.** They sum to `472`, i.e. `472/255 = 1.85`
/// per axis and `3.43x` over both, and that gain is the original's - it is a
/// large part of why its glow is as strong as the exhaust measurement on
/// `exhaust.md` found. Dividing it out would be a different effect.
pub const BLUR_WEIGHTS: [u8; 11] = [20, 30, 40, 50, 64, 64, 64, 50, 40, 30, 20];

/// How far the outermost tap reaches, in texels of the bloom buffer. The
/// original's offsets run `i - 5` for `i` in `0..11`.
pub const BLUR_RADIUS: i32 = 5;

/// `g_bloom_composite_strength` (`DAT_08ab2344`), the byte `0xaf` the
/// composite's `GU_FIX` source factor carries on every channel.
pub const COMPOSITE_STRENGTH: u8 = 0xaf;

/// The uniform block `bloom.wgsl` reads. `repr(C)` and 16-byte aligned: the
/// two `vec2`s pack into one row, the strength starts another, and the drawn
/// sub-rectangle fills a third.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
struct Constants {
    texel: [f32; 2],
    direction: [f32; 2],
    strength: f32,
    _pad: [f32; 3],
    /// Only the bright pass reads these, and only it is handed a rectangle
    /// that can be short: it samples the *scene* target, which since ADR-0037
    /// may be larger than what was drawn into it. The blurs and the composite
    /// read the fixed scratch buffers, which are always whole.
    uv_scale: [f32; 2],
    uv_max: [f32; 2],
}

/// One frame's worth of input to [`Bloom::render`].
#[derive(Debug, Clone, Copy)]
pub struct Frame<'a> {
    /// The scene target: sampled by the bright pass, added back into by the
    /// composite. Legal because the two happen in different passes.
    pub scene: &'a wgpu::TextureView,
    /// `scene`'s real dimensions - the allocation.
    pub size: (u32, u32),
    /// The rectangle of `scene` that was drawn, `<= size` on both axes. The
    /// bright pass reads exactly it and the composite writes exactly it.
    pub viewport: (u32, u32),
}

/// The three passes and their two ping-pong buffers.
#[derive(Debug)]
pub struct Bloom {
    bright: wgpu::RenderPipeline,
    blur: wgpu::RenderPipeline,
    composite: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    a: Target,
    b: Target,
    /// Bind groups whose source never changes: blur-x reads A, blur-y reads B,
    /// composite reads A. Only the bright pass's source is a caller's view, so
    /// only that one is built per frame.
    blur_x: wgpu::BindGroup,
    blur_y: wgpu::BindGroup,
    composite_group: wgpu::BindGroup,
    constants_bright: wgpu::Buffer,
    /// The composite's own copy, fixed at the whole rectangle.
    ///
    /// It shared `constants_bright` until the drawn sub-rectangle joined the
    /// block, and cannot any more: the two passes read different textures -
    /// the bright pass the scene, the composite the fixed scratch buffer - so
    /// a scale that is right for one is wrong for the other.
    #[expect(dead_code, reason = "held so composite_group's binding stays valid")]
    constants_composite: wgpu::Buffer,
    /// What was last written into `constants_bright`, so a frame that moved
    /// nothing writes nothing.
    written: std::cell::Cell<Option<Constants>>,
}

impl Bloom {
    /// Builds the pipelines and both scratch buffers.
    ///
    /// `format` is the scene target's, so the composite blends into the same
    /// surface the scene was drawn on.
    ///
    /// # Errors
    ///
    /// Propagates a scratch buffer that will not allocate.
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Result<Self> {
        let format = format.remove_srgb_suffix();
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("bloom"),
            source: wgpu::ShaderSource::Wgsl(include_str!("bloom.wgsl").into()),
        });

        let layout = super::fullscreen_layout(device, "bloom");

        // Bilinear and clamped. The taps land on texel centres of the bloom
        // buffer, and the composite's 2x stretch is the one place filtering
        // does real work - the original gets the same from the GE's own
        // `GU_LINEAR` magnification, and clamping matches its
        // `Gu_TexWrap(CLAMP, CLAMP)` on the bright pass.
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("bloom"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("bloom"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });

        let pipeline = |label: &str, entry: &str, blend: Option<wgpu::BlendState>, mask| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some(entry),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend,
                        write_mask: mask,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
                cache: None,
            })
        };

        // The bright pass and the blur both replace their whole target: the
        // original clears the scratch buffer and then accumulates, which for a
        // shader that sums its own taps is the same as writing the sum.
        let bright = pipeline("bloom bright", "fs_bright", None, wgpu::ColorWrites::ALL);
        let blur = pipeline("bloom blur", "fs_blur", None, wgpu::ColorWrites::ALL);
        // `framebuffer += strength * bloom`, and **alpha is left alone** - the
        // original wraps this draw in `Gu_PixelMask(0xff000000)` precisely so
        // the composite cannot disturb the glow mask it just consumed.
        let composite = pipeline(
            "bloom composite",
            "fs_composite",
            Some(wgpu::BlendState {
                color: wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::One,
                    dst_factor: wgpu::BlendFactor::One,
                    operation: wgpu::BlendOperation::Add,
                },
                alpha: wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::Zero,
                    dst_factor: wgpu::BlendFactor::One,
                    operation: wgpu::BlendOperation::Add,
                },
            }),
            wgpu::ColorWrites::COLOR,
        );

        let a = Target::new(device, "bloom a", format, (BLOOM_WIDTH, BLOOM_HEIGHT))?;
        let b = Target::new(device, "bloom b", format, (BLOOM_WIDTH, BLOOM_HEIGHT))?;

        let texel = [1.0 / BLOOM_WIDTH as f32, 1.0 / BLOOM_HEIGHT as f32];
        let strength = f32::from(COMPOSITE_STRENGTH) / 255.0;
        // Mapped at creation rather than written through a queue: every value
        // here is a compile-time constant, so the buffer never needs updating
        // and `new` never needs a `Queue`.
        let buffer = |label: &str, c: Constants, usage: wgpu::BufferUsages| {
            let bytes = bytemuck::bytes_of(&c);
            let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: bytes.len() as u64,
                usage,
                mapped_at_creation: true,
            });
            buffer
                .slice(..)
                .get_mapped_range_mut()
                .expect("a freshly mapped buffer maps")
                .copy_from_slice(bytes);
            buffer.unmap();
            buffer
        };
        let whole = Constants {
            texel,
            direction: [0.0, 0.0],
            strength,
            _pad: [0.0; 3],
            uv_scale: [1.0, 1.0],
            uv_max: [1.0, 1.0],
        };
        // The one buffer here that is not a compile-time constant: the bright
        // pass's rectangle moves whenever a resolution controller moves the
        // render extent, so this alone is written through the queue.
        let constants_bright = buffer(
            "bloom bright constants",
            whole,
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        );
        let constants_composite = buffer(
            "bloom composite constants",
            whole,
            wgpu::BufferUsages::UNIFORM,
        );
        let constants_x = buffer(
            "bloom blur-x constants",
            Constants {
                direction: [1.0, 0.0],
                ..whole
            },
            wgpu::BufferUsages::UNIFORM,
        );
        let constants_y = buffer(
            "bloom blur-y constants",
            Constants {
                direction: [0.0, 1.0],
                ..whole
            },
            wgpu::BufferUsages::UNIFORM,
        );

        let group = |label: &str, view: &wgpu::TextureView, constants: &wgpu::Buffer| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(label),
                layout: &layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: constants.as_entire_binding(),
                    },
                ],
            })
        };
        let blur_x = group("bloom blur-x", &a.view, &constants_x);
        let blur_y = group("bloom blur-y", &b.view, &constants_y);
        let composite_group = group("bloom composite", &a.view, &constants_composite);

        Ok(Self {
            bright,
            blur,
            composite,
            layout,
            sampler,
            a,
            b,
            blur_x,
            blur_y,
            composite_group,
            constants_bright,
            constants_composite,
            written: std::cell::Cell::new(Some(whole)),
        })
    }

    /// Runs all four steps against the scene, reading its alpha as the glow
    /// mask and adding the result back onto it.
    ///
    /// The scene view is both sampled and written, which is legal because the
    /// two happen in different passes: the bright pass reads it, the composite
    /// writes it, and the blurs touch only the scratch buffers in between.
    ///
    /// **The scratch buffers stay a fixed 240x136 whatever the frame is drawn
    /// at** - see the module docs for why - so a short render extent changes
    /// only where the bright pass reads and where the composite writes. The
    /// blur radius therefore stays five texels of 240, about 2 % of the drawn
    /// frame, at every render scale.
    pub fn render(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        frame: Frame<'_>,
    ) {
        let Frame {
            scene,
            size,
            viewport,
        } = frame;

        let (uv_scale, uv_max) = super::sub_rectangle(viewport, size);
        let wanted = Constants {
            texel: [1.0 / BLOOM_WIDTH as f32, 1.0 / BLOOM_HEIGHT as f32],
            direction: [0.0, 0.0],
            strength: f32::from(COMPOSITE_STRENGTH) / 255.0,
            _pad: [0.0; 3],
            uv_scale,
            uv_max,
        };
        if self.written.get() != Some(wanted) {
            queue.write_buffer(&self.constants_bright, 0, bytemuck::bytes_of(&wanted));
            self.written.set(Some(wanted));
        }

        let bright_group = crate::perfprobe::bind_group(
            device,
            &wgpu::BindGroupDescriptor {
                label: Some("bloom bright"),
                layout: &self.layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(scene),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: self.constants_bright.as_entire_binding(),
                    },
                ],
            },
        );

        let mut pass = |label: &str,
                        pipeline: &wgpu::RenderPipeline,
                        group: &wgpu::BindGroup,
                        target: &wgpu::TextureView,
                        load: wgpu::LoadOp<wgpu::Color>,
                        rect: (u32, u32)| {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some(label),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(pipeline);
            pass.set_viewport(
                0.0,
                0.0,
                rect.0.max(1) as f32,
                rect.1.max(1) as f32,
                0.0,
                1.0,
            );
            pass.set_bind_group(0, group, &[]);
            pass.draw(0..3, 0..1);
        };

        let discard = wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT);
        // The three scratch passes fill their whole buffer, which is the fixed
        // 240x136 whatever the scene is drawn at.
        pass(
            "bloom bright",
            &self.bright,
            &bright_group,
            &self.a.view,
            discard,
            (BLOOM_WIDTH, BLOOM_HEIGHT),
        );
        pass(
            "bloom blur x",
            &self.blur,
            &self.blur_x,
            &self.b.view,
            discard,
            (BLOOM_WIDTH, BLOOM_HEIGHT),
        );
        pass(
            "bloom blur y",
            &self.blur,
            &self.blur_y,
            &self.a.view,
            discard,
            (BLOOM_WIDTH, BLOOM_HEIGHT),
        );
        // The only pass that keeps what is already there - it is adding to it -
        // and the only one that writes at the scene's resolution, so the only
        // one whose rectangle a controller moves.
        pass(
            "bloom composite",
            &self.composite,
            &self.composite_group,
            scene,
            wgpu::LoadOp::Load,
            viewport,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The kernel is the eleven bytes at `DAT_08ab2348`, in order, and it is
    /// symmetric. A future edit that "tidies" it into a Gaussian would change
    /// a recovered value into a fitted one.
    #[test]
    fn the_kernel_is_the_recovered_bytes() {
        assert_eq!(BLUR_WEIGHTS, [20, 30, 40, 50, 64, 64, 64, 50, 40, 30, 20]);
        let reversed: Vec<u8> = BLUR_WEIGHTS.iter().copied().rev().collect();
        assert_eq!(reversed, BLUR_WEIGHTS, "the kernel is symmetric");
        assert_eq!(
            BLUR_WEIGHTS.len(),
            (BLUR_RADIUS * 2 + 1) as usize,
            "eleven taps span -5..+5"
        );
    }

    /// **The gain is the point.** `bloom.md` records `472/255 = 1.85` per axis
    /// and `3.43x` over both; normalising the kernel would silently remove it.
    #[test]
    fn the_kernel_deliberately_brightens() {
        let sum: u32 = BLUR_WEIGHTS.iter().map(|&w| u32::from(w)).sum();
        assert_eq!(sum, 472);
        let gain = sum as f32 / 255.0;
        assert!(
            (gain - 1.851).abs() < 0.001,
            "per-axis gain {gain}, expected the recovered 1.85"
        );
        assert!(
            (gain * gain - 3.427).abs() < 0.01,
            "two-axis gain {}, expected the recovered 3.43",
            gain * gain
        );
    }

    /// `0xaf / 255`, the composite's `GU_FIX` source factor.
    #[test]
    fn the_composite_strength_is_the_recovered_byte() {
        assert_eq!(COMPOSITE_STRENGTH, 0xaf);
        let strength = f32::from(COMPOSITE_STRENGTH) / 255.0;
        assert!(
            (strength - 0.6863).abs() < 0.0001,
            "strength {strength}, expected the recovered 0.686"
        );
    }

    /// Half of the PSP's own 480 x 272, and fixed rather than derived from the
    /// scene size - see this module's docs for why.
    #[test]
    fn the_buffers_are_half_the_psp_framebuffer() {
        assert_eq!((BLOOM_WIDTH * 2, BLOOM_HEIGHT * 2), (480, 272));
    }
}
