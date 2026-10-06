//! The movie pipeline: three 8-bit planes, drawn as one quad where the draw
//! list's [`oag_ui::frontend::Draw::Video`] sits.
//!
//! Moved out of `render.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`, with no behaviour change.

use anyhow::Result;

use super::VideoFormat;
use super::resources::{blank_r8, sampler_entry, texture_entry, uniform_entry};

pub(super) struct Video {
    pub(super) pipeline: wgpu::RenderPipeline,
    pub(super) bind_group: wgpu::BindGroup,
    pub(super) planes: [wgpu::Texture; 3],
    pub(super) format: VideoFormat,
}

impl Video {
    pub(super) fn new(
        device: &wgpu::Device,
        uniform_buffer: &wgpu::Buffer,
        target: wgpu::TextureFormat,
        format: VideoFormat,
    ) -> Result<Self> {
        // Linear filtering here, unlike the glyphs: the frame is being scaled up
        // from 480x272 and nearest would look worse than the original did.
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("planes"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            ..Default::default()
        });

        let planes = [
            blank_r8(device, "plane y", format.width, format.height),
            blank_r8(device, "plane u", format.chroma_width, format.chroma_height),
            blank_r8(device, "plane v", format.chroma_width, format.chroma_height),
        ];
        let views: Vec<wgpu::TextureView> = planes
            .iter()
            .map(|t| t.create_view(&wgpu::TextureViewDescriptor::default()))
            .collect();

        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("video"),
            entries: &[
                uniform_entry(0),
                texture_entry(1),
                texture_entry(2),
                texture_entry(3),
                sampler_entry(4, wgpu::SamplerBindingType::Filtering),
            ],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("video"),
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&views[0]),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&views[1]),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&views[2]),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("video"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!(concat!(env!("OUT_DIR"), "/video.wgsl")).into(),
            ),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("video"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("video"),
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
                targets: &[Some(target.into())],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        Ok(Self {
            pipeline,
            bind_group,
            planes,
            format,
        })
    }
}
