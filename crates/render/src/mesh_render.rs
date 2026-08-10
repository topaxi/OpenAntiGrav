//! Renders a decoded model, offscreen or into a surface.
//!
//! Deliberately plain: depth buffer, perspective camera, two-light rig. The
//! point is to see the geometry clearly and catch decoding errors, not to
//! reproduce Pulse's look.

use anyhow::{Context, Result};
use oag_core::math::{Mat4, Vec3, camera};
use std::path::Path;

use crate::mesh::{GpuVertex, Model};
use oag_formats::vex;

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

/// The fog block `mesh.wgsl` reads from bind group 2.
///
/// Deliberately **not** part of [`Uniforms`]. That struct is mirrored by every
/// pipeline in this crate and by the asset viewer, so growing it means moving
/// four `.wgsl` declarations and `oag-view`'s own buffer sizing in lockstep;
/// only the pipelines that fog need these fields.
///
/// The field order is the WGSL declaration's, and the padding is real: a WGSL
/// `vec3` aligns to 16 bytes, so the `f32` after each one occupies the slot that
/// alignment would otherwise waste.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Fog {
    /// Fog colour, linear.
    pub colour: [f32; 3],
    /// Distance at which fog starts.
    pub near: f32,
    /// Eye position, so the fragment stage can measure distance.
    pub camera: [f32; 3],
    /// Distance at which fog is total.
    pub far: f32,
    /// `1.0` to fog, `0.0` to pass colour through untouched.
    pub enabled: f32,
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
}

impl Fog {
    /// Fog that does nothing.
    ///
    /// What the sky, the asset viewer and a track with no `fogCube` bind. It is
    /// a value rather than an unbound group because WGSL has no optional
    /// bindings: the alternative is a second pipeline per fog state.
    #[must_use]
    pub fn off() -> Self {
        Self {
            colour: [0.0; 3],
            near: 0.0,
            camera: [0.0; 3],
            far: 1.0,
            enabled: 0.0,
            _pad0: 0.0,
            _pad1: 0.0,
            _pad2: 0.0,
        }
    }

    /// Fog from one sampled [`oag_formats::fog::FogParams`] and the eye it was
    /// sampled at.
    #[must_use]
    pub fn new(params: &oag_formats::fog::FogParams, camera: [f32; 3]) -> Self {
        Self {
            colour: params.colour,
            near: params.near,
            camera,
            // A degenerate range would divide by zero in the shader; the shader
            // clamps the span, and this keeps the ordering sane regardless.
            far: params.far.max(params.near + f32::EPSILON),
            enabled: 1.0,
            _pad0: 0.0,
            _pad1: 0.0,
            _pad2: 0.0,
        }
    }
}

/// Size, in bytes, of the fog uniform buffer.
pub const FOG_SIZE: u64 = std::mem::size_of::<Fog>() as u64;

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

    // **Raw, because the shader now writes gamma-space values and nothing may
    // encode them a second time.** See
    // [ADR-0020](../../../docs/architecture/adr/0020-gamma-authoritative-colour-space.md).
    //
    // This was `Rgba8UnormSrgb`, and the reasoning that put it there is
    // superseded rather than wrong: given a texture upload that *linearised*,
    // `mesh.wgsl` multiplied linear light by the light rig and only an
    // encode-on-write target matched the window. That premise is gone - the
    // upload below is raw now - so the multiply happens in gamma space, the
    // window no longer encodes either, and an encoding target here would be the
    // double-encode. The old measurement (mean ~2.4/255, up to 57/255 against
    // the window) is exactly the error a *partial* migration reintroduces,
    // which is why the format here and the format of `make`'s upload move
    // together or not at all.
    let format = wgpu::TextureFormat::Rgba8Unorm;
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
        additive_pipeline,
        unblended_pipeline,
        bind_group,
        vertex_buffer,
        index_buffer,
        texture_binds,
        fog_bind,
        fog_buffer: _,
    } = build(
        &device,
        &queue,
        model,
        format,
        anisotropy,
        1,
        Depth::Scene,
        TRANSPARENT_BLEND,
    )?;

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
        // `Buffer::slice` panics on a zero-length buffer, so a model with no
        // geometry has to skip every command below rather than binding one. The
        // pass still runs, so the clear above lands and a valid frame comes out;
        // deciding whether an empty model is worth capturing at all belongs to
        // the caller, not here.
        if !model.vertices.is_empty() && !model.indices.is_empty() {
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            // Group 2 never changes within a pass and survives `set_pipeline`, so it
            // is bound once here rather than per draw call. The capture path leaves
            // it at `Fog::off`.
            pass.set_bind_group(2, &fog_bind, &[]);
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

            // Third pipeline group, same pass: `Model::transparent_draws`,
            // blended rather than replaced and drawn last so opaque and cutout
            // depth is already resolved. See `crate::exhaust::Pipeline` for the
            // precedent of pairing an opaque and a blended pipeline this way.
            //
            // **One `set_pipeline` per draw call, because the batch's own
            // `pass_mask` picks its blend equation** - the `0x0700` class
            // splits three ways and a single model mixes them. Grouping the
            // draws by class first would reorder them, and transparent draws
            // are order-dependent by definition.
            let mut current: Option<(Option<vex::BlendClass>, bool)> = None;
            for draw in &model.transparent_draws {
                let key = (draw.blend, draw.culled);
                if current != Some(key) {
                    let set = match draw.blend {
                        Some(vex::BlendClass::Additive) => &additive_pipeline,
                        Some(vex::BlendClass::None) => &unblended_pipeline,
                        Some(vex::BlendClass::AlphaOver) | None => &blend_pipeline,
                    };
                    pass.set_pipeline(&set[usize::from(draw.culled)]);
                    current = Some(key);
                }
                let slot = draw.texture.map_or(0, |t| t + 1);
                pass.set_bind_group(1, &texture_binds[slot.min(texture_binds.len() - 1)], &[]);
                pass.draw_indexed(draw.range.clone(), 0, 0..1);
            }
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

/// The blend for a transparent batch of class
/// [`vex::BlendClass::AlphaOver`] - `pass_mask & 0x100`.
///
/// **Recovered.** `Gfx_BuildBatchStateList`'s `0x100` branch programs
/// `Gu_Enable(GU_BLEND)` with `Gu_BlendFunc(GU_ADD, GU_SRC_ALPHA,
/// GU_ONE_MINUS_SRC_ALPHA, 0, 0)` and disables the colour test. The colour
/// factors here are that call. This constant's doc used to say the opposite -
/// "not recovered ... a plausible reading" - and it happened to be right; what
/// was wrong was applying it to **every** transparent batch. See
/// [`ADDITIVE_BLEND`] and [`vex::BlendClass`].
///
/// **The alpha factors are still ours**, and deliberately unchanged: PSP
/// blending is RGB-only, so the original's call says nothing about the alpha
/// channel and there is nothing to copy.
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

/// The blend for a transparent batch of class [`vex::BlendClass::Additive`] -
/// `pass_mask & 0x200`.
///
/// **Recovered, and identical to [`crate::exhaust::BLEND`].**
/// `Gfx_BuildBatchStateList`'s `0x200` branch programs
/// `Gu_BlendFunc(GU_ADD, GU_SRC_ALPHA, GU_FIX, 0x000000, 0xffffff)` - a fixed
/// destination factor of white, i.e. `src * srcAlpha + dst`. That is the same
/// equation `ExhaustFlare_BuildDisplayList` programs for the engine flare, so
/// **the flare, the boost plume and every other `0x200` batch in the game
/// share one blend**, which was not known until the `0x0700` class was split.
///
/// Alpha matches the colour factors here rather than following
/// [`TRANSPARENT_BLEND`]'s split, for the same reason `exhaust::BLEND` does:
/// a target later read as premultiplied should not disagree with its own
/// colour channels.
pub const ADDITIVE_BLEND: wgpu::BlendState = wgpu::BlendState {
    color: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::SrcAlpha,
        dst_factor: wgpu::BlendFactor::One,
        operation: wgpu::BlendOperation::Add,
    },
    alpha: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::SrcAlpha,
        dst_factor: wgpu::BlendFactor::One,
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
    /// Blended pass: depth write off, the `blend` the caller passed to
    /// [`build`] - [`TRANSPARENT_BLEND`] for ordinary scene geometry. Draws
    /// [`Model::transparent_draws`] last, as a third `set_pipeline` in the
    /// same render pass - see [`crate::exhaust::Pipeline`] for the precedent
    /// of pairing an opaque and a blended pipeline this way.
    ///
    /// Indexed by `culled as usize`: `[0]` two-sided, `[1]` back-face culled.
    pub blend_pipeline: [wgpu::RenderPipeline; 2],
    /// Blended pass for [`vex::BlendClass::Additive`] batches
    /// (`pass_mask & 0x200`): [`ADDITIVE_BLEND`], depth write off.
    ///
    /// A second blended pipeline rather than a second pass: the three classes
    /// interleave freely within one model's `transparent_draws`, so they are
    /// selected per draw call by [`Model`]'s own `DrawCall::blend`.
    pub additive_pipeline: [wgpu::RenderPipeline; 2],
    /// Blended pass for [`vex::BlendClass::None`] batches
    /// (`pass_mask & 0x400`): sorted with the transparent list, drawn with
    /// blending **off**, depth write still off.
    pub unblended_pipeline: [wgpu::RenderPipeline; 2],
    pub bind_group: wgpu::BindGroup,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub texture_binds: Vec<wgpu::BindGroup>,
    /// Bind group 2, holding [`Fog`]. Bound by every draw; write
    /// [`Built::fog_buffer`] to change it.
    pub fog_bind: wgpu::BindGroup,
    /// The buffer behind [`Built::fog_bind`], initialised to [`Fog::off`].
    pub fog_buffer: wgpu::Buffer,
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
///
/// `blend` is the state [`Built::blend_pipeline`] draws
/// [`Model::transparent_draws`] with. Almost every caller wants
/// [`TRANSPARENT_BLEND`]; the one exception is the boost plume, which reuses
/// this same opaque/cutout/blend machinery but needs `crate::exhaust::BLEND`
/// instead, because its `_ADD` texture is additive rather than a lerp - see
/// `race::Scene`'s `boost` field.
#[allow(clippy::too_many_arguments)]
pub fn build(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    model: &Model,
    format: wgpu::TextureFormat,
    anisotropy: Anisotropy,
    sample_count: u32,
    depth: Depth,
    blend: wgpu::BlendState,
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

    let fog_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("fog"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    });

    // Every pipeline gets a fog buffer, initialised to `Fog::off`. A caller that
    // never writes it therefore renders exactly as it did before fog existed,
    // which is what keeps the asset viewer and the offscreen capture path
    // unchanged without either of them knowing fog is there.
    let fog_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("fog"),
        size: FOG_SIZE,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    queue.write_buffer(&fog_buffer, 0, bytemuck::bytes_of(&Fog::off()));
    let fog_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("fog"),
        layout: &fog_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: fog_buffer.as_entire_binding(),
        }],
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("mesh"),
        bind_group_layouts: &[Some(&layout), Some(&texture_layout), Some(&fog_layout)],
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
            // Colour only: alpha is the bloom's glow mask, which ordinary
            // geometry must not touch. See the blended pipeline below and
            // `crate::post::bloom`.
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: None,
                write_mask: wgpu::ColorWrites::COLOR,
            })],
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
            // Colour only: alpha is the bloom's glow mask, which ordinary
            // geometry must not touch. See the blended pipeline below and
            // `crate::post::bloom`.
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: None,
                write_mask: wgpu::ColorWrites::COLOR,
            })],
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
    // cannot occlude one drawn after it - see `crate::exhaust::Pipeline`,
    // which established this opaque/blended pairing first. `blend` is almost
    // always `TRANSPARENT_BLEND`; see this function's own doc comment for the
    // one caller that passes something else.
    // **One pipeline per recovered blend class.** The `0x0700` transparent
    // class splits three ways and a single mesh mixes them, so the choice is
    // per draw call rather than per model - see `vex::Batch::blend_class` and
    // `Model`'s `DrawCall::blend`. `blend_pipeline` keeps the caller's own
    // `blend` argument so an override still works; the other two are the
    // recovered equations and take no argument.
    // **Cull mode is a per-batch property, so every pipeline exists twice.**
    // The original culls most of its geometry - measured 1,632 of 1,734
    // batches on `01_Track` - but not all of it, and `shipboost.vex` culls
    // none of its four. It matters most on transparent batches, where a back
    // face is not hidden by the depth test but blended a second time. See
    // `mesh::DrawCall::culled`.
    //
    // Front faces are counter-clockwise, which is wgpu's default and is
    // **measured rather than assumed**: culling back faces at four camera
    // poses changes 19 to 129 pixels of a 522,240-pixel frame against not
    // culling, while culling front faces changes 2,666 - and the pixels that
    // do change are back-facing slivers that were wrongly visible. That is
    // the check the old blanket `cull_mode: None` asked for and never got.
    let make_pipeline = |label: &str, blend: Option<wgpu::BlendState>, cull: bool| {
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(label),
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
                    blend,
                    // **Colour only: alpha is the bloom's glow mask.**
                    // `Mesh_SetBatchDrawState` protects the alpha channel for
                    // every batch that does not set `pass_mask & 0x40`, and a
                    // census of 6,923 authored batches across four circuits and
                    // two ship files found **none** that do. So ordinary
                    // geometry never writes the mask in the original, and must
                    // not here either - see `crate::post::bloom` and
                    // `docs/ghidra/functions/psp-pulse-usa/bloom.md`.
                    write_mask: wgpu::ColorWrites::COLOR,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                cull_mode: cull.then_some(wgpu::Face::Back),
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
        })
    };
    let blend_pipeline = [
        make_pipeline("mesh blend (alpha over, two-sided)", Some(blend), false),
        make_pipeline("mesh blend (alpha over, culled)", Some(blend), true),
    ];
    let additive_pipeline = [
        make_pipeline(
            "mesh blend (additive, two-sided)",
            Some(ADDITIVE_BLEND),
            false,
        ),
        make_pipeline("mesh blend (additive, culled)", Some(ADDITIVE_BLEND), true),
    ];
    let unblended_pipeline = [
        make_pipeline("mesh blend (none, two-sided)", None, false),
        make_pipeline("mesh blend (none, culled)", None, true),
    ];

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
            // **Raw, so the sampler hands the shader the disc's own bytes.**
            // The GE blends stored bytes, so every stage of this pipeline works
            // in gamma space and nothing linearises - see
            // [ADR-0020](../../../docs/architecture/adr/0020-gamma-authoritative-colour-space.md).
            //
            // This was `Rgba8UnormSrgb`. Vertex colour was never sRGB-decoded
            // (`mesh.rs` builds it as `byte / 255.0`), so under that format the
            // shader multiplied a *linear* texel by a *gamma* vertex colour -
            // two spaces in one expression - and `race.rs` carried a load-time
            // re-encode of the boost plume's texels purely to cancel it. That
            // re-encode is gone with this; putting the sRGB format back without
            // restoring it would darken the plume by the same ~30% the
            // re-encode was compensating for.
            format: wgpu::TextureFormat::Rgba8Unorm,
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
        fog_bind,
        fog_buffer,
        pipeline,
        alpha_test_pipeline,
        blend_pipeline,
        additive_pipeline,
        unblended_pipeline,
        bind_group,
        vertex_buffer,
        index_buffer,
        texture_binds,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A model with no geometry must write a frame rather than panic.
    ///
    /// `wgpu::Buffer::slice` panics on a zero-length buffer, which is how
    /// `oag-view --collision` died on any `.vex` with no recognised collision
    /// class - see `docs/formats/pure-status.md`. Worth knowing if this ever
    /// regresses: `create_buffer(size: 0)` and `write_buffer(&[])` both
    /// *succeed*, so the death is two frames later at `set_vertex_buffer`, and
    /// clamping the buffer to a nonzero size is the fix that looks right and
    /// still crashes.
    ///
    /// Deliberately not `#[ignore]`d, unlike `tests/collision_capture.rs`: that
    /// one exists to produce a picture, this one guards a regression, and an
    /// `#[ignore]`d regression test is a test nobody runs. The adapter probe is
    /// the pattern `post::fxaa` and `post::fsr1` already use, so a machine
    /// without a GPU skips instead of failing.
    #[test]
    fn an_empty_model_captures_a_frame_instead_of_panicking() {
        // Probed here rather than left to `capture_from`, which reports a
        // missing adapter as an error - indistinguishable, from the test's
        // side, from the guard not working.
        let instance = wgpu::Instance::default();
        if pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
            .is_err()
        {
            eprintln!("no GPU adapter: skipping");
            return;
        }

        // Built the way the bug arrived rather than hand-assembled: a `.vex`
        // with no recognised collision class decodes to zero nodes, and
        // `build_model` over zero nodes is what reached the render pass.
        let model =
            crate::collision::build_model("empty", &[], crate::collision::Style::Wireframe, true);
        assert!(model.vertices.is_empty() && model.indices.is_empty());

        let path = std::env::temp_dir().join("oag-empty-model.png");
        capture_from(&model, &path, 64, 64, 0.9, 0.85, Anisotropy::default())
            .expect("capturing an empty model");

        // Checked through the PNG header rather than the pixels: reaching this
        // line at all is the regression, since the old code panicked inside the
        // render pass and never wrote a file. Byte length carries no signal -
        // `oag_formats::png` emits stored deflate blocks, so every 64x64 frame
        // is the same ~16 KB whatever is in it.
        let bytes = std::fs::read(&path).expect("reading the capture back");
        assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n", "not a PNG");
        assert_eq!(
            &bytes[16..24],
            &[0, 0, 0, 64, 0, 0, 0, 64],
            "wrong IHDR size"
        );
    }
}
