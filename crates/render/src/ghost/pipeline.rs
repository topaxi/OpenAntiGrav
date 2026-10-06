//! [`Pipeline::new`]: the three GPU pipelines behind [`super`]'s three
//! passes, each one row of `ghost.md`'s state table.

use oag_fx::exhaust::FlareTexture;
use oag_gpu::formats::VELOCITY_FORMAT;
use oag_mesh::mesh::GpuVertex;
use oag_mesh::mesh_render::{self, DEPTH_FORMAT};

use super::{Pipeline, STRIDE, Uniforms};

/// The cross-fade: `src * k + dst * (1 - k)`, `k` the pass's blend constant.
/// `GU_ADD` with both factors `GU_FIX`, the source fix `k` and the
/// destination `1 - k`, on colour only.
const CROSS_FADE: wgpu::BlendState = wgpu::BlendState {
    color: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::Constant,
        dst_factor: wgpu::BlendFactor::OneMinusConstant,
        operation: wgpu::BlendOperation::Add,
    },
    alpha: wgpu::BlendComponent::REPLACE,
};

/// One pass's state, in the terms `ghost.md` tabulates it.
struct PassState {
    label: &'static str,
    fragment: &'static str,
    depth_write: bool,
    depth_compare: wgpu::CompareFunction,
    colour_writes: wgpu::ColorWrites,
    blend: Option<wgpu::BlendState>,
    /// Whether the velocity target is written - only by the pass that writes
    /// depth, so the two buffers describe one surface.
    velocity: bool,
}

impl Pipeline {
    /// Builds the three passes for a scene target of `format` at
    /// `sample_count`, with the race's velocity attachment.
    ///
    /// `stamps_glow` is whether the target's alpha is Pulse's glow mask: the
    /// depth lay's `BASE` and the third pass's static are both stamps into
    /// it, and a source whose mask is not Pulse's gets neither. `static_glow`
    /// is `staticglow.mip`, decoded; `None` builds no third pass.
    #[must_use]
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        sample_count: u32,
        stamps_glow: bool,
        static_glow: Option<&FlareTexture>,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ghost"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!(concat!(env!("OUT_DIR"), "/ghost.wgsl")).into(),
            ),
        });
        let uniform_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ghost uniforms"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let material_layout = mesh_render::material_bind_group_layout(device);
        let static_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ghost static"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("ghost"),
            bind_group_layouts: &[
                Some(&uniform_layout),
                Some(&material_layout),
                Some(&static_layout),
            ],
            immediate_size: 0,
        });

        let mask = if stamps_glow {
            wgpu::ColorWrites::ALPHA
        } else {
            wgpu::ColorWrites::empty()
        };
        let build = |state: PassState| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(state.label),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: &[Some(wgpu::VertexBufferLayout {
                        array_stride: STRIDE,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        // Position and texture coordinate only: the ghost is
                        // unlit, so the normal and colour are not read.
                        attributes: &[
                            wgpu::VertexAttribute {
                                format: wgpu::VertexFormat::Float32x3,
                                offset: std::mem::offset_of!(GpuVertex, position) as u64,
                                shader_location: 0,
                            },
                            wgpu::VertexAttribute {
                                format: wgpu::VertexFormat::Float32x2,
                                offset: std::mem::offset_of!(GpuVertex, texcoord) as u64,
                                shader_location: 3,
                            },
                        ],
                    })],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some(state.fragment),
                    targets: &[
                        Some(wgpu::ColorTargetState {
                            format,
                            blend: state.blend,
                            write_mask: state.colour_writes,
                        }),
                        Some(wgpu::ColorTargetState {
                            format: VELOCITY_FORMAT,
                            blend: None,
                            write_mask: if state.velocity {
                                wgpu::ColorWrites::ALL
                            } else {
                                wgpu::ColorWrites::empty()
                            },
                        }),
                    ],
                    compilation_options: wgpu::PipelineCompilationOptions {
                        constants: mesh_render::linear_constants(format),
                        ..Default::default()
                    },
                }),
                primitive: wgpu::PrimitiveState {
                    // The original culls all three (`pass_mask & 0x20` clear).
                    // Not here, for `mesh_render`'s own opaque reason - strip
                    // winding is reconstructed - and it costs nothing: the
                    // depth lay keeps the nearest surface whatever its facing,
                    // and the two `EQUAL` passes see only that surface.
                    cull_mode: None,
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: DEPTH_FORMAT,
                    depth_write_enabled: Some(state.depth_write),
                    depth_compare: Some(state.depth_compare),
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: wgpu::MultisampleState {
                    count: sample_count,
                    ..Default::default()
                },
                multiview_mask: None,
                cache: None,
            })
        };

        let depth = build(PassState {
            label: "ghost depth lay",
            fragment: "fs_depth",
            depth_write: true,
            depth_compare: wgpu::CompareFunction::Less,
            colour_writes: mask,
            blend: None,
            velocity: true,
        });
        let hull = build(PassState {
            label: "ghost hull",
            fragment: "fs_hull",
            depth_write: false,
            depth_compare: wgpu::CompareFunction::Equal,
            colour_writes: wgpu::ColorWrites::COLOR,
            blend: Some(CROSS_FADE),
            velocity: false,
        });
        let stamp = (stamps_glow && static_glow.is_some()).then(|| {
            build(PassState {
                label: "ghost static",
                fragment: "fs_static",
                depth_write: false,
                depth_compare: wgpu::CompareFunction::Equal,
                colour_writes: wgpu::ColorWrites::ALPHA,
                blend: None,
                velocity: false,
            })
        });

        let uniforms = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ghost uniforms"),
            size: std::mem::size_of::<Uniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let uniform_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ghost uniforms"),
            layout: &uniform_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniforms.as_entire_binding(),
            }],
        });
        let transparent = FlareTexture {
            width: 1,
            height: 1,
            rgba: vec![0; 4],
        };
        let static_bind = bind_static(
            device,
            queue,
            &static_layout,
            static_glow.unwrap_or(&transparent),
        );
        Self {
            depth,
            hull,
            stamp,
            uniforms,
            uniform_bind,
            static_bind,
            stamps_glow,
        }
    }
}

/// Uploads the static texture, wrapping on both axes (`Gu_TexWrap(REPEAT,
/// REPEAT)`) and point-sampled: the original's is an 8 bpp palette read at
/// texel resolution, and a filtered static would soften the edge the
/// `GREATER 0x80` test cuts.
fn bind_static(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
    texture: &FlareTexture,
) -> wgpu::BindGroup {
    let (width, height) = (texture.width.max(1), texture.height.max(1));
    let size = wgpu::Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    };
    let gpu = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("ghost static"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &gpu,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &texture.rgba,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(width * 4),
            rows_per_image: Some(height),
        },
        size,
    );
    let view = gpu.create_view(&wgpu::TextureViewDescriptor::default());
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("ghost static"),
        address_mode_u: wgpu::AddressMode::Repeat,
        address_mode_v: wgpu::AddressMode::Repeat,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        ..Default::default()
    });
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("ghost static"),
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
        ],
    })
}
