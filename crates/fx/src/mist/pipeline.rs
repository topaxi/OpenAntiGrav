//! The mist's draw: two textured screen quads, additive, colour only.

use crate::exhaust::FlareTexture;
use oag_mesh::mesh_render::{DEPTH_FORMAT, Velocity, fragment_options};

/// Two quads of two triangles each.
const VERTICES: usize = 12;

/// One vertex: clip-space position, texture coordinate, the layer's alpha.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 2],
    pub texcoord: [f32; 2],
    pub alpha: f32,
}

/// The two quads as triangles, from [`super::Layers::quads`] and
/// [`super::Layers::alphas`]: strip order `0 1 2 3` is the triangles
/// `0 1 2` and `2 1 3`.
#[must_use]
pub fn vertices(quads: &[[[f32; 2]; 4]; 2], alphas: [f32; 2]) -> [Vertex; VERTICES] {
    let mut out = [Vertex::default(); VERTICES];
    for (layer, quad) in quads.iter().enumerate() {
        for (slot, corner) in [0, 1, 2, 2, 1, 3].into_iter().enumerate() {
            out[layer * 6 + slot] = Vertex {
                position: super::CORNERS[corner],
                texcoord: quad[corner],
                alpha: alphas[layer],
            };
        }
    }
    out
}

/// The mist pipeline, its texture and its twelve vertices.
#[derive(Debug)]
pub struct Pipeline {
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    vertices: wgpu::Buffer,
    /// Whether the last [`Pipeline::upload`] had anything to draw.
    visible: bool,
}

impl Pipeline {
    /// Builds the pipeline for a pass writing `format` at `sample_count`, with
    /// `texture` (`Data\Tex\ScreenFX\Mist.mip`, decoded) bound with repeat
    /// wrap and a bilinear filter, as `WeatherMist_Draw` and `Gfx_Init` leave
    /// the GE.
    #[must_use]
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        texture: &FlareTexture,
        sample_count: u32,
        velocity: Velocity,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("mist"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!(concat!(env!("OUT_DIR"), "/mist.wgsl")).into(),
            ),
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("mist"),
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
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("mist"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let mut targets = vec![Some(wgpu::ColorTargetState {
            format,
            // `Gu_BlendFunc(ADD, SRC_ALPHA, FIX, 0, 0xffffff)`: additive.
            blend: Some(crate::psys::BLEND),
            // `Bloom_SetGlowMaskWritable(g_bloom, 0)` before the draw.
            write_mask: wgpu::ColorWrites::COLOR,
        })];
        targets.extend(velocity.target(true));
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("mist"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![
                        0 => Float32x2, 1 => Float32x2, 2 => Float32
                    ],
                })],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &targets,
                compilation_options: fragment_options(format),
            }),
            // Culling off (`Gu_Disable(GU_CULL_FACE)`).
            primitive: wgpu::PrimitiveState::default(),
            // Depth test off; the attachment is still bound, so the state
            // names it.
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::Always),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: sample_count,
                ..Default::default()
            },
            multiview_mask: None,
            cache: None,
        });
        let bind_group = bind_texture(device, queue, &layout, texture);
        let vertices = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("mist vertices"),
            size: (VERTICES * std::mem::size_of::<Vertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self {
            pipeline,
            bind_group,
            vertices,
            visible: false,
        }
    }

    /// This frame's quads; `None` draws nothing, as `WeatherMist_Draw` does
    /// while the opacity is not above zero.
    pub fn upload(&mut self, queue: &wgpu::Queue, vertices: Option<&[Vertex; VERTICES]>) {
        self.visible = vertices.is_some();
        if let Some(vertices) = vertices {
            queue.write_buffer(&self.vertices, 0, bytemuck::cast_slice(vertices));
        }
    }

    /// Draws into a pass the caller opened, after the world and before the
    /// HUD.
    pub fn draw(&self, pass: &mut wgpu::RenderPass<'_>) {
        if !self.visible {
            return;
        }
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.set_vertex_buffer(0, self.vertices.slice(..));
        pass.draw(0..VERTICES as u32, 0..1);
    }
}

/// Uploads `texture` with repeat wrap (`Gu_TexWrap(GU_REPEAT, GU_REPEAT)`)
/// and bilinear filtering (`Gu_TexFilter`, the only one in the binary).
fn bind_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
    texture: &FlareTexture,
) -> wgpu::BindGroup {
    let size = wgpu::Extent3d {
        width: texture.width.max(1),
        height: texture.height.max(1),
        depth_or_array_layers: 1,
    };
    let gpu_texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("mist texture"),
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
            texture: &gpu_texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &texture.rgba,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(size.width * 4),
            rows_per_image: Some(size.height),
        },
        size,
    );
    let view = gpu_texture.create_view(&wgpu::TextureViewDescriptor::default());
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("mist texture"),
        address_mode_u: wgpu::AddressMode::Repeat,
        address_mode_v: wgpu::AddressMode::Repeat,
        address_mode_w: wgpu::AddressMode::Repeat,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    });
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("mist texture"),
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
