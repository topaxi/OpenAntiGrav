// The mesh shader's bind groups 0-3: per-draw, per-material, scene and animation resources.

import package::types::{Emissives, NodeAnims, Scene, TexAnims, Uniforms};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;

@if(!bindless) @group(1) @binding(0) var albedo: texture_2d<f32>;

@if(!bindless) @group(1) @binding(1) var albedo_sampler: sampler;

// The circuit's baked lighting atlas, on the surfaces whose material names one.
// Every other draw binds a white 1x1, so the multiply below is the identity and
// no branch is needed - the same arrangement `albedo` already uses.
@if(!bindless) @group(1) @binding(2) var lightmap: texture_2d<f32>;

// HD's pad mask (`_ne`): RGB a tangent-space normal, alpha the light-bar mask.
// Bound third, beside the lightmap, and read only where `slots::PAD_NE` (bit
// 14) is set - see `mesh::rcs::pad_ne`. A flat 1x1 everywhere else.
@if(!bindless) @group(1) @binding(3) var pad_mask: texture_2d<f32>;

// HD's magstrip wave, read only where `slots::MAG_WAVE` is set. `mesh::rcs::mag_wave`.
@if(!bindless) @group(1) @binding(4) var wave_map: texture_2d<f32>;

// A material's fifth and sixth pictures, for an HD program that samples more than
// four units (`Model::fifth_maps`, `Model::sixth_maps`). A white 1x1 elsewhere, and
// read only where the slot's own flag asks for them.
@if(!bindless) @group(1) @binding(5) var fifth_map: texture_2d<f32>;

@if(!bindless) @group(1) @binding(6) var sixth_map: texture_2d<f32>;

// The `bindless` variant (`mesh_render::bindless`): group 1 is a drawable's distinct
// textures and samplers in two arrays, bound once per drawable, and the drawn slot's row
// naming which of them its seven bindings were (`material`). Every function that reads
// the names below declares them from that row first, so the rest of the shader is
// one source.
// Native only; WebGPU has no binding arrays. The `enable` directive is `mesh.wesl`'s,
// the root module's.
@if(bindless) @group(1) @binding(0) var textures: binding_array<texture_2d<f32>>;

@if(bindless) @group(1) @binding(1) var samplers: binding_array<sampler>;

// The drawn slot's indices into `textures` and `samplers`, which the draw pushes as
// immediates when its slot changes (`mesh_render::bindless::Bindless::row`). Immediates
// rather than an index read from the vertex stage: a flat varying is not provably
// uniform, so every array read through it took the non-uniform path, which the Arrow
// Lake iGPU's driver pays for per fragment.
@if(bindless) struct MaterialSlot {
    albedo: u32,
    lightmap: u32,
    pad_mask: u32,
    wave_map: u32,
    albedo_sampler: u32,
    fifth_map: u32,
    sixth_map: u32,
}

@if(bindless) var<immediate> material: MaterialSlot;

@group(2) @binding(0) var<uniform> scene: Scene;

// The Zone stage's `zoneModeTrack<n>.gtf`, and a sampler of its own
// because it is addressed by a coordinate this shader builds rather than by
// the mesh's own - `scene.zone.uv_scale * (1 - meshUV)` runs outside [0, 1]
// wherever the scale does, and the albedo sampler's `Repeat` is what the
// original's own sampler state says there too. Every draw outside a Zone race
// binds a 1x1 black, which is the identity once `scene.zone.enabled` is 0.
// Layer 0 the showing stage, layer 1 the stage the transition sphere sweeps out.
@group(2) @binding(1) var zone_tex: texture_2d_array<f32>;

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
@group(2) @binding(8) var shadow_depth_tex: texture_depth_2d_array;

@group(2) @binding(9) var shadow_depth_sampler: sampler;

// The Zone stage's `zoneMode<n>.gtf`, sampled through `zone_sampler` on the
// same coordinate as `zone_tex`, for a chunk without `ZONE_TRACK` - see
// `zone_set`.
@group(2) @binding(10) var zone_scene_tex: texture_2d_array<f32>;

// Bindings 11 and 12 are free: the stage-transition sphere's Outer half is layer 1 of `zone_tex`
// and `zone_scene_tex` (`mesh_render::zone::layered`), which leaves two of the fragment stage's
// sampled textures for the material group's six.

// The per-craft sun-occlusion maps, one layer per craft, read through the
// coverage map's own clamped filtering sampler - see `sun_occlusion` and
// `oag_render::shadow::occlusion`.
@group(2) @binding(13) var sun_occlusion_tex: texture_2d_array<f32>;

// The per-craft self-shadow depth maps, the same layer per craft, compared
// against through the `mapped` tier's non-filtering depth sampler - see
// `sun_occlusion` and `oag_render::shadow::self_shadow`.
@group(2) @binding(14) var self_shadow_tex: texture_depth_2d_array;

// Wipeout HD's behind-the-glass target, which Vineta K's tunnel glass reads
// through `scene.refraction` and the shadow map's clamped filtering sampler -
// see `refracted` and `mesh::rcs::refraction`. A black placeholder elsewhere.
@group(2) @binding(15) var behind_glass_tex: texture_2d<f32>;

// The per-craft environment probes, one layer per craft: Wipeout HD's
// `paraboloidIblTex`, the blend of the circuit's baked `.probes` at the craft. A hull
// reads it through the coverage map's clamped filtering sampler at the layer
// `uniforms.env_layer` names, and only there - see `lit_texel`.
@group(2) @binding(16) var env_probe_tex: texture_2d_array<f32>;

// The circuit's sky as a dual hemisphere map: Wipeout HD's `paraboloidReflectionTex`, the one
// engine-wide 512x256 target its glass and water programs read at `env_probe_uv(reflect(-V, N))`
// - see `mesh::sky_paraboloid` and `sky_probe`. A black placeholder where there is none.
@group(2) @binding(17) var sky_probe_tex: texture_2d<f32>;

// The per-craft spherical-harmonic ambient: Omega's `shCoeffs`, the blend of the circuit's
// `track.shprobes` at the craft. Ten `vec4`s a row, one row per craft (`uniforms.env_layer - 1`):
// nine rgb coefficients and then `(K, Ks, 1, 0)`; see `sh_hull_ambient`. A buffer rather than a
// texture because the scene group's sixteen sampled textures are the WebGPU default limit. Zero
// (no row flagged) where there is none.
@group(2) @binding(18) var<uniform> sh_probe_table: array<vec4<f32>, 80>;

@group(3) @binding(0) var<uniform> anims: TexAnims;

@group(3) @binding(1) var<uniform> node_anims: NodeAnims;

@group(3) @binding(2) var<uniform> emissives: Emissives;

// **Whether any draw of this pipeline names a row of `emissives`**
// (`mesh_render::emissive_rows`). Row 0 is all zeros (`Emissives::of`), so where no vertex
// carries a material index a read is that zero, and spelling it out lets the driver drop every
// test on the table: a read through a uniform buffer is a value it cannot fold, whatever the
// index folds to. Every `emissive_*` read goes through these two.
override emissive_rows: u32 = 1u;

// **Whether any row of `emissives` carries a program** (`Emissive::program`, a row's `scale.zw`;
// `mesh_render::specialise::programs_present`). 0.0 where every row's is zero, so each program
// test reads the constant zero and the driver drops it: a craft hull names rows for its glow
// alone, and so kept every title's program branches. 1.0, the table deciding, by default.
override programs_present: f32 = 1.0;

// **Which row kinds the table holds** (`mesh_render::specialise::row_codes`): bit `k` of
// `program_codes` is set where some row's program (`scale.z`) is nearest `k`, of `rate_codes`
// where some row's rate (`scale.y`) is; bit 31 stands for every value from 30.5 up. A kind test
// goes through [`program_in`], [`rate_in`] or [`rate_from`], which also ask the bit, so a kind no
// row carries is a constant `false` and the driver drops its branch. All bits, the table
// deciding, by default.
override program_codes: u32 = 4294967295u;
override rate_codes: u32 = 4294967295u;

// The bits `lo` to `hi` of a code mask.
fn code_bits(lo: u32, hi: u32) -> u32 {
    return (0xffffffffu >> (31u - hi)) & (0xffffffffu << lo);
}

// Whether a row's program `v` is a code from `lo` to `hi`: `v > lo - 0.5 && v < hi + 0.5`.
fn program_in(v: f32, lo: u32, hi: u32) -> bool {
    return (program_codes & code_bits(lo, hi)) != 0u && v > f32(lo) - 0.5 && v < f32(hi) + 0.5;
}

// Whether a row's rate `v` is a code from `lo` to `hi`: `v > lo - 0.5 && v < hi + 0.5`.
fn rate_in(v: f32, lo: u32, hi: u32) -> bool {
    return (rate_codes & code_bits(lo, hi)) != 0u && v > f32(lo) - 0.5 && v < f32(hi) + 0.5;
}

// Whether a row's rate `v` is a code `lo` or above: `v > lo - 0.5`.
fn rate_from(v: f32, lo: u32) -> bool {
    return (rate_codes & code_bits(lo, 31u)) != 0u && v > f32(lo) - 0.5;
}

// `slots::MATERIAL_SHIFT` and `slots::MATERIAL_MASK`.
const MATERIAL_SHIFT: u32 = 24u;
const MATERIAL_MASK: u32 = 127u;

// The glow-table row a vertex's slot word names (`slots::material_index`), 0 for none.
fn material_row(slots: u32) -> u32 {
    return (slots >> MATERIAL_SHIFT) & MATERIAL_MASK;
}

fn emissive_scale(entry: u32) -> vec4<f32> {
    if emissive_rows == 0u {
        return vec4<f32>(0.0);
    }
    let row = emissives.scale[entry];
    if programs_present == 0.0 {
        return vec4<f32>(row.xy, 0.0, 0.0);
    }
    return row;
}

fn emissive_tint(entry: u32) -> vec4<f32> {
    if emissive_rows == 0u {
        return vec4<f32>(0.0);
    }
    return emissives.tint_offset[entry];
}

