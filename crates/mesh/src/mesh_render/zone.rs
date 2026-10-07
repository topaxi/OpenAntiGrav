//! Bind group 2's Zone half: the stage texture the Zone material variant
//! samples, the sampler it reads it through, and the visualiser lookup the
//! glow term indexes.
//!
//! Split out of `mesh_render` rather than written inline for one reason worth
//! stating: the scene group is now two unrelated things in one binding set -
//! the fog and light *buffer*, which every title writes, and a set of
//! *textures* only a Zone race on Wipeout HD/Fury ever has. Keeping the
//! second one here means the first reads as it always did.
//!
//! See [`super::Zone`] for what the shader does with the stage texture, and
//! `mesh.wesl`'s own `zone_glow` for the visualiser term this module now also
//! feeds. [`write_vis`] is the seam a caller updates every frame; the rest of
//! bind group 2's own contents are built once at model load, by
//! [`scene_bind_group`] - except [`StageArt`]'s four texture views, which
//! [`rebind`] rebuilds alone, on the transition sphere's stage-change edge.

use std::sync::Arc;

use super::texture;
use crate::mesh::ModelTexture;

/// Texels in [`Resources::vis_texture`]. Matches the original's own
/// `zoneTexVis` - a 256-entry lookup keyed on a texel's alpha - not a choice
/// this project made; see `shaders/zone.wesl`'s `zone_glow` and
/// `docs/ghidra/functions/ps3-hdfury-eu/zone-shader.md`.
pub const VIS_WIDTH: u32 = 256;

/// How many texels one band's bar meter occupies, and therefore the stride
/// from one band's bar to the next.
///
/// **Recovered**, from `Environment_UpdateStageBlend`'s own pointer set: it
/// holds ten pointers into the pixel buffer, at texels `1` through `10`, and
/// advances every one of them by `0x28` bytes - ten texels - per band. See
/// [`write_vis`] and
/// `docs/ghidra/functions/ps3-hdfury-eu/zone-visualiser.md`.
pub const SEGMENTS: usize = 10;

/// Where the smooth half of the lookup begins: one texel per band, after the
/// bars.
///
/// **Recovered**: the eleventh pointer the same function holds is `r9 +
/// 0x284`, texel `161`, and it advances by one texel per band rather than
/// ten. `161 = 1 + 10 * 16`, immediately past the sixteenth bar.
pub const SMOOTH_BASE: usize = 161;

/// The showing stage's two textures and the stage being swept out's own two,
/// as [`resources`] binds them.
///
/// **Two pairs, because the original publishes two pairs.** HD's Zone
/// parameters go out in two paired blocks: `zoneModeTrack<n>.gtf` beside the
/// `Track.*` colours for a chunk whose render-block flags carry the track bit
/// (`oag_rcs::rcsmodel::RENDER_TRACK`), and `zoneMode<n>.gtf` beside the
/// `Scene.*` colours for every other chunk - `mesh.wesl` selects per fragment
/// on `slots::ZONE_TRACK`. Each of those two is published again as `zoneTex
/// Inner`/`zoneTexOuter`: the showing stage's own texture and the one being
/// swept out by the transition sphere, selected per fragment on
/// `zone_inside`. See [`super::Zone`] for both selections together.
///
/// Any slot `None` binds a 1x1 black, on the terms [`resources`] states.
#[derive(Debug, Clone, Default)]
pub struct StageArt {
    /// `zoneModeTrack<n>.gtf` for the showing stage - `zoneTexTrackInner`.
    pub track: Option<Arc<ModelTexture>>,
    /// `zoneMode<n>.gtf` for the showing stage - `zoneTexInner`. A flat white
    /// on every stage HD/Fury ships, and bound rather than replaced by a
    /// constant because it is the file's own statement of what a Scene chunk
    /// samples.
    pub scene: Option<Arc<ModelTexture>>,
    /// `zoneModeTrack<n>.gtf` for the stage being swept out -
    /// `zoneTexTrackOuter`. The showing stage's own texture where the
    /// caller has no previous stage to offer - see
    /// `oag_raceplay::zone_grade::ZoneGrade::stage_art_pair` - so the
    /// sphere test is a no-op rather than a black hole outside it.
    pub track_outer: Option<Arc<ModelTexture>>,
    /// `zoneMode<n>.gtf` for the stage being swept out - `zoneTexOuter`, on
    /// the same fallback terms as [`Self::track_outer`].
    pub scene_outer: Option<Arc<ModelTexture>>,
}

impl StageArt {
    /// Neither pair: what every model outside an HD Zone race binds.
    pub const NONE: Self = Self {
        track: None,
        scene: None,
        track_outer: None,
        scene_outer: None,
    };
}

/// The eight layout entries the Zone half of bind group 2 adds: the track
/// stage's Inner texture and its sampler at bindings 1-2, a nearest-filtered
/// clone of the same stage texture at binding 3 (the visualiser's own band
/// index is read from a texel's alpha, and interpolating between two bands
/// would smear them together), the visualiser lookup itself at bindings 4-5,
/// the scene stage's Inner texture at binding 10 - past the shadow map's
/// 6-9, which were numbered before the second texture set was bound - and
/// the track and scene stages' Outer textures at bindings 11-12, numbered
/// past the shadow map for the same reason.
///
/// A sampler of its own for the stage textures rather than the albedo's,
/// because the coordinate is one the shader builds, `zoneColourTint.xy *
/// (1 - meshUV)`, and runs outside `[0, 1]` wherever the file's own scale
/// does. All four stage textures share both samplers: same coordinate, same
/// filter, on either side of the transition sphere.
pub(super) fn layout_entries() -> [wgpu::BindGroupLayoutEntry; 8] {
    let texture_entry = |binding| wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        count: None,
    };
    let sampler_entry = |binding| wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
        count: None,
    };
    [
        texture_entry(1),
        sampler_entry(2),
        sampler_entry(3),
        texture_entry(4),
        sampler_entry(5),
        texture_entry(10),
        texture_entry(11),
        texture_entry(12),
    ]
}

/// Everything [`resources`] builds: the four stage textures' views and the
/// two samplers they share, and the visualiser lookup's texture, view and
/// sampler.
///
/// [`Self::vis_texture`] is kept, unlike the stage textures' own
/// `wgpu::Texture`s - it is written every frame by [`write_vis`], where a
/// stage texture is loaded once and never touched again.
pub(super) struct Resources {
    pub sampler: wgpu::Sampler,
    /// [`StageArt::track`].
    pub view: wgpu::TextureView,
    /// [`StageArt::scene`].
    pub scene_view: wgpu::TextureView,
    /// [`StageArt::track_outer`].
    pub outer_view: wgpu::TextureView,
    /// [`StageArt::scene_outer`].
    pub scene_outer_view: wgpu::TextureView,
    /// The same texels as [`Self::view`], point-filtered - see
    /// [`layout_entries`].
    pub nearest_sampler: wgpu::Sampler,
    pub vis_texture: wgpu::Texture,
    pub vis_view: wgpu::TextureView,
    pub vis_sampler: wgpu::Sampler,
}

/// One [`StageArt`] slot as a bound view: the texture's own decode, or a 1x1
/// black where `texture` is `None` - see [`resources`] for why black rather
/// than white or the neighbouring slot.
fn upload_view(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &Option<Arc<ModelTexture>>,
    label: &str,
) -> wgpu::TextureView {
    // A texture no level of which fits the device takes the absent slot's 1x1
    // black, like one that was never authored.
    texture
        .as_ref()
        .and_then(|texture| {
            texture::upload(
                device,
                queue,
                texture,
                device
                    .features()
                    .contains(wgpu::Features::TEXTURE_COMPRESSION_BC),
            )
        })
        .map(|placed| placed.view)
        .unwrap_or_else(|| {
            texture::upload_rgba(device, queue, 1, 1, &[0, 0, 0, 255], label, None)
                .expect("a 1x1 texture fits every device")
                .view
        })
}

/// [`StageArt`]'s four views alone, rebuilt on a stage-change edge by
/// [`rebind`] - see [`RebindResources`] for what stays bound underneath
/// them.
pub(super) fn retexture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    stage: &StageArt,
) -> [wgpu::TextureView; 4] {
    [
        upload_view(device, queue, &stage.track, "no zone track stage"),
        upload_view(device, queue, &stage.scene, "no zone scene stage"),
        upload_view(
            device,
            queue,
            &stage.track_outer,
            "no zone track outer stage",
        ),
        upload_view(
            device,
            queue,
            &stage.scene_outer,
            "no zone scene outer stage",
        ),
    ]
}

/// The samplers and the views to bind, for a model that has the Zone stage
/// textures and for one that does not - plus the visualiser lookup, which
/// every model gets regardless, silent until [`write_vis`] is called.
///
/// **Black, not white, where there is no stage texture.**
/// [`super::Scene::off`] leaves `zone.enabled` at zero so the sample is
/// multiplied out anyway, and a black placeholder means even a caller that
/// writes a Zone uniform without a texture adds nothing rather than adding a
/// white sheet - the failure this project wants from a missing asset is an
/// absence. That holds for each of [`StageArt`]'s four slots on its own: a
/// stage whose scene texture failed to decode draws its scene chunks black,
/// not through the track set. [`Resources::vis_texture`] starts the same
/// way: all-black, so a model drawn before the first [`write_vis`] call - or
/// on a build with no audio device at all - shows no glow rather than an
/// invented one.
pub(super) fn resources(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    anisotropy: super::Anisotropy,
    stage: &StageArt,
) -> Resources {
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("zone"),
        address_mode_u: wgpu::AddressMode::Repeat,
        address_mode_v: wgpu::AddressMode::Repeat,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::MipmapFilterMode::Linear,
        anisotropy_clamp: anisotropy.clamp(),
        ..Default::default()
    });
    // Repeat, on the same terms as the linear sampler above: it addresses the
    // same texels with the same shader-built coordinate, and only the filter
    // differs.
    let nearest_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("zone nearest"),
        address_mode_u: wgpu::AddressMode::Repeat,
        address_mode_v: wgpu::AddressMode::Repeat,
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        ..Default::default()
    });
    let [view, scene_view, outer_view, scene_outer_view] = retexture(device, queue, stage);

    let vis_texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("zone vis"),
        size: wgpu::Extent3d {
            width: VIS_WIDTH,
            height: 1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: texture::FORMAT,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &vis_texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &[0u8; (VIS_WIDTH * 4) as usize],
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(VIS_WIDTH * 4),
            rows_per_image: Some(1),
        },
        wgpu::Extent3d {
            width: VIS_WIDTH,
            height: 1,
            depth_or_array_layers: 1,
        },
    );
    let vis_view = vis_texture.create_view(&wgpu::TextureViewDescriptor::default());
    // Clamp, not repeat: the shader indexes this by a texel alpha in
    // `0.0..=1.0` (see `shaders/zone.wesl`'s `zone_glow`), which never leaves that
    // range, and nearest-filtered for the same reason the stage texture's own
    // clone is - a band lookup wants a discrete answer, not a blend between
    // two neighbours.
    let vis_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("zone vis"),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        ..Default::default()
    });

    Resources {
        sampler,
        view,
        scene_view,
        outer_view,
        scene_outer_view,
        nearest_sampler,
        vis_texture,
        vis_view,
        vis_sampler,
    }
}

/// Everything bind group 2 keeps across a stage-change edge: the layout, the
/// two stage samplers, the visualiser lookup and the shadow map. What
/// changes on the edge is [`StageArt`]'s four texture views alone, which
/// [`rebind`] rebuilds; this is what it rebuilds the bind group *against*.
///
/// **`pub` only because it has to be.** [`Built`](super::Built) needs a
/// field of this type to hand a [`scene_bind_group`] call's leftovers back
/// to [`rebind`] later, `Built`'s own fields are public and cross the crate
/// boundary into `oag_game`, and a public field of a private type is a hard
/// compile error under this workspace's `-D warnings` (`private_interfaces`).
/// Every field here stays private regardless: nothing outside this module
/// constructs one, reads one apart, or has anywhere to get one besides
/// [`scene_bind_group`]'s own return value.
#[derive(Debug)]
pub struct RebindResources {
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    nearest_sampler: wgpu::Sampler,
    vis_view: wgpu::TextureView,
    vis_sampler: wgpu::Sampler,
    shadow: super::shadow_map::Resources,
}

/// Bind group 2 itself, from the stage's four texture views and everything
/// [`RebindResources`] keeps underneath them. The one assembly
/// [`scene_bind_group`]'s initial build and [`rebind`]'s later one share, so
/// the fourteen entries are written down exactly once.
fn bind_group(
    device: &wgpu::Device,
    fog_buffer: &wgpu::Buffer,
    track: &wgpu::TextureView,
    scene_tex: &wgpu::TextureView,
    track_outer: &wgpu::TextureView,
    scene_outer: &wgpu::TextureView,
    kept: &RebindResources,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("scene"),
        layout: &kept.layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: fog_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(track),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Sampler(&kept.sampler),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: wgpu::BindingResource::Sampler(&kept.nearest_sampler),
            },
            wgpu::BindGroupEntry {
                binding: 4,
                resource: wgpu::BindingResource::TextureView(&kept.vis_view),
            },
            wgpu::BindGroupEntry {
                binding: 5,
                resource: wgpu::BindingResource::Sampler(&kept.vis_sampler),
            },
            wgpu::BindGroupEntry {
                binding: 6,
                resource: wgpu::BindingResource::TextureView(&kept.shadow.view),
            },
            wgpu::BindGroupEntry {
                binding: 7,
                resource: wgpu::BindingResource::Sampler(&kept.shadow.sampler),
            },
            wgpu::BindGroupEntry {
                binding: 8,
                resource: wgpu::BindingResource::TextureView(&kept.shadow.depth_view),
            },
            wgpu::BindGroupEntry {
                binding: 9,
                resource: wgpu::BindingResource::Sampler(&kept.shadow.depth_sampler),
            },
            wgpu::BindGroupEntry {
                binding: 10,
                resource: wgpu::BindingResource::TextureView(scene_tex),
            },
            wgpu::BindGroupEntry {
                binding: 11,
                resource: wgpu::BindingResource::TextureView(track_outer),
            },
            wgpu::BindGroupEntry {
                binding: 12,
                resource: wgpu::BindingResource::TextureView(scene_outer),
            },
            wgpu::BindGroupEntry {
                binding: 13,
                resource: wgpu::BindingResource::TextureView(&kept.shadow.occlusion_view),
            },
            wgpu::BindGroupEntry {
                binding: 14,
                resource: wgpu::BindingResource::TextureView(&kept.shadow.self_shadow_view),
            },
            wgpu::BindGroupEntry {
                binding: 15,
                resource: wgpu::BindingResource::TextureView(&kept.shadow.behind_glass_view),
            },
        ],
    })
}

/// Builds bind group 2 whole: its layout, the [`super::Scene`] buffer
/// (initialised to [`super::Scene::off`]), the bind group itself, the Zone
/// visualiser's own texture for [`write_vis`], and what a later stage change
/// needs to rebuild just the bind group - see [`RebindResources`] and
/// [`rebind`].
///
/// Split out of `mesh_render::build` under the 1,000-line rule in
/// `scripts/check-file-size.py`: the file that assembles every other
/// pipeline state had no room left to also own the one binding set a Zone
/// stage change touches, and this crate's own convention for a file at that
/// ceiling is to move the seam rather than exempt it - a move, not a
/// behaviour change. See `docs/rendering/hd-zone-recolour.md`.
#[allow(clippy::too_many_arguments)]
pub(super) fn scene_bind_group(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    anisotropy: super::Anisotropy,
    zone: &StageArt,
    shadow_maps: super::ShadowMaps<'_>,
) -> (
    wgpu::BindGroupLayout,
    wgpu::Buffer,
    wgpu::BindGroup,
    wgpu::Texture,
    RebindResources,
) {
    let zone_entries = layout_entries();
    let shadow_entries = super::shadow_map::layout_entries();
    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        // Labelled for what the buffer holds, which is `Scene` - fog *and*
        // the light rig. The three "fog" labels here predated `Light`
        // joining it.
        label: Some("scene"),
        entries: &[
            // Both stages: the fragment stage reads the fog, the rig, the
            // Zone grade and the shadow map; the vertex stage reads the SPU
            // vertex-light list (`spu_light_sum`), per vertex as the original
            // does.
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            // The Zone stage's four textures, their two samplers and the
            // visualiser lookup, in the *scene* group rather than the
            // per-material one: they are a property of the race, not of a
            // material slot, so binding them here uploads them once per
            // model instead of once per slot naming a material. See
            // [`layout_entries`].
            zone_entries[0],
            zone_entries[1],
            zone_entries[2],
            zone_entries[3],
            zone_entries[4],
            zone_entries[5],
            // The shadow map, in the scene group for the same reason the
            // Zone stage's textures are: it is a property of the frame, not
            // of a material slot. Bound on every pipeline whether or not
            // anything shadows - `Scene::shadow.strength` at zero is what
            // makes it inert, so a title with no map still binds a
            // placeholder rather than needing a second layout.
            shadow_entries[0],
            shadow_entries[1],
            shadow_entries[2],
            shadow_entries[3],
            // The Outer half of the Zone stage's two texture sets - see
            // [`StageArt::track_outer`]/[`StageArt::scene_outer`]. Numbered
            // past the shadow map for the same reason binding 10 already is.
            zone_entries[6],
            zone_entries[7],
            // The per-craft sun-occlusion array, binding 13 - past everything
            // above for the same reason, and in this group because a hull
            // picks its layer by a uniform field rather than by a bind group.
            // The self-shadow depth array, binding 14, beside it for the same
            // reason: the same hull compares against the same layer.
            shadow_entries[4],
            shadow_entries[5],
            // The behind-the-glass target, binding 15: a picture of the frame
            // like the maps above, read by Vineta K's tunnel glass alone.
            shadow_entries[6],
        ],
    });

    // Every pipeline gets a fog buffer, initialised to `Fog::off`. A caller
    // that never writes it therefore renders exactly as it did before fog
    // existed, which is what keeps the asset viewer and the offscreen
    // capture path unchanged without either of them knowing fog is there.
    let fog_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("scene"),
        size: super::SCENE_SIZE,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    queue.write_buffer(&fog_buffer, 0, bytemuck::bytes_of(&super::Scene::off()));

    let zone_resources = resources(device, queue, anisotropy, zone);
    let shadow = super::shadow_map::resources(device, queue, shadow_maps);
    let kept = RebindResources {
        layout: layout.clone(),
        sampler: zone_resources.sampler,
        nearest_sampler: zone_resources.nearest_sampler,
        vis_view: zone_resources.vis_view,
        vis_sampler: zone_resources.vis_sampler,
        shadow,
    };
    let bind = bind_group(
        device,
        &fog_buffer,
        &zone_resources.view,
        &zone_resources.scene_view,
        &zone_resources.outer_view,
        &zone_resources.scene_outer_view,
        &kept,
    );

    (layout, fog_buffer, bind, zone_resources.vis_texture, kept)
}

/// Rebuilds bind group 2 with `stage`'s four textures alone, from a
/// [`RebindResources`] a earlier [`scene_bind_group`] call returned -
/// everything else in the group (the pipeline is not even reachable from
/// here) stays exactly as that call left it.
///
/// **Call this on the stage-change edge alone.** `oag_raceplay::Drawable`
/// keeps its own `RebindResources` beside the [`wgpu::BindGroup`] this
/// replaces and calls it when
/// `oag_raceplay::zone_grade::ZoneGrade::follow`/`commit` reports a new
/// `(current, previous)` pair - not every frame. A per-frame call would
/// still draw the right picture (uploading the same four textures again is
/// wasteful, not wrong), but the caller gates it the same "log the edge, not
/// the state" way `race::Scene::sync_zone_grade` already gates the stage
/// step itself.
pub fn rebind(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    fog_buffer: &wgpu::Buffer,
    kept: &RebindResources,
    stage: &StageArt,
) -> wgpu::BindGroup {
    let [track, scene_tex, track_outer, scene_outer] = retexture(device, queue, stage);
    bind_group(
        device,
        fog_buffer,
        &track,
        &scene_tex,
        &track_outer,
        &scene_outer,
        kept,
    )
}

/// The visualiser's per-band peak-hold, which the original keeps in its own
/// environment struct and this keeps beside the texture it feeds.
///
/// **Recovered ballistics**, read off `Environment_UpdateStageBlend` at
/// instruction level: a band's held level takes a rising sample immediately
/// and falls by [`Self::DECAY`] per *frame* otherwise, clamped at zero -
///
/// ```text
/// if new >= held { held = new } else { held = max(held - 0.1, 0.0) }
/// ```
///
/// The original stores `held` as a sixteen-float array at `+0x32f8` of the
/// same struct that holds the `zoneTexVis` wrapper, and the raw sample beside
/// it at `+0x32b8`. See
/// `docs/ghidra/functions/ps3-hdfury-eu/zone-visualiser.md`.
///
/// **Per frame, not per second.** The original's caller is
/// `Scene_PrepareFrame`, so the fall rate is frame-rate dependent in the
/// original too; converting it to a per-second rate would be a change, not a
/// port, and is deliberately not done here.
#[derive(Debug, Default)]
pub struct Hold {
    held: Vec<f32>,
}

impl Hold {
    /// How much a band's held level falls in one frame it is not rising.
    ///
    /// `0x3dcccccd`, the float this function loads from its own TOC at
    /// `-0x5a7c(r2)`.
    pub const DECAY: f32 = 0.1;

    /// Folds one frame's `bands` in and returns the held levels.
    pub fn advance(&mut self, bands: &[f32]) -> &[f32] {
        self.held.resize(bands.len(), 0.0);
        for (held, &new) in self.held.iter_mut().zip(bands) {
            *held = if new >= *held {
                new
            } else {
                (*held - Self::DECAY).max(0.0)
            };
        }
        &self.held
    }
}

/// Rewrites the visualiser lookup from `bands` levels, each `0.0..=1.0`,
/// tinted by `tint` - or blanks it when `tint` is `None`, which is what a
/// stage that authors no `EQ colour tint` gets: a missing input draws
/// nothing, not an invented white.
///
/// # This is the original's own layout, recovered
///
/// It used to be an invention: `bands` spread linearly across all 256
/// texels, on the grounds that HD/Fury's per-frame write was untraced past
/// the float it converts. It is traced now
/// (`docs/ghidra/functions/ps3-hdfury-eu/zone-visualiser.md`), the invention
/// was wrong in a way that made the effect invisible, and what replaces it is
/// read off the executable:
///
/// ```text
/// texel 0                     never written
/// texels 1 + 10b ..= 10 + 10b band b's bar: segment s is lit when s <= level
/// texel  161 + b              band b's smooth level, the tint scaled by it
/// ```
///
/// with `level = min(10, trunc(held * 11.0))` and a lit segment carrying the
/// **whole** tint rather than a fraction of it - the original stores one
/// packed colour word into each lit segment and zero into each unlit one, so
/// a bar is a bar and not a gradient. `11.0` and `255.0` are the two floats
/// the function loads from its own TOC (`0x41300000`, `0x437f0000`).
///
/// **The shipped art is the independent confirmation, and it is exact.**
/// `zone-shader.md`'s alpha histogram of the stage textures measures
/// `zonemodetrack9`/`10` at exactly `{31..40}` - which is band 3's ten
/// segments, `1 + 10*3` through `10 + 10*3` - `track14` at "groups of 4 on a
/// stride of 10", the band stride itself, and `track6`/`7` at `{1..162}`,
/// every bar plus the first two smooth slots. Three histograms measured
/// before this layout was read, all three landing on it.
///
/// `bands.len()` need not be sixteen; a shorter slice simply leaves the
/// higher bars dark, and anything past texel [`VIS_WIDTH`] is dropped.
pub fn write_vis(
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    bands: &[f32],
    tint: Option<[u8; 3]>,
) {
    let pixels = vis_pixels(bands, tint);
    let pixels: &[u8] = pixels.as_flattened();
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        pixels,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(VIS_WIDTH * 4),
            rows_per_image: Some(1),
        },
        wgpu::Extent3d {
            width: VIS_WIDTH,
            height: 1,
            depth_or_array_layers: 1,
        },
    );
}

/// The 256 RGBA texels [`write_vis`] uploads - its whole arithmetic, split
/// out so it is testable without a GPU, the same reason
/// `mesh_render::uniforms::view_projection` is split from
/// `mesh_render::uniforms::write_uniforms`.
///
/// Every texel this frame does not light stays `0`, alpha included: the
/// original writes a literal zero word into each unlit segment (`r15`, set
/// by `li r15,0`), and that is also the explicit "draw nothing" a `None`
/// tint or an empty `bands` gets, rather than leaving whatever the texture
/// held from a previous frame.
fn vis_pixels(bands: &[f32], tint: Option<[u8; 3]>) -> [[u8; 4]; VIS_WIDTH as usize] {
    let mut texels = [[0u8; 4]; VIS_WIDTH as usize];
    let Some(tint) = tint.filter(|_| !bands.is_empty()) else {
        return texels;
    };
    let lit = [tint[0], tint[1], tint[2], 255];
    let scaled = |channel: u8, by: f32| {
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "`by` is clamped to 0.0..=1.0 and the channel is a u8"
        )]
        let out = (f32::from(channel) * by).round() as u8;
        out
    };
    for (band, &level) in bands.iter().enumerate() {
        for step in 0..segments(level) {
            if let Some(texel) = texels.get_mut(1 + SEGMENTS * band + step) {
                *texel = lit;
            }
        }
        // The smooth half: the same tint scaled by the level rather than
        // quantised to a segment. The original builds it by multiplying each
        // 0-255 colour component by the 0-255 level and keeping the high
        // byte, which is this to within a rounding step.
        if let Some(texel) = texels.get_mut(SMOOTH_BASE + band) {
            let by = level.clamp(0.0, 1.0);
            *texel = [
                scaled(tint[0], by),
                scaled(tint[1], by),
                scaled(tint[2], by),
                255,
            ];
        }
    }
    texels
}

/// How many of a bar's [`SEGMENTS`] a level lights: `min(10, trunc(level *
/// 11.0))`.
///
/// **`11.0`, not `10.0`, and the truncation is the original's.** It converts
/// with `fctiwz` - round toward zero - and then clamps at ten, so a full
/// level lands on ten lit segments while every value below `10/11` shares the
/// remaining nine evenly. Scaling by `10.0` instead would light the tenth
/// segment only at exactly `1.0`.
fn segments(level: f32) -> usize {
    // `matches!` on the ordering rather than `!(level > 0.0)`: a NaN level
    // must light nothing, and clippy will not take the negated comparison.
    if !matches!(level.partial_cmp(&0.0), Some(std::cmp::Ordering::Greater)) {
        return 0;
    }
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "guarded above zero and clamped to SEGMENTS immediately below"
    )]
    let steps = (level * 11.0) as usize;
    steps.min(SEGMENTS)
}

#[cfg(test)]
mod tests;
