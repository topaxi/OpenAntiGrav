//! Renders a decoded model, offscreen or into a surface.
//!
//! Deliberately plain: depth buffer, perspective camera, two-light rig. The
//! point is to see the geometry clearly and catch decoding errors, not to
//! reproduce Pulse's look.

use anyhow::{Context, Result};
use oag_core::math::{Mat4, Vec3, camera};
use std::path::Path;

use crate::mesh::{GpuVertex, Model};

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniforms {
    view_projection: [[f32; 4]; 4],
    model: [[f32; 4]; 4],
    anim_phase: f32,
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
}

/// Builds the camera and model matrices for a given orbit angle.
///
/// The model is framed from its own bounding sphere, so any model fills the
/// view regardless of the scale baked into the file. That also means a wrong
/// scale looks *right* here, so this is not a check on the scale factor; the
/// bounding-box assertion in `oag-formats` is.
///
/// `zoom` scales the orbit distance; 1.0 is the default framing described above.
///
/// `anim_phase` is the blink-light palette scroll offset - see
/// `oag_render::mesh::GpuVertex::glow` - in the texture's own V (row) units.
/// Callers with no game clock (this crate's own viewer and capture paths)
/// pass `0.0`, which shows every blink light at its authored, unanimated row.
fn matrices(
    model: &Model,
    aspect: f32,
    yaw: f32,
    pitch: f32,
    zoom: f32,
    anim_phase: f32,
) -> Uniforms {
    let distance = model.radius * 3.0 * zoom;
    let eye = Vec3::new(
        distance * yaw.cos() * pitch.cos(),
        distance * pitch.sin(),
        distance * yaw.sin() * pitch.cos(),
    );

    let projection = camera::perspective(45f32.to_radians(), aspect, 0.01, distance * 10.0);
    let view = camera::look_at(eye, Vec3::ZERO, Vec3::Y);
    let centre = Vec3::from_array(model.centre);

    Uniforms {
        view_projection: (projection * view).to_cols_array_2d(),
        model: Mat4::from_translation(-centre).to_cols_array_2d(),
        anim_phase,
        _pad0: 0.0,
        _pad1: 0.0,
        _pad2: 0.0,
    }
}

/// The depth format `build`'s pipeline is fixed to; a caller's own depth
/// texture must match it.
pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

/// Recomputes the camera and model matrices and uploads them to `buffer`.
///
/// Returns the view-projection matrix it just wrote, so a caller can build a
/// [`oag_core::math::frustum::Frustum`] from the same camera without
/// recomputing it.
#[allow(clippy::too_many_arguments)]
pub fn write_uniforms(
    queue: &wgpu::Queue,
    buffer: &wgpu::Buffer,
    model: &Model,
    aspect: f32,
    yaw: f32,
    pitch: f32,
    zoom: f32,
    anim_phase: f32,
) -> Mat4 {
    let uniforms = matrices(model, aspect, yaw, pitch, zoom, anim_phase);
    queue.write_buffer(buffer, 0, bytemuck::bytes_of(&uniforms));
    Mat4::from_cols_array_2d(&uniforms.view_projection)
}

/// Size, in bytes, of the uniform buffer `write_uniforms` expects.
pub const UNIFORMS_SIZE: u64 = std::mem::size_of::<Uniforms>() as u64;

/// Renders one frame of `model` to a PNG from a given orbit angle.
///
/// `pitch` near zero looks along the ground; near `PI / 2` looks straight down,
/// which is what a track wants and a model does not.
pub fn capture_from(
    model: &Model,
    path: &Path,
    width: u32,
    height: u32,
    yaw: f32,
    pitch: f32,
    anisotropy: Anisotropy,
) -> Result<()> {
    let instance = wgpu::Instance::default();
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        compatible_surface: None,
        ..Default::default()
    }))
    .context("no GPU adapter available")?;
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("oag-view mesh"),
        ..Default::default()
    }))
    .context("requesting the device")?;

    // **sRGB, so the hardware encodes on write exactly as the window's surface
    // does**, and the bytes copied out are already the ones a PNG wants.
    //
    // The tempting alternative - a plain `Rgba8Unorm` target, on the grounds
    // that a readback going straight into a PNG must not be gamma-encoded twice
    // - is what this was, and it was wrong here. It is right only where the
    // shader writes a value that is already sRGB, which is true of the front
    // end's text and sprites and false of anything lit: `mesh.wgsl` multiplies
    // a *linear* texel by the light rig, so an unencoded write stores linear
    // light, and the capture comes out darker than the window it is supposed to
    // match. Measured on the Feisar livery against the same view rendered both
    // ways: the two disagree by a mean of ~2.4/255 and up to 57/255, the error
    // being largest exactly where the rig is furthest from 1.0.
    let format = wgpu::TextureFormat::Rgba8UnormSrgb;
    let size = wgpu::Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    };

    let colour = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("colour"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let depth = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("depth"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: DEPTH_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });

    let Built {
        pipeline,
        alpha_test_pipeline,
        blend_pipeline,
        bind_group,
        vertex_buffer,
        index_buffer,
        texture_binds,
    } = build(&device, &queue, model, format, anisotropy, 1, Depth::Scene)?;

    let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("uniforms"),
        size: UNIFORMS_SIZE,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    write_uniforms(
        &queue,
        &uniform_buffer,
        model,
        width as f32 / height as f32,
        yaw,
        pitch,
        1.0,
        0.0,
    );

    // The bind group must reference the buffer we just filled.
    let bind_group_layout = pipeline.get_bind_group_layout(0);
    let bind_group = {
        let _ = bind_group;
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("uniforms"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        })
    };

    let colour_view = colour.create_view(&wgpu::TextureViewDescriptor::default());
    let depth_view = depth.create_view(&wgpu::TextureViewDescriptor::default());

    let unpadded = width as usize * 4;
    let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize;
    let padded = unpadded.div_ceil(align) * align;
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("readback"),
        size: (padded * height as usize) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("mesh"),
    });
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("mesh"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &colour_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.06,
                        g: 0.07,
                        b: 0.09,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Discard,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.set_vertex_buffer(0, vertex_buffer.slice(..));
        pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);

        // One draw per material run. Slot 0 is the white fallback, so a texture
        // index of n binds slot n + 1.
        for draw in &model.draws {
            let slot = draw.texture.map_or(0, |t| t + 1);
            pass.set_bind_group(1, &texture_binds[slot.min(texture_binds.len() - 1)], &[]);
            pass.draw_indexed(draw.range.clone(), 0, 0..1);
        }

        // Second pipeline, same pass: `Model::alpha_tested_draws` wants a
        // cutout, not a hardcoded alpha of 1.0 - see `fs_main_alpha_test`.
        pass.set_pipeline(&alpha_test_pipeline);
        for draw in &model.alpha_tested_draws {
            let slot = draw.texture.map_or(0, |t| t + 1);
            pass.set_bind_group(1, &texture_binds[slot.min(texture_binds.len() - 1)], &[]);
            pass.draw_indexed(draw.range.clone(), 0, 0..1);
        }

        // Third pipeline, same pass: `Model::transparent_draws` is meant to be
        // blended rather than replace, and drawn last so opaque and cutout
        // depth is already resolved. See `crate::exhaust::Pipeline` for the
        // precedent of pairing an opaque and a blended pipeline this way.
        pass.set_pipeline(&blend_pipeline);
        for draw in &model.transparent_draws {
            let slot = draw.texture.map_or(0, |t| t + 1);
            pass.set_bind_group(1, &texture_binds[slot.min(texture_binds.len() - 1)], &[]);
            pass.draw_indexed(draw.range.clone(), 0, 0..1);
        }
    }
    encoder.copy_texture_to_buffer(
        colour.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded as u32),
                rows_per_image: Some(height),
            },
        },
        size,
    );
    queue.submit(Some(encoder.finish()));

    let slice = readback.slice(..);
    slice.map_async(wgpu::MapMode::Read, |_| {});
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .context("waiting for the GPU")?;
    let mapped = slice.get_mapped_range().context("mapping readback")?;
    let mut pixels = Vec::with_capacity(unpadded * height as usize);
    for row in mapped.chunks(padded).take(height as usize) {
        pixels.extend_from_slice(&row[..unpadded]);
    }
    drop(mapped);
    readback.unmap();

    std::fs::write(path, oag_formats::png::encode_rgba(width, height, &pixels))
        .with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

/// Anisotropic filtering level: the one texture-filtering knob modern
/// renderers expose to a user. Mip generation itself always runs (see
/// [`mip_chain`]) and is not a setting - every renderer just does it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Anisotropy {
    Off,
    X2,
    X4,
    X8,
    #[default]
    X16,
}

impl Anisotropy {
    /// The `wgpu::SamplerDescriptor::anisotropy_clamp` value this level maps to.
    const fn clamp(self) -> u16 {
        match self {
            Self::Off => 1,
            Self::X2 => 2,
            Self::X4 => 4,
            Self::X8 => 8,
            Self::X16 => 16,
        }
    }
}

impl std::str::FromStr for Anisotropy {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "off" | "1" | "1x" => Ok(Self::Off),
            "2" | "2x" => Ok(Self::X2),
            "4" | "4x" => Ok(Self::X4),
            "8" | "8x" => Ok(Self::X8),
            "16" | "16x" => Ok(Self::X16),
            other => Err(format!(
                "{other:?} is not an anisotropy level; try off, 2x, 4x, 8x or 16x"
            )),
        }
    }
}

impl std::fmt::Display for Anisotropy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Off => "off",
            Self::X2 => "2x",
            Self::X4 => "4x",
            Self::X8 => "8x",
            Self::X16 => "16x",
        })
    }
}

/// Downsamples `rgba` into a full mip chain by repeated 2x2 box filtering,
/// down to a 1x1 level.
///
/// Track and ship textures are seen at every distance and grazing angle a
/// chase camera produces; without mips, minification aliases into shimmer
/// that a single sample can't fix. Filtering happens on `Rgba8UnormSrgb`
/// sample data, matching what the sampler itself blends between levels.
fn mip_chain(width: u32, height: u32, rgba: &[u8]) -> Vec<(u32, u32, Vec<u8>)> {
    let mut levels: Vec<(u32, u32, Vec<u8>)> = vec![(width, height, rgba.to_vec())];
    loop {
        let (w, h, data) = levels.last().expect("levels is never empty");
        let (w, h) = (*w, *h);
        if w == 1 && h == 1 {
            break;
        }
        let next_width = (w / 2).max(1);
        let next_height = (h / 2).max(1);
        let mut next = vec![0u8; (next_width * next_height * 4) as usize];
        for y in 0..next_height {
            let y0 = (y * 2).min(h - 1);
            let y1 = (y * 2 + 1).min(h - 1);
            for x in 0..next_width {
                let x0 = (x * 2).min(w - 1);
                let x1 = (x * 2 + 1).min(w - 1);
                for c in 0..4usize {
                    let texel = |sx: u32, sy: u32| data[((sy * w + sx) * 4) as usize + c] as u32;
                    let sum = texel(x0, y0) + texel(x1, y0) + texel(x0, y1) + texel(x1, y1);
                    next[((y * next_width + x) * 4) as usize + c] = ((sum + 2) / 4) as u8;
                }
            }
        }
        levels.push((next_width, next_height, next));
    }
    levels
}

/// The blend this crate uses for a model's `transparent_draws` (list B).
///
/// **Not recovered from the game.** Unlike [`crate::exhaust::BLEND`], no GE
/// blend-function state has been read out of a decompile for these batches;
/// this is a plausible reading (source-over, the ordinary "glass" lerp), not
/// a confirmed one - an unscored placeholder in the same sense
/// `crates/game/src/race.rs`'s `GLOW_SCROLL_PERIOD_TICKS` documents its own
/// invented constants. `Batch::is_transparent`/`is_alpha_tested` in
/// `oag_formats::vex` are what is actually confirmed: that these batches are
/// meant to be blended rather than replaced, and are not alpha-tested
/// cutouts. Revise this the moment the real state is recovered.
pub const TRANSPARENT_BLEND: wgpu::BlendState = wgpu::BlendState {
    color: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::SrcAlpha,
        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
        operation: wgpu::BlendOperation::Add,
    },
    alpha: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::One,
        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
        operation: wgpu::BlendOperation::Add,
    },
};

/// Everything [`build`] hands back: geometry, texture bindings, and the three
/// pipelines a `Model` draws through.
#[derive(Debug)]
pub struct Built {
    /// Opaque pass: depth write on, no blending. Draws [`Model::draws`].
    pub pipeline: wgpu::RenderPipeline,
    /// Cutout pass: depth write on, no blending, `discard` below
    /// `mesh.wgsl`'s `ALPHA_TEST_THRESHOLD` (an invented placeholder, not a
    /// recovered GE reference value). Draws [`Model::alpha_tested_draws`] as
    /// a second `set_pipeline` in the same render pass as `pipeline`, before
    /// `blend_pipeline`.
    pub alpha_test_pipeline: wgpu::RenderPipeline,
    /// Blended pass: depth write off, [`TRANSPARENT_BLEND`]. Draws
    /// [`Model::transparent_draws`] last, as a third `set_pipeline` in the
    /// same render pass - see [`crate::exhaust::Pipeline`] for the precedent
    /// of pairing an opaque and a blended pipeline this way.
    pub blend_pipeline: wgpu::RenderPipeline,
    pub bind_group: wgpu::BindGroup,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub texture_binds: Vec<wgpu::BindGroup>,
}

/// Which depth state [`build`] gives a model's pipelines.
///
/// The race pass clears depth to 1.0 and everything else compares `Less`, so a
/// sky drawn at the far plane would fail the test and be invisible. Rather than
/// place it in depth at all, [`Depth::Sky`] takes it out of the question: drawn
/// first, comparing `Always` and writing nothing, so it fills the frame and then
/// every later draw covers it wherever there is geometry. That also means the
/// sky needs no far plane large enough to contain it, which matters because its
/// authored cube is tens of units across while a track is thousands.
///
/// **The one size constraint that remains: a sky's half-extent must exceed the
/// near plane.** Taking depth out of the question does not take *clipping* out
/// of it, and a cube centred on the eye whose faces fall inside the near plane is
/// clipped away entirely. Pulse's shipped skies have radii of 18.4 to 61.7
/// against a near plane of 1.0, so the margin is wide - but that is a property of
/// the data rather than of this code, and it is why the sky is never scaled down
/// to fit anything.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Depth {
    /// Scene geometry: test `Less`, write depth.
    #[default]
    Scene,
    /// The sky: occludes nothing and is occluded by everything.
    Sky,
}

/// Builds the pipeline, geometry and texture bindings for `model`.
///
/// Shared by the offscreen capture path and the interactive orbit window, so
/// both draw through the one pipeline. The returned `BindGroup` is a
/// placeholder bound to an empty buffer; a real uniform buffer and bind group
/// must be created against `pipeline.get_bind_group_layout(0)` by the caller.
///
/// `sample_count` must match the render pass's colour and depth attachments -
/// 1 outside a race, or `[graphics] anti_aliasing`'s MSAA count inside one.
/// See `race::Scene::new`.
pub fn build(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    model: &Model,
    format: wgpu::TextureFormat,
    anisotropy: Anisotropy,
    sample_count: u32,
    depth: Depth,
) -> Result<Built> {
    // The blended pipeline never writes depth whichever role this is; the rest
    // is what [`Depth`] chooses between.
    let (depth_write, depth_compare) = match depth {
        Depth::Scene => (true, wgpu::CompareFunction::Less),
        Depth::Sky => (false, wgpu::CompareFunction::Always),
    };

    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("mesh"),
        source: wgpu::ShaderSource::Wgsl(include_str!("mesh.wgsl").into()),
    });

    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("mesh"),
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
        label: Some("albedo"),
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
        label: Some("mesh"),
        bind_group_layouts: &[Some(&layout), Some(&texture_layout)],
        immediate_size: 0,
    });

    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("mesh"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[Some(wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<GpuVertex>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &wgpu::vertex_attr_array![
                    0 => Float32x3, 1 => Float32x3, 2 => Float32x4, 3 => Float32x2,
                    4 => Float32, 5 => Float32
                ],
            })],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(format.into())],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState {
            // Culling is off on purpose. Strip winding is reconstructed rather
            // than read from the file, so culling would turn any mistake there
            // into invisible geometry instead of a visible artefact.
            cull_mode: None,
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: Some(depth_write),
            depth_compare: Some(depth_compare),
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

    // Second pipeline for `Model::alpha_tested_draws`: same shader module,
    // bind group layouts, vertex layout and depth state as the opaque
    // pipeline (a cutout is meant to occlude and be occluded exactly like
    // opaque geometry), except the fragment shader discards pixels below a
    // threshold instead of always returning alpha 1.0.
    let alpha_test_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("mesh alpha test"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[Some(wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<GpuVertex>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &wgpu::vertex_attr_array![
                    0 => Float32x3, 1 => Float32x3, 2 => Float32x4, 3 => Float32x2,
                    4 => Float32, 5 => Float32
                ],
            })],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main_alpha_test"),
            targets: &[Some(format.into())],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState {
            cull_mode: None,
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: Some(depth_write),
            depth_compare: Some(depth_compare),
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

    // Third pipeline for `Model::transparent_draws` (list B): same shader
    // module, bind group layouts and vertex layout, blended instead of
    // replaced and with depth write off so an overlapping transparent draw
    // cannot occlude one drawn after it - see `TRANSPARENT_BLEND` and
    // `crate::exhaust::Pipeline`, which established this opaque/blended
    // pairing first.
    let blend_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("mesh blend"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[Some(wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<GpuVertex>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &wgpu::vertex_attr_array![
                    0 => Float32x3, 1 => Float32x3, 2 => Float32x4, 3 => Float32x2,
                    4 => Float32, 5 => Float32
                ],
            })],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main_blend"),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(TRANSPARENT_BLEND),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState {
            cull_mode: None,
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: Some(false),
            depth_compare: Some(depth_compare),
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

    let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("vertices"),
        size: std::mem::size_of_val(&model.vertices[..]) as u64,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    queue.write_buffer(&vertex_buffer, 0, bytemuck::cast_slice(&model.vertices));

    let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("indices"),
        size: std::mem::size_of_val(&model.indices[..]) as u64,
        usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    queue.write_buffer(&index_buffer, 0, bytemuck::cast_slice(&model.indices));

    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("albedo"),
        address_mode_u: wgpu::AddressMode::Repeat,
        address_mode_v: wgpu::AddressMode::Repeat,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::MipmapFilterMode::Linear,
        anisotropy_clamp: anisotropy.clamp(),
        ..Default::default()
    });

    let make = |width: u32, height: u32, rgba: &[u8], label: &str| {
        let mips = mip_chain(width, height, rgba);
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size,
            mip_level_count: mips.len() as u32,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        for (level, (mip_width, mip_height, mip_rgba)) in mips.iter().enumerate() {
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: level as u32,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                mip_rgba,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(mip_width * 4),
                    rows_per_image: Some(*mip_height),
                },
                wgpu::Extent3d {
                    width: *mip_width,
                    height: *mip_height,
                    depth_or_array_layers: 1,
                },
            );
        }
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(label),
            layout: &texture_layout,
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
    };

    // A white 1x1 stands in for untextured draws, so the shader needs no branch.
    // It also fills the slot of a `Texture` node we could not decode, keeping
    // every later texture at the index its materials expect.
    let white = make(1, 1, &[255, 255, 255, 255], "white");
    let mut texture_binds = vec![white];
    for slot in &model.textures {
        texture_binds.push(match slot {
            Some(t) => make(t.width, t.height, &t.rgba, &t.label),
            None => make(1, 1, &[255, 255, 255, 255], "undecoded"),
        });
    }

    let placeholder = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("placeholder"),
        size: std::mem::size_of::<Uniforms>() as u64,
        usage: wgpu::BufferUsages::UNIFORM,
        mapped_at_creation: false,
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("placeholder"),
        layout: &layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: placeholder.as_entire_binding(),
        }],
    });

    Ok(Built {
        pipeline,
        alpha_test_pipeline,
        blend_pipeline,
        bind_group,
        vertex_buffer,
        index_buffer,
        texture_binds,
    })
}
