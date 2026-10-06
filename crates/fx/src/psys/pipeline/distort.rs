//! The offset target [`super::Pipeline`] draws blend class 8 into.
//!
//! The executable's pass 9 (`FUN_0161d040`, `docs/ghidra/functions/ps4-omega-eu/
//! heat-haze.md`): one colour attachment, **no depth of its own**, cleared to
//! `(0, 0, 0, 0)` every frame, drawn into additively (colour `(One, Add, One)`),
//! and read by the composite as `DistortionTexture`.
//!
//! What differs from the executable, stated rather than hidden:
//!
//! - **Half floats, not `R8G8_SNORM`** ([`FORMAT`]): WebGPU cannot render to
//!   `rg8snorm`. The composite clamps its read to `[-1, 1]`. **Chosen, not
//!   measured.**
//! - **The depth gate is the depth test.** The executable samples an `R16F`
//!   copy of the scene's view-space depth in its fragment and keeps the pixel
//!   when `depth >= w`; this draws against the scene's own depth attachment,
//!   read-only, with `LessEqual`. The same comparison, planar like `w`, at
//!   the buffer's own precision. What the executable's scene shaders store in
//!   its `R16F` (planar `w` or a radial distance) is unread.
//! - **Sized like the scene**, and drawn over the same viewport rectangle.

use oag_mesh::mesh::GpuVertex;

/// The offset target's format: see the module doc.
use oag_gpu::formats::DISTORTION_FORMAT as FORMAT;

/// The most vertices one frame's offset quads may take: `4096` quads. A
/// shock ring plus a heat-haze set is a few dozen.
pub const MAX_VERTICES: usize = 4096 * 6;

/// One target and its resolve, at one size.
#[derive(Debug)]
struct Target {
    size: (u32, u32),
    /// The multisampled attachment, when the scene's depth is multisampled.
    msaa: Option<wgpu::TextureView>,
    resolved_texture: wgpu::Texture,
    resolved: wgpu::TextureView,
}

/// The pipeline, the geometry and the target.
#[derive(Debug)]
pub(super) struct Distort {
    pipeline: wgpu::RenderPipeline,
    vertices: wgpu::Buffer,
    count: u32,
    sample_count: u32,
    target: Option<Target>,
}

impl Distort {
    pub(super) fn new(
        device: &wgpu::Device,
        layout: &wgpu::PipelineLayout,
        sample_count: u32,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("psys distort"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!(concat!(env!("OUT_DIR"), "/distort.wgsl")).into(),
            ),
        });
        let one = wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::One,
            dst_factor: wgpu::BlendFactor::One,
            operation: wgpu::BlendOperation::Add,
        };
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("psys distort"),
            layout: Some(layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<GpuVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x3,
                            offset: std::mem::offset_of!(GpuVertex, position) as u64,
                            shader_location: 0,
                        },
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x4,
                            offset: std::mem::offset_of!(GpuVertex, colour) as u64,
                            shader_location: 2,
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
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: FORMAT,
                    blend: Some(wgpu::BlendState {
                        color: one,
                        alpha: one,
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: oag_mesh::mesh_render::DEPTH_FORMAT,
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::LessEqual),
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
        let vertices = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("psys distort vertices"),
            size: (MAX_VERTICES * std::mem::size_of::<GpuVertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self {
            pipeline,
            vertices,
            count: 0,
            sample_count,
            target: None,
        }
    }

    pub(super) fn upload(&mut self, queue: &wgpu::Queue, vertices: &[GpuVertex]) {
        let n = vertices.len().min(MAX_VERTICES);
        queue.write_buffer(&self.vertices, 0, bytemuck::cast_slice(&vertices[..n]));
        self.count = n as u32;
    }

    fn target(&mut self, device: &wgpu::Device, size: (u32, u32)) -> &Target {
        let size = (size.0.max(1), size.1.max(1));
        if self.target.as_ref().is_none_or(|t| t.size != size) {
            let texture = |label: &str, sample_count: u32, usage: wgpu::TextureUsages| {
                device.create_texture(&wgpu::TextureDescriptor {
                    label: Some(label),
                    size: wgpu::Extent3d {
                        width: size.0,
                        height: size.1,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count,
                    dimension: wgpu::TextureDimension::D2,
                    format: FORMAT,
                    usage,
                    view_formats: &[],
                })
            };
            let resolved_texture = texture(
                "psys distort",
                1,
                wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::COPY_SRC,
            );
            self.target = Some(Target {
                size,
                msaa: (self.sample_count > 1).then(|| {
                    texture(
                        "psys distort msaa",
                        self.sample_count,
                        wgpu::TextureUsages::RENDER_ATTACHMENT,
                    )
                    .create_view(&wgpu::TextureViewDescriptor::default())
                }),
                resolved: resolved_texture.create_view(&wgpu::TextureViewDescriptor::default()),
                resolved_texture,
            });
        }
        self.target.as_ref().expect("built above")
    }

    /// The texture behind the last frame's [`Self::encode`] view, for a readback.
    pub(super) fn texture(&self) -> Option<&wgpu::Texture> {
        self.target.as_ref().map(|t| &t.resolved_texture)
    }

    /// Draws this frame's offsets, or `None` when there are none.
    pub(super) fn encode(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        bind_group: &wgpu::BindGroup,
        depth: &wgpu::TextureView,
        size: (u32, u32),
        viewport: (f32, f32, f32, f32),
    ) -> Option<&wgpu::TextureView> {
        if self.count == 0 {
            return None;
        }
        let count = self.count;
        self.target(device, size);
        let target = self.target.as_ref().expect("built above");
        {
            let (view, resolve_target) = match &target.msaa {
                Some(msaa) => (msaa, Some(&target.resolved)),
                None => (&target.resolved, None),
            };
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("psys distort"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    depth_slice: None,
                    resolve_target,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: if resolve_target.is_some() {
                            wgpu::StoreOp::Discard
                        } else {
                            wgpu::StoreOp::Store
                        },
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: depth,
                    depth_ops: None,
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_viewport(viewport.0, viewport.1, viewport.2, viewport.3, 0.0, 1.0);
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, bind_group, &[]);
            pass.set_vertex_buffer(0, self.vertices.slice(..));
            pass.draw(0..count, 0..1);
        }
        Some(&target.resolved)
    }
}
