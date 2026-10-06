//! Pulse PS2's own bloom: a downsample masked by the frame's alpha, a 7-tap
//! separable blur, and a half-strength additive composite.
//!
//! **Not the PSP's chain with other numbers** - [`super::bloom`] is that, and it
//! is left exactly as it was. Read off GS dumps of the PS2 original running on
//! PCSX2 (`docs/rendering/ps2-bloom.md`, confidence 90), the passes differ in
//! every constant and in three ways of doing the arithmetic:
//!
//! | | PSP ([`super::bloom`]) | PS2 (this) |
//! | --- | --- | --- |
//! | scratch buffers | 240 x 136 | 320 x 224 |
//! | blur taps | 11, weights over 255, edge clamped | 7, weights over 128 (`16 32 32 64 32 32 16`), **zero past the edge** |
//! | composite | 175/255, drawn at 1:1 of the bright pass's grid | `127/128 * 64/128`, drawn five pixels up and left |
//! | mask | the stencil: a constant per batch, `4` elsewhere | the alpha channel: a glow batch's own alpha, `0` elsewhere |
//!
//! The mask is the other half of the port: `mesh_render::GlowMask::StampedByTexel`.
//!
//! # What is measured and what is chosen
//!
//! Measured, every one a register or vertex colour in a dump: the pass order,
//! the 320 x 224 size, the weights, `ALPHA 0x88` (`Cs * As`), `ALPHA 0x68` with
//! `FIX 0x80` (`Cs + Cd`) for the taps, `ALPHA 0x68` with `FIX 0x40` and a
//! vertex colour of 127 for the composite, the `-5, -5` destination offset,
//! bilinear on the downsample and the composite, and 1:1 point reads for the
//! taps.
//!
//! **Chosen, not measured:** the original draws into a 512 x 512 frame and
//! reads all of it, so the buffer maps the *whole viewport*, whatever its
//! aspect or size - which is what this does, the way [`super::bloom`] maps its
//! own - and the five-pixel offset is `5/512` of the viewport on each axis. At a
//! viewport that is not 512 x 512 that is a scale of the original's offset, not
//! a measurement of it.

use anyhow::Result;

use super::bloom::{Frame, Target};

/// Width of the original's bloom buffer, read off the downsample's quad
/// (`x` 1792..2112 in 1/16 pixels of the GS's primitive space).
pub const BLOOM_WIDTH: u32 = 320;

/// Height of the same (`y` 1792..2016).
pub const BLOOM_HEIGHT: u32 = 224;

/// The tap weights over 128, the vertex colour of each of the seven strips.
pub const BLUR_WEIGHTS: [u8; 7] = [16, 32, 32, 64, 32, 32, 16];

/// The composite quad's destination offset from the frame's corner, in pixels
/// of the 512 x 512 frame the original draws into.
pub const COMPOSITE_OFFSET: f32 = 5.0;

/// The frame the offset is measured in, per axis.
pub const FRAME_SIZE: f32 = 512.0;

/// `FIX` of the composite's `ALPHA 0x4000000068`, over 128.
pub const COMPOSITE_FIX: u8 = 0x40;

/// The composite quad's vertex colour, which `MODULATE` multiplies the texel by.
pub const COMPOSITE_COLOUR: u8 = 127;

/// The uniform block `ps2_bloom.wesl` reads: 32 bytes, three rows of `vec2`.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
struct Constants {
    uv_scale: [f32; 2],
    uv_max: [f32; 2],
    direction: [f32; 2],
    shift: [f32; 2],
}

/// The three passes and their two buffers.
#[derive(Debug)]
pub struct Ps2Bloom {
    down: wgpu::RenderPipeline,
    blur: wgpu::RenderPipeline,
    composite: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    a: Target,
    b: Target,
    blur_x: wgpu::BindGroup,
    blur_y: wgpu::BindGroup,
    composite_group: wgpu::BindGroup,
    constants_down: wgpu::Buffer,
    #[expect(dead_code, reason = "held so the groups' bindings stay valid")]
    fixed: [wgpu::Buffer; 3],
    written: std::cell::Cell<Option<Constants>>,
}

impl Ps2Bloom {
    /// Builds the pipelines and both buffers. `format` is the scene target's.
    ///
    /// # Errors
    ///
    /// Propagates a buffer that will not allocate.
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Result<Self> {
        let format = format.remove_srgb_suffix();
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ps2 bloom"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!(concat!(env!("OUT_DIR"), "/ps2_bloom.wgsl")).into(),
            ),
        });
        let layout = super::fullscreen_layout(device, "ps2 bloom");
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("ps2 bloom"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("ps2 bloom"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = |label: &str,
                        entry: &str,
                        blend: Option<wgpu::BlendState>,
                        mask,
                        constants: &[(&str, f64)]| {
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
                    compilation_options: wgpu::PipelineCompilationOptions {
                        constants,
                        ..Default::default()
                    },
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
                cache: None,
            })
        };
        let down = pipeline(
            "ps2 bloom down",
            "fs_down",
            None,
            wgpu::ColorWrites::ALL,
            &[],
        );
        let blur = pipeline(
            "ps2 bloom blur",
            "fs_blur",
            None,
            wgpu::ColorWrites::ALL,
            &[],
        );
        // `framebuffer += texel`, alpha left alone: `FBMSK 0xff000000`.
        let composite = pipeline(
            "ps2 bloom composite",
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
            &[
                ("composite_colour", f64::from(COMPOSITE_COLOUR)),
                ("composite_fix", f64::from(COMPOSITE_FIX)),
            ],
        );

        let a = Target::new(device, "ps2 bloom a", format, (BLOOM_WIDTH, BLOOM_HEIGHT))?;
        let b = Target::new(device, "ps2 bloom b", format, (BLOOM_WIDTH, BLOOM_HEIGHT))?;

        let whole = Constants {
            uv_scale: [1.0, 1.0],
            uv_max: [1.0, 1.0],
            direction: [0.0, 0.0],
            shift: [COMPOSITE_OFFSET / FRAME_SIZE; 2],
        };
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
        let constants_down = buffer(
            "ps2 bloom down constants",
            whole,
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        );
        let constants_x = buffer(
            "ps2 bloom blur-x constants",
            Constants {
                direction: [1.0, 0.0],
                ..whole
            },
            wgpu::BufferUsages::UNIFORM,
        );
        let constants_y = buffer(
            "ps2 bloom blur-y constants",
            Constants {
                direction: [0.0, 1.0],
                ..whole
            },
            wgpu::BufferUsages::UNIFORM,
        );
        let constants_composite = buffer(
            "ps2 bloom composite constants",
            whole,
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
        // Down writes A, the horizontal blur reads A and writes B, the
        // vertical reads B and writes A, the composite reads A.
        let blur_x = group("ps2 bloom blur-x", &a.view, &constants_x);
        let blur_y = group("ps2 bloom blur-y", &b.view, &constants_y);
        let composite_group = group("ps2 bloom composite", &a.view, &constants_composite);

        Ok(Self {
            down,
            blur,
            composite,
            layout,
            sampler,
            a,
            b,
            blur_x,
            blur_y,
            composite_group,
            constants_down,
            fixed: [constants_x, constants_y, constants_composite],
            written: std::cell::Cell::new(Some(whole)),
        })
    }

    /// Runs the three passes over the scene, reading its alpha as the mask and
    /// adding the result back onto it: [`Ps2Bloom::prepare`] then
    /// [`Ps2Bloom::composite`] into the same view. `frame` is
    /// [`super::bloom::Frame`]'s, with the same meaning: the scene view is
    /// sampled by the first pass and written by the last, which is legal
    /// because they are separate passes.
    pub fn render(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        frame: Frame<'_>,
    ) {
        self.prepare(device, queue, encoder, frame);
        self.composite(encoder, frame.scene, frame.origin, frame.viewport);
    }

    /// The downsample and the two blurs: reads the scene and its glow mask,
    /// leaves the blurred glow in buffer A for [`Ps2Bloom::composite`].
    ///
    /// Split from the composite so a caller can draw its HUD between the two,
    /// which is the order Pulse PSP's own queue draws in
    /// ([`super::bloom::Bloom::prepare`]). **Inherited, not measured on the
    /// PS2**: whether the PS2's HUD groups take the composite is read from
    /// registers only (`docs/rendering/ps2-bloom.md`, "Still needed").
    pub fn prepare(
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
            ..
        } = frame;
        let (uv_scale, uv_max) = super::sub_rectangle(viewport, size);
        let wanted = Constants {
            uv_scale,
            uv_max,
            direction: [0.0, 0.0],
            shift: [COMPOSITE_OFFSET / FRAME_SIZE; 2],
        };
        if self.written.get() != Some(wanted) {
            queue.write_buffer(&self.constants_down, 0, bytemuck::bytes_of(&wanted));
            self.written.set(Some(wanted));
        }
        let down_group = oag_gpu::perfprobe::bind_group(
            device,
            &wgpu::BindGroupDescriptor {
                label: Some("ps2 bloom down"),
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
                        resource: self.constants_down.as_entire_binding(),
                    },
                ],
            },
        );
        let mut pass = |label: &str,
                        pipeline: &wgpu::RenderPipeline,
                        group: &wgpu::BindGroup,
                        target: &wgpu::TextureView,
                        load: wgpu::LoadOp<wgpu::Color>,
                        origin: (f32, f32),
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
                origin.0,
                origin.1,
                rect.0.max(1) as f32,
                rect.1.max(1) as f32,
                0.0,
                1.0,
            );
            pass.set_bind_group(0, group, &[]);
            pass.draw(0..3, 0..1);
        };
        let clear = wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT);
        let buffer = (BLOOM_WIDTH, BLOOM_HEIGHT);
        pass(
            "ps2 bloom down",
            &self.down,
            &down_group,
            &self.a.view,
            clear,
            (0.0, 0.0),
            buffer,
        );
        pass(
            "ps2 bloom blur x",
            &self.blur,
            &self.blur_x,
            &self.b.view,
            clear,
            (0.0, 0.0),
            buffer,
        );
        pass(
            "ps2 bloom blur y",
            &self.blur,
            &self.blur_y,
            &self.a.view,
            clear,
            (0.0, 0.0),
            buffer,
        );
    }

    /// Adds the glow [`Ps2Bloom::prepare`] left in buffer A onto `target`,
    /// stretched over the rectangle at `origin` of size `viewport`. `target`
    /// need not be the scene: it is wherever the HUD went, at whatever size,
    /// since the stretch is from the fixed bloom buffer.
    pub fn composite(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        origin: (f32, f32),
        viewport: (u32, u32),
    ) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("ps2 bloom composite"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&self.composite);
        pass.set_viewport(
            origin.0,
            origin.1,
            viewport.0.max(1) as f32,
            viewport.1.max(1) as f32,
            0.0,
            1.0,
        );
        pass.set_bind_group(0, &self.composite_group, &[]);
        pass.draw(0..3, 0..1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The shader spells the weights as a literal array, which a Rust constant
    /// cannot reach; this is what fails when the two drift apart.
    #[test]
    fn the_shaders_weights_are_the_constants() {
        let wgsl = include_str!(concat!(env!("OUT_DIR"), "/ps2_bloom.wgsl"));
        let spelled = BLUR_WEIGHTS
            .iter()
            .map(|w| format!("{w}u"))
            .collect::<Vec<_>>()
            .join(", ");
        assert!(
            wgsl.contains(&format!("array<u32, 7>({spelled})")),
            "ps2_bloom.wesl's WEIGHTS is not {BLUR_WEIGHTS:?}"
        );
        assert_eq!(BLUR_WEIGHTS.len(), 7);
        assert_eq!((BLOOM_WIDTH, BLOOM_HEIGHT), (320, 224));
    }

    /// The weights sum to 224/128, the 1.75 per axis the dump's vertex colours
    /// give - and the same in both axes, which is why one pass serves both.
    #[test]
    fn the_taps_gain_one_and_three_quarters_per_axis() {
        let sum: u32 = BLUR_WEIGHTS.iter().map(|&w| u32::from(w)).sum();
        assert_eq!(sum, 224);
    }
}
