//! Renders a decoded model, offscreen or into a surface.
//!
//! Deliberately plain: depth buffer, perspective camera, two-light rig. The
//! point is to see the geometry clearly and catch decoding errors, not to
//! reproduce Pulse's look.

use anyhow::Result;

use crate::mesh::{GpuVertex, Model};

mod blend;
mod uniforms;

pub use blend::{ADDITIVE_BLEND, TRANSPARENT_BLEND, TransparentPipelines};
use uniforms::Uniforms;
pub use uniforms::{DEPTH_FORMAT, Fog, Light, SCENE_SIZE, Scene, UNIFORMS_SIZE, write_uniforms};

/// The texture-transform table `mesh.wgsl` reads from bind group 3: one
/// `(scale, offset)` pair per entry of [`Model::anim_tracks`], already sampled
/// for this frame.
///
/// Slot 0 is the identity, which is what [`GpuVertex::anim`] `== 0` selects, so
/// the shader needs no branch for the overwhelming majority of vertices.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TexAnims {
    /// `[scale_u, scale_v, offset_u, offset_v]` per track.
    pub transform: [[f32; 4]; crate::mesh::ANIM_TRACK_LIMIT],
}

impl Default for TexAnims {
    fn default() -> Self {
        Self {
            transform: [[1.0, 1.0, 0.0, 0.0]; crate::mesh::ANIM_TRACK_LIMIT],
        }
    }
}

impl TexAnims {
    /// Samples every track of `model` at `seconds` and packs the table.
    ///
    /// `seconds` is the model's animation clock. The original gives each model
    /// its own - the boost plume's is its flare's life timer, reset at every
    /// reveal - but for **world meshes** it passes the race clock, which is
    /// what a track's scenery gets here. See
    /// `docs/ghidra/functions/psp-pulse-usa/texture-animation.md`, "The values
    /// gap is closed"; one mesh in that page's one-frame census was seen on a
    /// different clock, and which models get their own is not recovered.
    ///
    /// Every track wraps on its own authored period, so there is no shared
    /// phase to keep them in step and nothing to seam at a global wrap.
    ///
    /// Clamped to the table's size here as well as in the builder, because
    /// [`Model`] is a plain struct anyone can fill in and [`crate::mesh::merge`]
    /// concatenates track lists without re-checking the ceiling. A model past
    /// it animates its first [`crate::mesh::ANIM_TRACK_LIMIT`] `- 1` tracks and
    /// leaves the rest at identity, which is what the builder does too.
    #[must_use]
    pub fn sample(model: &Model, seconds: f32) -> Self {
        let mut out = Self::default();
        for (slot, track) in model
            .anim_tracks
            .iter()
            .take(crate::mesh::ANIM_TRACK_LIMIT - 1)
            .enumerate()
        {
            let (scale, offset) = track.sample(seconds);
            out.transform[slot + 1] = [scale[0], scale[1], offset[0], offset[1]];
        }
        out
    }
}

/// Size, in bytes, of the [`TexAnims`] uniform buffer.
pub const TEX_ANIMS_SIZE: u64 = std::mem::size_of::<TexAnims>() as u64;

/// The node-transform table `mesh.wgsl` reads from bind group 4: one world
/// matrix per entry of [`Model::anim_nodes`], sampled for this frame.
///
/// Slot 0 is the identity, which is what [`GpuVertex::xform`] `== 0` selects,
/// so the 93% of a circuit's geometry that does not move costs one indexed load
/// and no branch - deliberately the same shape as [`TexAnims`].
///
/// 8 KiB at [`crate::mesh::NODE_ANIM_LIMIT`], which is inside the 64 KiB
/// uniform binding every wgpu backend guarantees.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct NodeAnims {
    /// One column-major world matrix per node. The layout
    /// `oag_formats::vex` already uses - see `mesh.wgsl`'s own note.
    pub transform: [[f32; 16]; crate::mesh::NODE_ANIM_LIMIT],
}

impl Default for NodeAnims {
    fn default() -> Self {
        Self {
            transform: [oag_formats::vex::IDENTITY; crate::mesh::NODE_ANIM_LIMIT],
        }
    }
}

impl NodeAnims {
    /// Samples every `Anim Transform` of `model` at `seconds` and packs the
    /// table.
    ///
    /// `seconds` is the model's animation clock, the same one
    /// [`TexAnims::sample`] takes and for the same reason: the original passes
    /// the race clock to every world mesh's updater, and each node wraps on its
    /// own authored `LoopEnd`.
    #[must_use]
    pub fn sample(model: &Model, seconds: f32) -> Self {
        let mut out = Self::default();
        for (slot, matrix) in model.sample_anim_nodes(seconds).into_iter().enumerate() {
            out.transform[slot + 1] = matrix;
        }
        out
    }
}

/// Size, in bytes, of the [`NodeAnims`] uniform buffer.
pub const NODE_ANIMS_SIZE: u64 = std::mem::size_of::<NodeAnims>() as u64;

/// Headless capture, split into `crate::capture` so a pixel-returning entry
/// point could be added there without pushing this file past its frozen
/// size ceiling - see [`crate::capture`] for both functions' own docs.
pub use crate::capture::{capture_from, capture_pixels_from};

mod target;
pub use target::{fragment_options, is_linear_target, linear_constants};

mod anisotropy;
pub use anisotropy::Anisotropy;

mod texture;
use texture::mip_chain;

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
    /// One pipeline pair per distinct equation the *model's own file* authors,
    /// for the draws that carry a [`crate::mesh::DrawCall::blend_state`].
    ///
    /// **Built from the model rather than from a fixed list**, because the set
    /// is a property of the data: Wipeout HD's materials author a source and a
    /// destination factor each drawn from four values, and Talon's Junction
    /// alone uses five of the pairs that makes. A Pulse model authors none and
    /// this is empty, so nothing is created for a title that does not use it.
    pub authored_pipelines: Vec<(wgpu::BlendState, [wgpu::RenderPipeline; 2])>,
    pub bind_group: wgpu::BindGroup,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub texture_binds: Vec<wgpu::BindGroup>,
    /// Bind group 2, holding [`Scene`] - [`Fog`] **and** [`Light`]. Bound by
    /// every draw; write [`Built::fog_buffer`] to change it.
    pub fog_bind: wgpu::BindGroup,
    /// The buffer behind [`Built::fog_bind`], initialised to [`Scene::off`].
    pub fog_buffer: wgpu::Buffer,
    /// Bind group 3, holding [`TexAnims`]. Bound by every draw; write
    /// [`Built::anim_buffer`] once a frame to animate.
    pub anim_bind: wgpu::BindGroup,
    /// The buffer behind [`Built::anim_bind`], initialised to all-identity.
    pub anim_buffer: wgpu::Buffer,
    /// The [`NodeAnims`] buffer, bound as **binding 1 of the same group 3** as
    /// [`Built::anim_buffer`]. Write it once a frame to move scenery.
    pub node_anim_buffer: wgpu::Buffer,
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
/// Whether a model's draws may write the scene target's alpha channel, which
/// [`crate::post::bloom`] reads as its glow mask.
///
/// The original decides this **per draw path**, not per material, and the two
/// paths disagree: `FUN_089307b4`'s batch group calls
/// `Bloom_SetPixelMask(g_bloom, 0)`, masking alpha off, while the path the
/// boost plume actually takes - `Mesh_CompileGeometryPass` (`0x0890d0cc`) with
/// state from `Gfx_BuildBatchStateList` (`0x0891f890`) - opens every channel
/// with an unconditional `Gu_PixelMask(0)` at the top of its state list. So a
/// hull and a track surface leave the mask alone and the plume writes it, and
/// that difference is a reading of the two functions rather than a look
/// choice. See `docs/ghidra/functions/psp-pulse-usa/bloom.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlowMask {
    /// Colour only - the default, and what every authored batch measured on
    /// the disc gets.
    Protected,
    /// Alpha reaches the target, so this model's fragments feed the bloom.
    Written,
}

impl GlowMask {
    /// The colour write mask this choice implies.
    #[must_use]
    pub fn writes(self) -> wgpu::ColorWrites {
        match self {
            Self::Protected => wgpu::ColorWrites::COLOR,
            Self::Written => wgpu::ColorWrites::ALL,
        }
    }
}

// One knob per pipeline decision, and they are genuinely independent: format,
// filtering, sample count, depth role, blend and glow mask do not group into a
// meaningful struct without inventing a name for the grouping. The same call
// `Drawable::new` already makes.
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
    glow: GlowMask,
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
    // The target format is the statement about colour space - see
    // [`is_linear_target`] - and it reaches the shader as a pipeline
    // constant, so every pipeline built here answers for the target it was
    // built against and no uniform needs a field the sky's zeroed scene
    // buffer would miss.
    // Two overrides, and they answer different questions: `linear_out` is the
    // target's colour space and `colour_is_light` is the model's own vertex
    // semantics. Conflating them is a bug this had - see
    // [`Model::vertex_colour_is_light`].
    //
    // A third answers a third: `flame_*` is the parameter set Wipeout HD's
    // engine-flare material authors, carried per model because that is where
    // the values are - see [`crate::mesh::Flame`]. A `Vec` rather than the
    // match this used to be, since those six are numbers off the disc rather
    // than a fixed table.
    let mut constants: Vec<(&str, f64)> = Vec::new();
    if is_linear_target(format) {
        constants.push(("linear_out", 1.0));
    }
    if model.vertex_colour_is_light {
        constants.push(("colour_is_light", 1.0));
    }
    if let Some(flame) = model.flame {
        constants.extend([
            ("flame_shading", 1.0),
            ("flame_rim_power", f64::from(flame.rim_power)),
            ("flame_rim_scale", f64::from(flame.rim_scale)),
            ("flame_rim_min", f64::from(flame.rim_min)),
            ("flame_alpha_scale", f64::from(flame.alpha_scale)),
            ("flame_colour_scale", f64::from(flame.colour_scale)),
        ]);
    }
    let constants = constants.as_slice();

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
            // The lightmap shares the albedo's sampler: it is the same
            // filtering on the same kind of texture, and a second sampler would
            // be a second thing to keep in step for no difference in the
            // picture.
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
        ],
    });

    let fog_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        // Labelled for what the buffer holds, which is `Scene` - fog *and* the
        // light rig. The three "fog" labels here predated `Light` joining it.
        label: Some("scene"),
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
        label: Some("scene"),
        size: SCENE_SIZE,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    queue.write_buffer(&fog_buffer, 0, bytemuck::bytes_of(&Scene::off()));
    let fog_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("scene"),
        layout: &fog_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: fog_buffer.as_entire_binding(),
        }],
    });

    // **Two bindings in one group, not two groups.** wgpu's downlevel limit is
    // four bind groups and 0 to 3 are already the uniforms, the texture, the
    // fog and this - so the node matrices ride here as binding 1 rather than
    // claiming a fifth group, which fails shader creation outright.
    let anim_entry = |binding| wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::VERTEX,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    };
    let anim_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("animation"),
        entries: &[anim_entry(0), anim_entry(1)],
    });

    // Initialised to all-identity for the same reason the fog buffer is
    // initialised to `Fog::off`: a caller that never writes it draws every
    // surface at its authored, unanimated coordinates.
    let anim_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("texture animation"),
        size: TEX_ANIMS_SIZE,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    queue.write_buffer(&anim_buffer, 0, bytemuck::bytes_of(&TexAnims::default()));
    // All-identity for the same reason the texture table is: a caller that never
    // writes it draws every mesh where the file's static chain puts it.
    let node_anim_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("node animation"),
        size: NODE_ANIMS_SIZE,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    queue.write_buffer(
        &node_anim_buffer,
        0,
        bytemuck::bytes_of(&NodeAnims::default()),
    );
    let anim_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("animation"),
        layout: &anim_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: anim_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: node_anim_buffer.as_entire_binding(),
            },
        ],
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("mesh"),
        bind_group_layouts: &[
            Some(&layout),
            Some(&texture_layout),
            Some(&fog_layout),
            Some(&anim_layout),
        ],
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
                    4 => Float32, 5 => Uint32, 6 => Float32x2, 7 => Uint32, 8 => Float32,
                    9 => Uint32
                ],
            })],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            // Alpha is the bloom's glow mask - see [`GlowMask`].
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: None,
                write_mask: glow.writes(),
            })],
            compilation_options: wgpu::PipelineCompilationOptions {
                constants,
                ..Default::default()
            },
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
                    4 => Float32, 5 => Uint32, 6 => Float32x2, 7 => Uint32, 8 => Float32,
                    9 => Uint32
                ],
            })],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main_alpha_test"),
            // Alpha is the bloom's glow mask - see [`GlowMask`].
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: None,
                write_mask: glow.writes(),
            })],
            compilation_options: wgpu::PipelineCompilationOptions {
                constants,
                ..Default::default()
            },
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
                        4 => Float32, 5 => Uint32, 6 => Float32x2, 7 => Uint32, 8 => Float32,
                        9 => Uint32
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
                    // Alpha is the bloom's glow mask - see [`GlowMask`] for
                    // which draw paths are allowed to write it and why.
                    write_mask: glow.writes(),
                })],
                compilation_options: wgpu::PipelineCompilationOptions {
                    constants,
                    ..Default::default()
                },
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
    // One pair per *distinct* state the model's transparent draws name, in
    // first-seen order. A linear scan over a handful of entries rather than a
    // map: `wgpu::BlendState` is not `Hash`, the count is five on the widest
    // circuit measured, and first-seen order keeps a rebuild of the same file
    // byte-identical - the same reason `Model::anim_tracks` is ordered that way.
    let mut authored_pipelines: Vec<(wgpu::BlendState, [wgpu::RenderPipeline; 2])> = Vec::new();
    for state in model
        .transparent_draws
        .iter()
        .filter_map(|draw| draw.blend_state)
    {
        if authored_pipelines.iter().any(|(seen, _)| *seen == state) {
            continue;
        }
        let label = format!("mesh blend (authored {state:?})");
        authored_pipelines.push((
            state,
            [
                make_pipeline(&label, Some(state), false),
                make_pipeline(&label, Some(state), true),
            ],
        ));
    }

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
        texture.create_view(&wgpu::TextureViewDescriptor::default())
    };

    // One bind group per material slot: its albedo and its lightmap, which the
    // shader multiplies. Split from `make` because the two are separate
    // textures paired per slot rather than one texture per group.
    let bind = |albedo: &wgpu::TextureView, lightmap: &wgpu::TextureView, label: &str| {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(label),
            layout: &texture_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(albedo),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(lightmap),
                },
            ],
        })
    };

    // A white 1x1 stands in for untextured draws, so the shader needs no branch.
    // It also fills the slot of a `Texture` node we could not decode, keeping
    // every later texture at the index its materials expect.
    //
    // A draw with no lightmap binds **black with alpha 1** instead. Black
    // switches the prelit term off, which is what a surface with no lightmap
    // gets in the original too. The alpha of 1 now gates **only the
    // specular** - the diffuse sun term it used to gate as well was removed
    // from `mesh.wgsl` once `track_surface.rcsmaterial` block #8 was read and
    // turned out to light a lightmap-less surface with `(f[TC1] + k) *
    // albedo`, no `N.L` and no sun colour at all. See renderer.md, "The lit
    // track material". Full gloss is the honest default there: the specular
    // block that *was* read (#7) masks by the lightmap's own alpha, and a
    // surface with no lightmap has no mask to apply.
    let white = make(1, 1, &[255, 255, 255, 255], "white");
    let no_lightmap = make(1, 1, &[0, 0, 0, 255], "no lightmap");
    let mut texture_binds = vec![bind(&white, &no_lightmap, "white")];
    for (index, slot) in model.textures.iter().enumerate() {
        let albedo = match slot {
            Some(t) => make(t.width, t.height, &t.rgba, &t.label),
            None => make(1, 1, &[255, 255, 255, 255], "undecoded"),
        };
        let lightmap = match model.lightmaps.get(index).and_then(Option::as_ref) {
            Some(t) => make(t.width, t.height, &t.rgba, &t.label),
            None => make(1, 1, &[0, 0, 0, 255], "no lightmap"),
        };
        let label = slot.as_ref().map_or("undecoded", |t| t.label.as_str());
        texture_binds.push(bind(&albedo, &lightmap, label));
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
        anim_bind,
        anim_buffer,
        node_anim_buffer,
        fog_bind,
        fog_buffer,
        pipeline,
        alpha_test_pipeline,
        blend_pipeline,
        additive_pipeline,
        unblended_pipeline,
        authored_pipelines,
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
        capture_from(&model, &path, 64, 64, 0.9, 0.85, Anisotropy::default(), 0.0)
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
