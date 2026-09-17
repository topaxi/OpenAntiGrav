//! [`Pipeline`]: the GPU half of [`super`] - a single additive-blended
//! triangle-strip pipeline over the disc's own ribbon texture.
//!
//! Deliberately the smallest pipeline in this crate: one texture, one blend,
//! one draw call, modelled on [`crate::exhaust::Pipeline`]'s shape but with
//! none of that pipeline's trail/HD-dual-texture complexity, because the
//! ribbon has neither. Its own texture upload is hand-rolled rather than
//! reusing [`crate::exhaust::FlareTexture::bind`] - that method is
//! `pub(super)` to the `exhaust` module, and widening it for one caller here
//! is not worth the merge risk while another lane is also touching
//! `crates/render/src/`.

use crate::exhaust::FlareTexture;
use crate::mesh::GpuVertex;

use super::MAX_VERTICES;

#[derive(Debug)]
pub struct Pipeline {
    pipeline: wgpu::RenderPipeline,
    uniforms: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    texture: wgpu::BindGroup,
    vertices: wgpu::Buffer,
    /// Vertices actually uploaded by the last [`Pipeline::upload`].
    count: u32,
}

impl Pipeline {
    /// Builds the pipeline and uploads the ribbon texture.
    ///
    /// `texture` is RGBA8, normally
    /// `Data\Weapons\Textures\pulse_leechbeam1_ADD.mip` decoded by
    /// `oag_texture::texture`. `format` must match the caller's render pass
    /// and `sample_count` its multisample state - see `mesh_render::build`.
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        texture: &FlareTexture,
        sample_count: u32,
        velocity: crate::mesh_render::Velocity,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("beam"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../beam.wgsl").into()),
        });

        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("beam uniforms"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let texture_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("beam texture"),
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
            label: Some("beam"),
            bind_group_layouts: &[Some(&layout), Some(&texture_layout)],
            immediate_size: 0,
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("beam"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<GpuVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![
                        0 => Float32x3, 1 => Float32x3, 2 => Float32x4, 3 => Float32x2,
                        4 => Float32
                    ],
                })],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                // No frame-to-frame vertex correspondence - rebuilt from
                // scratch every draw - so the velocity target, when present,
                // is write-masked empty, the same as `exhaust`'s flare quad.
                targets: &{
                    let mut targets = vec![Some(wgpu::ColorTargetState {
                        format,
                        blend: Some(crate::exhaust::BLEND),
                        write_mask: wgpu::ColorWrites::COLOR,
                    })];
                    targets.extend(velocity.target(true));
                    targets
                },
                compilation_options: wgpu::PipelineCompilationOptions {
                    constants: crate::mesh_render::linear_constants(format),
                    ..Default::default()
                },
            }),
            primitive: wgpu::PrimitiveState {
                // The recovered draw is one `GU_TRIANGLE_STRIP`.
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                // The ribbon has no meaningful winding - it crosses two
                // strips through each other and is meant to read from any
                // angle, the same reasoning `exhaust`'s camera-facing quad
                // uses for its own cull-off.
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: crate::mesh_render::DEPTH_FORMAT,
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::Less),
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

        let uniforms = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("beam uniforms"),
            size: crate::mesh_render::UNIFORMS_SIZE,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("beam uniforms"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniforms.as_entire_binding(),
            }],
        });

        let vertices = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("beam vertices"),
            size: (MAX_VERTICES * std::mem::size_of::<GpuVertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let texture = bind_texture(device, queue, &texture_layout, texture);

        Self {
            pipeline,
            uniforms,
            bind_group,
            texture,
            vertices,
            count: 0,
        }
    }

    /// Uploads this frame's camera matrix and geometry.
    ///
    /// The geometry arrives already in world space - see [`super::build`] -
    /// so the uniform block's `model` half is left at identity, mirroring
    /// `LeachBeam_SubmitStrip`'s own `Gu_SetMatrix` reset right before its
    /// draw.
    pub fn upload(
        &mut self,
        queue: &wgpu::Queue,
        view_projection: &[[f32; 4]; 4],
        vertices: &[GpuVertex],
    ) {
        let mut block = [[0.0f32; 4]; 8];
        block[..4].copy_from_slice(view_projection);
        block[4] = [1.0, 0.0, 0.0, 0.0];
        block[5] = [0.0, 1.0, 0.0, 0.0];
        block[6] = [0.0, 0.0, 1.0, 0.0];
        block[7] = [0.0, 0.0, 0.0, 1.0];
        queue.write_buffer(&self.uniforms, 0, bytemuck::cast_slice(&block));

        let n = vertices.len().min(MAX_VERTICES);
        if n > 0 {
            queue.write_buffer(&self.vertices, 0, bytemuck::cast_slice(&vertices[..n]));
        }
        self.count = n as u32;
    }

    /// Draws into a pass the caller already opened, after the opaque
    /// geometry - depth write is off, so it relies on the scene's depth
    /// already being present to be occluded by it.
    pub fn draw(&self, pass: &mut wgpu::RenderPass<'_>) {
        if self.count == 0 {
            return;
        }
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.set_bind_group(1, &self.texture, &[]);
        pass.set_vertex_buffer(0, self.vertices.slice(..));
        pass.draw(0..self.count, 0..1);
    }
}

/// Uploads `texture` and binds it plus a filtering, repeat-wrapped sampler -
/// `Gu_TexWrap(0, 0)` in `LeachBeam_SubmitStrip` is repeat on both axes.
///
/// Hand-rolled rather than [`FlareTexture::bind`] - see the module doc
/// comment for why.
fn bind_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
    texture: &FlareTexture,
) -> wgpu::BindGroup {
    let (width, height) = (texture.width.max(1), texture.height.max(1));
    let gpu_texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("beam texture"),
        size: wgpu::Extent3d {
            width,
            height,
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
            texture: &gpu_texture,
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
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    let view = gpu_texture.create_view(&wgpu::TextureViewDescriptor::default());
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("beam texture"),
        address_mode_u: wgpu::AddressMode::Repeat,
        address_mode_v: wgpu::AddressMode::Repeat,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    });
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("beam texture"),
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
