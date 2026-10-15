//! The caster half of Wipeout HD's model shadows: what fills `shadowMapTex`.
//!
//! `Job RenderModelShadowMaps` generates it and
//! `Job RenderModelShadowsOnTrack` composites it, at positions 4 and 9 of the
//! thirteen-job frame order
//! ([`rcsmaterial.md`](../../../docs/formats/rcsmaterial.md)). What the map
//! holds is settled by the receiving side rather than by either job's name:
//! the track material samples it projectively and uses the sample as
//! `1 - shadow` with no compare, so it is **coverage**, not depth. See
//! [`oag_mesh::mesh_render::ShadowMap`].
//!
//! # What is the original's and what is ours
//!
//! **The original's**: that models are rendered into a map from the light, that
//! the receiver is the track, that the sample is projective, and the sun
//! direction the projection is built around
//! ([`envsettings.md`](../../../docs/formats/envsettings.md)).
//!
//! **One map per craft**, as the original keeps (`Shadow_RenderModelShadowMaps`
//! draws each ship into a map of its own, fitted to that ship alone), at the
//! sides it allocates by the ship's distance rank ([`RANK_SIDES`]), and each
//! craft's map is read by the road through its own matrix, the maps' coverage
//! combined the way the original's one redraw per ship blends. See
//! [`Map::render`].
//!
//! **Ours**: where the maps live (one atlas, [`ATLAS_WIDTH`] by
//! [`ATLAS_HEIGHT`], one render pass, instead of eight textures bound at
//! eight slots), the box's depth bounds on the receiving side, and what the
//! rank is measured from (the caller's choice). The box is the one the
//! sun-occlusion map already uses ([`Fit::craft`]), which the original shares
//! between a ship's maps. How dark a covered texel draws is the original's:
//! the road lit without its sun, which `mesh`'s `shadowed` reproduces.

use oag_core::math::{Mat4, Vec3, camera};

use oag_mesh::mesh::GpuVertex;
use oag_mesh::mesh_render::{COVERAGE_FORMAT, CraftShadow, SHADOW_DEPTH_FORMAT};

/// The side of each craft's map, in texels, by the craft's rank in distance
/// from the reference point, nearest first.
///
/// **The original's**: `Shadow_AllocShipMapPool` allocates eight depth maps of
/// these sides (`g_ShipMapSizes`,
/// [`ship-sun-occlusion.md`](../../../../docs/ghidra/functions/ps3-hdfury-eu/ship-sun-occlusion.md))
/// and `Ship_RankAndAssignMapSlots` hands them out nearest first by squared
/// distance
/// ([`shadow-track-redraw.md`](../../../../docs/ghidra/functions/ps3-hdfury-eu/shadow-track-redraw.md)).
pub const RANK_SIDES: [u32; MAX_CASTERS] = [512, 512, 256, 128, 64, 32, 32, 32];

/// The coverage atlas's width, in texels.
pub const ATLAS_WIDTH: u32 = 1024;

/// The coverage atlas's height, in texels.
pub const ATLAS_HEIGHT: u32 = 768;

/// Where each rank's square sits in the atlas: `(x, y, side)` in texels.
///
/// **Ours.** The two 512 squares side by side on top, the rest in a row below.
/// The squares tile no more than 613,376 of the atlas's 786,432 texels (the
/// rank sum) and overlap nowhere; a test pins that.
pub const RANK_SQUARES: [(u32, u32, u32); MAX_CASTERS] = [
    (0, 0, 512),
    (512, 0, 512),
    (0, 512, 256),
    (256, 512, 128),
    (384, 512, 64),
    (448, 512, 32),
    (480, 512, 32),
    (512, 512, 32),
];

/// Half the side of the box each craft's maps are fitted to, in world units:
/// 12, the original's fallback box (`(-6, -6, -6)..(6, 6, 6)`,
/// `Shadow_BuildShipShadowMatrices`) plus a margin of 6 so a hull's silhouette
/// never clips its own map's edge. **The margin is ours**; the original fits
/// the box to the ship's own bbox with none.
pub const CRAFT_HALF_EXTENT: f32 = 12.0;

/// The radius of the sphere around a craft outside which the receiver skips its
/// map without projecting, in world units. **Not a new bound**: it encloses the
/// receiver's slab (50 units past the craft along the sun, the box's 12-unit
/// half width across it, so 52.8 at the corner) with room to spare, and the
/// original redraws the track chunks within 50 units of each ship too.
pub const CRAFT_CULL_RADIUS: f32 = 56.0;

/// How far each craft's box reaches along the sun either side of the craft,
/// in world units.
///
/// **The original's**: `Shadow_BuildShipShadowMatrices` looks from 70 units up
/// the sun with near 1 and far 140
/// ([`shadow-model-maps.md`](../../../../docs/ghidra/functions/ps3-hdfury-eu/shadow-model-maps.md)),
/// and every map a ship carries is projected through that one matrix.
pub const CRAFT_REACH: f32 = 70.0;

/// How far behind the casters the light's own near plane sits, in world units.
///
/// **Ours, and it has to be generous**: a caster is only in the map if the
/// light's frustum contains it, and the fit below is built from the casters'
/// own bounding sphere rather than from the scene, so anything above them -
/// a bridge, a tube's ceiling - is *not* a caster and does not need to fit.
const NEAR_MARGIN: f32 = 8.0;

/// Where the light's view is placed and how much it covers.
///
/// A separate type because the fit is the part of this that is ours: it is
/// stated once, here, rather than inlined into a matrix builder where nobody
/// would find it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fit {
    /// Centre of what the map has to cover, in world space.
    pub centre: Vec3,
    /// Radius of the same, in world units.
    pub radius: f32,
    /// Unit vector **towards** the light, matching
    /// [`oag_mesh::mesh_render::Light::direction`].
    pub towards_light: Vec3,
}

impl Fit {
    /// The box every map a craft carries is fitted to: centred on the craft,
    /// [`CRAFT_HALF_EXTENT`] each way across the sun, looking down it from
    /// [`CRAFT_REACH`] units up. The coverage maps, the sun-occlusion map and
    /// the self-shadow map all take it, because the original projects a ship's
    /// maps through one matrix.
    #[must_use]
    pub fn craft(centre: Vec3, towards_light: Vec3) -> Self {
        Self {
            centre,
            radius: CRAFT_HALF_EXTENT,
            towards_light,
        }
    }

    /// World to a craft map's clip space: [`Self::matrix_reaching`] at
    /// [`CRAFT_REACH`], the original's depth for a ship's box.
    #[must_use]
    pub fn craft_matrix(&self) -> Mat4 {
        self.matrix_reaching(CRAFT_REACH)
    }

    /// The light direction actually used for the view: [`Self::towards_light`],
    /// or a fallback that keeps the view non-degenerate.
    ///
    /// Shared by [`Self::matrix`] and [`Self::snapped`] so both build the same
    /// basis from the same fallback - two independent readings of "is this
    /// direction usable" would let them disagree on a degenerate rig.
    fn resolved_direction(&self) -> Vec3 {
        let direction = self.towards_light.normalize_or_zero();
        if direction.length_squared() < 0.5 {
            // A rig with no usable direction points straight down rather than
            // producing a degenerate view: nothing is shadowed either way,
            // since the strength that reaches the shader is the caller's.
            Vec3::NEG_Y
        } else {
            direction
        }
    }

    /// World to the map's clip space: an orthographic box down the light,
    /// sized to [`Self::radius`].
    ///
    /// **Orthographic because the light is directional.** The sun in an
    /// `.envsettings` is a direction and nothing else - there is no position to
    /// build a perspective frustum around.
    ///
    /// **Fitted to the casters, not to the track**, which is the whole reason
    /// this type exists: a track-sized box is metres per texel and a craft's
    /// shadow becomes a smear.
    ///
    /// **The box's own axes come from [`Self::towards_light`] alone - never
    /// from a caster.** Whatever moves [`Self::centre`] between calls (a
    /// caster's own position, a lead ahead of it) only ever *translates* the
    /// box; nothing here rotates it. A caller reporting the box as
    /// "rotating" is describing that translation swinging in an arc as its
    /// source turns, not an actual change of orientation - `oag-game`'s
    /// `race::scene::frame::shadow` module (outside this crate) is a caller
    /// this bit in exactly that way, before its `mapped_centre` was fixed to
    /// swing less.
    #[must_use]
    pub fn matrix(&self) -> Mat4 {
        self.matrix_reaching(self.radius.max(1.0) + NEAR_MARGIN)
    }

    /// [`Self::matrix`] with the box's *depth* stated by the caller: the eye
    /// sits `reach` units towards the light from [`Self::centre`] and the far
    /// plane the same distance beyond it, so the box spans `2 * reach` along
    /// the light while its width stays [`Self::radius`].
    ///
    /// For a map whose depth is a reading rather than a fit: Wipeout HD's
    /// per-ship sun-view box looks from 70 units up the sun with near 1 and
    /// far 140 (`Shadow_BuildShipShadowMatrices`), which is `reach = 70`
    /// around a craft a few units across - the sun-occlusion map needs that
    /// depth to see the same overhead deck the original's does.
    #[must_use]
    pub fn matrix_reaching(&self, reach: f32) -> Mat4 {
        let radius = self.radius.max(1.0);
        let reach = reach.max(1.0);
        let direction = self.resolved_direction();
        let eye = self.centre + direction * reach;
        // Any up that is not the direction itself; the choice only rotates the
        // map's texels, which nothing reads across frames.
        let up = if direction.y.abs() > 0.99 {
            Vec3::Z
        } else {
            Vec3::Y
        };
        let view = camera::look_at(eye, self.centre, up);
        // `0.0..1.0` depth, which is wgpu's clip convention and what the
        // receiver's own `ndc.z` test in `mesh`'s shader assumes.
        // The same `directx` clip convention `camera::perspective` uses, which
        // is wgpu's: depth runs 0 to 1, and `mesh`'s shader's own `ndc.z` test
        // assumes it.
        let projection = camera::orthographic(-radius, radius, -radius, radius, 0.0, reach * 2.0);
        projection * view
    }

    /// This fit, with [`Self::centre`] snapped to whole texels of a `map_size`
    /// square map, in the light's own basis.
    ///
    /// **Why this exists**: a directional light's ortho box whose centre moves
    /// by arbitrary sub-texel amounts re-quantises every texel of the map it
    /// fits, every frame - the standard cause of a shadow that crawls or swims
    /// under a moving view. Snapping the centre to the texel grid **in light
    /// space** removes that: the box still translates as its caster moves, but
    /// only by whole texels, so a texel that was covered stays covered exactly
    /// until the box has moved a full texel width.
    ///
    /// Only the two axes perpendicular to the light are snapped; the axis
    /// along it is untouched; because it is depth *into* the light and never
    /// decides which texel a receiver samples.
    ///
    /// The caller supplies `map_size` because the size is the map's own: a
    /// craft's coverage map changes side with its distance rank
    /// ([`RANK_SIDES`]) - snapping only makes sense where the size fed in is
    /// the one the fit is about to render into.
    ///
    /// **Testable without a GPU**: nudging [`Self::centre`] by a fraction of a
    /// texel must leave the snapped centre - and therefore
    /// [`Self::matrix`] built from it - byte-identical.
    #[must_use]
    pub fn snapped(&self, map_size: u32) -> Self {
        let radius = self.radius.max(1.0);
        let texel = radius * 2.0 / map_size as f32;
        let direction = self.resolved_direction();
        // The same up-vector tie-break `matrix` uses, read back out of the
        // view it would build - the same pattern `frame.rs` uses for the
        // render camera's own axes - rather than re-deriving right/up a
        // second, independent way that could disagree with it. Only the
        // rotation matters here, so any point along `direction` stands in for
        // the eye.
        let up_hint = if direction.y.abs() > 0.99 {
            Vec3::Z
        } else {
            Vec3::Y
        };
        let view = camera::look_at(direction, Vec3::ZERO, up_hint);
        let right = Vec3::new(view.x_axis.x, view.y_axis.x, view.z_axis.x);
        let up = Vec3::new(view.x_axis.y, view.y_axis.y, view.z_axis.y);
        let snap_offset = |axis: Vec3| {
            let projected = self.centre.dot(axis);
            (projected / texel).round() * texel - projected
        };
        Self {
            centre: self.centre + right * snap_offset(right) + up * snap_offset(up),
            ..*self
        }
    }
}

/// One thing that casts into the map.
///
/// Borrowed rather than owned: the geometry is already on the GPU as some
/// drawable's own buffers, and re-uploading it per frame to shadow it would
/// cost more than the pass does.
#[derive(Debug, Clone, Copy)]
pub struct Caster<'a> {
    /// The caster's vertex buffer, in [`GpuVertex`] layout.
    pub vertices: &'a wgpu::Buffer,
    /// Its index buffer, `u32`.
    pub indices: &'a wgpu::Buffer,
    /// Which index ranges to draw - a model's opaque draw calls, so a hull
    /// casts and its own transparent flare does not.
    pub ranges: &'a [std::ops::Range<u32>],
    /// Model to world.
    pub model: Mat4,
}

/// The depth map's resolution, square.
///
/// **Ours.** The coverage maps are fitted to one craft each, where this one
/// covers everything in front of the camera - a far larger box, so it needs
/// more texels to keep a usable world size per texel.
pub const DEPTH_SIZE: u32 = 2048;

/// The record the receiver reads for a craft whose map is rendered at `rank`:
/// its box snapped to the texels of that rank's side, projected, and the
/// square it occupies in the atlas as a `0..1` rectangle.
///
/// Separate from [`Map::render`] because it is arithmetic, so the same record
/// the pass projects with is checked without a GPU.
#[must_use]
pub fn craft_shadow(fit: &Fit, rank: usize) -> CraftShadow {
    let (x, y, side) = RANK_SQUARES[rank.min(MAX_CASTERS - 1)];
    let matrix = fit.snapped(side).craft_matrix();
    let (width, height) = (ATLAS_WIDTH as f32, ATLAS_HEIGHT as f32);
    CraftShadow {
        matrix: matrix.to_cols_array_2d(),
        rect: [
            x as f32 / width,
            y as f32 / height,
            side as f32 / width,
            side as f32 / height,
        ],
        centre: [
            fit.centre.x,
            fit.centre.y,
            fit.centre.z,
            CRAFT_CULL_RADIUS * CRAFT_CULL_RADIUS,
        ],
    }
}

/// One craft's map to render: which craft, how near it ranks, the box to fit
/// and what to draw.
#[derive(Debug, Clone, Copy)]
pub struct CraftCaster<'a> {
    /// The craft's slot, `0..MAX_CASTERS`: the index its record takes in
    /// [`Map::craft_shadows`], and so in the receiver's array.
    pub slot: usize,
    /// The craft's rank by distance from the reference point, `0` nearest:
    /// the index into [`RANK_SIDES`] and [`RANK_SQUARES`].
    pub rank: usize,
    /// The box to fit, [`Fit::craft`] around the craft.
    pub fit: Fit,
    /// The craft's own geometry.
    pub caster: Caster<'a>,
}

/// The coverage atlas, the `mapped` tier's depth map, their caster pipelines
/// and the per-caster uniforms.
#[derive(Debug)]
pub struct Map {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    pipeline: wgpu::RenderPipeline,
    /// One uniform slot per caster the pass can hold, each a
    /// `light view-projection * model`.
    uniforms: wgpu::Buffer,
    binds: Vec<wgpu::BindGroup>,
    /// Each craft's projection and atlas square as the last [`Map::render`]
    /// left them, for the receiver; [`CraftShadow::NONE`] for a craft that
    /// drew nothing.
    crafts: [CraftShadow; MAX_CASTERS],
    /// How many casters it actually drew.
    drawn: usize,
    /// The `mapped` tier's depth target, its view and the pipeline that fills
    /// it - built beside the coverage map so one type carries both tiers'
    /// plumbing and a caller switches between them per frame.
    depth: wgpu::Texture,
    depth_view: wgpu::TextureView,
    depth_pipeline: wgpu::RenderPipeline,
    /// The matrix the last [`Map::render_depth`] projected with.
    depth_matrix: Mat4,
    /// How many casters that pass drew.
    depth_drawn: usize,
}

/// The most casters one pass holds: a full grid.
///
/// Eight, matching `oag_gameplay::MAX_SHIPS` - stated as a literal because
/// this crate depends on no gameplay crate.
pub const MAX_CASTERS: usize = 8;

/// Bytes per caster uniform: one `mat4x4`, padded to the dynamic-offset
/// alignment every backend accepts.
const UNIFORM_STRIDE: u64 = 256;

impl Map {
    /// Builds the map and its caster pipeline.
    #[must_use]
    pub fn new(device: &wgpu::Device) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("shadow map atlas"),
            size: wgpu::Extent3d {
                width: ATLAS_WIDTH,
                height: ATLAS_HEIGHT,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: COVERAGE_FORMAT,
            // `COPY_SRC` so the map can be read back: a shadow map's only
            // other observable is a sampler inside another shader, and a pass
            // that draws nothing looks exactly like one that draws correctly.
            // See `tests/shadow_map_coverage.rs`.
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("shadow caster"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!(concat!(env!("OUT_DIR"), "/caster.wgsl")).into(),
            ),
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("shadow caster"),
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
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("shadow caster"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("shadow caster"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<GpuVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    // Position alone: a caster contributes coverage, so
                    // nothing downstream reads a normal, a uv or a colour.
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3],
                })],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: COVERAGE_FORMAT,
                    // **No blend, and that is the point**: a texel is covered
                    // or it is not, so two craft overlapping in the light's
                    // view darken the ground once rather than twice.
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                // Two-sided: a hull's own back faces are what a shadow sees
                // when the light is behind it, and there is no depth buffer
                // here to make the choice matter.
                cull_mode: None,
                ..Default::default()
            },
            // **No depth attachment at all.** Coverage does not care which
            // caster is nearest the light; every one of them writes the same
            // value.
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let uniforms = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("shadow caster uniforms"),
            size: UNIFORM_STRIDE * MAX_CASTERS as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let binds = (0..MAX_CASTERS)
            .map(|slot| {
                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("shadow caster"),
                    layout: &layout,
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                            buffer: &uniforms,
                            offset: UNIFORM_STRIDE * slot as u64,
                            size: std::num::NonZeroU64::new(64),
                        }),
                    }],
                })
            })
            .collect();

        let depth = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("shadow depth map"),
            size: wgpu::Extent3d {
                width: DEPTH_SIZE,
                height: DEPTH_SIZE,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: SHADOW_DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let depth_view = depth.create_view(&wgpu::TextureViewDescriptor::default());
        let depth_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("shadow caster depth"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<GpuVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3],
                })],
                compilation_options: Default::default(),
            },
            // **No fragment stage at all**: the depth pass writes depth and
            // nothing else, so there is no colour target to declare and no
            // shader to run per pixel.
            fragment: None,
            primitive: wgpu::PrimitiveState {
                // Front faces culled rather than back: shadowing from the far
                // side of a caster moves the depth comparison a whole object
                // away from the receiving surface, which is the cheapest way
                // to keep a flat lit surface from shadowing itself. See
                // `DEPTH_BIAS` in `mesh_render::ShadowMap`, which is what
                // handles the rest.
                // **Nothing culled**, and this is the one that cost the most
                // to find. Culling front faces is the usual trick against
                // self-shadowing, and here it removes exactly the surfaces
                // that should shadow: a craft's *top* is what the light sees,
                // so with it gone the map holds the road's own underside and
                // the comparison decides the road is lit. Two-sided plus the
                // slope-scaled bias below keeps the acne down without throwing
                // the caster away.
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: SHADOW_DEPTH_FORMAT,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: Default::default(),
                // **Slope-scaled, in the rasterizer**, which is where a depth
                // bias belongs: a surface seen edge-on from the light spans
                // many depth units per texel, and a constant bias big enough
                // for it detaches every other shadow from what casts it. The
                // first cut used a shader-side constant alone and covered a
                // circuit's lattice towers in acne. Ours - nothing on any disc
                // authors it - and the shader's own `depth_bias` stays as the
                // small constant floor beside it.
                bias: wgpu::DepthBiasState {
                    constant: 2,
                    slope_scale: 3.0,
                    clamp: 0.0,
                },
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        Self {
            texture,
            view,
            pipeline,
            uniforms,
            binds,
            crafts: [CraftShadow::NONE; MAX_CASTERS],
            drawn: 0,
            depth,
            depth_view,
            depth_pipeline,
            depth_matrix: Mat4::IDENTITY,
            depth_drawn: 0,
        }
    }

    /// The depth map's view, for the `mapped` tier's own binding.
    #[must_use]
    pub fn depth_view(&self) -> &wgpu::TextureView {
        &self.depth_view
    }

    /// The projection [`Self::render_depth`] last used.
    #[must_use]
    pub fn depth_matrix(&self) -> Mat4 {
        self.depth_matrix
    }

    /// How many casters that pass drew.
    #[must_use]
    pub fn depth_casters(&self) -> usize {
        self.depth_drawn
    }

    /// The depth texture, for a caller that reads it back.
    #[must_use]
    pub fn depth_texture(&self) -> &wgpu::Texture {
        &self.depth
    }

    /// Fills the depth map: the same casters, from the same light, recorded by
    /// distance instead of by coverage.
    ///
    /// **This is the `mapped` tier's half and none of it is the original's.**
    /// No title in the lineage renders a depth map: HD's is a coverage map
    /// (see this module's header) and Pulse projects an authored hull. What
    /// this buys over either is that *everything* can receive - a craft under
    /// a bridge, a craft beside another - which is the whole reason the tier
    /// exists.
    ///
    /// Cleared to the far plane every frame, for the reason [`Self::render`]
    /// clears its own target: a stale depth is a shadow that stays behind.
    pub fn render_depth(
        &mut self,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        fit: &Fit,
        casters: &[Caster<'_>],
    ) {
        self.depth_matrix = fit.matrix();
        self.depth_drawn = casters.len().min(MAX_CASTERS);
        for (slot, caster) in casters.iter().take(MAX_CASTERS).enumerate() {
            let mvp = self.depth_matrix * caster.model;
            queue.write_buffer(
                &self.uniforms,
                UNIFORM_STRIDE * slot as u64,
                bytemuck::cast_slice(&mvp.to_cols_array()),
            );
        }
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("shadow depth map"),
            color_attachments: &[],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&self.depth_pipeline);
        for (slot, caster) in casters.iter().take(MAX_CASTERS).enumerate() {
            pass.set_bind_group(0, &self.binds[slot], &[]);
            pass.set_vertex_buffer(0, caster.vertices.slice(..));
            pass.set_index_buffer(caster.indices.slice(..), wgpu::IndexFormat::Uint32);
            for range in caster.ranges {
                pass.draw_indexed(range.clone(), 0, 0..1);
            }
        }
    }

    /// The view every lit pipeline samples.
    #[must_use]
    pub fn view(&self) -> &wgpu::TextureView {
        &self.view
    }

    /// Each craft's projection and atlas square as the last [`Self::render`]
    /// left them, indexed by [`CraftCaster::slot`], for the receiver's own
    /// uniform. [`CraftShadow::NONE`] for a craft that drew nothing.
    #[must_use]
    pub fn craft_shadows(&self) -> [CraftShadow; MAX_CASTERS] {
        self.crafts
    }

    /// How many casters the last pass drew - an observable, since a map that
    /// was cleared and a map that was never rendered look identical.
    #[must_use]
    pub fn casters(&self) -> usize {
        self.drawn
    }

    /// Clears the atlas and draws each craft into its own square of it, from
    /// its own box: **one map per craft**, as the original keeps.
    ///
    /// The square is the craft's rank's ([`RANK_SQUARES`]), so the sides run
    /// 512, 512, 256, 128, 64, 32, 32, 32 by distance whichever craft holds
    /// the rank this frame. Each craft's fit is snapped to whole texels of the
    /// side it is about to render at ([`Fit::snapped`]), so the shadow of a
    /// moving craft does not swim across its own texels; the original fits the
    /// box to the bbox every frame and does not snap, which is **ours**.
    ///
    /// Cleared to zero every frame: an unwritten texel is unshadowed ground,
    /// and leaving the previous frame's coverage would drag a shadow behind
    /// every craft. An empty `crafts` is a clear and nothing else.
    pub fn render(
        &mut self,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        crafts: &[CraftCaster<'_>],
    ) {
        let crafts = &crafts[..crafts.len().min(MAX_CASTERS)];
        self.crafts = [CraftShadow::NONE; MAX_CASTERS];
        self.drawn = crafts.len();
        for (index, craft) in crafts.iter().enumerate() {
            let record = craft_shadow(&craft.fit, craft.rank);
            if let Some(slot) = self.crafts.get_mut(craft.slot) {
                *slot = record;
            }
            let mvp = Mat4::from_cols_array_2d(&record.matrix) * craft.caster.model;
            queue.write_buffer(
                &self.uniforms,
                UNIFORM_STRIDE * index as u64,
                bytemuck::cast_slice(&mvp.to_cols_array()),
            );
        }
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("shadow map atlas"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.view,
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
        pass.set_pipeline(&self.pipeline);
        for (index, craft) in crafts.iter().enumerate() {
            let (x, y, side) = RANK_SQUARES[craft.rank.min(MAX_CASTERS - 1)];
            // The viewport maps the craft's clip box onto its own square and
            // the scissor keeps what the box does not clip (the guard band)
            // out of the neighbouring squares.
            pass.set_viewport(x as f32, y as f32, side as f32, side as f32, 0.0, 1.0);
            pass.set_scissor_rect(x, y, side, side);
            pass.set_bind_group(0, &self.binds[index], &[]);
            pass.set_vertex_buffer(0, craft.caster.vertices.slice(..));
            pass.set_index_buffer(craft.caster.indices.slice(..), wgpu::IndexFormat::Uint32);
            for range in craft.caster.ranges {
                pass.draw_indexed(range.clone(), 0, 0..1);
            }
        }
    }

    /// The texture behind [`Self::view`].
    #[must_use]
    pub fn texture(&self) -> &wgpu::Texture {
        &self.texture
    }

    /// Reads the atlas back as one byte per texel, row-major from the top -
    /// [`ATLAS_WIDTH`] by [`ATLAS_HEIGHT`] of them - blocking on the GPU. For a
    /// headless check or a dump; the frame never calls this.
    #[must_use]
    pub fn read_back(&self, device: &wgpu::Device, queue: &wgpu::Queue) -> Vec<u8> {
        let (width, height) = (ATLAS_WIDTH as usize, ATLAS_HEIGHT as usize);
        let row = ATLAS_WIDTH.div_ceil(256) * 256;
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("shadow map readback"),
            size: u64::from(row * ATLAS_HEIGHT),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        encoder.copy_texture_to_buffer(
            self.texture.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(row),
                    rows_per_image: Some(ATLAS_HEIGHT),
                },
            },
            wgpu::Extent3d {
                width: ATLAS_WIDTH,
                height: ATLAS_HEIGHT,
                depth_or_array_layers: 1,
            },
        );
        queue.submit([encoder.finish()]);
        let slice = buffer.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("draining the queue");
        let view = slice.get_mapped_range().expect("mapping the readback");
        let mut out = vec![0u8; width * height];
        for y in 0..height {
            let start = y * row as usize;
            out[y * width..(y + 1) * width].copy_from_slice(&view[start..start + width]);
        }
        drop(view);
        buffer.unmap();
        out
    }
}

#[cfg(test)]
mod tests;
