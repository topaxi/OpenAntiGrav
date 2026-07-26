//! Rasterises a [`Draw`] list with wgpu.
//!
//! Two pipelines and nothing else: one for colour-modulated quads out of the
//! glyph atlas, which covers both text and solid fills, and one for a movie
//! frame out of three 8-bit planes. Everything is positioned in the PSP's
//! 480x272 screen space and letterboxed into whatever the window is, so the XML's
//! own coordinates can be used untouched.
//!
//! The same code path serves the window and the headless screenshot. That is
//! deliberate: a screenshot that went through a different pipeline would not
//! prove anything about what the window shows.

use anyhow::{Context, Result};

use crate::font::{self, Atlas};
use crate::frontend::{Align, Draw, SCREEN};

/// Shared with both shaders.
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniforms {
    viewport: [f32; 2],
    screen: [f32; 2],
    atlas: [f32; 2],
    padding: [f32; 2],
}

/// One quad.
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Quad {
    rect: [f32; 4],
    uv: [f32; 4],
    color: [f32; 4],
}

/// How many quads the instance buffer holds before it is grown.
const INITIAL_QUADS: usize = 1024;

/// A movie's plane geometry, so textures can be sized once.
#[derive(Debug, Clone, Copy)]
pub struct VideoFormat {
    /// Luma width.
    pub width: u32,
    /// Luma height.
    pub height: u32,
    /// Chroma width.
    pub chroma_width: u32,
    /// Chroma height.
    pub chroma_height: u32,
}

/// The renderer.
pub struct Renderer {
    ui_pipeline: wgpu::RenderPipeline,
    ui_bind_group: wgpu::BindGroup,
    uniform_buffer: wgpu::Buffer,
    quad_buffer: wgpu::Buffer,
    quad_capacity: usize,
    atlas: Atlas,
    video: Option<Video>,
    quads: Vec<Quad>,
}

impl std::fmt::Debug for Renderer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Renderer")
            .field("quad_capacity", &self.quad_capacity)
            .field("has_video", &self.video.is_some())
            .finish()
    }
}

struct Video {
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    planes: [wgpu::Texture; 3],
    format: VideoFormat,
}

impl Renderer {
    /// Builds the pipelines for a target of `format`.
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        video: Option<VideoFormat>,
    ) -> Result<Self> {
        let atlas = Atlas::build();

        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("oag-game uniforms"),
            size: std::mem::size_of::<Uniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let quad_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("oag-game quads"),
            size: (INITIAL_QUADS * std::mem::size_of::<Quad>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Nearest filtering: the glyphs are 5x7, and smoothing them would turn
        // crisp pixels into mush at the scales the menu uses.
        let atlas_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("atlas"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let atlas_texture = upload_r8(
            device,
            queue,
            "glyph atlas",
            atlas.width,
            atlas.height,
            &atlas.coverage,
        );
        let atlas_view = atlas_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let ui_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ui"),
            entries: &[
                uniform_entry(0),
                texture_entry(1),
                sampler_entry(2, wgpu::SamplerBindingType::NonFiltering),
            ],
        });

        let ui_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ui"),
            layout: &ui_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&atlas_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&atlas_sampler),
                },
            ],
        });

        let ui_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ui"),
            source: wgpu::ShaderSource::Wgsl(include_str!("ui.wgsl").into()),
        });

        let ui_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("ui"),
            bind_group_layouts: &[Some(&ui_layout)],
            immediate_size: 0,
        });

        let ui_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("ui"),
            layout: Some(&ui_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &ui_shader,
                entry_point: Some("vs_main"),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Quad>() as u64,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &wgpu::vertex_attr_array![
                        0 => Float32x4,
                        1 => Float32x4,
                        2 => Float32x4
                    ],
                })],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &ui_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let video = video
            .map(|format_info| Video::new(device, &uniform_buffer, format, format_info))
            .transpose()?;

        Ok(Self {
            ui_pipeline,
            ui_bind_group,
            uniform_buffer,
            quad_buffer,
            quad_capacity: INITIAL_QUADS,
            atlas,
            video,
            quads: Vec::new(),
        })
    }

    /// The plane geometry the video pipeline was built for.
    #[must_use]
    pub fn video_format(&self) -> Option<VideoFormat> {
        self.video.as_ref().map(|v| v.format)
    }

    /// Uploads one frame of yuv420p, laid out as three consecutive planes.
    pub fn upload_frame(&self, queue: &wgpu::Queue, frame: &[u8]) -> Result<()> {
        let video = self.video.as_ref().context("no video pipeline")?;
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

    /// Draws `list` into `view`.
    pub fn render(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        list: &[Draw],
        target: (u32, u32),
    ) {
        queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::bytes_of(&Uniforms {
                viewport: letterbox(target),
                screen: [SCREEN.0, SCREEN.1],
                atlas: [self.atlas.width as f32, self.atlas.height as f32],
                padding: [0.0, 0.0],
            }),
        );

        self.quads.clear();
        // Where the movie sits in the quad order. The list is painted back to
        // front, and the `LogoFMV` screen puts a black `Image` *behind* its
        // `Movie`, so drawing the movie first unconditionally would let that
        // black backdrop paint straight over the picture.
        let mut video_at = None;
        for draw in list {
            match draw {
                Draw::Fill { rect, color } => self.push_solid(*rect, *color),
                Draw::Video { .. } => video_at = Some(self.quads.len() as u32),
                Draw::Text {
                    x,
                    y,
                    scale,
                    color,
                    align,
                    text,
                } => self.push_text(*x, *y, *scale, *color, *align, text),
            }
        }

        if self.quads.len() > self.quad_capacity {
            self.quad_capacity = self.quads.len().next_power_of_two();
            self.quad_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("oag-game quads"),
                size: (self.quad_capacity * std::mem::size_of::<Quad>()) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("frame"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });

        let total = self.quads.len() as u32;
        let split = video_at.unwrap_or(total);

        // Quads behind the movie, the movie, then quads in front of it. Three
        // draws rather than two so the list's own order is honoured.
        if !self.quads.is_empty() {
            queue.write_buffer(&self.quad_buffer, 0, bytemuck::cast_slice(&self.quads));
            pass.set_pipeline(&self.ui_pipeline);
            pass.set_bind_group(0, &self.ui_bind_group, &[]);
            pass.set_vertex_buffer(0, self.quad_buffer.slice(..));
            if split > 0 {
                pass.draw(0..6, 0..split);
            }
        }

        if video_at.is_some()
            && let Some(video) = &self.video
        {
            pass.set_pipeline(&video.pipeline);
            pass.set_bind_group(0, &video.bind_group, &[]);
            pass.draw(0..6, 0..1);
        }

        if split < total {
            pass.set_pipeline(&self.ui_pipeline);
            pass.set_bind_group(0, &self.ui_bind_group, &[]);
            pass.set_vertex_buffer(0, self.quad_buffer.slice(..));
            pass.draw(0..6, split..total);
        }
    }

    fn push_solid(&mut self, rect: [f32; 4], color: [f32; 4]) {
        let solid = self.atlas.solid;
        self.quads.push(Quad {
            rect,
            // A single texel, sampled with nearest filtering, so the whole quad
            // reads full coverage.
            uv: [solid.x as f32 + 0.5, solid.y as f32 + 0.5, 0.0, 0.0],
            color,
        });
    }

    fn push_text(&mut self, x: f32, y: f32, scale: f32, color: [f32; 4], align: Align, text: &str) {
        let width = font::measure(&self.atlas, text) * scale;
        let mut pen = match align {
            Align::Left => x,
            Align::Centre => x - width / 2.0,
            Align::Right => x - width,
        };
        let advance = (font::GLYPH_WIDTH + 1) as f32 * scale;

        for ch in text.chars() {
            let Some(cell) = self.atlas.cell(ch) else {
                continue;
            };
            self.quads.push(Quad {
                rect: [
                    pen,
                    y,
                    font::GLYPH_WIDTH as f32 * scale,
                    font::GLYPH_HEIGHT as f32 * scale,
                ],
                uv: [
                    cell.x as f32,
                    cell.y as f32,
                    font::GLYPH_WIDTH as f32,
                    font::GLYPH_HEIGHT as f32,
                ],
                color,
            });
            pen += advance;
        }
    }
}

impl Video {
    fn new(
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
            source: wgpu::ShaderSource::Wgsl(include_str!("video.wgsl").into()),
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

fn uniform_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

fn texture_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        count: None,
    }
}

fn sampler_entry(binding: u32, kind: wgpu::SamplerBindingType) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Sampler(kind),
        count: None,
    }
}

fn blank_r8(device: &wgpu::Device, label: &str, width: u32, height: u32) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width: width.max(1),
            height: height.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::R8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    })
}

fn upload_r8(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    label: &str,
    width: u32,
    height: u32,
    bytes: &[u8],
) -> wgpu::Texture {
    let texture = blank_r8(device, label, width, height);
    queue.write_texture(
        texture.as_image_copy(),
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
    texture
}

/// Scale that fits 480x272 inside `target` without distorting it.
#[must_use]
pub fn letterbox(target: (u32, u32)) -> [f32; 2] {
    let (width, height) = (target.0.max(1) as f32, target.1.max(1) as f32);
    let target_aspect = width / height;
    let screen_aspect = SCREEN.0 / SCREEN.1;
    if target_aspect > screen_aspect {
        [screen_aspect / target_aspect, 1.0]
    } else {
        [1.0, target_aspect / screen_aspect]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_matching_aspect_ratio_needs_no_letterboxing() {
        let scale = letterbox((960, 544));
        assert!((scale[0] - 1.0).abs() < 1e-6, "{scale:?}");
        assert!((scale[1] - 1.0).abs() < 1e-6, "{scale:?}");
    }

    #[test]
    fn a_wide_window_shrinks_horizontally() {
        let scale = letterbox((1920, 544));
        assert!(scale[0] < 1.0);
        assert!((scale[1] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn a_tall_window_shrinks_vertically() {
        let scale = letterbox((480, 1000));
        assert!((scale[0] - 1.0).abs() < 1e-6);
        assert!(scale[1] < 1.0);
    }

    #[test]
    fn a_zero_sized_window_does_not_divide_by_zero() {
        let scale = letterbox((0, 0));
        assert!(scale[0].is_finite() && scale[1].is_finite());
    }
}
