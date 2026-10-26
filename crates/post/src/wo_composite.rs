//! Wipeout 2048's race post chain: a linear float scene target in the gamma-2
//! space 2048's lit programs work in, the `wo_*` gate, blurs and composite over
//! it, and the exposure the composite applies, adapted once a frame.
//!
//! Recovered from `eboot.elf` (patch v1.04) on 2026-10-26: the CPU side that
//! creates the targets (`PostFx_CreateTargets`, `0x81037670`), runs the passes
//! (`PostFx_RenderChain`, `0x810387b4`) and steps the exposure
//! (`Exposure_AdaptFromReadback`, `0x8103d148`), and every program read off
//! Vita3K's translation of it. See
//! [exposure.md](../../../../docs/ghidra/functions/vita-2048-eu-v104/exposure.md).
//! The arithmetic is [`law`]; this module runs it on the GPU, in the original's
//! order:
//!
//! 1. the gate, four scene taps into a quarter-resolution 8-bit target:
//!    `rgb = (E * clamp(Lg - Gate, 0, 1))^2 * s`, `alpha = Lg / 4`;
//! 2. two six-tap blurs along the two diagonals, quarter target to quarter
//!    target;
//! 3. the composite into the caller's view,
//!    `sqrt(e L / (1 + L) + BloomFactor * bloom)` with `e = E * main`;
//! 4. the step: `a` = the mean alpha byte of every fourth pixel of every fourth
//!    row of the gate target, over 64, and `E += (Scale / a - E) * Speed`,
//!    capped at `ExposureMax`.
//!
//! What is **not** the disc's, stated rather than hidden:
//!
//! - **The vignette and the screen tint are left out.** The composite's
//!   `vignetteTint` and the mesh it is drawn as, and the tint's source, are
//!   unread, so `v = 1` and the tint is 0.
//! - **`alphaTex` reads as empty**: the original composites a second texture
//!   over the scene (`c = alpha.rgb + (1 - alpha.a) * main`); this renderer
//!   draws everything into the scene, so `c = main`.
//! - **The first frame starts settled** on its own readback, where the
//!   executable starts from `Exposure_Set`'s 1.15 and walks.
//! - **One step per rendered frame**, on the GPU and with no readback latency.
//!   The original steps once per frame it renders, about 30 a second.
//! - **The scene target is `Rgba16Float`** where the original's is
//!   `U2F10F10F10` (format `0x41000000` in `0x81035b10`); both hold light
//!   above 1.
//! - **The Zone race's own composites** (`0x816a4d80`, `0x816a4d84`) are not
//!   read; every race runs this one.

pub mod law;

pub use law::{Exposure, Params, RESET_EXPOSURE};

use anyhow::Result;

use oag_gpu::formats::SCENE_FORMAT;

/// The quarter-resolution targets' format: the display surface's, which
/// `PostFx_CreateTargets` reads with `sceGxmColorSurfaceGetFormat` - the Vita
/// displays `A8B8G8R8`, so the gate's alpha is byte 3, where the readback
/// looks.
pub const QUARTER_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

/// Seats a split can hold, and so the most adaptations besides the window's own.
pub const MAX_SEATS: usize = 8;

/// One view of a split, as the chain runs over it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct View {
    /// The seat, whose exposure this view keeps from frame to frame.
    pub seat: usize,
    /// The view's rectangle, `x, y, width, height`: where it was drawn in the scene target
    /// and where its picture goes in the frame (the same place).
    pub rect: (u32, u32, u32, u32),
    /// Whether this is the first view of the frame, which clears the whole frame to opaque
    /// black first.
    pub first: bool,
}

/// The uniform block `wo_composite.wesl` reads.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniform {
    scene_offset: [f32; 2],
    scene_scale: [f32; 2],
    scene_max: [f32; 2],
    scene_texel: [f32; 2],
    quarter_scale: [f32; 2],
    quarter_max: [f32; 2],
    quarter_texel: [f32; 2],
    quarter_pixels: [u32; 2],
    bloom_factor: f32,
    bloom_gate: f32,
    exposure_scale: f32,
    exposure_max: f32,
    exposure_speed: f32,
    _pad: f32,
    _pad2: [f32; 2],
}

impl Uniform {
    fn new(p: Params, scene: (u32, u32), quarter: (u32, u32), placement: Placement) -> Self {
        let (scene_offset, scene_scale, scene_max) =
            super::sub_rectangle_at(placement.source, placement.viewport, scene);
        let drawn = quarter_of(placement.viewport);
        let (quarter_scale, quarter_max) = super::sub_rectangle(drawn, quarter);
        Self {
            scene_offset,
            scene_scale,
            scene_max,
            scene_texel: [1.0 / scene.0 as f32, 1.0 / scene.1 as f32],
            quarter_scale,
            quarter_max,
            quarter_texel: [1.0 / quarter.0 as f32, 1.0 / quarter.1 as f32],
            quarter_pixels: [drawn.0, drawn.1],
            bloom_factor: p.bloom_factor,
            bloom_gate: p.bloom_gate,
            exposure_scale: p.exposure_scale,
            exposure_max: p.exposure_max,
            exposure_speed: p.exposure_speed,
            _pad: 0.0,
            _pad2: [0.0; 2],
        }
    }
}

/// The quarter-resolution size of `size`: `PostFx_CreateTargets` shifts both
/// sides right by two.
fn quarter_of(size: (u32, u32)) -> (u32, u32) {
    ((size.0 >> 2).max(1), (size.1 >> 2).max(1))
}

/// `State` in the shader.
const STATE_SIZE: u64 = 16;

/// Where a run reads the scene from and how much of it: the corner of the drawn
/// rectangle in the scene target and its size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Placement {
    source: (u32, u32),
    viewport: (u32, u32),
}

/// One picture's exposure and the two groups that read and step it.
#[derive(Debug)]
struct Seat {
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "held so the bindings stay valid; the tests read it"
        )
    )]
    state: wgpu::Buffer,
    /// The gate's and the composite's read-only binding.
    read: wgpu::BindGroup,
    /// The step's read-write binding.
    step: wgpu::BindGroup,
    /// Whether the first frame has set the exposure.
    primed: std::cell::Cell<bool>,
}

impl Seat {
    fn new(
        device: &wgpu::Device,
        state_rw: &wgpu::BindGroupLayout,
        state_ro: &wgpu::BindGroupLayout,
        exposure: Exposure,
    ) -> Self {
        let start = match exposure {
            Exposure::Adapting => RESET_EXPOSURE,
            Exposure::Held(value) => value,
        };
        let mut words = [0_u8; STATE_SIZE as usize];
        words[..4].copy_from_slice(&start.to_le_bytes());
        let state = oag_gpu::init_buffer::init(
            device,
            "wo composite state",
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            &words,
        );
        let group = |layout: &wgpu::BindGroupLayout| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("wo composite state"),
                layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: state.as_entire_binding(),
                }],
            })
        };
        Self {
            read: group(state_ro),
            step: group(state_rw),
            state,
            primed: std::cell::Cell::new(false),
        }
    }
}

/// One colour texture and its view.
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
    ) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width: width.max(1),
                height: height.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            // `COPY_DST` so a test can lay known pixels into the scene.
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        Self { texture, view }
    }
}

/// The scene-sized half: the float target, the three quarter targets and the
/// groups that read each.
#[derive(Debug)]
struct Sized {
    scene: Target,
    size: (u32, u32),
    quarter: [Target; 3],
    quarter_size: (u32, u32),
    /// Reads the scene: the gate and the composite.
    scene_group: wgpu::BindGroup,
    /// Reads the gate target: the first blur and the step.
    gated_group: wgpu::BindGroup,
    /// Reads the first blur's target: the second blur.
    blurred_group: wgpu::BindGroup,
    /// The composite's bloom: the second blur's target.
    bloom_group: wgpu::BindGroup,
}

/// The chain. Built once per race for a circuit of a title that runs it.
#[derive(Debug)]
pub struct Chain {
    params: Params,
    gate: wgpu::RenderPipeline,
    blur_first: wgpu::RenderPipeline,
    blur_second: wgpu::RenderPipeline,
    composite: wgpu::RenderPipeline,
    adapt: wgpu::ComputePipeline,
    prime: wgpu::ComputePipeline,
    layout: wgpu::BindGroupLayout,
    bloom_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    uniform: wgpu::Buffer,
    /// The window's own picture's exposure.
    window: Seat,
    /// Each split seat's, built when a split first draws it ([`Chain::run_view`]).
    seats: std::cell::RefCell<Vec<Option<Seat>>>,
    /// How many seats' exposures have been built, for [`Chain::seat_lane_builds`].
    seat_builds: std::cell::Cell<u64>,
    state_rw: wgpu::BindGroupLayout,
    state_ro: wgpu::BindGroupLayout,
    sized: Sized,
    written: std::cell::Cell<Option<Placement>>,
}

impl Chain {
    /// Builds the pipelines and targets.
    ///
    /// `surface_format` is the caller's own target, which only the composite
    /// writes; the scene is drawn into [`SCENE_FORMAT`], which is what switches
    /// every scene pipeline to linear output (`mesh_render::is_linear_target`).
    ///
    /// # Errors
    ///
    /// Propagates a shader or pipeline that will not build.
    pub fn new(
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
        size: (u32, u32),
        params: Params,
    ) -> Result<Self> {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("wo composite"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!(concat!(env!("OUT_DIR"), "/wo_composite.wgsl")).into(),
            ),
        });
        let both = wgpu::ShaderStages::FRAGMENT | wgpu::ShaderStages::COMPUTE;
        let texture = |binding: u32, visibility: wgpu::ShaderStages| wgpu::BindGroupLayoutEntry {
            binding,
            visibility,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        };
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("wo composite"),
            entries: &[
                texture(0, both),
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: both,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: both,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let bloom_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("wo composite bloom"),
            entries: &[texture(0, wgpu::ShaderStages::FRAGMENT)],
        });
        let state_layout = |read_only: bool, visibility: wgpu::ShaderStages| {
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("wo composite state"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            })
        };
        let state_rw = state_layout(false, wgpu::ShaderStages::COMPUTE);
        let state_ro = state_layout(true, wgpu::ShaderStages::FRAGMENT);
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("wo composite"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let pipeline_layout = |groups: &[Option<&wgpu::BindGroupLayout>]| {
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("wo composite"),
                bind_group_layouts: groups,
                immediate_size: 0,
            })
        };
        let plain = pipeline_layout(&[Some(&layout)]);
        let exposed = pipeline_layout(&[Some(&layout), Some(&state_ro)]);
        let composed = pipeline_layout(&[Some(&layout), Some(&state_ro), Some(&bloom_layout)]);
        let stepped = pipeline_layout(&[Some(&layout), Some(&state_rw)]);
        let render = |label: &str,
                      entry: &str,
                      layout: &wgpu::PipelineLayout,
                      format: wgpu::TextureFormat| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(layout),
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
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
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
        let compute = |label: &str, entry: &str| {
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(label),
                layout: Some(&stepped),
                module: &shader,
                entry_point: Some(entry),
                compilation_options: Default::default(),
                cache: None,
            })
        };
        let gate = render("wo composite gate", "fs_gate", &exposed, QUARTER_FORMAT);
        let blur_first = render("wo composite blur", "fs_blur_first", &plain, QUARTER_FORMAT);
        let blur_second = render(
            "wo composite blur",
            "fs_blur_second",
            &plain,
            QUARTER_FORMAT,
        );
        // The composite's square root is the encode, written as numbers, so it
        // targets the caller's format without the sRGB suffix - the same as
        // `hd_bloom`'s encode and `omega_tonemap`'s apply.
        let composite = render(
            "wo composite",
            "fs_composite",
            &composed,
            surface_format.remove_srgb_suffix(),
        );
        let adapt = compute("wo composite adapt", "cs_adapt");
        let prime = compute("wo composite prime", "cs_prime");
        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("wo composite uniform"),
            size: std::mem::size_of::<Uniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let window = Seat::new(device, &state_rw, &state_ro, params.exposure);
        let sized = Self::sized(device, &layout, &bloom_layout, &sampler, &uniform, size);
        Ok(Self {
            params,
            gate,
            blur_first,
            blur_second,
            composite,
            adapt,
            prime,
            layout,
            bloom_layout,
            sampler,
            uniform,
            window,
            seats: std::cell::RefCell::new(Vec::new()),
            seat_builds: std::cell::Cell::new(0),
            state_rw,
            state_ro,
            sized,
            written: std::cell::Cell::new(None),
        })
    }

    fn sized(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        bloom_layout: &wgpu::BindGroupLayout,
        sampler: &wgpu::Sampler,
        uniform: &wgpu::Buffer,
        size: (u32, u32),
    ) -> Sized {
        let size = (size.0.max(1), size.1.max(1));
        let quarter_size = quarter_of(size);
        let scene = Target::new(device, "wo scene", SCENE_FORMAT, size);
        let quarter = ["wo gate", "wo blur", "wo bloom"]
            .map(|label| Target::new(device, label, QUARTER_FORMAT, quarter_size));
        let group = |label: &str, source: &wgpu::TextureView| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(label),
                layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(source),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: uniform.as_entire_binding(),
                    },
                ],
            })
        };
        let bloom_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("wo composite bloom"),
            layout: bloom_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&quarter[2].view),
            }],
        });
        Sized {
            scene_group: group("wo composite scene", &scene.view),
            gated_group: group("wo composite gated", &quarter[0].view),
            blurred_group: group("wo composite blurred", &quarter[1].view),
            bloom_group,
            scene,
            size,
            quarter,
            quarter_size,
        }
    }

    /// The circuit's keys, as the chain was built with them.
    #[must_use]
    pub fn params(&self) -> Params {
        self.params
    }

    /// The float scene target the race pass draws into.
    #[must_use]
    pub fn scene_view(&self) -> &wgpu::TextureView {
        &self.sized.scene.view
    }

    /// The window's exposure state buffer, for a test that reads it back.
    #[cfg(test)]
    fn window_state(&self) -> &wgpu::Buffer {
        &self.window.state
    }

    /// Rebuilds the targets for a new viewport size. The exposure carries over:
    /// the luminance it tracks is the picture's, not the target's.
    pub fn resize(&mut self, device: &wgpu::Device, size: (u32, u32)) {
        self.sized = Self::sized(
            device,
            &self.layout,
            &self.bloom_layout,
            &self.sampler,
            &self.uniform,
            size,
        );
        self.written.set(None);
    }

    /// Gates, blurs and composites the frame into `view` at `origin`, over
    /// `viewport` - the rectangle the race pass drew - and steps the exposure.
    pub fn run(
        &self,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        origin: (f32, f32),
        viewport: (u32, u32),
    ) {
        self.run_placed(
            queue,
            encoder,
            view,
            &self.window,
            ((0, 0), origin),
            viewport,
        );
    }

    /// [`Chain::run`] for a rectangle that starts at `placement.0` of the scene target and
    /// lands at `placement.1` of `view`, exposed by `seat`.
    fn run_placed(
        &self,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        seat: &Seat,
        (source, origin): ((u32, u32), (f32, f32)),
        viewport: (u32, u32),
    ) {
        let scene = self.sized.size;
        let placement = Placement {
            source,
            viewport: (viewport.0.clamp(1, scene.0), viewport.1.clamp(1, scene.1)),
        };
        if self.written.get() != Some(placement) {
            queue.write_buffer(
                &self.uniform,
                0,
                bytemuck::bytes_of(&Uniform::new(
                    self.params,
                    scene,
                    self.sized.quarter_size,
                    placement,
                )),
            );
            self.written.set(Some(placement));
        }
        let quarter = quarter_of(placement.viewport);
        let adapting = self.params.exposure == Exposure::Adapting;
        let gate = |encoder: &mut wgpu::CommandEncoder| {
            draw(
                encoder,
                "wo composite gate",
                &self.gate,
                &[&self.sized.scene_group, &seat.read],
                &self.sized.quarter[0].view,
                (0.0, 0.0),
                quarter,
            );
        };
        if adapting && !seat.primed.get() {
            // The gate's alpha does not depend on `E`, so the first frame's
            // readback is the same before and after the exposure is set.
            gate(encoder);
            self.step(encoder, &self.prime, seat);
            seat.primed.set(true);
        }
        gate(encoder);
        draw(
            encoder,
            "wo composite blur",
            &self.blur_first,
            &[&self.sized.gated_group],
            &self.sized.quarter[1].view,
            (0.0, 0.0),
            quarter,
        );
        draw(
            encoder,
            "wo composite blur",
            &self.blur_second,
            &[&self.sized.blurred_group],
            &self.sized.quarter[2].view,
            (0.0, 0.0),
            quarter,
        );
        draw(
            encoder,
            "wo composite",
            &self.composite,
            &[&self.sized.scene_group, &seat.read, &self.sized.bloom_group],
            view,
            origin,
            placement.viewport,
        );
        if adapting {
            self.step(encoder, &self.adapt, seat);
        }
    }

    /// One dispatch of the readback and a step over the gate target.
    fn step(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        pipeline: &wgpu::ComputePipeline,
        seat: &Seat,
    ) {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("wo composite step"),
            timestamp_writes: None,
        });
        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, &self.sized.gated_group, &[]);
        pass.set_bind_group(1, &seat.step, &[]);
        pass.dispatch_workgroups(1, 1, 1);
    }

    /// Runs the chain over one view of a split: the view's own exposure (the seat's), the
    /// composite over the view's rectangle and nothing outside it. **Call it once per view
    /// with its own encoder and submit each before the next**: the constants are written
    /// with the queue, which lands before the submission that follows. **The per-seat
    /// exposure is this project's, chosen, not measured**: the original has no split screen.
    pub fn run_view(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        run: View,
    ) {
        let seat = run.seat.min(MAX_SEATS - 1);
        {
            let mut seats = self.seats.borrow_mut();
            if seats.len() <= seat {
                seats.resize_with(seat + 1, || None);
            }
            if seats[seat].is_none() {
                seats[seat] = Some(Seat::new(
                    device,
                    &self.state_rw,
                    &self.state_ro,
                    self.params.exposure,
                ));
                self.seat_builds.set(self.seat_builds.get() + 1);
            }
        }
        let seats = self.seats.borrow();
        let (x, y, w, h) = run.rect;
        if run.first {
            // A pass that clears and draws nothing: the composite loads.
            encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("wo composite clear"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
        }
        self.run_placed(
            queue,
            encoder,
            view,
            seats[seat].as_ref().expect("built above"),
            ((x, y), (x as f32, y as f32)),
            (w, h),
        );
    }

    /// How many seats hold an exposure of their own right now.
    #[must_use]
    pub fn seat_lanes(&self) -> usize {
        self.seats.borrow().iter().flatten().count()
    }

    /// Forgets every seat's exposure, for the first whole-picture frame after a split.
    pub fn retire_seats(&self) {
        self.seats.borrow_mut().clear();
    }

    /// Forgets one seat's exposure, for a seat that was not drawn as a split view last frame.
    pub fn retire_seat(&self, seat: usize) {
        if let Some(lane) = self.seats.borrow_mut().get_mut(seat.min(MAX_SEATS - 1)) {
            lane.take();
        }
    }

    /// How many seat lanes have been built so far, a lane forgotten and built again counting
    /// twice (`hd_bloom::Chain::seat_lane_builds`).
    #[must_use]
    pub fn seat_lane_builds(&self) -> u64 {
        self.seat_builds.get()
    }
}

/// One full-screen pass with no blending, loading what is there.
fn draw(
    encoder: &mut wgpu::CommandEncoder,
    label: &str,
    pipeline: &wgpu::RenderPipeline,
    groups: &[&wgpu::BindGroup],
    target: &wgpu::TextureView,
    origin: (f32, f32),
    rect: (u32, u32),
) {
    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some(label),
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
    pass.set_pipeline(pipeline);
    pass.set_viewport(origin.0, origin.1, rect.0 as f32, rect.1 as f32, 0.0, 1.0);
    for (index, group) in groups.iter().enumerate() {
        pass.set_bind_group(index as u32, *group, &[]);
    }
    pass.draw(0..3, 0..1);
}

#[cfg(test)]
mod tests;
