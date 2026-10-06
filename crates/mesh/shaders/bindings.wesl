// The mesh shader's bind groups 0-3: per-draw, per-material, scene and animation resources.

import package::types::{Emissives, NodeAnims, Scene, TexAnims, Uniforms};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;

@group(1) @binding(0) var albedo: texture_2d<f32>;

@group(1) @binding(1) var albedo_sampler: sampler;

// The circuit's baked lighting atlas, on the surfaces whose material names one.
// Every other draw binds a white 1x1, so the multiply below is the identity and
// no branch is needed - the same arrangement `albedo` already uses.
@group(1) @binding(2) var lightmap: texture_2d<f32>;

// HD's pad mask (`_ne`): RGB a tangent-space normal, alpha the light-bar mask.
// Bound third, beside the lightmap, and read only where `slots::PAD_NE` (bit
// 14) is set - see `mesh::rcs::pad_ne`. A flat 1x1 everywhere else.
@group(1) @binding(3) var pad_mask: texture_2d<f32>;

// HD's magstrip wave, read only where `slots::MAG_WAVE` is set. `mesh::rcs::mag_wave`.
@group(1) @binding(4) var wave_map: texture_2d<f32>;

@group(2) @binding(0) var<uniform> scene: Scene;

// The Zone stage's `zoneModeTrack<n>.gtf`, and a sampler of its own
// because it is addressed by a coordinate this shader builds rather than by
// the mesh's own - `scene.zone.uv_scale * (1 - meshUV)` runs outside [0, 1]
// wherever the scale does, and the albedo sampler's `Repeat` is what the
// original's own sampler state says there too. Every draw outside a Zone race
// binds a 1x1 black, which is the identity once `scene.zone.enabled` is 0.
@group(2) @binding(1) var zone_tex: texture_2d<f32>;

@group(2) @binding(2) var zone_sampler: sampler;

// The same texels as `zone_tex`, point-filtered - see `zone_glow` below for
// why a band index wants this rather than `zone_sampler`.
@group(2) @binding(3) var zone_nearest_sampler: sampler;

// The visualiser lookup - `zoneTexVis` - a 256x1 strip `oag_mesh::mesh_render::zone::write_vis`
// rewrites every frame from a real audio spectrum. All-black until the first
// write, which is the identity on `zone_glow`'s own sum. See that function
// and `crates/mesh/src/mesh_render/zone.rs`.
@group(2) @binding(4) var zone_vis_tex: texture_2d<f32>;

@group(2) @binding(5) var zone_vis_sampler: sampler;

// The shadow map, and the sampler that reads it. Bound on every pipeline, a
// black one-texel placeholder where nothing casts - see
// `mesh_render::shadow_map::resources`.
@group(2) @binding(6) var shadow_tex: texture_2d<f32>;

@group(2) @binding(7) var shadow_sampler: sampler;

// The `mapped` tier's depth map and its **non-filtering** sampler: a filtered
// depth is the average of two surfaces and belongs to neither.
@group(2) @binding(8) var shadow_depth_tex: texture_depth_2d;

@group(2) @binding(9) var shadow_depth_sampler: sampler;

// The Zone stage's `zoneMode<n>.gtf`, sampled through `zone_sampler` on the
// same coordinate as `zone_tex`, for a chunk without `ZONE_TRACK` - see
// `zone_set`. Numbered past the shadow map because it was bound later.
@group(2) @binding(10) var zone_scene_tex: texture_2d<f32>;

// The stage-transition sphere's Outer half of the two textures above - the
// stage being swept out's own `zoneModeTrack<n>.gtf`/`zoneMode<n>.gtf`,
// through the same two samplers and the same coordinate. `zone_inside`
// selects the pair the way it already selects `ZoneSet`'s Inner/Outer
// colours; see `zone_sample` and `zone_glow`. Numbered past 10 for the same
// reason that binding is past the shadow map.
@group(2) @binding(11) var zone_tex_outer: texture_2d<f32>;

@group(2) @binding(12) var zone_scene_tex_outer: texture_2d<f32>;

// The per-craft sun-occlusion maps, one layer per craft, read through the
// coverage map's own clamped filtering sampler - see `sun_occlusion` and
// `oag_render::shadow::occlusion`.
@group(2) @binding(13) var sun_occlusion_tex: texture_2d_array<f32>;

// The per-craft self-shadow depth maps, the same layer per craft, compared
// against through the `mapped` tier's non-filtering depth sampler - see
// `sun_occlusion` and `oag_render::shadow::self_shadow`.
@group(2) @binding(14) var self_shadow_tex: texture_depth_2d_array;

@group(3) @binding(0) var<uniform> anims: TexAnims;

@group(3) @binding(1) var<uniform> node_anims: NodeAnims;

@group(3) @binding(2) var<uniform> emissives: Emissives;
