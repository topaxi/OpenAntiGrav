//! The movie pipeline: three 8-bit planes, drawn as one quad where the draw
//! list's [`oag_ui::frontend::Draw::Video`] sits.
//!
//! Moved out of `render.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`, with no behaviour change.

use anyhow::{Context, Result};

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

/// Writes `frame`'s three planes into `video`'s textures.
///
/// Slices by `video`'s own geometry, so a frame of another shape is a garbled
/// picture and not an error: the caller sizes the planes to the clip.
pub(super) fn upload_planes(
    video: &Video,
    queue: &wgpu::Queue,
    frame: &crate::movie::VideoFrame,
) -> Result<()> {
    anyhow::ensure!(
        frame.format == crate::movie::PixelFormat::I420,
        "upload_frame only knows I420 today, got {:?}",
        frame.format
    );
    let luma = (video.format.width * video.format.height) as usize;
    let chroma = (video.format.chroma_width * video.format.chroma_height) as usize;

    let planes = [
        (0usize, 0..luma, video.format.width, video.format.height),
        (
            1,
            luma..luma + chroma,
            video.format.chroma_width,
            video.format.chroma_height,
        ),
        (
            2,
            luma + chroma..luma + 2 * chroma,
            video.format.chroma_width,
            video.format.chroma_height,
        ),
    ];

    for (index, range, width, height) in planes {
        let bytes = frame
            .bytes
            .get(range)
            .context("frame is shorter than its declared planes")?;
        queue.write_texture(
            video.planes[index].as_image_copy(),
            bytes,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
    }
    Ok(())
}
