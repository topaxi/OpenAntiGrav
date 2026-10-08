//! Renders a decoded model, offscreen or into a surface.
//!
//! Deliberately plain: depth buffer, perspective camera, two-light rig. The
//! point is to see the geometry clearly and catch decoding errors, not to
//! reproduce Pulse's look.

use anyhow::Result;

use crate::mesh::Model;

mod blend;
mod glow_mask;
mod hull_lights;
mod spu_light;
mod stamp;
mod tables;
mod uniforms;

mod texlod;
pub use blend::{ADDITIVE_BLEND, TRANSPARENT_BLEND, TransparentPipelines};
pub use glow_mask::GlowMask;
pub use hull_lights::{HULL_LIGHTS, HullLights, ge_channel};
pub use spu_light::{MAX_SPU_LIGHTS, RGBE_ROUND_TRIP, SpuLight, SpuLights};
pub use stamp::Stamp;
pub use tables::{EMISSIVES_SIZE, Emissives, NODE_ANIMS_SIZE, NodeAnims, TEX_ANIMS_SIZE, TexAnims};
pub use texlod::{PSP_TEXLOD_BIAS, PSP_TEXLOD_SLOPE, TextureDetail};
use uniforms::Uniforms;
mod prepass;
mod velocity;
mod vertex_layout;
pub use prepass::Prepass;
pub use uniforms::{
    DEPTH_FORMAT, Fog, Light, SCENE_SIZE, Scene, ShadowMap, ShadowMaps, ShadowReceiver,
    UNIFORMS_SIZE, Zone, ZoneSet, write_uniforms, write_uniforms_raw,
};
pub use velocity::Velocity;
pub(crate) use velocity::velocity_targets;
pub use vertex_layout::Texcoords;

/// Headless capture, split into `crate::capture` so a pixel-returning entry
/// point could be added there without pushing this file past its frozen
/// size ceiling - see [`crate::capture`] for both functions' own docs.
pub use crate::capture::{capture_from, capture_pixels_from};

pub mod cutout;
pub use cutout::CutoutPipelines;

mod target;
pub use target::{fragment_options, is_linear_target, linear_constants};

mod anisotropy;
pub use anisotropy::Anisotropy;

mod pipeline_cache;
pub use pipeline_cache::Scope as BuildCacheScope;

mod shadow_map;
pub use shadow_map::{
    COVERAGE_FORMAT, OCCLUSION_FORMAT, OCCLUSION_LAYERS, SELF_SHADOW_FORMAT, SHADOW_DEPTH_FORMAT,
};
mod texture;
mod texture_sink;
pub use texture_sink::Scope as TextureSinkScope;
pub(crate) use texture_sink::offer as offer_to_texture_sink;
pub use texture_sink::without_scope as without_texture_sink;
pub mod zone;

mod device;
pub use device::{device_descriptor, optional_features, required_limits};

/// Everything [`build`] hands back: geometry, texture bindings, and the three
/// pipelines a `Model` draws through.
#[derive(Debug)]
pub struct Built {
    /// Opaque pass: depth write on, no blending. Draws [`Model::draws`].
    pub pipeline: wgpu::RenderPipeline,
    /// The depth prepass pair, when [`build_with`] was asked for it - see
    /// [`Prepass`].
    pub prepass: Option<Prepass>,
    /// Cutout pass: depth write on, no blending, `discard` below
    /// `shaders/alpha_test.wesl`'s `alpha_test_ref`. Draws [`Model::alpha_tested_draws`] as
    /// a second `set_pipeline` in the same render pass as `pipeline`, before
    /// `blend_pipeline`.
    ///
    /// **The reference is per model**, not a constant in the shader: a
    /// Wipeout HD material authors it (`GL_GREATER`/`0.5` disc-wide - see
    /// [`Model::alpha_test_ref`] and `mesh::rcs::cutout`) and [`build`]
    /// pushes it here as a pipeline override. A model that authors none keeps
    /// the shader's default, which is the PSP reference `mesh.wesl`'s own
    /// `ALPHA_TEST_THRESHOLD` carries the evidence for.
    pub alpha_test_pipeline: wgpu::RenderPipeline,
    /// One cutout pipeline per alpha-test reference the model's own batches
    /// ask for, in first-seen order - see [`cutout`] for the recovery and the
    /// measured cost, and [`CutoutPipelines::select`] for the pick. Empty for
    /// a model built from anything but a `.vex`.
    pub cutout_pipelines: Vec<(f32, wgpu::RenderPipeline)>,
    /// Blended pass: depth write off, the `blend` the caller passed to
    /// [`build`] - [`TRANSPARENT_BLEND`] for ordinary scene geometry. Draws
    /// [`Model::transparent_draws`] last, as a third `set_pipeline` in the
    /// same render pass - see `oag_fx::exhaust::Pipeline` for the precedent
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
    /// Alpha-only pass that stamps a blended batch's glow byte into the mask,
    /// drawn over [`Model::transparent_draws`] after their colour - see
    /// [`stamp`]. `Some` only for a [`GlowMask::Stamped`] model, and indexed by
    /// `culled as usize` like [`Built::blend_pipeline`], one pair per blend class.
    pub stamp_pipeline: Option<Stamp>,
    pub bind_group: wgpu::BindGroup,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub texture_binds: Vec<wgpu::BindGroup>,
    /// Bind group 2, holding [`Scene`] - [`Fog`] **and** [`Light`]. Bound by
    /// every draw; write [`Built::fog_buffer`] to change it.
    pub fog_bind: wgpu::BindGroup,
    /// The buffer behind [`Built::fog_bind`], initialised to [`Scene::off`].
    pub fog_buffer: wgpu::Buffer,
    /// The Zone visualiser's own lookup texture, bound at bind group 2's
    /// binding 4 - see [`zone::layout_entries`]. All-black until
    /// [`zone::write_vis`] is called on it; write it once a frame to animate
    /// the glow, the same rhythm [`Built::fog_buffer`] already runs at.
    pub zone_vis_texture: wgpu::Texture,
    /// What [`Built::fog_bind`] needs besides [`Built::fog_buffer`] and the
    /// stage's own four textures to be rebuilt on a stage-change edge - see
    /// [`zone::RebindResources`] and [`zone::rebind`], which
    /// `race::Drawable` calls with this.
    pub zone_rebind: zone::RebindResources,
    /// Bind group 3, holding [`TexAnims`]. Bound by every draw; write
    /// [`Built::anim_buffer`] once a frame to animate.
    pub anim_bind: wgpu::BindGroup,
    /// The buffer behind [`Built::anim_bind`], initialised to all-identity.
    pub anim_buffer: wgpu::Buffer,
    /// The [`NodeAnims`] buffer, bound as **binding 1 of the same group 3** as
    /// [`Built::anim_buffer`]. Write it once a frame to move scenery.
    pub node_anim_buffer: wgpu::Buffer,
    /// The [`Emissives`] buffer, **binding 2 of the same group 3**. Written once
    /// from the model's glow table; rewrite it to retint an entry per frame.
    pub emissive_buffer: wgpu::Buffer,
}

/// Which depth state [`build`] gives a model's pipelines.
///
/// The race pass clears depth to 1.0 and everything else compares `Less`, so a
/// sky drawn at the far plane would fail the test and be invisible. Rather than
/// place it in depth by its geometry, [`Depth::Sky`] writes nothing and
/// compares `LessEqual`, and the race draws it after the solid circuit with its
/// viewport's depth range pinned to 1.0 - so it passes on exactly the samples
/// still at the clear value, the ones no solid geometry covers. See
/// `race::Scene::draw_track`. Drawn first instead, it passes everywhere (its
/// cube sits well short of the far plane) and every later draw covers it,
/// which is how it was drawn until 2026-10-03 and costs a full frame of
/// shading. Either way the sky needs no far plane large enough to contain it,
/// which matters because its authored cube is tens of units across while a
/// track is thousands.
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
    /// A second pass over geometry already drawn - the absorb hull overlay,
    /// `crate::hull_overlay`: test `LessEqual` so a coplanar redraw passes,
    /// write nothing. The original tests `EQUAL`; this is the chosen stand-in.
    Overlay,
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
    velocity: Velocity,
    zone: &zone::StageArt,
    shadow_maps: ShadowMaps<'_>,
    receives_shadow: ShadowReceiver,
) -> Result<Built> {
    build_with(
        device,
        queue,
        model,
        format,
        anisotropy,
        sample_count,
        depth,
        blend,
        glow,
        velocity,
        zone,
        shadow_maps,
        receives_shadow,
        Texcoords::Interleaved,
        false,
        false,
    )
}

/// [`build`], with the coordinates' source chosen - see [`Texcoords`].
#[allow(clippy::too_many_arguments)]
pub fn build_with(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    model: &Model,
    format: wgpu::TextureFormat,
    anisotropy: Anisotropy,
    sample_count: u32,
    depth: Depth,
    blend: wgpu::BlendState,
    glow: GlowMask,
    velocity: Velocity,
    // The Zone stage's two textures, or [`zone::StageArt::NONE`] outside an
    // HD Zone race - see that type for why there are two.
    zone: &zone::StageArt,
    // The frame's shadow maps, or [`ShadowMaps::NONE`] for the placeholders -
    // see `shadow_map::resources`. A `Built` binds whichever it was given for
    // its whole life, so a caller that gains a map mid-race rebuilds rather
    // than rebinding, the same way a Zone stage texture does.
    shadow_maps: ShadowMaps<'_>,
    // Which maps this model's surfaces may read - see `ShadowReceiver`, which
    // carries why there are three states. A pipeline constant rather than a
    // uniform field keeps it a property of the model, like `flame_*` and
    // `colour_is_light` beside it.
    receives_shadow: ShadowReceiver,
    texcoords: Texcoords,
    // Whether to build [`Built::prepass`] - see [`Prepass`].
    prepass: bool,
    // Whether the opaque and cutout pipelines cull back faces, which no model
    // asks for but Wipeout HD's behind-the-glass target (`oag_raceplay`'s
    // `scene::behind_glass`), whose every chunk draw the original culls.
    cull_back: bool,
) -> Result<Built> {
    // The blended pipeline never writes depth whichever role this is; the rest
    // is what [`Depth`] chooses between.
    let (depth_write, depth_compare) = match depth {
        Depth::Scene => (true, wgpu::CompareFunction::Less),
        Depth::Sky => (false, wgpu::CompareFunction::LessEqual),
        Depth::Overlay => (false, wgpu::CompareFunction::LessEqual),
    };

    // Shared across every drawable an open `pipeline_cache::Scope` covers -
    // `race::Scene::new`'s whole build - rather than parsed fresh per model;
    // see `pipeline_cache` for why and by how much. A caller with no scope
    // open (the viewer, every test but the cache's own) gets exactly the
    // fresh module this always created.
    let shader = pipeline_cache::shared_shader_module(device);
    let shader = &shader;
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
    // A fourth: the alpha-test reference the model's own materials author,
    // which on Wipeout HD is the `GL_GREATER`/`0.5` pair its mode-2 materials
    // carry. Left off for a model with none, so `mesh.wesl`'s own default -
    // the recovered PSP reference - stands. See `mesh::rcs::cutout`.
    if let Some(reference) = model.alpha_test_ref {
        constants.push(("alpha_test_ref", f64::from(reference)));
    }
    // A model that stamps the mask stamps it whatever the call site asked -
    // see [`Model::stamps_glow`]. That is also what stops the PSP plume and
    // shield writing it: their batches are transparent without the glow bits.
    let glow = match (model.stamps_glow, model.glow_by_texel) {
        (true, true) => GlowMask::StampedByTexel,
        (true, false) => GlowMask::Stamped,
        // A PS2 model that opts out of the mask: it writes colour only.
        (false, true) => GlowMask::Protected,
        (false, false) => glow,
    };
    if glow.stamps() {
        constants.push(("glow_stamp", 1.0));
    }
    if glow == GlowMask::StampedByTexel {
        constants.push(("glow_texel", 1.0));
    }
    if receives_shadow != ShadowReceiver::Never {
        constants.push(("receives_shadow", receives_shadow.constant()));
    }
    constants.extend(texlod::constants(model, anisotropy));
    if let Some(flame) = model.flame {
        constants.extend([
            ("flame_shading", 1.0),
            ("flame_rim_power", f64::from(flame.rim_power)),
            ("flame_rim_scale", f64::from(flame.rim_scale)),
            ("flame_rim_min", f64::from(flame.rim_min)),
            ("flame_alpha_scale", f64::from(flame.alpha_scale)),
            ("flame_colour_scale", f64::from(flame.colour_scale)),
            ("flame_speed", f64::from(flame.scroll_speed)),
        ]);
    }
    // HD's absorb shell program, whose two numbers are literals in its own
    // microcode rather than material parameters - see `crate::absorb_shell`.
    if model.absorb_shell {
        constants.push(("absorb_shading", 1.0));
    }
    constants.extend(pipeline_cache::scope_constants());
    let constants = constants.as_slice();

    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("mesh"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            // Both stages: the fragment stage reads `sun_occlusion_layer`,
            // which picks a hull's layer of the occlusion array.
            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    });

    let texture_layout = material_bind_group_layout(device);

    // Bind group 2 whole - the `Scene` layout and buffer, the Zone stage's
    // four textures and the shadow map - lives in `zone.rs` now, under the
    // 1,000-line rule in `scripts/check-file-size.py`: this file had no room
    // left to grow the one binding set a Zone stage change touches. See
    // [`zone::scene_bind_group`] and [`zone::rebind`], which
    // `race::Drawable` calls on the stage-change edge.
    let (fog_layout, fog_buffer, fog_bind, zone_vis_texture, zone_rebind) =
        zone::scene_bind_group(device, queue, anisotropy, zone, shadow_maps);

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
    // Binding 2 is the glow table, and it is the one of the three the
    // *fragment* stage reads: the sample and its tint are per pixel, where the
    // two coordinate transforms above are per vertex.
    let anim_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("animation"),
        entries: &[
            anim_entry(0),
            anim_entry(1),
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
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
    // **Written here and never again**: every value in it is authored, and the
    // only moving part - the clock - is already a scene uniform. A model with
    // no glow layers writes an all-zero table, which adds nothing.
    let emissive_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("emissive glow"),
        size: EMISSIVES_SIZE,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    queue.write_buffer(
        &emissive_buffer,
        0,
        bytemuck::bytes_of(&Emissives::of(model)),
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
            wgpu::BindGroupEntry {
                binding: 2,
                resource: emissive_buffer.as_entire_binding(),
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

    // The vertex layout never varies with the model, so it is built once and
    // shared by reference across every `create_render_pipeline` call below,
    // in place of the three identical inline copies this used to be.
    let streamed = vertex_layout::streamed_attributes();
    let vertex_buffers = vertex_layout::buffers(texcoords, &streamed);
    // Shared by the opaque and the cutout pipeline below - identical for
    // both, so it is computed once. `pipeline_cache::cached_pipeline` still
    // keys each of its own callers separately; this only avoids constructing
    // the same value twice.
    let primitive = wgpu::PrimitiveState {
        // Culling is off on purpose unless asked for. Strip winding is
        // reconstructed rather than read from the file, so culling would turn
        // any mistake there into invisible geometry instead of an artefact.
        cull_mode: cull_back.then_some(wgpu::Face::Back),
        ..Default::default()
    };
    let multisample = wgpu::MultisampleState {
        count: sample_count,
        ..Default::default()
    };

    let targets = velocity_targets(
        wgpu::ColorTargetState {
            format,
            blend: None,
            write_mask: glow.writes(),
        },
        velocity.target(false),
    );
    let depth_stencil = wgpu::DepthStencilState {
        format: DEPTH_FORMAT,
        depth_write_enabled: Some(depth_write),
        depth_compare: Some(depth_compare),
        stencil: Default::default(),
        bias: Default::default(),
    };
    // Alpha is the bloom's glow mask - see [`GlowMask`]. The second target,
    // when [`Velocity::Write`] adds one, takes this surface's real screen
    // motion - this pipeline writes depth.
    let fragment_entry = velocity.entry("fs_main", "fs_main_velocity");
    let pipeline = pipeline_cache::cached_pipeline(
        "vs_main",
        &vertex_buffers,
        fragment_entry,
        &targets,
        primitive,
        Some(depth_stencil.clone()),
        multisample,
        constants,
        || {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("mesh"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: shader,
                    entry_point: Some("vs_main"),
                    buffers: &vertex_buffers,
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: shader,
                    entry_point: Some(fragment_entry),
                    targets: &targets,
                    compilation_options: wgpu::PipelineCompilationOptions {
                        constants,
                        ..Default::default()
                    },
                }),
                primitive,
                depth_stencil: Some(depth_stencil.clone()),
                multisample,
                multiview_mask: None,
                cache: None,
            })
        },
    );

    let prepass = prepass.then(|| {
        prepass::pipelines(&prepass::Shared {
            device,
            shader,
            layout: &pipeline_layout,
            vertex_buffers: &vertex_buffers,
            constants,
            velocity,
            targets: &targets,
            fragment_entry,
            primitive,
            multisample,
        })
    });

    // Second pipeline for `Model::alpha_tested_draws`: same shader module,
    // bind group layouts, vertex layout and depth state as the opaque
    // pipeline (a cutout is meant to occlude and be occluded exactly like
    // opaque geometry), except the fragment shader discards pixels below a
    // threshold instead of always returning alpha 1.0.
    //
    // Alpha is the bloom's glow mask - see [`GlowMask`]. A cutout writes
    // depth like the opaque pipeline, so it writes real velocity too - same
    // `targets` as the opaque pipeline above, reused rather than rebuilt.
    let cutout_fragment_entry = velocity.entry("fs_main_alpha_test", "fs_main_alpha_test_velocity");
    let make_cutout = |label: &str, reference: Option<f32>| {
        // A per-batch reference overrides the model-level one already in
        // `constants`; `wgpu` rejects a duplicate key, so it replaces rather
        // than shadows.
        let mut constants = constants.to_vec();
        if let Some(reference) = reference {
            constants.retain(|(name, _)| *name != "alpha_test_ref");
            constants.push(("alpha_test_ref", f64::from(reference)));
        }
        let depth_stencil = wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: Some(depth_write),
            depth_compare: Some(cutout::depth_compare(reference, depth_compare)),
            stencil: Default::default(),
            bias: Default::default(),
        };
        pipeline_cache::cached_pipeline(
            "vs_main",
            &vertex_buffers,
            cutout_fragment_entry,
            &targets,
            primitive,
            Some(depth_stencil.clone()),
            multisample,
            &constants,
            || {
                device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some(label),
                    layout: Some(&pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: shader,
                        entry_point: Some("vs_main"),
                        buffers: &vertex_buffers,
                        compilation_options: Default::default(),
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: shader,
                        entry_point: Some(cutout_fragment_entry),
                        targets: &targets,
                        compilation_options: wgpu::PipelineCompilationOptions {
                            constants: &constants,
                            ..Default::default()
                        },
                    }),
                    primitive,
                    depth_stencil: Some(depth_stencil.clone()),
                    multisample,
                    multiview_mask: None,
                    cache: None,
                })
            },
        )
    };
    let alpha_test_pipeline = make_cutout("mesh alpha test", None);
    let cutout_pipelines = cutout::pipelines(&model.alpha_tested_draws, |label, reference| {
        make_cutout(label, Some(reference))
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
    // Depth state never varies across this pipeline's callers - blended
    // geometry never writes depth, whichever `Depth` this model asked for -
    // so it is computed once outside the closure rather than per call.
    let blend_depth_stencil = wgpu::DepthStencilState {
        format: DEPTH_FORMAT,
        depth_write_enabled: Some(false),
        depth_compare: Some(depth_compare),
        stencil: Default::default(),
        bias: Default::default(),
    };
    let make_pipeline = |label: &str, blend: Option<wgpu::BlendState>, cull: bool| {
        // The second target rides along **write-masked empty** when
        // [`Velocity::Write`] adds one: a blended draw writes no depth, so
        // the velocity at its pixels stays the surface's behind it - see
        // [`Velocity`]. Alpha is the bloom's glow mask - see [`GlowMask`] for
        // which draw paths are allowed to write it and why.
        let targets = velocity_targets(
            wgpu::ColorTargetState {
                format,
                blend,
                write_mask: glow.blend_writes(),
            },
            velocity.target(true),
        );
        let primitive = wgpu::PrimitiveState {
            cull_mode: cull.then_some(wgpu::Face::Back),
            ..Default::default()
        };
        pipeline_cache::cached_pipeline(
            "vs_main",
            &vertex_buffers,
            "fs_main_blend",
            &targets,
            primitive,
            Some(blend_depth_stencil.clone()),
            multisample,
            constants,
            || {
                device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some(label),
                    layout: Some(&pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: shader,
                        entry_point: Some("vs_main"),
                        buffers: &vertex_buffers,
                        compilation_options: Default::default(),
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: shader,
                        entry_point: Some("fs_main_blend"),
                        targets: &targets,
                        compilation_options: wgpu::PipelineCompilationOptions {
                            constants,
                            ..Default::default()
                        },
                    }),
                    primitive,
                    depth_stencil: Some(blend_depth_stencil.clone()),
                    multisample,
                    multiview_mask: None,
                    cache: None,
                })
            },
        )
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

    let stamp_pipeline = glow.stamps().then(|| {
        stamp::pipelines(&stamp::Shared {
            device,
            shader,
            layout: &pipeline_layout,
            vertex_buffers: &vertex_buffers,
            constants,
            velocity,
            format,
            multisample,
            depth_stencil: &blend_depth_stencil,
        })
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
        texture::upload_rgba(device, queue, width, height, rgba, label, None)
            .expect("a 1x1 texture fits every device")
            .view
    };

    // One bind group per material slot: its albedo and its lightmap, which the
    // shader multiplies. Split from `make` because the two are separate
    // textures paired per slot rather than one texture per group.
    let bind = |albedo: &wgpu::TextureView,
                lightmap: &wgpu::TextureView,
                pad_mask: &wgpu::TextureView,
                wave: &wgpu::TextureView,
                label: &str| {
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
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(pad_mask),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(wave),
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
    // from `mesh.wesl` once `track_surface.rcsmaterial` block #8 was read and
    // turned out to light a lightmap-less surface with `(f[TC1] + k) *
    // albedo`, no `N.L` and no sun colour at all. See renderer.md, "The lit
    // track material". Full gloss is the honest default there: the specular
    // block that *was* read (#7) masks by the lightmap's own alpha, and a
    // surface with no lightmap has no mask to apply.
    let white = make(1, 1, &[255, 255, 255, 255], "white");
    let no_lightmap = make(1, 1, &[0, 0, 0, 255], "no lightmap");
    // A material with no pad mask binds a flat tangent-space normal
    // (128, 128, 255) with zero alpha: the identity of both terms the mask
    // feeds, though `slots::PAD_NE` gates the read so it is not sampled at all.
    let no_pad_mask = make(1, 1, &[128, 128, 255, 0], "no pad mask");

    // **One upload per distinct texture, not one per slot naming it.** The
    // slots are positional (a chunk names its material by ordinal) and a
    // circuit's lightmap atlas is named by 275 of Talon's Junction's 442, so
    // uploading per slot sent the same 175 pictures to the GPU 884 times -
    // 1,858 MiB in place of 277 MiB. The key is the shared texture's own
    // address, not its label: `mesh::rcs::skin::skin`'s decode cache is what
    // makes two slots hold one `Arc`, and that is exactly the relation worth
    // preserving here.
    let blocks = device
        .features()
        .contains(wgpu::Features::TEXTURE_COMPRESSION_BC);
    //
    // **`None` is a texture no level of which fits the device** (see
    // `texture::upload`): it takes the same fallback as a slot that never
    // decoded, so a refusal is the existing "undecoded" case rather than a
    // second mechanism.
    let mut views: std::collections::HashMap<usize, Option<wgpu::TextureView>> = Default::default();
    let mut view_of = |texture: &std::sync::Arc<crate::mesh::ModelTexture>| {
        views
            .entry(std::sync::Arc::as_ptr(texture) as usize)
            .or_insert_with(|| texture::upload_shared(device, queue, texture, blocks))
            .clone()
    };

    let mut texture_binds = vec![bind(&white, &no_lightmap, &no_pad_mask, &white, "white")];
    for (index, slot) in model.textures.iter().enumerate() {
        let albedo = slot
            .as_ref()
            .and_then(&mut view_of)
            .unwrap_or_else(|| make(1, 1, &[255, 255, 255, 255], "undecoded"));
        let lightmap = model
            .lightmaps
            .get(index)
            .and_then(Option::as_ref)
            .and_then(&mut view_of)
            .unwrap_or_else(|| make(1, 1, &[0, 0, 0, 255], "no lightmap"));
        let pad_mask = model
            .pad_masks
            .get(index)
            .and_then(Option::as_ref)
            .and_then(&mut view_of)
            .unwrap_or_else(|| no_pad_mask.clone());
        let wave = model
            .wave_maps
            .get(index)
            .and_then(Option::as_ref)
            .and_then(&mut view_of)
            .unwrap_or_else(|| white.clone());
        let label = slot.as_ref().map_or("undecoded", |t| t.label.as_str());
        texture_binds.push(bind(&albedo, &lightmap, &pad_mask, &wave, label));
    }

    texture::log_census(model, blocks, device.limits().max_texture_dimension_2d);

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
        prepass,
        anim_bind,
        anim_buffer,
        node_anim_buffer,
        emissive_buffer,
        fog_bind,
        fog_buffer,
        zone_vis_texture,
        zone_rebind,
        pipeline,
        alpha_test_pipeline,
        cutout_pipelines,
        blend_pipeline,
        additive_pipeline,
        unblended_pipeline,
        authored_pipelines,
        stamp_pipeline,
        bind_group,
        vertex_buffer,
        index_buffer,
        texture_binds,
    })
}

mod material_layout;
pub use material_layout::material_bind_group_layout;

mod albedo_bind;
pub use albedo_bind::albedo_bind_group;
