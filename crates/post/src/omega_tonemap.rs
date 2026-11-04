//! Wipeout: Omega Collection's tone map: a linear float scene target, the
//! adaptive exposure and the cubic curve the PS4 executable applies in its
//! MSAA resolve, and the display encode.
//!
//! Recovered from `eboot.bin` on 2026-10-05, statically: the CPU side that
//! turns the `.EnvSettings` `Tonemap.*` block into shader constants
//! (`ToneMap_ApplyEnvSettings`, `0x01620980`) and the GCN microcode of the
//! three shaders that use them. See
//! [tonemap.md](../../../../docs/ghidra/functions/ps4-omega-eu/tonemap.md).
//! The arithmetic is [`law`]; this module only runs it on the GPU, in the
//! original's order:
//!
//! 1. a 256x256 Rec.601 luma image of the scene, halved by bilinear copies to
//!    one texel - the mean luma of the frame;
//! 2. the coefficient step: the mean of the last `round(time * 60)` frames'
//!    luma, `LAvg` stepped toward it by at most `response / 60`, the Hermite
//!    cubic and the exposure;
//! 3. per pixel and per channel, `y = cubic(clamp(exposure * c, 0, t1))`;
//! 4. the race composite's terms that follow the curve ([`law::Composite`]):
//!    `ColorScale * vignette * (curved + BloomScale * Bloom) + ScreenTint`, the
//!    vignette from the circuit's `Vignette.*` keys, the bloom from its
//!    `Bloom.*` keys ([`bloom`]), `ColorScale` at its rest `(1, 1, 1)` and the
//!    tint at its rest zero;
//! 5. the display's encode of `y`: the piecewise sRGB curve
//!    ([`law::scanout_encode`]). The executable scans out an `A2R10G10B10`
//!    UNORM buffer (`0x88060000`, gamma 1.0) filled by a plain copy, so the
//!    video out encodes; shadPS4 0.20, which captured the reference frames,
//!    applies this curve to that format.
//!
//! What is **not** the disc's, stated rather than hidden:
//!
//! - **Per pixel, not per sample.** The original curves each MSAA sample and
//!   then averages; this chain curves the already resolved scene. The two
//!   differ only along edges.
//! - **The first frame starts settled** on its own luminance; the executable
//!   clears its history to zero. A single captured frame would otherwise
//!   show an exposure no player sees after the first second.
//! - **One step per rendered frame.** The executable steps once per frame at
//!   60 Hz; this does the same per frame it draws, at whatever rate that is.
//! - **No readback latency.** The executable averages luminance it reads back
//!   from earlier frames; this keeps the history on the GPU and includes the
//!   current frame.
//! - **The low-resolution additive layer** the original adds after the curve is
//!   not drawn: the particles draw into the scene, so they pass through it.
//! - **The bloom's mask.** The bright pass adds `AlphaScale * mask`, the mask being
//!   the scene's second target's `m_Alpha`, half-size. No draw here writes one, so
//!   only the frame's own bright pixels bloom ([`bloom`], tonemap.md, "The bloom chain").
//! - **The bloom reads the frame with no accumulated motion blur**: the original's
//!   source is that blur's target when it runs; this renderer has no such target.
//! - **Brightness** stays at the executable's middle setting, 1.0; this port
//!   grades brightness after the frame.
//!
//! The curve resamples the scene at an **offset** when a frame's heat-haze and
//! shock-ring particles drew one: the executable's composite reads its
//! `DistortionTexture` and moves every scene sample by
//! `0.0100021 * (16/9 * d.x, d.y)` ([`DISTORT_SCALE`], [`DISTORT_ASPECT`];
//! `docs/ghidra/functions/ps4-omega-eu/heat-haze.md`, confidence 80). Frames
//! that drew none sample a zero texture and are byte-identical to before.

pub mod bloom;
pub mod law;

pub use law::{Bloom, Composite, Curve, Params};

use anyhow::Result;

use oag_gpu::formats::{DISTORTION_FORMAT, SCENE_FORMAT};

/// The luma image's side, the executable's own `0x100` (format `R16F`).
pub const LADDER_SIZE: u32 = 256;

/// The composite's offset scale: the immediate `0x3c23e000` every
/// `wo_composite*` shader multiplies the distortion texture's `rg` by.
pub const DISTORT_SCALE: f32 = 0.010_002_1;

/// The horizontal factor on that offset: `ScreenTint_Aspect.w`, which
/// `FUN_01625750` writes as `0x3fe38e39` (16/9) whenever its mode byte is not
/// 2. The other mode was not read, and the value is **not** the window's
/// aspect: it is the constant the original wrote.
pub const DISTORT_ASPECT: f32 = 16.0 / 9.0;

/// Seats a split can hold, and so the most adaptations besides the window's own.
pub const MAX_SEATS: usize = 8;

/// One view of a split, as the chain runs over it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct View {
    /// The seat, whose adaptation this view keeps from frame to frame.
    pub seat: usize,
    /// The view's rectangle, `x, y, width, height`: where it was drawn in the scene target
    /// and where its picture goes in the frame (the same place).
    pub rect: (u32, u32, u32, u32),
    /// Whether this is the first view of the frame, which clears the whole frame to opaque
    /// black first (the gap between views, and any stale frame under it); every later view
    /// loads what the ones before it wrote. The whole-picture [`Chain::run`] draws every
    /// pixel of its rectangle and clears nothing.
    pub first: bool,
}

/// The luma ladder's format.
pub const LADDER_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::R16Float;

/// The uniform block `omega_tonemap.wesl` reads.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniform {
    luminance_a: f32,
    luminance_b: f32,
    exposure_minimum: f32,
    exposure_maximum: f32,
    step: f32,
    history: u32,
    source_end_a: f32,
    source_end_b: f32,
    start_angle: f32,
    end_angle: f32,
    output_end: f32,
    brightness: f32,
    uv_scale: [f32; 2],
    uv_max: [f32; 2],
    distortion: [f32; 2],
    /// Where the drawn rectangle starts in the scene target: a split view's rectangle need
    /// not be at the corner. `(0, 0)` for a whole picture, which then reads the bits it
    /// always did.
    uv_offset: [f32; 2],
    /// The composite's vignette `(a, b)`.
    vignette: [f32; 2],
    _pad: [f32; 2],
    color_scale: [f32; 4],
    screen_tint: [f32; 4],
    bright: [f32; 4],
    bloom_tap: [f32; 2],
    bloom_uv: [f32; 2],
    bloom_scale: f32,
    _pad_bloom: f32,
    uv_min: [f32; 2],
}

/// Where a view sits in the bloom levels: the share of level 0 it fills, level 0's
/// rectangle that makes in texels, and the first scene texel centre the bright pass may read.
#[derive(Debug, Clone, Copy)]
struct BloomView {
    uv: [f32; 2],
    rect: (u32, u32),
    uv_min: [f32; 2],
}

impl BloomView {
    /// The view of a chain with no levels.
    const NONE: Self = Self {
        uv: [1.0, 1.0],
        rect: (1, 1),
        uv_min: [0.0, 0.0],
    };
}

impl Uniform {
    fn new(
        p: Params,
        (uv_offset, uv_scale, uv_max): ([f32; 2], [f32; 2], [f32; 2]),
        view: BloomView,
    ) -> Self {
        Self {
            luminance_a: p.luminance_a,
            luminance_b: p.luminance_b,
            exposure_minimum: p.exposure_minimum,
            exposure_maximum: p.exposure_maximum,
            step: p.step(),
            history: p.history_frames(),
            source_end_a: p.source_end_a,
            source_end_b: p.source_end_b,
            start_angle: p.start_angle.to_radians(),
            end_angle: p.end_angle.to_radians(),
            output_end: law::SDR_OUTPUT_END,
            brightness: law::BRIGHTNESS,
            uv_scale,
            uv_max,
            distortion: [DISTORT_SCALE * DISTORT_ASPECT, DISTORT_SCALE],
            uv_offset,
            vignette: p.composite.vignette_ab(),
            _pad: [0.0; 2],
            color_scale: [
                p.composite.color_scale[0],
                p.composite.color_scale[1],
                p.composite.color_scale[2],
                1.0,
            ],
            screen_tint: [
                p.composite.screen_tint[0],
                p.composite.screen_tint[1],
                p.composite.screen_tint[2],
                0.0,
            ],
            bright: p.composite.bloom.map_or([0.0; 4], |b| {
                [b.threshold, b.offset, b.scale, b.power]
            }),
            bloom_tap: [0.25 / view.rect.0 as f32, 0.25 / view.rect.1 as f32],
            bloom_uv: view.uv,
            bloom_scale: p.composite.bloom.map_or(0.0, |b| b.blend_factors[0]),
            _pad_bloom: 0.0,
            uv_min: view.uv_min,
        }
    }
}

/// `State` in the shader: four words, two `vec4`s and the 256-entry ring.
const STATE_SIZE: u64 = 16 + 16 + 16 + 256 * 4;

/// One picture's adaptation: the luminance history, the stepped `LAvg` and the curve built
/// from them, and the two groups that read and write it.
#[derive(Debug)]
struct Seat {
    #[expect(dead_code, reason = "held so the bindings stay valid")]
    state: wgpu::Buffer,
    /// The coefficient pass's read-write binding.
    coefficients: wgpu::BindGroup,
    /// The apply pass's read-only binding of the same buffer.
    curve: wgpu::BindGroup,
}

impl Seat {
    fn new(
        device: &wgpu::Device,
        state_rw: &wgpu::BindGroupLayout,
        state_ro: &wgpu::BindGroupLayout,
    ) -> Self {
        // Zeroed: `primed` is 0, so the first dispatch seeds the history.
        let state = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("omega tonemap state"),
            size: STATE_SIZE,
            usage: wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });
        let group = |layout: &wgpu::BindGroupLayout| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("omega tonemap state"),
                layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: state.as_entire_binding(),
                }],
            })
        };
        Self {
            coefficients: group(state_rw),
            curve: group(state_ro),
            state,
        }
    }
}

/// The rectangle last written into the uniform: its corner in the scene target, its size
/// and the share of the frame it is.
type Drawn = ((u32, u32), (u32, u32), [u32; 2]);

/// One colour texture and its view.
#[derive(Debug)]
struct Target {
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "held so the view stays valid; the tests read it")
    )]
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

/// The scene-sized half: the float target and the two groups that read it.
#[derive(Debug)]
struct Sized {
    scene: Target,
    size: (u32, u32),
    luma_group: wgpu::BindGroup,
    apply_group: wgpu::BindGroup,
}

/// The chain. Built once per race for a circuit whose `.EnvSettings` carries
/// a `Tonemap` block.
#[derive(Debug)]
pub struct Chain {
    params: Params,
    luma: wgpu::RenderPipeline,
    halve: wgpu::RenderPipeline,
    apply: wgpu::RenderPipeline,
    coefficients: wgpu::ComputePipeline,
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    uniform: wgpu::Buffer,
    /// The ladder, 256x256 down to 1x1, and for each level after the first
    /// the group that reads the level before it.
    ladder: Vec<(Target, u32)>,
    halve_groups: Vec<wgpu::BindGroup>,
    /// The coefficient pass's input: the ladder's last texel.
    coefficients_in: wgpu::BindGroup,
    /// The window's own picture's adaptation.
    window: Seat,
    /// Each split seat's, built when a split first draws it ([`Chain::run_view`]).
    seats: std::cell::RefCell<Vec<Option<Seat>>>,
    /// How many seats' lanes have been built, for [`Chain::seat_lane_builds`].
    seat_builds: std::cell::Cell<u64>,
    state_rw: wgpu::BindGroupLayout,
    state_ro: wgpu::BindGroupLayout,
    /// The composite's `DistortionTexture` slot, and the zero texture that
    /// fills it on a frame that drew no distortion.
    distortion_layout: wgpu::BindGroupLayout,
    no_distortion: wgpu::BindGroup,
    /// The group over the distortion texture the last frame read; the layout it
    /// binds is fixed, so the view is the whole key.
    distortion_group: super::group_cache::ViewGroup,
    sized: Sized,
    written: std::cell::Cell<Option<Drawn>>,
    /// The bright pass, into the levels' format.
    bright: wgpu::RenderPipeline,
    bloom_pipelines: bloom::Pipelines,
    /// The levels, when the circuit authors a `Bloom` block.
    levels: Option<bloom::Levels>,
    /// The composite's `BloomTexture` slot, a zero texture with no levels.
    bloom_group: wgpu::BindGroup,
}

impl Chain {
    /// Builds the pipelines and targets.
    ///
    /// `surface_format` is the caller's own target, which only the apply pass
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
            label: Some("omega tonemap"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!(concat!(env!("OUT_DIR"), "/omega_tonemap.wgsl")).into(),
            ),
        });
        let both = wgpu::ShaderStages::FRAGMENT | wgpu::ShaderStages::COMPUTE;
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("omega tonemap"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: both,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
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
        let state_layout = |read_only: bool, visibility: wgpu::ShaderStages| {
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("omega tonemap state"),
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
            label: Some("omega tonemap"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let pipeline_layout = |groups: &[Option<&wgpu::BindGroupLayout>]| {
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("omega tonemap"),
                bind_group_layouts: groups,
                immediate_size: 0,
            })
        };
        let distortion_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("omega tonemap distortion"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            }],
        });
        let plain = pipeline_layout(&[Some(&layout)]);
        let with_curve = pipeline_layout(&[
            Some(&layout),
            Some(&state_ro),
            Some(&distortion_layout),
            Some(&distortion_layout),
        ]);
        let reads_curve = pipeline_layout(&[Some(&layout), Some(&state_ro)]);
        let with_state = pipeline_layout(&[Some(&layout), Some(&state_rw)]);
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
        let luma = render("omega tonemap luma", "fs_luma", &plain, LADDER_FORMAT);
        let halve = render("omega tonemap halve", "fs_halve", &plain, LADDER_FORMAT);
        let bright = render(
            "omega bloom bright",
            "fs_bright",
            &reads_curve,
            bloom::LEVEL_FORMAT,
        );
        // The encode is written as numbers, so it targets the caller's format
        // without the sRGB suffix - the same as `hd_bloom`'s encode.
        let apply = render(
            "omega tonemap apply",
            "fs_apply",
            &with_curve,
            surface_format.remove_srgb_suffix(),
        );
        let coefficients = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("omega tonemap coefficients"),
            layout: Some(&with_state),
            module: &shader,
            entry_point: Some("cs_coefficients"),
            compilation_options: Default::default(),
            cache: None,
        });

        let uniform = oag_gpu::init_buffer::init(
            device,
            "omega tonemap uniform",
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            bytemuck::bytes_of(&Uniform::new(
                params,
                ([0.0, 0.0], [1.0, 1.0], [1.0, 1.0]),
                BloomView::NONE,
            )),
        );
        let group = |label: &str, source: &wgpu::TextureView| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(label),
                layout: &layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(source),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: uniform.as_entire_binding(),
                    },
                ],
            })
        };
        let mut ladder = Vec::new();
        let mut side = LADDER_SIZE;
        loop {
            ladder.push((
                Target::new(device, "omega tonemap luma", LADDER_FORMAT, (side, side)),
                side,
            ));
            if side == 1 {
                break;
            }
            side /= 2;
        }
        let halve_groups = ladder
            .windows(2)
            .map(|pair| group("omega tonemap halve", &pair[0].0.view))
            .collect();
        let last = &ladder.last().expect("the ladder ends at one texel").0.view;
        let coefficients_in = group("omega tonemap coefficients", last);
        let window = Seat::new(device, &state_rw, &state_ro);
        let sized = Self::sized(device, &layout, &sampler, &uniform, size);
        let zero = Target::new(device, "omega no distortion", DISTORTION_FORMAT, (1, 1));
        let no_distortion = Self::distortion_group(device, &distortion_layout, &zero.view);
        let no_bloom = Target::new(device, "omega no bloom", bloom::LEVEL_FORMAT, (1, 1));
        let levels = params
            .composite
            .bloom
            .as_ref()
            .map(|authored| bloom::Levels::new(device, &layout, &sampler, authored, size));
        let bloom_view = levels.as_ref().map_or(&no_bloom.view, |l| l.level_zero());
        let bloom_group = Self::distortion_group(device, &distortion_layout, bloom_view);
        let bloom_pipelines = bloom::Pipelines::new(device, &layout);
        Ok(Self {
            params,
            luma,
            halve,
            apply,
            coefficients,
            layout,
            sampler,
            uniform,
            ladder,
            halve_groups,
            coefficients_in,
            window,
            seats: std::cell::RefCell::new(Vec::new()),
            seat_builds: std::cell::Cell::new(0),
            state_rw,
            state_ro,
            distortion_layout,
            no_distortion,
            distortion_group: Default::default(),
            sized,
            written: std::cell::Cell::new(None),
            bright,
            bloom_pipelines,
            levels,
            bloom_group,
        })
    }

    fn distortion_group(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        view: &wgpu::TextureView,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("omega tonemap distortion"),
            layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(view),
            }],
        })
    }

    fn sized(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        sampler: &wgpu::Sampler,
        uniform: &wgpu::Buffer,
        size: (u32, u32),
    ) -> Sized {
        let size = (size.0.max(1), size.1.max(1));
        let scene = Target::new(device, "omega scene", SCENE_FORMAT, size);
        let group = |label: &str| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(label),
                layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&scene.view),
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
        let luma_group = group("omega tonemap luma");
        let apply_group = group("omega tonemap apply");
        Sized {
            scene,
            size,
            luma_group,
            apply_group,
        }
    }

    /// The circuit's block, as the chain was built with it.
    #[must_use]
    pub fn params(&self) -> Params {
        self.params
    }

    /// The float scene target the race pass draws into.
    #[must_use]
    pub fn scene_view(&self) -> &wgpu::TextureView {
        &self.sized.scene.view
    }

    /// The scene target's texture, for a test that writes pixels into it.
    #[cfg(test)]
    fn scene_texture(&self) -> &wgpu::Texture {
        &self.sized.scene.texture
    }

    /// Rebuilds the scene target for a new viewport size. The adaptation
    /// state carries over: the luminance it tracks is the picture's, not the
    /// target's.
    pub fn resize(&mut self, device: &wgpu::Device, size: (u32, u32)) {
        self.sized = Self::sized(device, &self.layout, &self.sampler, &self.uniform, size);
        self.written.set(None);
        if let Some(authored) = self.params.composite.bloom
            && self.levels.as_ref().is_some_and(|l| !l.fits(size))
        {
            let levels = bloom::Levels::new(device, &self.layout, &self.sampler, &authored, size);
            self.bloom_group =
                Self::distortion_group(device, &self.distortion_layout, levels.level_zero());
            self.levels = Some(levels);
        }
    }

    /// Measures the frame, steps the adaptation and writes the curved,
    /// encoded frame into `view` at `origin`, over `viewport` - the rectangle
    /// the race pass drew, exactly as `hd_bloom::Chain::run` takes it.
    ///
    /// `distortion` is the frame's offset texture, laid out like the scene
    /// target ([`DISTORTION_FORMAT`], the pixels the race pass drew), or
    /// `None` for a frame that drew none.
    #[allow(clippy::too_many_arguments)]
    pub fn run(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        origin: (f32, f32),
        viewport: (u32, u32),
        distortion: Option<&wgpu::TextureView>,
    ) {
        self.run_placed(
            device,
            queue,
            encoder,
            view,
            &self.window,
            ((0, 0), origin),
            viewport,
            [1.0, 1.0],
            distortion,
        );
    }

    /// [`Chain::run`] for a rectangle that starts at `placement.0` of the scene target and
    /// lands at `placement.1` of `view`, adapting in `seat`. `share` is how much of the
    /// frame the rectangle is: it sizes the bloom's levels, whatever the scene target's
    /// resolution scale.
    #[allow(clippy::too_many_arguments)]
    fn run_placed(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        seat: &Seat,
        placement: ((u32, u32), (f32, f32)),
        viewport: (u32, u32),
        share: [f32; 2],
        distortion: Option<&wgpu::TextureView>,
    ) {
        let (source, origin) = placement;
        let scene = self.sized.size;
        let viewport = (viewport.0.clamp(1, scene.0), viewport.1.clamp(1, scene.1));
        let drawn_key = (source, viewport, share.map(f32::to_bits));
        if self.written.get() != Some(drawn_key) {
            let rect = super::sub_rectangle_at(source, viewport, scene);
            let bloom_view = self.levels.as_ref().map_or(BloomView::NONE, |levels| {
                let filled = levels.base_filled(share);
                let base = levels.base();
                BloomView {
                    uv: [
                        filled.0 as f32 / base.0 as f32,
                        filled.1 as f32 / base.1 as f32,
                    ],
                    rect: filled,
                    uv_min: [
                        (source.0 as f32 + 0.5) / scene.0 as f32,
                        (source.1 as f32 + 0.5) / scene.1 as f32,
                    ],
                }
            });
            queue.write_buffer(
                &self.uniform,
                0,
                bytemuck::bytes_of(&Uniform::new(self.params, rect, bloom_view)),
            );
            self.written.set(Some(drawn_key));
        }
        draw(
            encoder,
            "omega tonemap luma",
            &self.luma,
            &[&self.sized.luma_group],
            &self.ladder[0].0.view,
            (0.0, 0.0),
            (LADDER_SIZE, LADDER_SIZE),
        );
        for ((target, side), group) in self.ladder.iter().skip(1).zip(&self.halve_groups) {
            draw(
                encoder,
                "omega tonemap halve",
                &self.halve,
                &[group],
                &target.view,
                (0.0, 0.0),
                (*side, *side),
            );
        }
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("omega tonemap coefficients"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.coefficients);
            pass.set_bind_group(0, &self.coefficients_in, &[]);
            pass.set_bind_group(1, &seat.coefficients, &[]);
            pass.dispatch_workgroups(1, 1, 1);
        }
        if let Some(levels) = &self.levels {
            draw(
                encoder,
                "omega bloom bright",
                &self.bright,
                &[&self.sized.apply_group, &seat.curve],
                levels.level_zero(),
                (0.0, 0.0),
                levels.base_filled(share),
            );
            levels.run(queue, encoder, &self.bloom_pipelines, share);
        }
        let drawn = distortion.map(|view| {
            self.distortion_group.get(view, || {
                Self::distortion_group(device, &self.distortion_layout, view)
            })
        });
        draw(
            encoder,
            "omega tonemap apply",
            &self.apply,
            &[
                &self.sized.apply_group,
                &seat.curve,
                drawn.as_ref().unwrap_or(&self.no_distortion),
                &self.bloom_group,
            ],
            view,
            origin,
            viewport,
        );
    }

    /// Runs the chain over one view of a split: the view's own mean luminance and
    /// adaptation (the seat's), the curve applied over the view's rectangle and nothing
    /// outside it. **Call it once per view with its own encoder and submit each before the
    /// next**: the constants are written with the queue, which lands before the submission
    /// that follows. **The per-seat adaptation is this project's, chosen, not measured**:
    /// the original has no split screen. Draws no distortion (that pass reads the depth
    /// attachment, which holds the last view's alone).
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
                seats[seat] = Some(Seat::new(device, &self.state_rw, &self.state_ro));
                self.seat_builds.set(self.seat_builds.get() + 1);
            }
        }
        let seats = self.seats.borrow();
        let (x, y, w, h) = run.rect;
        let scene = self.sized.size;
        let share = [w as f32 / scene.0 as f32, h as f32 / scene.1 as f32].map(|s| s.min(1.0));
        if run.first {
            // A pass that clears and draws nothing: the apply pass loads.
            encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("omega tonemap clear"),
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
            device,
            queue,
            encoder,
            view,
            seats[seat].as_ref().expect("built above"),
            ((x, y), (x as f32, y as f32)),
            (w, h),
            share,
            None,
        );
    }

    /// How many seats hold an adaptation of their own right now.
    #[must_use]
    pub fn seat_lanes(&self) -> usize {
        self.seats.borrow().iter().flatten().count()
    }

    /// Forgets every seat's adaptation, for the first whole-picture frame after a split.
    pub fn retire_seats(&self) {
        self.seats.borrow_mut().clear();
    }

    /// Forgets one seat's adaptation, for a seat that was not drawn as a split view last frame.
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
mod bloom_tests;
#[cfg(test)]
mod split_tests;
#[cfg(test)]
mod tests;
