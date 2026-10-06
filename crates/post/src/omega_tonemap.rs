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
//! 3. per pixel and per channel, `y = cubic(clamp(exposure * c, 0, t1))`.
//!
//! What is **not** the disc's, stated rather than hidden:
//!
//! - **The display encode.** The curve's `[0, 1]` is taken as linear light and
//!   encoded with the `pow(1/2.2)` every linear target here ends on
//!   ([ADR-0026](../../../../docs/architecture/adr/0026-hd-authored-lighting-is-linear.md)).
//!   The original's scanout format was not read; that its HDR twin ends at
//!   40.0 (forty times paper white) is why linear is the reading. **Chosen,
//!   not measured.**
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
//! - **Bloom and the low-resolution additive layer** the original adds after
//!   the curve are not drawn here: this title's bloom chain is off in this
//!   renderer, and its particles draw into the scene, so they pass through
//!   the curve.
//! - **Brightness** stays at the executable's middle setting, 1.0; this port
//!   grades brightness after the frame.

pub mod law;

pub use law::{Curve, Params};

use anyhow::Result;

use oag_gpu::formats::SCENE_FORMAT;

/// The luma image's side, the executable's own `0x100` (format `R16F`).
pub const LADDER_SIZE: u32 = 256;

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
}

impl Uniform {
    fn new(p: Params, (uv_scale, uv_max): ([f32; 2], [f32; 2])) -> Self {
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
        }
    }
}

/// `State` in the shader: four words, two `vec4`s and the 256-entry ring.
const STATE_SIZE: u64 = 16 + 16 + 16 + 256 * 4;

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
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
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
    coefficient_groups: [wgpu::BindGroup; 2],
    curve_group: wgpu::BindGroup,
    sized: Sized,
    written: std::cell::Cell<Option<(u32, u32)>>,
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
        let plain = pipeline_layout(&[Some(&layout)]);
        let with_curve = pipeline_layout(&[Some(&layout), Some(&state_ro)]);
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

        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("omega tonemap uniform"),
            size: size_of::<Uniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: true,
        });
        uniform
            .slice(..)
            .get_mapped_range_mut()
            .expect("a freshly mapped buffer maps")
            .copy_from_slice(bytemuck::bytes_of(&Uniform::new(
                params,
                ([1.0, 1.0], [1.0, 1.0]),
            )));
        uniform.unmap();
        // Zeroed: `primed` is 0, so the first dispatch seeds the history.
        let state = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("omega tonemap state"),
            size: STATE_SIZE,
            usage: wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

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
        let state_group = |layout: &wgpu::BindGroupLayout| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("omega tonemap state"),
                layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: state.as_entire_binding(),
                }],
            })
        };
        let coefficient_groups = [
            group("omega tonemap coefficients", last),
            state_group(&state_rw),
        ];
        let curve_group = state_group(&state_ro);
        let sized = Self::sized(device, &layout, &sampler, &uniform, size);
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
            coefficient_groups,
            curve_group,
            sized,
            written: std::cell::Cell::new(None),
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

    /// Rebuilds the scene target for a new viewport size. The adaptation
    /// state carries over: the luminance it tracks is the picture's, not the
    /// target's.
    pub fn resize(&mut self, device: &wgpu::Device, size: (u32, u32)) {
        self.sized = Self::sized(device, &self.layout, &self.sampler, &self.uniform, size);
        self.written.set(None);
    }

    /// Measures the frame, steps the adaptation and writes the curved,
    /// encoded frame into `view` at `origin`, over `viewport` - the rectangle
    /// the race pass drew, exactly as `hd_bloom::Chain::run` takes it.
    pub fn run(
        &self,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        origin: (f32, f32),
        viewport: (u32, u32),
    ) {
        let scene = self.sized.size;
        let viewport = (viewport.0.clamp(1, scene.0), viewport.1.clamp(1, scene.1));
        if self.written.get() != Some(viewport) {
            let rect = super::sub_rectangle(viewport, scene);
            queue.write_buffer(
                &self.uniform,
                0,
                bytemuck::bytes_of(&Uniform::new(self.params, rect)),
            );
            self.written.set(Some(viewport));
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
            pass.set_bind_group(0, &self.coefficient_groups[0], &[]);
            pass.set_bind_group(1, &self.coefficient_groups[1], &[]);
            pass.dispatch_workgroups(1, 1, 1);
        }
        draw(
            encoder,
            "omega tonemap apply",
            &self.apply,
            &[&self.sized.apply_group, &self.curve_group],
            view,
            origin,
            viewport,
        );
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
