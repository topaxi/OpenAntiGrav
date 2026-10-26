//! The static's draw: two textured triangle strips, alpha-over, no depth.

use oag_mesh::mesh_render::{DEPTH_FORMAT, Velocity, fragment_options};

use super::{TEXELS, VERTICES, Vertex};
use crate::exhaust::FlareTexture;

/// What the last upload left to draw, as a value: a split screen records its views at once, so
/// each view keeps its own while a later view's upload refills the pipeline. `draw` draws it.
#[derive(Clone, Copy, Debug, Default)]
pub struct Drawn {
    visible: bool,
}

/// What [`Pipeline::draw`] binds, cloned into a view's recording (`crate::exhaust::Drawer`).
#[derive(Clone, Debug)]
pub struct Drawer {
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    vertices: wgpu::Buffer,
}

/// The static's pipeline, its noise texture and its eight vertices.
#[derive(Debug)]
pub struct Pipeline {
    drawer: Drawer,
    texture: wgpu::Texture,
    /// Whether [`Self::sync_texture`] has written the noise: until it has, the
    /// texture is transparent and an armed node draws nothing.
    synced: bool,
    /// Whether the last [`Pipeline::upload`] had anything to draw.
    visible: bool,
}

impl Pipeline {
    /// Builds the pipeline for a pass writing `format` at `sample_count`, with
    /// the race's velocity target when `velocity` says so.
    #[must_use]
    pub fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        // What the scene target stores - see `oag_gpu::formats::SceneStore`.
        store: oag_gpu::formats::SceneStore,
        sample_count: u32,
        velocity: Velocity,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("cockpit hit"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!(concat!(env!("OUT_DIR"), "/cockpit_hit.wgsl")).into(),
            ),
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("cockpit hit"),
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
            label: Some("cockpit hit"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let mut targets = vec![Some(wgpu::ColorTargetState {
            format,
            // `BlendFunc(ADD, SRC_ALPHA, ONE_MINUS_SRC_ALPHA)`: the smoke's blend class.
            blend: Some(crate::psys::BLEND_ALPHA_OVER),
            // The static writes no glow mask, as the screen flash's draw clears it.
            write_mask: wgpu::ColorWrites::COLOR,
        })];
        targets.extend(velocity.target(true));
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("cockpit hit"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![
                        0 => Float32x2, 1 => Float32x2, 2 => Float32x4
                    ],
                })],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &targets,
                compilation_options: fragment_options(format, store),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                cull_mode: None,
                ..Default::default()
            },
            // Depth test off, as the draw sets it; the attachment is still
            // bound, so the state names it.
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
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("cockpit hit noise"),
            size: wgpu::Extent3d {
                width: TEXELS,
                height: TEXELS,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        // `sceGuTexWrap(GU_REPEAT, GU_REPEAT)`; the filter is chosen, see the module doc.
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("cockpit hit noise"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("cockpit hit"),
            layout: &layout,
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
        });
        let vertices = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("cockpit hit vertices"),
            size: (VERTICES * std::mem::size_of::<Vertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self {
            drawer: Drawer {
                pipeline,
                bind_group,
                vertices,
            },
            texture,
            synced: false,
            visible: false,
        }
    }

    /// Writes the noise the first time it is handed one, and ignores every call
    /// after. A texture that is not [`TEXELS`] square is refused (`false`): the
    /// draw's extents divide by that size, so any other would be sampled wrong.
    pub fn sync_texture(
        &mut self,
        queue: &dyn oag_gpu::writes::Writes,
        noise: &FlareTexture,
    ) -> bool {
        if self.synced {
            return true;
        }
        if noise.width != TEXELS
            || noise.height != TEXELS
            || noise.rgba.len() != (TEXELS * TEXELS * 4) as usize
        {
            return false;
        }
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &noise.rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(TEXELS * 4),
                rows_per_image: Some(TEXELS),
            },
            wgpu::Extent3d {
                width: TEXELS,
                height: TEXELS,
                depth_or_array_layers: 1,
            },
        );
        self.synced = true;
        true
    }

    /// This frame's strips, [`super::Node::strips`]; `None` draws nothing.
    pub fn upload(
        &mut self,
        queue: &dyn oag_gpu::writes::Writes,
        strips: Option<[Vertex; VERTICES]>,
    ) {
        self.visible = self.synced && strips.is_some();
        if let (true, Some(strips)) = (self.visible, strips) {
            queue.write_buffer(&self.drawer.vertices, 0, bytemuck::cast_slice(&strips));
        }
    }

    /// What the last upload left to draw, for [`Self::draw`].
    #[must_use]
    pub fn drawn(&self) -> Drawn {
        Drawn {
            visible: self.visible,
        }
    }

    /// Draws the strips: [`Drawer::draw`].
    pub fn draw(&self, pass: &mut wgpu::RenderPass<'_>, drawn: Drawn) {
        self.drawer.draw(pass, drawn);
    }

    /// The handles [`Self::draw`] binds.
    #[must_use]
    pub fn drawer(&self) -> &Drawer {
        &self.drawer
    }
}

impl Drawer {
    /// Draws both strips into a pass the caller opened, after everything they
    /// should cover.
    pub fn draw(&self, pass: &mut wgpu::RenderPass<'_>, drawn: Drawn) {
        if !drawn.visible {
            return;
        }
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.set_vertex_buffer(0, self.vertices.slice(..));
        pass.draw(0..4, 0..1);
        pass.draw(4..8, 0..1);
    }
}
