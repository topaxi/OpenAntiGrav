//! The bloom levels behind the composite's `BloomTexture`: four targets halving from
//! [`law::bloom_base`], each blurred, then added back up, coarse to fine. The bright pass
//! that feeds level 0 lives in `omega_tonemap.wesl`, because it reads the curve.
//!
//! `FUN_0177fbc0` (`docs/ghidra/functions/ps4-omega-eu/tonemap.md`, "The bloom chain").

use super::{Target, draw, law};
use oag_gpu::formats::SCENE_FORMAT;

/// The levels' format, **chosen, not measured**: the executable's are `R11G11B10F`
/// (`0x3ac706`), which is not a render target on every device; alpha is unused either way.
pub const LEVEL_FORMAT: wgpu::TextureFormat = SCENE_FORMAT;

/// One pass's constants, `Step` in `omega_bloom.wesl`.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct StepUniform {
    uv_scale: [f32; 2],
    uv_max: [f32; 2],
    texel: [f32; 2],
    scale: f32,
    _pad: f32,
}

/// The three pixel programs.
#[derive(Debug)]
pub(super) struct Pipelines {
    blur: wgpu::RenderPipeline,
    down: wgpu::RenderPipeline,
    add: wgpu::RenderPipeline,
}

impl Pipelines {
    pub(super) fn new(device: &wgpu::Device, layout: &wgpu::BindGroupLayout) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("omega bloom"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!(concat!(env!("OUT_DIR"), "/omega_bloom.wgsl")).into(),
            ),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("omega bloom"),
            bind_group_layouts: &[Some(layout)],
            immediate_size: 0,
        });
        let render = |entry: &str, blend: Option<wgpu::BlendState>| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(entry),
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
                        format: LEVEL_FORMAT,
                        blend,
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
        let one_plus_one = wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::One,
            dst_factor: wgpu::BlendFactor::One,
            operation: wgpu::BlendOperation::Add,
        };
        Self {
            blur: render("fs_blur", None),
            down: render("fs_down", None),
            add: render(
                "fs_add",
                Some(wgpu::BlendState {
                    color: one_plus_one,
                    alpha: one_plus_one,
                }),
            ),
        }
    }
}

/// Which program a pass runs.
#[derive(Debug, Clone, Copy)]
enum Program {
    Blur,
    Down,
    Add,
}

/// One pass of the chain: a source level, a target and its constants.
#[derive(Debug)]
struct Pass {
    program: Program,
    uniform: wgpu::Buffer,
    group: wgpu::BindGroup,
    /// The level that sets this pass's viewport.
    level: usize,
    /// The source's size, for its share of a split view.
    source: (u32, u32),
    /// One source texel along the axis, as a distance in texels.
    axis: (f32, f32),
    scale: f32,
    target: Slot,
}

/// A level's two targets.
#[derive(Debug, Clone, Copy)]
enum Slot {
    A(usize),
    B(usize),
}

/// The level targets and the passes over them, for one frame shape.
#[derive(Debug)]
pub(super) struct Levels {
    base: (u32, u32),
    a: Vec<Target>,
    b: Vec<Target>,
    passes: Vec<Pass>,
    /// The share the constants were last written for.
    written: std::cell::Cell<Option<[f32; 2]>>,
}

/// The texels of a `size` target a view covering `share` of the frame fills.
fn filled(size: (u32, u32), share: [f32; 2]) -> (u32, u32) {
    let axis = |side: u32, share: f32| ((side as f32 * share).ceil() as u32).clamp(1, side);
    (axis(size.0, share[0]), axis(size.1, share[1]))
}

impl Levels {
    pub(super) fn new(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        sampler: &wgpu::Sampler,
        bloom: &law::Bloom,
        size: (u32, u32),
    ) -> Self {
        let base = law::bloom_base(size);
        let sizes: Vec<_> = (0..law::BLOOM_LEVELS)
            .map(|level| law::bloom_level_size(base, level))
            .collect();
        let target = |label: &str, size: (u32, u32)| Target::new(device, label, LEVEL_FORMAT, size);
        let a: Vec<_> = sizes.iter().map(|s| target("omega bloom a", *s)).collect();
        let b: Vec<_> = sizes.iter().map(|s| target("omega bloom b", *s)).collect();
        let pass = |program, source: Slot, target, level, axis, scale| {
            let (view, size) = match source {
                Slot::A(i) => (&a[i].view, sizes[i]),
                Slot::B(i) => (&b[i].view, sizes[i]),
            };
            let uniform = oag_gpu::init_buffer::init(
                device,
                "omega bloom pass",
                wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                bytemuck::bytes_of(&<StepUniform as bytemuck::Zeroable>::zeroed()),
            );
            let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("omega bloom pass"),
                layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(view),
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
            });
            Pass {
                program,
                uniform,
                group,
                level,
                source: size,
                axis,
                scale,
                target,
            }
        };
        let mut passes = Vec::new();
        for level in 0..law::BLOOM_LEVELS {
            let (program, source) = if level == 0 {
                (Program::Blur, Slot::A(0))
            } else {
                (Program::Down, Slot::A(level - 1))
            };
            passes.push(pass(
                program,
                source,
                Slot::B(level),
                level,
                (1.0, 0.0),
                1.0,
            ));
            passes.push(pass(
                Program::Blur,
                Slot::B(level),
                Slot::A(level),
                level,
                (0.0, 1.0),
                1.0,
            ));
        }
        for level in (1..law::BLOOM_LEVELS).rev() {
            passes.push(pass(
                Program::Add,
                Slot::A(level),
                Slot::A(level - 1),
                level - 1,
                (0.0, 0.0),
                bloom.blend_factors[level],
            ));
        }
        Self {
            base,
            a,
            b,
            passes,
            written: std::cell::Cell::new(None),
        }
    }

    /// Level 0's size.
    pub(super) fn base(&self) -> (u32, u32) {
        self.base
    }

    /// Whether these targets fit a frame of `size`.
    pub(super) fn fits(&self, size: (u32, u32)) -> bool {
        self.base == law::bloom_base(size)
    }

    /// What the bright pass writes and the composite reads.
    pub(super) fn level_zero(&self) -> &wgpu::TextureView {
        &self.a[0].view
    }

    /// Level 0's rectangle for a view filling `share` of the frame.
    pub(super) fn base_filled(&self, share: [f32; 2]) -> (u32, u32) {
        filled(self.base, share)
    }

    /// Blurs and adds up the levels of a view filling `share` of the frame, from the bright
    /// pass already in level 0. A split view fills the corner of each level, not its own
    /// place: it is consumed before the next view starts.
    pub(super) fn run(
        &self,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        pipelines: &Pipelines,
        share: [f32; 2],
    ) {
        if self.written.get() != Some(share) {
            for pass in &self.passes {
                let (w, h) = filled(pass.source, share);
                let (sw, sh) = (pass.source.0 as f32, pass.source.1 as f32);
                let uniform = StepUniform {
                    uv_scale: [w as f32 / sw, h as f32 / sh],
                    uv_max: [(w as f32 - 0.5) / sw, (h as f32 - 0.5) / sh],
                    texel: [pass.axis.0 / sw, pass.axis.1 / sh],
                    scale: pass.scale,
                    _pad: 0.0,
                };
                queue.write_buffer(&pass.uniform, 0, bytemuck::bytes_of(&uniform));
            }
            self.written.set(Some(share));
        }
        for pass in &self.passes {
            let (pipeline, label) = match pass.program {
                Program::Blur => (&pipelines.blur, "omega bloom blur"),
                Program::Down => (&pipelines.down, "omega bloom down"),
                Program::Add => (&pipelines.add, "omega bloom add"),
            };
            let target = match pass.target {
                Slot::A(i) => &self.a[i].view,
                Slot::B(i) => &self.b[i].view,
            };
            let rect = filled(law::bloom_level_size(self.base, pass.level), share);
            draw(
                encoder,
                label,
                pipeline,
                &[&pass.group],
                target,
                (0.0, 0.0),
                rect,
            );
        }
    }
}
