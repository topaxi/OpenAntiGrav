//! The flare's draw: its own pass over the resolved scene, no depth attachment.
//!
//! **Chosen, not measured**: a pass of its own rather than a draw inside the scene pass,
//! because the occlusion test reads the scene's depth, which a pass cannot sample while
//! it is attached. The 16 samples are `textureLoad`s in the vertex stage (sample 0
//! under MSAA), so nothing is read back to the CPU and the GPU never stalls.

use oag_mesh::mesh_render::fragment_options;
use wgpu::util::DeviceExt;

use super::{Atlas, Placement, RINGS};

/// Bytes of the frame uniform: one `vec4` and ten rings of three `vec4`s.
const UNIFORM_LEN: usize = 16 + RINGS * 48;

/// The flare's pipeline and its atlas.
#[derive(Debug)]
pub struct Pipeline {
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    atlas: wgpu::TextureView,
    sampler: wgpu::Sampler,
}

impl Pipeline {
    /// Builds the pipeline for a pass writing `format`; `multisampled` is whether the
    /// depth attachment it reads is. `None` for an atlas that does not fit its levels.
    #[must_use]
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        store: oag_gpu::formats::SceneStore,
        multisampled: bool,
        atlas: &Atlas,
    ) -> Option<Self> {
        let levels = u32::try_from(atlas.levels.len()).ok()?;
        let mut width = atlas.width;
        let mut height = atlas.height;
        for level in &atlas.levels {
            if width == 0 || height == 0 || level.len() != (width * height * 4) as usize {
                return None;
            }
            width = (width / 2).max(1);
            height = (height / 2).max(1);
        }
        let source = include_str!(concat!(env!("OUT_DIR"), "/lens_flare.wgsl"));
        let source = if multisampled {
            source.replace(
                "var depth_tex: texture_2d<f32>;",
                "var depth_tex: texture_multisampled_2d<f32>;",
            )
        } else {
            source.to_owned()
        };
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("lens flare"),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });
        let entry = |binding: u32, visibility, ty| wgpu::BindGroupLayoutEntry {
            binding,
            visibility,
            ty,
            count: None,
        };
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("lens flare"),
            entries: &[
                entry(
                    0,
                    wgpu::ShaderStages::VERTEX,
                    wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                ),
                entry(
                    1,
                    wgpu::ShaderStages::FRAGMENT,
                    wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                ),
                entry(
                    2,
                    wgpu::ShaderStages::FRAGMENT,
                    wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                ),
                entry(
                    3,
                    wgpu::ShaderStages::VERTEX,
                    wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled,
                    },
                ),
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("lens flare"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("lens flare"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    // `BLENDMODE 0xA2`, `FIXB 0xFFFFFF`: `src.rgb * src.a + dst`.
                    blend: Some(crate::psys::BLEND),
                    write_mask: wgpu::ColorWrites::COLOR,
                })],
                compilation_options: fragment_options(format, store),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });
        let texture = device.create_texture_with_data(
            queue,
            &wgpu::TextureDescriptor {
                label: Some("lens flare atlas"),
                size: wgpu::Extent3d {
                    width: atlas.width,
                    height: atlas.height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: levels,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
            wgpu::util::TextureDataOrder::MipMajor,
            &atlas.levels.concat(),
        );
        // Linear magnify, mipmap-linear minify on the four authored mips.
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("lens flare"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            ..Default::default()
        });
        Some(Self {
            pipeline,
            layout,
            atlas: texture.create_view(&wgpu::TextureViewDescriptor::default()),
            sampler,
        })
    }

    /// Records the flare of `placement` over `target`, sampling `depth` for the sun's
    /// occlusion. The pass is clipped to the placement's viewport so a split screen's
    /// flare stays in its own view. A small uniform is made per call: one view's draw
    /// shares nothing with another's.
    pub fn encode(
        &self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        depth: &wgpu::TextureView,
        placement: &Placement,
    ) {
        let mut words: Vec<f32> = Vec::with_capacity(UNIFORM_LEN / 4);
        words.extend([
            placement.sun_pixel[0],
            placement.sun_pixel[1],
            placement.pixel_scale,
            0.0,
        ]);
        for ring in &placement.rings {
            words.extend(ring.centre);
            words.extend(ring.half);
            words.extend(ring.rgba);
            words.extend([f32::from_bits(u32::from(ring.cell)), 0.0, 0.0, 0.0]);
        }
        let uniforms = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("lens flare"),
            contents: bytemuck::cast_slice(&words),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("lens flare"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniforms.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&self.atlas),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(depth),
                },
            ],
        });
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("lens flare"),
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
        let [x, y, w, h] = placement.viewport;
        pass.set_viewport(x, y, w, h, 0.0, 1.0);
        pass.set_scissor_rect(x as u32, y as u32, w as u32, h as u32);
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.draw(0..(RINGS * 6) as u32, 0..1);
    }
}
