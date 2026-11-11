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

use super::bloom::{Frame, Target, ensure_view_lane, lane_group, scene_group, scratch_pass};

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

/// The uniform block `ps2_bloom.wesl` reads: 40 bytes, five `vec2`s.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
struct Constants {
    uv_scale: [f32; 2],
    uv_max: [f32; 2],
    direction: [f32; 2],
    shift: [f32; 2],
    /// Where the drawn rectangle's corner sits in the scene target, as a UV
    /// offset; `(0, 0)` for a rectangle that starts at the target's own corner.
    /// Only `fs_down` reads it.
    uv_offset: [f32; 2],
}

/// One set of buffers and the bind groups over them: a race has one, at the
/// original's size, and a split screen one more per view, so every view can be
/// blurred and added back before the next view's downsample overwrites the
/// buffer ([`Ps2Bloom::prepare_view`]).
#[derive(Debug)]
struct Lane {
    /// The buffers' own size: the original's for a whole picture, a view's share
    /// of it under a split (`super::bloom::lane_extent`).
    extent: (u32, u32),
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

/// The three passes and their lanes of two buffers.
#[derive(Debug)]
pub struct Ps2Bloom {
    down: wgpu::RenderPipeline,
    blur: wgpu::RenderPipeline,
    composite: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    format: wgpu::TextureFormat,
    /// The whole picture's.
    base: Lane,
    /// A split screen's, one per view.
    views: std::cell::RefCell<Vec<Lane>>,
}

impl Lane {
    fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        layout: &wgpu::BindGroupLayout,
        sampler: &wgpu::Sampler,
        extent: (u32, u32),
    ) -> Result<Self> {
        let a = Target::new(device, "ps2 bloom a", format, extent)?;
        let b = Target::new(device, "ps2 bloom b", format, extent)?;

        let whole = Constants {
            uv_scale: [1.0, 1.0],
            uv_max: [1.0, 1.0],
            direction: [0.0, 0.0],
            shift: [COMPOSITE_OFFSET / FRAME_SIZE; 2],
            uv_offset: [0.0, 0.0],
        };
        let buffer = |label: &str, c: Constants, usage: wgpu::BufferUsages| {
            oag_gpu::init_buffer::init(device, label, usage, bytemuck::bytes_of(&c))
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
            lane_group(device, layout, sampler, label, view, constants)
        };
        // Down writes A, the horizontal blur reads A and writes B, the
        // vertical reads B and writes A, the composite reads A.
        let blur_x = group("ps2 bloom blur-x", &a.view, &constants_x);
        let blur_y = group("ps2 bloom blur-y", &b.view, &constants_y);
        let composite_group = group("ps2 bloom composite", &a.view, &constants_composite);

        Ok(Self {
            extent,
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

        let base = Lane::new(
            device,
            format,
            &layout,
            &sampler,
            (BLOOM_WIDTH, BLOOM_HEIGHT),
        )?;
        Ok(Self {
            down,
            blur,
            composite,
            layout,
            sampler,
            format,
            base,
            views: std::cell::RefCell::new(Vec::new()),
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
        queue: &dyn oag_gpu::writes::Writes,
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
        queue: &dyn oag_gpu::writes::Writes,
        encoder: &mut wgpu::CommandEncoder,
        frame: Frame<'_>,
    ) {
        self.prepare_in(&self.base, device, queue, encoder, frame);
    }

    /// [`Ps2Bloom::prepare`] for split-screen view `view`, into buffers of its own
    /// (built here, rebuilt when `extent` changes - see
    /// [`super::bloom::lane_extent`]), so the next view's downsample does not
    /// overwrite what this one's composite owes. The scene is read from the view's
    /// own rectangle (`frame.origin`, `frame.viewport`).
    ///
    /// # Errors
    ///
    /// Propagates a buffer that will not allocate; the view then draws no glow.
    pub fn prepare_view(
        &self,
        device: &wgpu::Device,
        queue: &dyn oag_gpu::writes::Writes,
        encoder: &mut wgpu::CommandEncoder,
        view: usize,
        extent: (u32, u32),
        frame: Frame<'_>,
    ) -> Result<()> {
        ensure_view_lane(
            &self.views,
            view,
            extent,
            |lane| lane.extent,
            || Lane::new(device, self.format, &self.layout, &self.sampler, extent),
        )?;
        self.prepare_in(&self.views.borrow()[view], device, queue, encoder, frame);
        Ok(())
    }

    fn prepare_in(
        &self,
        lane: &Lane,
        device: &wgpu::Device,
        queue: &dyn oag_gpu::writes::Writes,
        encoder: &mut wgpu::CommandEncoder,
        frame: Frame<'_>,
    ) {
        let scene = frame.scene;
        let (uv_offset, uv_scale, uv_max) = frame.uv();
        let wanted = Constants {
            uv_scale,
            uv_max,
            direction: [0.0, 0.0],
            shift: [COMPOSITE_OFFSET / FRAME_SIZE; 2],
            uv_offset,
        };
        if lane.written.get() != Some(wanted) {
            queue.write_buffer(&lane.constants_down, 0, bytemuck::bytes_of(&wanted));
            lane.written.set(Some(wanted));
        }
        let down_group = scene_group(
            device,
            &self.layout,
            &self.sampler,
            "ps2 bloom down",
            scene,
            &lane.constants_down,
        );
        let mut pass = |label: &'static str,
                        pipeline: &wgpu::RenderPipeline,
                        group: &wgpu::BindGroup,
                        target: &wgpu::TextureView| {
            scratch_pass(encoder, lane.extent, label, pipeline, group, target);
        };
        pass("ps2 bloom down", &self.down, &down_group, &lane.a.view);
        pass("ps2 bloom blur x", &self.blur, &lane.blur_x, &lane.b.view);
        pass("ps2 bloom blur y", &self.blur, &lane.blur_y, &lane.a.view);
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
        self.composite_in(&self.base, encoder, target, origin, viewport);
    }

    /// [`Ps2Bloom::composite`] of what [`Ps2Bloom::prepare_view`] left for view
    /// `view`. Nothing is drawn for a view that was never prepared.
    pub fn composite_view(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: usize,
        target: &wgpu::TextureView,
        origin: (f32, f32),
        viewport: (u32, u32),
    ) {
        if let Some(lane) = self.views.borrow().get(view) {
            self.composite_in(lane, encoder, target, origin, viewport);
        }
    }

    fn composite_in(
        &self,
        lane: &Lane,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        origin: (f32, f32),
        viewport: (u32, u32),
    ) {
        crate::pass::draw(
            encoder,
            "ps2 bloom composite",
            &self.composite,
            &[&lane.composite_group],
            target,
            origin,
            (viewport.0.max(1), viewport.1.max(1)),
        );
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

    const SCENE: (u32, u32) = (128, 64);

    /// The scene (its right half carries a glow mask) after `glow` has run over the
    /// right half, as RGBA bytes.
    fn glowed(
        glow: impl Fn(
            &Ps2Bloom,
            &wgpu::Device,
            &wgpu::Queue,
            &mut wgpu::CommandEncoder,
            Frame<'_>,
            &wgpu::TextureView,
        ),
    ) -> Option<Vec<u8>> {
        let instance = wgpu::Instance::default();
        let adapter = oag_gpu::test_adapter::ordered(&instance)
            .into_iter()
            .next()?;
        let (device, queue) =
            pollster::block_on(adapter.request_device(&Default::default())).expect("a device");
        let format = wgpu::TextureFormat::Rgba8Unorm;
        let extent = wgpu::Extent3d {
            width: SCENE.0,
            height: SCENE.1,
            depth_or_array_layers: 1,
        };
        let scene = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("ps2 bloom test scene"),
            size: extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let texels: Vec<u8> = (0..SCENE.0 * SCENE.1)
            .flat_map(|i| {
                [
                    150,
                    150,
                    150,
                    if i % SCENE.0 >= SCENE.0 / 2 { 0xaf } else { 0 },
                ]
            })
            .collect();
        queue.write_texture(
            scene.as_image_copy(),
            &texels,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(SCENE.0 * 4),
                rows_per_image: Some(SCENE.1),
            },
            extent,
        );
        let bloom = Ps2Bloom::new(&device, format).expect("the pipelines build");
        let view = scene.create_view(&wgpu::TextureViewDescriptor::default());
        let frame = Frame {
            scene: &view,
            size: SCENE,
            origin: (SCENE.0 as f32 / 2.0, 0.0),
            viewport: (SCENE.0 / 2, SCENE.1),
        };
        let mut encoder = device.create_command_encoder(&Default::default());
        glow(&bloom, &device, &queue, &mut encoder, frame, &view);
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ps2 bloom test readback"),
            size: u64::from(SCENE.0 * SCENE.1 * 4),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        encoder.copy_texture_to_buffer(
            scene.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(SCENE.0 * 4),
                    rows_per_image: Some(SCENE.1),
                },
            },
            extent,
        );
        queue.submit([encoder.finish()]);
        readback
            .slice(..)
            .map_async(wgpu::MapMode::Read, |r| r.expect("map"));
        device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("poll");
        let bytes = readback
            .slice(..)
            .get_mapped_range()
            .expect("mapped")
            .to_vec();
        Some(bytes)
    }

    /// The red bytes of the left and the right half.
    fn halves(bytes: &[u8]) -> [u64; 2] {
        let mut sums = [0u64; 2];
        for (i, p) in bytes.as_chunks::<4>().0.iter().enumerate() {
            sums[usize::from(i as u32 % SCENE.0 >= SCENE.0 / 2)] += u64::from(p[0]);
        }
        sums
    }

    /// **A view's lane draws what the whole-picture lane draws for the same rectangle.**
    /// The right half glows and the left is outside the composite; the same bytes come out
    /// of `prepare_view` + `composite_view` as out of `prepare` + `composite`, a view that
    /// was never prepared draws nothing, and a lane asked for at another extent is rebuilt
    /// and still draws.
    #[test]
    fn a_view_lane_draws_what_the_whole_lane_draws_for_its_rectangle() {
        let at = (SCENE.0 as f32 / 2.0, 0.0);
        let half = (SCENE.0 / 2, SCENE.1);
        let Some(whole) = glowed(|bloom, device, queue, encoder, frame, view| {
            bloom.prepare(device, queue, encoder, frame);
            bloom.composite(encoder, view, at, half);
        }) else {
            return;
        };
        let untouched = glowed(|_, _, _, _, _, _| {}).expect("a device");
        let through = glowed(|bloom, device, queue, encoder, frame, view| {
            bloom
                .prepare_view(
                    device,
                    queue,
                    encoder,
                    1,
                    (BLOOM_WIDTH, BLOOM_HEIGHT),
                    frame,
                )
                .expect("the lane builds");
            bloom.composite_view(encoder, 1, view, at, half);
        })
        .expect("a device");
        assert_eq!(
            through, whole,
            "the view lane drew other bytes than the whole lane"
        );
        let [left, right] = halves(&whole);
        let [left_before, right_before] = halves(&untouched);
        assert_eq!(left, left_before, "the glow reached outside its rectangle");
        assert!(right > right_before, "the right half did not glow");

        let unprepared = glowed(|bloom, _, _, encoder, _, view| {
            bloom.composite_view(encoder, 1, view, at, half);
        })
        .expect("a device");
        assert_eq!(unprepared, untouched, "a view never prepared drew");

        let rebuilt = glowed(|bloom, device, queue, encoder, frame, view| {
            for extent in [
                (BLOOM_WIDTH, BLOOM_HEIGHT),
                (BLOOM_WIDTH / 2, BLOOM_HEIGHT / 2),
            ] {
                bloom
                    .prepare_view(device, queue, encoder, 1, extent, frame)
                    .expect("the lane builds");
                assert_eq!(
                    bloom.views.borrow().len(),
                    2,
                    "lanes up to the view are built"
                );
                assert_eq!(bloom.views.borrow()[1].extent, extent);
            }
            bloom.composite_view(encoder, 1, view, at, half);
        })
        .expect("a device");
        let [_, right_rebuilt] = halves(&rebuilt);
        assert!(
            right_rebuilt > right_before,
            "the rebuilt lane drew no glow"
        );
    }
}
