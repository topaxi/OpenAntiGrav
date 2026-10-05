// Draws a decoded .vex model.
//
// Lighting is a fixed two-light rig rather than the game's own: the point here
// is to see the geometry clearly and spot decoding errors, not to reproduce
// Pulse's look. A single light leaves faces pointing away from it unreadably
// black.

struct Uniforms {
    view_projection: mat4x4<f32>,
    model: mat4x4<f32>,
    // Which layer of the per-craft sun-occlusion array this model samples,
    // **plus one** - `0.0` for none, which is every draw but a Wipeout HD hull
    // under the `original` tier. See `sun_occlusion` below and
    // `oag_render::shadow::occlusion`. The slot was a global
    // texture-animation phase before the authored per-material keyframe
    // blocks (`TexAnims`) replaced it, and sat as padding until this; reusing
    // it keeps the layout `UNIFORMS_SIZE` and four other pipelines mirror.
    //
    // Three trailing f32 fields rather than a vec3: WGSL aligns vec3 to 16
    // bytes, which would silently insert padding this struct's Rust mirror
    // (a flat, tightly packed repr(C)) does not have.
    sun_occlusion_layer: f32,
    // The model's own animation clock, seconds: what an HD material's
    // `UV_offset` is bound to (`slots::CLOCK_SCROLL_RING`/`_HALO`). Zero for
    // every other draw. Was padding until 2026-10-05.
    model_clock: f32,
    _pad1: f32,
    _pad2: f32,
    // The previous simulation tick's `view_projection * model`, premultiplied.
    // **Deliberately not symmetric with the pair above**: fog and lighting
    // need world position, so the current camera and model stay separate;
    // velocity needs only clip position, and splitting this one back into a
    // pair would add 64 bytes to every draw for nothing. Applied to the same
    // node-transformed local position `vs_main` builds `world` from - the
    // node matrices are this frame's, so scenery moved by an `Anim Transform`
    // carries camera velocity only, a recorded approximation.
    prev_mvp: mat4x4<f32>,
};

// Each material's authored texture transform, already sampled for this frame
// on the CPU - `xy` is `TEXSCALE`, `zw` is `TEXOFFSET`. Entry 0 is the
// identity, so a vertex whose `anim` is 0 costs one indexed load and no
// branch.
//
// A fixed-size array rather than a storage buffer: this has to run on the GL
// backend too, where a vertex shader may not read storage. `16_Track`, the
// heaviest circuit measured, authors 17 distinct tracks, so the ceiling is
// generous - `oag_mesh::mesh::ANIM_TRACK_LIMIT` holds the same number and
// the builder clamps against it.
struct TexAnims {
    transform: array<vec4<f32>, 64>,
};

// The authored fog of whichever `fogCube` volume the camera is inside, already
// interpolated across that volume on the CPU - see `oag_formats::fog`. Its own
// bind group rather than fields on `Uniforms`, because that struct is mirrored
// by every pipeline in this crate and by the asset viewer, and only the ones
// that fog need these.
struct Fog {
    colour: vec3<f32>,
    // Distance at which fog begins. Beyond `far` it is total. `enabled` is 0.0
    // or 1.0 rather than a branch so the sky and the viewer can bind a
    // zeroed buffer and get no fog without a second pipeline.
    near: f32,
    camera: vec3<f32>,
    far: f32,
    enabled: f32,
    // Wipeout HD's fog coefficient - see `curve`.
    density: f32,
    // 0.0: the GE's linear ramp over radial distance (Pulse). 1.0: Wipeout
    // HD's curve, read out of its own fragment microcode - every fogged
    // .rcsmaterial variant computes `exp(-(density * view_depth)^2)` (a MUL by
    // log2(e) into EX2_SAT, the product squared and negated) and lerps the fog
    // colour in by it. The distance is the clip-space w its vertex programs
    // write into the interpolant, i.e. view depth, not radial distance.
    curve: f32,
    // Levels added to the PSP slope law's - the player's TEXTURE DETAIL, see
    // `mesh_render::TextureDetail::level_shift`. 0.0 is the recovered law, and
    // what a zeroed buffer holds.
    texlod_shift: f32,
};

// The light rig a Wipeout HD circuit authors in its own `.envsettings`, or the
// stand-in below when nothing does. `enabled` is 0.0 or 1.0 rather than a
// branch, for the reason `Fog::enabled` is: one pipeline, not two.
//
// The direction, the sun's colour and the ambient are all the disc's,
// magnitude included - see `mesh_render::Light`. What is not the disc's is
// the tonemap: this target saturates in its place.
struct Light {
    direction: vec3<f32>,
    enabled: f32,
    ambient: vec3<f32>,
    _lpad0: f32,
    sun: vec3<f32>,
    // 1.0 where the prelit term is Wipeout: Omega Collection's - see
    // `mesh_render::Light::with_nova_prelit` and `lit_texel`.
    nova: f32,
    // The prelit (baked-lightmap) curve and the specular weight, from the
    // circuit's `.envsettings`, applied exactly where its own fragment
    // microcode applies them - see `lit_texel` and `mesh_render::Light`.
    prelit_scale: vec3<f32>,
    specular_scale: f32,
    prelit_power: vec3<f32>,
    prelit_bias: f32,
    // Pulse's GE light list for a hull - `mesh_render::HullLights`. `.w` of
    // the ambient is the switch, `.w` of each direction that light's enable.
    hull_ambient: vec4<f32>,
    hull_direction: array<vec4<f32>, 4>,
    hull_diffuse: array<vec4<f32>, 4>,
};

// One of the two colour groups a Zone stage authors - `mesh_render::ZoneSet`.
struct ZoneSet {
    // `zoneEffect<Inner|Outer>`: the showing stage's `Texture Colour`. `.w`
    // is `EQ brightness`: how hard the visualiser glow drives at this stage,
    // `0.0` before the race starts and `20.0` from `Sub Venom` on for the
    // track set - see `zone_glow` below.
    effect: vec4<f32>,
    // `zoneBase<Inner|Outer>`: `Base Colour Highlight`, the `rim^10`
    // summand. `.w` unused - the exponent is a microcode literal.
    base: vec4<f32>,
    // `zoneBaseAlt<Inner|Outer>`: `Base Colour`, the `rim^5` summand.
    base_alt: vec4<f32>,
};

// The Zone effect's per-stage parameters. See `mesh_render::Zone`, which
// carries the microcode this reproduces and, more importantly, the list of
// terms deliberately left out of it for want of a source on the disc.
struct Zone {
    // `zoneColourTint.xy`, the `.effectSettings` keys `Texture U/V scale`.
    uv_scale: vec2<f32>,
    // 1.0 only when every input this path reads resolved off the disc.
    enabled: f32,
    // `zoneColourTint.w`: the stage-transition sphere's radius this frame.
    radius: f32,
    // `zoneOrigin`: the sphere's centre, the local craft's world position.
    origin: vec4<f32>,
    // The showing stage's two groups - `zone*Inner`: the `Track.*` group,
    // published beside `zone_tex` for a chunk whose `slots` carry
    // `ZONE_TRACK`; the `Scene.*` group beside `zone_scene_tex` for every
    // other chunk. Which is the file's own per-chunk bit - see `zone_set`
    // below and `mesh_render::Zone`.
    track: ZoneSet,
    scene: ZoneSet,
    // The stage being swept out - `zone*Outer` - for a fragment outside the
    // sphere. Equal to the pair above whenever no transition is in flight.
    track_outer: ZoneSet,
    scene_outer: ZoneSet,
};

// Wipeout HD's SPU vertex lights, in the record layout `SpuLight_AddCandidate`
// stores: `position.xyz` and the falloff exponent `w`, then `colour.rgb` and
// the range `D`. `mesh_render::SpuLights` is the mirror; see `spu_light_sum`
// below for the formula and where the evidence for it lives. A fixed-size
// array on the same GL-backend grounds as `TexAnims`: the sum runs in the
// vertex stage, which may not read storage there. 128 is the original's own
// candidate cap; a race fills 8.
struct SpuLight {
    position: vec4<f32>,
    colour: vec4<f32>,
};

struct SpuLights {
    // `.x` is the live count; the other lanes pad to the array's alignment.
    count: vec4<u32>,
    lights: array<SpuLight, 128>,
};

struct Scene {
    fog: Fog,
    light: Light,
    // Engine parameter slot 0, `time`: a global clock in seconds, splatted to
    // four lanes exactly as Wipeout HD's draw-state builders bind it. Only the
    // flame path below reads it. See `mesh_render::Scene::time`.
    time: vec4<f32>,
    zone: Zone,
    shadow: ShadowMap,
    spu_lights: SpuLights,
    // World to each craft's sun-occlusion map's clip space, one per layer of
    // `sun_occlusion_tex` - the `directionalLight0Proj` a Wipeout HD hull is
    // bound. Read only by the layer `uniforms.sun_occlusion_layer` names.
    sun_occlusion: array<mat4x4<f32>, 8>,
};

// The shadow map's projection and how hard it darkens. See
// `mesh_render::ShadowMap`, which carries the microcode this is read from and
// the line between what Wipeout HD does and what this renderer decided.
struct ShadowMap {
    // World to the map's own clip space.
    matrix: mat4x4<f32>,
    // Zero for off, which is what every title but a shadowed one binds.
    strength: f32,
    // 1.0 = Wipeout HD's coverage map, read by the track alone. 2.0 = the
    // `mapped` tier's depth map, read by everything. See
    // `mesh_render::ShadowMap`.
    mode: f32,
    // How far a receiver is pushed towards the light before comparing, in the
    // depth map's own units. Ours; a depth comparison without it makes a lit
    // surface shadow itself.
    depth_bias: f32,
    _pad0: f32,
};

// 1.0 when the render target holds linear light - Wipeout HD's float scene
// target, where the bloom gate reads pre-exposure luminance and the encode
// happens in `post::hd_bloom`'s own pass. 0.0 for every gamma target, where
// this shader encodes (or never decodes) exactly as it always has. Set from
// the target format at pipeline build - see `mesh_render::is_linear_target` -
// so no uniform needs a new field and the sky's zeroed scene buffer cannot
// miss it.
override linear_out: f32 = 0.0;

// 0.0 on a pipeline whose scene can never light by Omega's nova prelit curve,
// so `lit_texel` folds `scene.light.nova` away and the driver deletes the
// curve's second `pow` a channel: 0.55 ms of HD's 22.5 ms opaque track at
// 3200x1800 on the Raphael iGPU. **1.0, the uniform deciding, by default**,
// so every pipeline built outside a race scene - the viewer, a test, a front
// end - shades as it always did. Set by `race::Scene::new` through
// `mesh_render::BuildCacheScope::lit_by`. A constant rather than a rewrite of
// the curve: selecting the `pow`'s input instead stops the driver folding
// HD's `pow(pow(x, 2.2), power)` into one `exp2` and moves pixels.
override nova_prelit: f32 = 1.0;

// **Wipeout HD's engine-flare program**, off by default and on only for a model
// whose material declares the whole parameter set it reads - see
// `oag_mesh::mesh::Flame`, which carries the arithmetic and the evidence, and
// `docs/ghidra/functions/ps3-hdfury-eu/engine-flare.md`, which carries the
// disassembly the arithmetic came from.
//
// **Every number is the material's, passed in as a pipeline constant**, because
// they are a property of the *model* exactly as `colour_is_light` is, and a
// uniform field would have to be mirrored by every pipeline in this crate and
// by the asset viewer. The defaults are zero rather than the shipped values on
// purpose: a model that reaches this path without its parameters must draw
// nothing recognisable, not a flame with numbers this file invented.
// Which shadow maps this model's surfaces may read: 0 reads none, 1 reads the
// `mapped` tier's depth map, 2 reads that *and* Wipeout HD's coverage map.
// Three states because the two tiers have different receivers and the sky is
// out of both - see `mesh_render::ShadowReceiver`.
override receives_shadow: f32 = 0.0;

// The GE's texture level slope and bias for a PSP `.vex` model, off by default
// (0 = the sampler's own derivative-driven selection). See
// `mesh_render::PSP_TEXLOD_SLOPE` for the recovered values and their limits.
override texlod_slope: f32 = 0.0;
override texlod_bias: f32 = 0.0;
// The sampler's `anisotropy_clamp`, for `sample_at_slope_level`. 1 is off.
override aniso_max: f32 = 1.0;

// Samples `albedo` at mip level `lod` and still takes an anisotropic footprint.
// `textureSampleLevel` is an explicit level and the hardware then filters
// isotropically, so the sampler's `anisotropy_clamp` does nothing. This uses
// `textureSampleGrad` with the screen-space derivatives of `uv` reshaped so
// the level the hardware picks is `lod` and the probe count is only what the
// surface needs: with `w = 2^lod` the width of one texel of that level, the
// footprint's long axis is `pmax` base texels, so `probes = ceil(pmax / w)`
// (at most the sampler's clamp). The long gradient is set to `probes * w` and
// the short one to `w`, each along its own direction: the hardware then reads
// a ratio of `probes` and a level of `log2(probes * w / probes) = lod`.
//
// Where the footprint is no wider than one texel of the level (a near surface,
// or anisotropy off) that is one probe, and the plain explicit level. The
// level is the slope law's either way; anisotropy only antialiases along the
// long axis, and never picks a finer level.
fn sample_at_slope_level(uv: vec2<f32>, lod: f32) -> vec4<f32> {
    let size = vec2<f32>(textureDimensions(albedo, 0));
    let tx = dpdx(uv) * size;
    let ty = dpdy(uv) * size;
    let lx = length(tx);
    let ly = length(ty);
    let pmax = max(lx, ly);
    let width = exp2(lod);
    let probes = clamp(ceil(pmax / width), 1.0, aniso_max);
    if probes < 2.0 {
        return textureSampleLevel(albedo, albedo_sampler, uv, lod);
    }
    // The short gradient is a hair over `width` so the hardware's own
    // `ceil(pmax / pmin)` cannot round up to `probes + 1`.
    let short_length = width * 1.01;
    let x_is_long = lx >= ly;
    let long_tx = select(ty, tx, x_is_long);
    let short_tx = select(tx, ty, x_is_long);
    let long_axis = long_tx * (probes * width / pmax);
    let short_len = length(short_tx);
    // A short gradient with no direction of its own takes the long one's normal.
    let normal = vec2<f32>(-long_tx.y, long_tx.x) / pmax;
    let short_axis = select(normal * short_length, short_tx * (short_length / max(short_len, 1.0e-6)), short_len > 1.0e-6);
    let new_x = select(short_axis, long_axis, x_is_long) / size;
    let new_y = select(long_axis, short_axis, x_is_long) / size;
    return textureSampleGrad(albedo, albedo_sampler, uv, new_x, new_y);
}

override flame_shading: f32 = 0.0;
override flame_rim_power: f32 = 0.0;
override flame_rim_scale: f32 = 0.0;
override flame_rim_min: f32 = 0.0;
override flame_alpha_scale: f32 = 0.0;
override flame_colour_scale: f32 = 0.0;
// `Speed`, the sixth number the material authors: what multiplies `scene.time`
// into the noise tap's `v`. 2.0 on all fourteen craft.
override flame_speed: f32 = 0.0;

// **Wipeout HD's absorb shell program** (`hd_absorbinternal.rcsmaterial`), off
// by default and on only for a team's `AbsorbEffect` model - see
// `oag_fx::absorb_shell` and
// docs/ghidra/functions/ps3-hdfury-eu/absorb-feedback.md. Its `0.5` and `5.0`
// are literals in the microcode, not material parameters, so this is a flag
// rather than a parameter set.
override absorb_shading: f32 = 0.0;


// **Whether `in.colour` is a baked light rather than a tint** - 1.0 only for a
// Wipeout HD `.rcsmodel`, set from `Model::vertex_colour_is_light`. HD's
// fragment programs *add* the interpolated colour to the lightmap term before
// multiplying the albedo, so the stand-in path below must not multiply by it.
//
// A separate override from `linear_out` on purpose: that one is the *target's*
// colour space and this is the *model's* vertex semantics. Keying this off
// `linear_out` was tried and rendered HD dark wherever it draws into a gamma
// target - the capture and viewer paths both do.
override colour_is_light: f32 = 0.0;

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

// The world matrix of each `Anim Transform` node the model carries, sampled for
// this frame on the CPU. Binding 1 of group 3 rather than a group of its own:
// wgpu guarantees four bind groups and 0 to 3 are already taken.
//
// Slot 0 is the identity, which is what
// `GpuVertex::xform == 0` selects, so static geometry costs one indexed load and
// no branch - the same shape as `TexAnims` above.
//
// Column-major, which is what `oag_formats::vex`'s row-major-row-vector layout
// already is when read as a `mat4x4<f32>`: `transform_point` computes
// `p.x*m[0] + p.y*m[4] + p.z*m[8] + m[12]`, which is exactly `M * v`.
struct NodeAnims {
    transform: array<mat4x4<f32>, 384>,
};
@group(3) @binding(1) var<uniform> node_anims: NodeAnims;

// Wipeout HD's additive glow, per material - see `oag_mesh::mesh::Emissive`
// and `mesh::slots::ADD_SECOND`. The disc's emissive family samples unit 1 at
// `(u, (v + a) * b + time)`, multiplies by a tint and **adds** the result to
// the albedo gated by the diffuse alpha, where everything below *selects*
// between the two textures. Slot 0 is all zeros, which adds nothing, and is
// what `slots::material_index == 0` selects.
//
// The one table here that is not resampled every frame: `tint`, `a` and `b`
// are authored constants and `scene.time` is already a uniform, so this is
// written once at build.
struct Emissives {
    // `rgb` the tint, `w` the coordinate offset `a`.
    tint_offset: array<vec4<f32>, 64>,
    // `x` the coordinate scale `b`, `y` whether the clock moves this layer.
    scale: array<vec4<f32>, 64>,
};
@group(3) @binding(2) var<uniform> emissives: Emissives;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) colour: vec4<f32>,
    @location(3) texcoord: vec2<f32>,
    @location(4) lit: f32,
    @location(5) anim: u32,
    @location(6) lightmap_texcoord: vec2<f32>,
    @location(7) xform: u32,
    // HD's sun-occlusion mask - see `oag_mesh::mesh::GpuVertex::sun_mask`.
    // Not carried by `colour.a`, which is already the boost plume's baked
    // falloff on other titles and part of a `Written` model's glow mask.
    @location(8) sun_mask: f32,
    // Which texture this surface's colour and coverage come from - see
    // `oag_mesh::mesh::slots`, whose bit layout this file decodes and which
    // is changed together with it. Read off the material's own fragment
    // microcode; `slots::DEFAULT` is the first texture's colour and the first
    // texture's alpha, which is what every title but Wipeout HD carries.
    @location(9) slots: u32,
    // This material's specular exponent, resolved per material - see
    // `oag_mesh::mesh::vertex::GpuVertex::specular_exponent`.
    @location(10) specular_exponent: f32,
    // What this surface stamps into the bloom's glow mask - see
    // `oag_mesh::mesh::GpuVertex::glow` and `glow_stamp` below.
    @location(11) glow: f32,
};

struct VertexOutput {
    @builtin(position) clip: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) colour: vec4<f32>,
    @location(2) texcoord: vec2<f32>,
    @location(3) lit: f32,
    @location(4) world: vec3<f32>,
    @location(5) lightmap_texcoord: vec2<f32>,
    // Clip-space w, which for a perspective projection is view-space depth.
    // What Wipeout HD's own vertex programs hand their fog - see `Fog::curve`.
    @location(6) view_depth: f32,
    @location(7) sun_mask: f32,
    // Flat, because it is a property of the material rather than of the
    // vertex: interpolating a bit field would produce roles nothing authored.
    @location(8) @interpolate(flat) slots: u32,
    // This vertex's clip position under the current and the previous tick's
    // cameras, for the velocity target. The current one is carried
    // explicitly rather than recovered from `@builtin(position)`, which by
    // fragment time is framebuffer coordinates; both divides happen per
    // fragment because dividing per vertex and interpolating is not
    // perspective-correct.
    @location(9) cur_clip: vec4<f32>,
    @location(10) prev_clip: vec4<f32>,
    // Flat for the same reason `slots` is: a property of the material, not
    // of the vertex.
    @location(11) @interpolate(flat) specular_exponent: f32,
    // Wipeout HD's SPU vertex-light sum at this vertex - `spu_light_sum`,
    // computed per vertex and interpolated exactly as the original's
    // `SpuVertexColours` stream is. Zero wherever no list is bound.
    @location(12) spu_light: vec3<f32>,
    // `VertexInput::glow`, flat: a batch stamps one value.
    @location(13) @interpolate(flat) glow: f32,
};

// **Wipeout HD's `EdgeGeom` light loop, per vertex.** Read off the SPU job's
// own binary (`docs/ghidra/functions/ps3-hdfury-eu/renderer.md`, "`EdgeGeom`'s
// light path is read", confidence 80): for every light within range,
//
//     max(0, 1 - |light.pos - P| / D) ^ w  *  max(0, N . L)  *  colour
//
// summed, with `L` the unit vector from the vertex to the light and `w` the
// record's own exponent - `1.0` on every record ever captured, which is the
// job's fast path, so the `pow` is only reached where a record says
// otherwise. The `256/255` is the RGBE round trip the `SVC1` decode's own
// `255`/`128` literals introduce (same page, "The `SVC1` combine is read",
// confidence 88) - the packer's quantisation itself is not reproduced.
//
// **Chosen, not measured**: no per-chunk sphere cull. The original's
// `LightCulling` job drops a light from a chunk's list when it is more than
// `D` from the chunk's bounding sphere - a light whose falloff would be zero
// on every vertex in it anyway - so the sum is the same either way.
fn spu_light_sum(world: vec3<f32>, normal: vec3<f32>) -> vec3<f32> {
    var sum = vec3<f32>(0.0);
    let count = min(scene.spu_lights.count.x, 128u);
    let n = select(vec3<f32>(0.0), normalize(normal), dot(normal, normal) > 0.0);
    for (var i = 0u; i < count; i++) {
        let light = scene.spu_lights.lights[i];
        let range = light.colour.w;
        let d = light.position.xyz - world;
        let dist = length(d);
        if range <= 0.0 || dist >= range {
            continue;
        }
        let linear = 1.0 - dist / range;
        let w = light.position.w;
        let att = select(pow(linear, w), linear, w == 1.0);
        let ndl = max(dot(n, d / max(dist, 1e-6)), 0.0);
        sum += light.colour.rgb * (att * ndl);
    }
    return sum * (256.0 / 255.0);
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    // The `Anim Transform` above this vertex, if any. Slot 0 is the identity,
    // so static geometry passes through unchanged. Applied *before* the model
    // matrix because it is part of the file's own scene-graph chain: the
    // vertex is baked in the anim node's space, and this is the matrix that
    // takes it back to the model's. See `oag_formats::vex::anim_anchors`.
    let node = node_anims.transform[in.xform];
    let placed = node * vec4<f32>(in.position, 1.0);
    let world = uniforms.model * placed;
    out.clip = uniforms.view_projection * world;
    // **The normal goes through the node matrix's inverse transpose**, not the
    // matrix itself: a per-axis scale skews a normal turned by the plain
    // matrix. The model matrix is uniform-scale, so it needs none. The cofactor
    // matrix (columns `c1 x c2`, `c2 x c0`, `c0 x c1`) is the inverse transpose
    // times the determinant, so only its sign is kept - a mirroring node must
    // not flip the normal - and the fragment stage renormalises. Identity for
    // slot 0 (and for a rotation or a uniform scale it is the matrix's own
    // direction), so static geometry and every uniformly-scaled node draw as
    // they did. Wipeout 2048 is the title that needed it: 51 moving, lit
    // meshes across its fourteen race circuits sit under a non-uniformly
    // scaled skeleton node. `tests/moving_normal_inverse_transpose.rs` is the
    // pixel check against the baked path.
    let c0 = node[0].xyz;
    let c1 = node[1].xyz;
    let c2 = node[2].xyz;
    let handed = select(-1.0, 1.0, dot(c0, cross(c1, c2)) >= 0.0);
    let cofactor = mat3x3<f32>(cross(c1, c2), cross(c2, c0), cross(c0, c1));
    let turned = (cofactor * in.normal) * handed;
    out.normal = (uniforms.model * vec4<f32>(turned, 0.0)).xyz;
    out.colour = in.colour;
    out.lit = in.lit;
    out.glow = in.glow;
    // **Pulse's hull, lit the way the GE lights it**: per vertex, the circuit's
    // own ambient plus each directional light's `N . L`, clamped to `0..1` and
    // carried to the fragment as the vertex colour the texel is modulated by.
    // The material is white, so nothing else multiplies it. It replaces both
    // the stand-in rig and the grey an uncoloured vertex otherwise carries,
    // which is why `lit` is cleared. See `mesh_render::HullLights` and
    // docs/ghidra/functions/psp-pulse-usa/scene-light.md.
    if scene.light.hull_ambient.w > 0.5 && in.lit > 0.5 {
        let n = normalize(out.normal);
        var sum = scene.light.hull_ambient.rgb;
        for (var i = 0u; i < 4u; i = i + 1u) {
            let l = scene.light.hull_direction[i];
            sum = sum + scene.light.hull_diffuse[i].rgb * max(dot(n, l.xyz), 0.0) * l.w;
        }
        out.colour = vec4<f32>(clamp(sum, vec3<f32>(0.0), vec3<f32>(1.0)), in.colour.a);
        out.lit = 0.0;
    }
    // Not run through the texture-animation transform below: an animated
    // surface scrolls its diffuse across itself, while its patch in the
    // circuit's lightmap atlas stays where the bake put it.
    out.lightmap_texcoord = in.lightmap_texcoord;
    // `uv * scale + offset`, the GE's own `TEXSCALE`/`TEXOFFSET` arithmetic -
    // the four words `TexAnim_CompileTransformList` (`0x08927358`) writes into
    // each animated material's display list. The direction is the authored
    // sign and is not negated here: a ship's blink lights author `v` running
    // 0 to -1, which is the backward sweep through the palette's rows that a
    // real capture confirmed, and the arrows on `16_Track` author `v` running
    // 0 to +1, which runs the other way on purpose.
    let anim = anims.transform[in.anim];
    out.texcoord = in.texcoord * anim.xy + anim.zw;
    out.world = world.xyz;
    out.view_depth = out.clip.w;
    out.sun_mask = in.sun_mask;
    out.slots = in.slots;
    out.specular_exponent = in.specular_exponent;
    out.cur_clip = out.clip;
    out.prev_clip = uniforms.prev_mvp * placed;
    out.spu_light = spu_light_sum(out.world, out.normal);
    return out;
}

// Colour and screen-space velocity together, for the pipelines built against
// the race's two attachments - see `mesh_render::Velocity`.
struct MrtOutput {
    @location(0) colour: vec4<f32>,
    @location(1) velocity: vec2<f32>,
}

// How far this surface point moved on screen since the previous tick, in uv
// units: the NDC delta halved and y-flipped, which is the encoding the
// reconstruction filter expects. A point that was behind the previous
// camera's eye plane has no meaningful previous position and reports zero.
fn velocity_of(in: VertexOutput) -> vec2<f32> {
    if in.prev_clip.w <= 0.0 {
        return vec2<f32>(0.0);
    }
    let cur = in.cur_clip.xy / in.cur_clip.w;
    let prev = in.prev_clip.xy / in.prev_clip.w;
    return (cur - prev) * vec2<f32>(0.5, -0.5);
}

// The GE's fog is a linear ramp between `near` and `far` - `Gu_Fog`
// (`0x08811748`) sends `far` and `1/(far - near)` and nothing else, so there is
// no curve to reproduce here. What the original varies is the *parameters*,
// re-sampled every frame from the camera's position inside the fog volume;
// that happens on the CPU before this uniform is written. See
// `docs/ghidra/functions/psp-pulse-usa/fog.md`.
//
// **Radial distance from the eye, where the hardware uses view-space depth.**
// The two differ towards the screen edges, by up to `1 / cos(fov / 2)` - about
// 15 % at the corners of Pulse's authored field of view. Reproducing view-space
// z needs the view matrix separately, which this uniform block does not carry;
// recorded as a known divergence rather than silently accepted.
// How much of this surface point the shadow map covers, `0.0` to `1.0`.
//
// **A projective sample of a coverage map, not a depth compare** - which is
// what Wipeout HD's own track material does: `TXP R1.x, f[TC0] unit2`, then
// `1 - R1.x`, with nothing compared against anything. See
// `mesh_render::ShadowMap`.
//
// Outside the map's own frustum the sample is clamped to its border, which the
// caster pass leaves at zero, so a surface the map does not cover is lit.
fn shadow_coverage(world: vec3<f32>) -> f32 {
    if scene.shadow.strength <= 0.0 {
        return 0.0;
    }
    // **Who may read which map.** In the coverage mode the receiver is the
    // track alone, which is what Wipeout HD's own materials say; in the depth
    // mode every lit surface reads, which is the whole point of a tier that
    // is not the original's.
    let coverage_mode = scene.shadow.mode < 1.5;
    let allowed = select(1.0, 2.0, coverage_mode);
    if receives_shadow < allowed {
        return 0.0;
    }
    let clip = scene.shadow.matrix * vec4<f32>(world, 1.0);
    if clip.w <= 0.0 {
        return 0.0;
    }
    let ndc = clip.xyz / clip.w;
    // Behind the map's near plane or past its far one: nothing to sample.
    if ndc.z < 0.0 || ndc.z > 1.0 {
        return 0.0;
    }
    // **`ndc.y` is up, and the map's row 0 is its top.** `oag_core::math::
    // camera`'s projections are glam's `rh::proj::directx` pair, which is
    // Y-up NDC with a `0..1` depth (glam's own `camera/rh/proj.rs` says so),
    // and the caster pass rasterises `ndc.y = +1` into texel row 0 like any
    // wgpu render target. So the lookup is `0.5 - ndc.y * 0.5`. The other
    // sign mirrors the map about its own horizontal axis, and that is a bug
    // that hides itself: a caster near the middle still lands near the
    // middle, and only the *side* the shadow falls on is wrong - which, for
    // a low sun, reads as the shadow sitting on the wrong side of the craft
    // and its silhouette turning the wrong way as the craft turns. Pinned by
    // `tests/shadow_map_coverage.rs`, which reads a caster's texel back
    // through this same formula.
    let uv = vec2<f32>(ndc.x * 0.5 + 0.5, 0.5 - ndc.y * 0.5);
    // Outside the map's own rectangle the clamped sampler would smear its
    // border across the whole circuit, so a receiver the map does not cover is
    // lit rather than edge-coloured.
    if uv.x < 0.0 || uv.x > 1.0 || uv.y < 0.0 || uv.y > 1.0 {
        return 0.0;
    }
    if coverage_mode {
        return textureSample(shadow_tex, shadow_sampler, uv).r * scene.shadow.strength;
    }
    // The depth mode: four taps a texel apart, each compared, and their mean.
    // **The softening is in the comparison, not in the sample** - filtering
    // the depth itself would average two surfaces into one that is neither.
    // Four rather than sixteen because this runs on every lit pixel of the
    // frame; it is a choice, and it is ours, like everything else in this
    // tier.
    let texel = 1.0 / f32(textureDimensions(shadow_depth_tex).x);
    var lit = 0.0;
    for (var i = 0; i < 4; i = i + 1) {
        let offset = vec2<f32>(
            f32(i % 2) * 2.0 - 1.0,
            f32(i / 2) * 2.0 - 1.0
        ) * texel * 0.5;
        let nearest = textureSampleLevel(
            shadow_depth_tex,
            shadow_depth_sampler,
            uv + offset,
            // A depth texture's own `textureSampleLevel` takes an `i32` mip
            // level, not the `f32` a colour texture's does.
            0
        );
        lit = lit + select(0.0, 1.0, ndc.z - scene.shadow.depth_bias <= nearest);
    }
    return (1.0 - lit * 0.25) * scene.shadow.strength;
}

// How far a Wipeout HD hull is pushed towards the sun before its depth is
// compared against its own self-shadow map, in that map's `0..1` units - a
// 140-unit-deep box, so this is a seventh of a unit.
//
// **Ours, and it can be small**: the caster pass culls front faces, so a
// sunlit face compares against the hull's far side rather than against
// itself, and the slope-scaled bias in `shadow::self_shadow::Maps::new`'s
// rasteriser handles the panels seen edge-on. This is the floor under it for
// a thin panel whose two sides nearly coincide.
const SELF_SHADOW_BIAS: f32 = 0.001;

// How much sun reaches this point of a Wipeout HD hull, `0.0` to `1.0` - the
// product of the two maps the original's `ShadowMap` hull variant multiplies
// its sun by:
//
//     TXP R0.x, f[TC0] unit1      <- directionalLight0ShadowTex, compared
//     TXP R0.z, f[TC0] unit2      <- directionalLight0LightmapTex, projected
//     MUL H0.w, R0.xxxx, R0.zzzz
//     MAD H0.xyz, H0, H0.wwww, {constantAmbientColour}
//
// `docs/ghidra/functions/ps3-hdfury-eu/ship-sun-occlusion.md`. The first is
// the craft's own depth from the sun, front faces culled, so a wing shadows
// the fuselage under it; the second is the track within ten units of the
// craft drawn from the sun as its own `lightmap.a`/`colourSet.w`, over black,
// so a craft over nothing reads 0. Both project through the one matrix, as
// the original's one `f[TC0]` does, so a texel of each is the same point of
// the hull.
//
// `1.0` for every draw that names no layer, which is everything but a hull
// under the `original` tier on a title that renders these maps - **and that
// early return is the whole of every other title's inertness**: nothing
// below it runs for a draw that names none.
fn sun_occlusion(world: vec3<f32>) -> f32 {
    let layer = i32(uniforms.sun_occlusion_layer + 0.5) - 1;
    if layer < 0 {
        return 1.0;
    }
    let clip = scene.sun_occlusion[layer] * vec4<f32>(world, 1.0);
    if clip.w <= 0.0 {
        return 1.0;
    }
    let ndc = clip.xyz / clip.w;
    // The same `0.5 - ndc.y * 0.5` flip `shadow_coverage` documents: the
    // pass rasterises `ndc.y = +1` into row 0.
    let uv = vec2<f32>(ndc.x * 0.5 + 0.5, 0.5 - ndc.y * 0.5);
    // Off the map's own rectangle the original's clamped sample would return
    // the border, which its black clear leaves at 0 - a craft's hull always
    // fits its own box, so this is the box's margin and reads as no sun.
    // `textureSampleLevel` rather than `textureSample`: the layer index is
    // dynamic and this sits behind a branch, and a map with one mip has no
    // level to pick anyway.
    let occlusion = textureSampleLevel(sun_occlusion_tex, shadow_sampler, uv, layer, 0.0).r;
    // The self-shadow tap: the four compared taps `shadow_coverage`'s depth
    // mode takes, half a texel apart, and their mean. The original's is one
    // hardware-compared `TXP`; how the RSX filters that is unread, and four
    // is the softening this side already uses for a depth compare.
    let texel = 1.0 / f32(textureDimensions(self_shadow_tex).x);
    var lit = 0.0;
    for (var i = 0; i < 4; i = i + 1) {
        let offset = vec2<f32>(
            f32(i % 2) * 2.0 - 1.0,
            f32(i / 2) * 2.0 - 1.0
        ) * texel * 0.5;
        let nearest = textureSampleLevel(self_shadow_tex, shadow_depth_sampler, uv + offset, layer, 0);
        lit = lit + select(0.0, 1.0, ndc.z - SELF_SHADOW_BIAS <= nearest);
    }
    return occlusion * lit * 0.25;
}

// The shadow term, applied to colour.
//
// **On colour rather than alpha, and that is a deviation.** Wipeout HD puts
// `1 - shadow` in the fragment's *alpha* - which is what its `ShadowToAlpha`
// material flag names - and a later compositing pass consumes it; that pass is
// unread. This renderer's alpha is the bloom's glow mask, so writing there
// would bloom the shadow. See `docs/rendering/shadows.md`.
//
// **Called from every entry point that draws a lit surface**, which is five of
// them and not one: a race draws through the `_velocity` pair, so a version of
// this that only reached `fs_main` shadowed the scenery a blended material drew
// and left the road untouched. That is exactly what the first cut did.
fn shadowed(colour: vec3<f32>, world: vec3<f32>) -> vec3<f32> {
    return colour * (1.0 - shadow_coverage(world));
}

fn fogged(colour: vec3<f32>, world: vec3<f32>, view_depth: f32) -> vec3<f32> {
    let distance = length(world - scene.fog.camera);
    let span = max(scene.fog.far - scene.fog.near, 1e-6);
    // 1.0 is clear, 0.0 is fully fogged, matching the GE's own sense.
    let linear = clamp((scene.fog.far - distance) / span, 0.0, 1.0);
    // Wipeout HD's curve, from its own microcode - see `Fog::curve` above.
    // Saturated exactly as the original's EX2_SAT is.
    let scaled = scene.fog.density * view_depth;
    let authored = clamp(exp(-scaled * scaled), 0.0, 1.0);
    let factor = mix(linear, authored, scene.fog.curve);
    return mix(scene.fog.colour, colour, mix(1.0, factor, scene.fog.enabled));
}

// **The Zone recolour**, read out of the Zone variant compiled into the disc's
// own materials rather than out of any engine program - see
// `docs/ghidra/functions/ps3-hdfury-eu/zone-shader.md`:
//
//     zoneUV  = zoneColourTint.xy * (1 - meshUV)
//     rim     = 1 - dot(N, toEye)
//     surface = zoneTex(zoneUV).rgb * zoneEffect.rgb
//             + zoneBase.rgb    * rim^10
//             + zoneBaseAlt.rgb * rim^5
//
// **The albedo is not in that sum.** On a circuit the Zone variant replaces
// the material's shading rather than tinting it, which is why the original's
// Zone frame reads near-monochrome: crowds, banners and concrete all go, and
// what is left is the stage's own two colours over the stage's own texture.
//
// A census of all 20,214 Zone-bearing fragment blocks on the disc finds
// two shapes and no block in both: this one, 20,084 blocks, which never
// carries the `blackMask` literal at all; and a `zoneAnisoPalette` shape of
// 130 blocks which always does. All twelve racing circuits are 100% this
// shape; the other lives only in `zone_1`..`zone_4`, HD's Zone arenas. The
// `albedo + zoneCol * (1 - blackMask)` rule this file used to carry was the
// arena shape, read correctly from an arena material and generalised.
//
// The zone texture is sampled at the *mesh's own* UV, not in screen space and
// not projected through `zoneOrigin` - which is the counter-intuitive half of
// the reading, and is confirmed from the file side by the engine's own schema
// naming the two lanes "Texture U scale" and "Texture V scale".
//
// **This returns the bare sample, not the finished term**, because the two
// shading paths below sum in different colour spaces and `pow(a * b, g)` is
// not `pow(a, g) * b`. `zoneTex` is a texture and wants the same decode every
// other sample here gets; `zoneEffect` is a shader parameter and wants none -
// it is authored well past 1.0 (to 3.0 on stage 12), which is a multiplier's
// range and not a colour's. `zoneBase*` are parameters on the same terms.
//
// **Which of the two publications a fragment reads is the chunk's own
// bit.** HD publishes the parameters twice - `zoneModeTrack*` beside the
// `Track.*` colours, `zoneMode*` beside the `Scene.*` ones - and
// `FUN_003ff860` picks per chunk on bit 0 of its render-block flags, which
// `mesh::rcs` carries here as `slots::ZONE_TRACK` (bit 9 of `in.slots`).
// Both textures are sampled and one selected, rather than branched on: a
// `textureSample` wants uniform control flow, and a flat-interpolated
// per-vertex word is not that.
//
// **Which of the Inner and Outer copies a fragment reads is a world-space
// sphere: `distance(worldPos, zoneOrigin) < zoneColourTint.w`** - read with
// opposite compiler polarity in two materials, and the same rule both times
// (`zone-shader.md`, 82). Inside it the stage that just committed, outside
// it the one before: a stage change on HD is a sphere growing out of the
// local craft that repaints the world as it passes, not a cross-fade. The
// radius and origin come from the game side every frame by the law read out
// of `Environment_UpdateStageBlend` and reproduced live - see
// `mesh_render::Zone`, "the stage-transition wavefront", for the numbers
// and for the one input still unmeasured (the radius's unit against this
// renderer's world). Before any transition the two pairs are equal and this
// selects the same colours either side, which is also every draw outside a
// Zone race.
//
// **Scope, stated because it is an approximation.** This applies to every
// surface `mesh.wgsl` draws for a Zone race, and the original applies it per
// material. Craft are the exclusion that mattered and they are handled at the
// call site rather than here: `data/materials/ships/*` carries no Zone block
// at all, so `race::scene::frame` binds a default (disabled) Zone to the
// ships. The sky cube, the pads and the collision wireframe do still reach
// this path, which is the same shape of blanket approximation as the shared
// specular exponent and the blanket sun term above. **The stage texture now
// follows the sphere too**: `zone_tex_outer`/`zone_scene_tex_outer` are the
// stage being swept out's own `zoneTexInner`/`zoneTexOuter` publication,
// rebuilt on the same stage-change edge `oag_raceplay::Scene::rebind_zone_art`
// rebinds bind group 2 on rather than every frame - see
// `oag_mesh::mesh_render::zone::rebind`.
fn zone_is_track(slots: u32) -> bool {
    return (slots & 512u) != 0u;
}

fn zone_inside(world: vec3<f32>) -> bool {
    return distance(world, scene.zone.origin.xyz) < scene.zone.radius;
}

fn zone_set(slots: u32, world: vec3<f32>) -> ZoneSet {
    let inside = zone_inside(world);
    if zone_is_track(slots) {
        if inside {
            return scene.zone.track;
        }
        return scene.zone.track_outer;
    }
    if inside {
        return scene.zone.scene;
    }
    return scene.zone.scene_outer;
}

// Samples all four stage textures unconditionally and `select`s between them
// - never branches to decide *which* texture to sample - the same shape
// `zone_set` above already reads its four `ZoneSet`s in, and needed here for
// the same reason: `textureSample` wants uniform control flow, and a `select`
// on its *result* (or, as below, on which pre-sampled result to keep) satisfies
// that where an `if` around the call itself would not.
fn zone_sample(uv: vec2<f32>, slots: u32, world: vec3<f32>) -> vec3<f32> {
    let zone_uv = scene.zone.uv_scale * (1.0 - uv);
    let track = textureSample(zone_tex, zone_sampler, zone_uv).rgb;
    let track_outer = textureSample(zone_tex_outer, zone_sampler, zone_uv).rgb;
    let general = textureSample(zone_scene_tex, zone_sampler, zone_uv).rgb;
    let general_outer = textureSample(zone_scene_tex_outer, zone_sampler, zone_uv).rgb;
    let inside = zone_inside(world);
    return select(
        select(general_outer, track_outer, zone_is_track(slots)),
        select(general, track, zone_is_track(slots)),
        inside,
    );
}

// The two rim-lit summands, `zoneBase.rgb * rim^10 + zoneBaseAlt.rgb * rim^5`.
//
// Both exponents are **inline literals** in every one of the 20,084 blocks
// that carry this shape, not parameters, so nothing per-stage drives them and
// there is no uniform for them to come from.
//
// **`max(rim, 0)` is load-bearing.** The microcode's own `ADD R3.w, -R0,
// {1, 0, 0, 0}` is *unsaturated*, so `rim` runs negative on a back-facing or
// badly-normalled fragment; the hardware's `LG2` -> `MUL` -> `EX2` chain floors
// there at zero, while WGSL's `pow` with a negative base is undefined and hands
// back NaN. A NaN here survives every arithmetic below and paints a hole no
// unit test on the formula would catch.
//
// Undecoded on both shading paths, like the set's `effect`: these are shader
// *parameters*, authored in the `.effectSettings` file's own float domain, not
// texture samples that owe an sRGB decode.
fn zone_base_term(rim: f32, colours: ZoneSet) -> vec3<f32> {
    let r = max(rim, 0.0);
    return colours.base.rgb * pow(r, 10.0) + colours.base_alt.rgb * pow(r, 5.0);
}

// The visualiser glow: `saturate(N.y - 0.5) * (1 - windowDepth) * E.w *
// zoneTexVis[band].rgb`, read out of the Zone shader's own fragment microcode
// - `docs/ghidra/functions/ps3-hdfury-eu/zone-shader.md`. Re-read
// instruction by instruction against the project's own fixed decoder on
// 2026-08-31 (the condition-code and parameter-patch defects that page's own
// "Two `ps3-microcode.py` defects" section fixed the same day) and found
// **complete**: every register in the block traces to a named parameter, a
// known texture unit or the fog term already read elsewhere in this file, so
// nothing multiplies into it that this project has not already read.
// Confidence 84 on the structure, unchanged by the re-read since it only
// confirmed rather than moved it.
//
//     band = zoneTex<I|O>Nearest(zoneUV).a
//     glow = max( saturate(N.y - 0.5) * (1 - windowDepth) * E.w
//                * zoneTexVis[band].rgb , 0 )
//
// `zoneUV` is the same coordinate `zone_sample` builds - the microcode reuses
// it rather than a second one. `E.w` is the chunk's set's `effect.w` - `EQ
// brightness`, `Track.` or `Scene.` per `zone_set`: the disc's own per-stage
// drive scalar, `0.0` before the race starts and `20.0` from `Sub Venom` on.
//
// **`zoneTexVis` here is not the original's own table.** HD/Fury zero-fills
// it at load and rewrites it every frame from `Environment_UpdateStageBlend`
// - confirmed, not a static ramp - but *what* it writes is a float compared
// against the recovered per-stage threshold ladder, and whether that float is
// itself audio-reactive or a plain stage-progress fraction is unread; closing
// it needs an RPCS3 watchpoint on the value the comparison reads. See
// `docs/ghidra/functions/ps3-hdfury-eu/zone-effectsettings-loader.md`'s
// twenty-sixth pass. This samples `zone_vis_tex`, which
// `oag_mesh::mesh_render::zone::write_vis` fills every frame from a genuine
// spectrum of this project's own mixer output - the honest reading of "feed
// the shader audio data" while the original's own feed is unread, not a
// transcription of anything on the disc. See that function's own doc
// comment.
//
// **One additive term of the microcode's own glow is left out**:
// `5.0 * saturate(1 - 0.1 * (distance - zoneColourTint.w))`. Both of its
// inputs are bound now - `zone_inside` reads the same origin and radius -
// and it is still not drawn, for a different reason than before: as read,
// the term is `5.0` for every fragment *inside* the sphere (the argument
// saturates to `1` wherever `distance <= radius`) and only tails off over
// ten units past the boundary, which would flood the new stage's whole
// interior white. The live RPCS3 frame at radius `799` shows nothing of the
// kind - the near track is plainly the new stage's colours. So the reading
// of that term is what is in doubt, not its inputs, and it stays out until
// the microcode is re-read rather than being tuned into an edge glow that
// looks right.
//
// `window_depth` is `@builtin(position).z` read at fragment time - WebGPU's
// own window-space depth in `0..1`, the same convention the microcode's own
// `f[POS]` carries.
fn zone_glow(n: vec3<f32>, window_depth: f32, uv: vec2<f32>, slots: u32, world: vec3<f32>) -> vec3<f32> {
    let zone_uv = scene.zone.uv_scale * (1.0 - uv);
    // The band comes from whichever stage texture this chunk publishes - the
    // scene set's is alpha 255 everywhere, so a scene chunk indexes the last
    // entry - on whichever side of the sphere this fragment is, and `E.w`
    // from the matching colour group: `band = zoneTex<I|O>Nearest(zoneUV).a`
    // above already names both sides. Four samples, `select`ed rather than
    // branched into, for the same uniform-control-flow reason `zone_sample`
    // is.
    let track_band = textureSample(zone_tex, zone_nearest_sampler, zone_uv).a;
    let track_outer_band = textureSample(zone_tex_outer, zone_nearest_sampler, zone_uv).a;
    let scene_band = textureSample(zone_scene_tex, zone_nearest_sampler, zone_uv).a;
    let scene_outer_band = textureSample(zone_scene_tex_outer, zone_nearest_sampler, zone_uv).a;
    let inside = zone_inside(world);
    let band = select(
        select(scene_outer_band, track_outer_band, zone_is_track(slots)),
        select(scene_band, track_band, zone_is_track(slots)),
        inside,
    );
    let vis = textureSample(zone_vis_tex, zone_vis_sampler, vec2<f32>(band, 0.5)).rgb;
    let up = clamp(n.y - 0.5, 0.0, 1.0);
    let drive = zone_set(slots, world).effect.w;
    return max(up * (1.0 - window_depth) * drive * vis, vec3<f32>(0.0));
}

// **The pad's tangent-space normal, in world space.** The pad programs build
// `N = nx*T + ny*B + nz*N0` from the `_ne` texel (`x*2-1` on each lane), with
// `T` an authored vertex attribute and `B = cross(N0, T) * w`
// (`docs/rendering/pads.md`, "The pad's tangent frame"). Two measurements
// shape this function:
//
// - The authored `T` points along `dP/du` of its triangle (mean dot 0.94 to
//   0.99 over 252 to 503 triangles per chunk, `hd_pad_tangent_probe`), so
//   `dP/du` from screen-space derivatives of the world position and the
//   texture coordinate stands in for it, with no new vertex attribute.
//   **The derived frame is chosen, not measured**: the authored stream is
//   per vertex and smoothed, and was not decoded into `GpuVertex`.
// - The authored `w` byte that scales `B` is `0` on 232 of 232 vertices of
//   `12_sol_2`'s `Weapon Pad` chunks, on 839 and 836 of 840 on
//   `talons_junction`'s, and on 209 to 213 of 216 on `12_sol_2`'s `Speedup
//   Pad` chunks, so `B` is the zero vector and `ny` never reaches `N`. The
//   disc's own normal is `nx*T + nz*N0`, and that is what this builds.
@diagnostic(off, derivative_uniformity)
fn pad_normal(ne: vec4<f32>, n: vec3<f32>, world: vec3<f32>, uv: vec2<f32>) -> vec3<f32> {
    let q1 = dpdx(world);
    let q2 = dpdy(world);
    let s1 = dpdx(uv);
    let s2 = dpdy(uv);
    let det = s1.x * s2.y - s2.x * s1.y;
    // A degenerate or unmapped pixel keeps the vertex normal.
    if abs(det) < 1.0e-20 {
        return n;
    }
    let dp_du = (q1 * s2.y - q2 * s1.y) / det;
    let t = normalize(dp_du - n * dot(n, dp_du));
    return normalize((ne.x * 2.0 - 1.0) * t + (ne.z * 2.0 - 1.0) * n);
}

// **A branch on `slots` keeps its derivatives.** `slots` is a flat varying,
// and a 2x2 fragment quad - helper lanes included - is always shaded for one
// primitive, so every lane of a quad reads the same flat value and takes the
// same side of the branch. An implicit-derivative fetch inside it therefore
// has the neighbours it needs. WGSL's analysis cannot see that about a
// varying, so it is told here.
@diagnostic(off, derivative_uniformity)
fn lit_texel(in: VertexOutput) -> vec4<f32> {
    let n = normalize(in.normal);
    // **HD's pads** (`slots::PAD_NE`): the `_ne` texel replaces the vertex
    // normal in the program's own `N.L` and `N.H` below, and its alpha gates
    // the glow added after the light. `n` itself stays the vertex normal for
    // everything else this function reads it for.
    let pad_ne = (in.slots & 16384u) != 0u;
    var pad_ne_texel = vec4<f32>(0.0);
    var n_lit = n;
    if pad_ne {
        pad_ne_texel = textureSample(pad_mask, albedo_sampler, in.texcoord);
        n_lit = pad_normal(pad_ne_texel, n, in.world, in.texcoord);
    }

    // The stand-in: two invented directions, kept for every title whose own
    // rig has not been recovered. See this file's header.
    let key = max(dot(n, normalize(vec3<f32>(0.4, 0.8, 0.5))), 0.0);
    let fill = max(dot(n, normalize(vec3<f32>(-0.5, 0.2, -0.7))), 0.0);
    let stand_in = vec3<f32>(0.15 + 0.75 * key + 0.25 * fill);

    let ndl = clamp(dot(n_lit, scene.light.direction), 0.0, 1.0);

    // **Everything from here to `authored` is the authored rig's, and only
    // runs when the rig is on.** Off - `Light::stand_in`, which is every
    // circuit with no `.envsettings` to read, Pulse's and Pure's included -
    // the output mixes this whole path in at weight zero, and it was costing
    // a lightmap fetch, the sun-occlusion lookup and half a dozen `pow`s a
    // pixel for nothing: half the race pass on an integrated GPU.
    // `enabled` is a uniform, so the fetches keep their derivatives.

    // **The circuit's own baked lighting.** A material naming an
    // `lmaps/*-lmap.gtf` in its second texture slot is drawn through it,
    // sampled at the `lightmapUV` the chunk's vertex declaration names - see
    // `docs/formats/rcsmaterial.md`. Every draw without one binds a black,
    // alpha-1 placeholder, which zeroes the prelit term and passes the sun
    // below - exactly the equation the original's own lightmap-less shader
    // variants state.
    // **Only when the second texture actually is one.** Binding the second
    // slot unconditionally is what let its other roles be read at all, and it
    // is also how the prelit term could silently start sampling a cloud mask:
    // a non-lightmap chunk declares no `lightmapUV`, so this would read one
    // texel at (0, 0) of whatever is bound instead of the black placeholder
    // that switches the term off. `slots::SECOND_IS_LIGHTMAP` is the material's
    // own four-signal reading; without it the value is the placeholder's, to
    // the bit.
    var authored = vec3<f32>(0.0);
    var mask = 0.0;
    var emissive = false;
    if scene.light.enabled != 0.0 {
        let atlas = textureSample(lightmap, albedo_sampler, in.lightmap_texcoord);
        let baked = select(
            vec4<f32>(0.0, 0.0, 0.0, 1.0),
            atlas,
            (in.slots & 1u) != 0u,
        );

        // The authored rig, and the combination is no longer this project's: it is
        // the one every lit variant of an HD circuit `.rcsmaterial` computes,
        // read out of the fragment microcode
        // (docs/ghidra/functions/ps3-hdfury-eu/renderer.md, "The lit track
        // material"). The lightmap enters through a power curve - `Prelit ambient
        // colour scale/power`, the .envsettings names - and is *added* to a
        // constant, not multiplied by one:
        //
        //     block #9, lightmapped:  pow(lightmap, power) * scale + f[TC1]
        //     block #8, no lightmap:  f[TC1] + constantAmbientColour
        //
        // and that sum multiplies the albedo. **Neither block has an `N.L` or a
        // sun colour of its own** - `track_surface`'s 15 chunks of Talon's
        // Junction compute exactly this and nothing more.
        //
        // `scene.light.ambient` is `constantAmbientColour` - the preimage of the
        // hash the microcode patches into block #8's `{const}`. `f[TC1]` is HD's
        // per-vertex light, which `mesh/rcs.rs` decodes out of the colour set and
        // hands over in `in.colour.rgb`; it is **added**, as both variants add
        // it, and is zero on a chunk that declares no colour set - which is what
        // the original's own vertex programs broadcast there. Note `f[TC1]` is
        // that material's interpolator and not a convention: the same closer
        // arrives on `f[TC0]` in `talons_junction/bluemetal`, so a per-material
        // path will have to read which varying carries it.
        let baked_linear = pow(baked.rgb, vec3<f32>(2.2));
        // **Wipeout: Omega Collection's combination, on a lightmapped draw.** Read
        // out of its circuit pixel shaders (ps4-omega-eu/lightmap-prelit.md):
        // `v_log_f32` / `v_mul_f32 power` / `v_exp_f32` / `v_mad_f32 scale, bias` on
        // the *raw* atlas - its descriptor is BC7 UNORM on all 1,028 atlases, so
        // there is no sRGB decode in front of the curve - and no constant ambient
        // joins the sum (0 of 4,664 nova shaders declare one). `scale`, `bias` and
        // `power` are the authored triple's three scalars. Gated on the lightmap
        // bit: a draw with no lightmap binds the black placeholder, and the bias
        // must not light it.
        let nova = nova_prelit != 0.0 && scene.light.nova > 0.5 && (in.slots & 1u) != 0u;
        let prelit_hd = scene.light.prelit_scale * pow(baked_linear, scene.light.prelit_power);
        let prelit_nova = scene.light.prelit_scale * pow(baked.rgb, scene.light.prelit_power)
            + vec3<f32>(scene.light.prelit_bias);
        let prelit = select(prelit_hd, prelit_nova, nova);
        // `select` rather than a multiply by the override, so the value is not
        // touched at all where the flag is set. (Measured: it makes no difference
        // to the frame either way - both forms move the same 9 pixels of 1,175,040
        // by one level, and that movement is the `tint` line below rather than this
        // one. Kept because not multiplying is the clearer statement.)
        let vertex_light = select(vec3<f32>(0.0), in.colour.rgb, colour_is_light > 0.5);

        // `slots::NO_AMBIENT`/`slots::NO_SUN`, bits 6/7 - read here, ahead of
        // `emissive`'s own paragraph below, because the ambient fix that follows
        // needs `no_ambient` before that point and this keeps the two bit reads
        // from drifting apart.
        let no_ambient = (in.slots & 64u) != 0u;
        let no_sun = (in.slots & 128u) != 0u;
        emissive = no_ambient && no_sun;

        // **The disc's own compiled shader table carries three ambient sources
        // per chunk, not one** - see
        // docs/ghidra/functions/ps3-hdfury-eu/renderer.md, "The per-material
        // microcode sweep: none of the four ceiling programs ever references
        // the ambient constant...". Selected by `Features::chunk_word`:
        // `IleLightmap` -> the `prelit` curve above (already this chunk's only
        // term, untouched here); `IleVertex` -> the same curve shape applied to
        // the baked per-vertex colour set instead of the lightmap texel -
        // `pow(colour_set, prelitBias) * prelitScaleSpecular`; `Ambient`
        // (neither) -> flat `constantAmbientColour`, i.e. `scene.light.ambient`
        // unmodified. `slots::NO_AMBIENT` is exactly "this chunk's own program
        // is not the `Ambient` case" - confirmed on every chunk measured on
        // Amphiseum, Anulpha Pass and Talon's Junction. Dropping
        // `scene.light.ambient` alone on these chunks regresses an
        // `IleVertex`/no-lightmap/sun-occluded chunk to near-black, because the
        // raw (uncurved) vertex colour this project already wires as
        // `vertex_light` is far dimmer than the curved value - so the drop and
        // the curve are one change, not two.
        //
        // `prelitBias`/`prelitScaleSpecular` resolve to the same two
        // `.envsettings` keys already bound above as `prelit_scale`/
        // `prelit_power`: `Scene_PrepareFrame` binds `Prelit ambient colour
        // scale`/`power` into the per-draw shader-parameter table at offsets
        // `+0x198`/`+0x1b8` (renderer.md, "`Constant ambient color` (`+0x420`)
        // is confirmed wired..."), and the engine's own 81-entry parameter
        // table uses `offset = 0x18 + slot * 0x20` (renderer.md, "The engine's
        // own parameter table") - solving that for `0x198`/`0x1b8` gives slots
        // 12/13, which the same table names `prelitScaleSpecular`/`prelitBias`.
        // Two independently-traced offset chains landing on the same slot
        // indices, not a name-string or value/shape match alone.
        //
        // No sRGB predecode here, unlike `prelit` above: the colour-set vertex
        // program reads `v[3]` raw (`LG2 -> MUL -> EX2 -> MUL`), not through the
        // lightmap's `pow(_, 2.2)` step. `EMISSIVE` chunks (`no_ambient &&
        // no_sun`) are excluded from the curve below - this is additive to that
        // branch, not a rewrite of it.
        let vertex_light_curved = scene.light.prelit_scale
            * pow(vertex_light, scene.light.prelit_power);
        let vertex_light_term = select(vertex_light, vertex_light_curved, no_ambient && !emissive);
        let ambient_term = select(scene.light.ambient, vec3<f32>(0.0), no_ambient || nova);

        // **The sun-occlusion mask, restored 2026-08-20.** Two independently
        // decoded carriers of the same scalar: a lightmapped chunk's shadow lives
        // in its lightmap's own alpha, and a vertex-lit chunk's lives in its
        // colour set's fourth byte - `in.sun_mask`, from
        // `oag_formats::rcsmodel::Mesh::vertex_light`. They never both carry real
        // data (a chunk bakes into the lightmap atlas *or* its vertices, never
        // both - see `mesh/rcs.rs`), and each side's absence is `1.0`
        // (unmasked): the no-lightmap placeholder's alpha, and `sun_mask`'s own
        // default for a chunk with no colour set. So the product reads whichever
        // side is real and is the identity where neither is.
        //
        // **A prior version of this comment said no sun term belonged here at
        // all**, because the general block #8/#9 formula above has none. That
        // was the read on two materials; seven read variant-by-variant on
        // 2026-08-20 refute it. `diffuse_with_specular_from_alpha` (86 chunks of
        // Talon's Junction), `..._scalar` (80), `diffusewithalphachannel` (43),
        // `track_wall` (33) and `glasstest` (33) - **275 of Talon's Junction's
        // 301 drawn materials, against `track_surface`'s 15** - each normalise
        // their interpolated world normal, dot it against the sun direction,
        // multiply by the sun colour, gate it by this same mask, add the prelit
        // term and multiply the sum into the albedo. So this is the rule and
        // `track_surface` the exception, applied to every chunk alike for want of
        // the per-material branch that would tell the two apart - a stand-in of
        // the same shape as the shared specular exponent below, and the
        // 15-of-301 minority it is wrong for reads unlit rather than lit, which
        // this project has not yet measured against the frame.
        //
        // **What this replaces.** A `sun * (ndl * baked.a)` summand once stood
        // here with `baked.a` fixed at the no-lightmap placeholder's `1.0` -
        // full sun, unoccluded, everywhere - measured at 14.3 % of the frame
        // clipped to white against the reference's 5.8 %, worse than the sun
        // term being absent entirely, which is why it was removed rather than
        // left wrong. `mesh/rcs.rs` used to take only `.rgb` from the colour set
        // and this mask did not exist yet.
        // **Times the road's mask under a craft**, `sun_occlusion` above: 1.0 on
        // every draw that is not a hull with a map, so nothing else here moves.
        // **Not even called on the track's pipelines** (`receives_shadow` 2,
        // `ShadowReceiver::Both`, which is the track and the gantry and never
        // a hull): its early return was already taken there, but the
        // function's four-tap loop held registers for it, 0.25 ms of HD's
        // 22.5 ms opaque track at 3200x1800 on the Raphael iGPU. Only a hull
        // names a layer, and a hull is `ShadowReceiver::Mapped`.
        mask = baked.a * in.sun_mask;
        if receives_shadow != 2.0 {
            mask = mask * sun_occlusion(in.world);
        }
        let sun_diffuse = scene.light.sun * (ndl * mask);
        // **`scene.light.ambient` and `vertex_light` are gated above** (`ambient_term`,
        // `vertex_light_term`), on `slots::NO_AMBIENT` - 251 of Anulpha Pass's 309
        // drawn materials, 922 of its 1,101 chunks. Gating `scene.light.ambient`
        // on that bit *alone* was tried on 2026-08-24 and was a regression: the
        // materials without an ambient are three families, not one, and
        // `sign_emissive` and its kin (declaring only `fogColour`) went black
        // when multiplied by an `authored` with the ambient simply dropped. The
        // fix above is the drop *and* the vertex-colour curve together, which is
        // what keeps the `IleLightmap`/`IleVertex` families lit; `EMISSIVE`
        // below is the third family, excluded from both terms the same way it
        // always was.
        //
        // **A program fed no scene light is emissive, and the rig must not touch
        // it.** `slots::EMISSIVE` is the material's own declaration: neither
        // `constantAmbientColour` nor `directionalLight0*`. On Anulpha Pass that
        // is 33 materials over 95 chunks and on Talon's Junction 35 over 151, and
        // every name in both is a sign or a glow - `sign_emissive`, `cf_glow_tube`,
        // `cf_plasma_glow2`, `dc_lightcone`, `scanlinebillboard` - which is a
        // confirmation the split was not designed for.
        //
        // Multiplying by `1.0` rather than adding anything: the albedo *is* the
        // picture for these, exactly as the microcode leaves it. The specular goes
        // with it, because a program with no sun has no half-vector term either.
        // **Plus Wipeout HD's SPU vertex lights**, which the `SVC1` fragment
        // programs add in exactly this slot - the pre-albedo diffuse sum beside
        // the ambient, the sun and the lightmap, never the specular, the fog or
        // the emissive add (renderer.md, "The `SVC1` combine is read"). Zero on
        // every draw that binds no list, and on an `EMISSIVE` chunk the whole
        // sum is replaced below, which is the original's own no-op there.
        let lit_sum = ambient_term + prelit + vertex_light_term + sun_diffuse + in.spu_light;
        authored = select(lit_sum, vec3<f32>(1.0), emissive);
    }

    // **Which texture is the picture and which is the coverage, off the
    // material's own microcode** - see `oag_mesh::mesh::slots` and
    // `docs/formats/rcsmaterial.md`. The common case is `slots::DEFAULT`:
    // colour and alpha both from the first texture, which is what every title
    // but Wipeout HD carries and what an HD material whose program this
    // reading could not follow keeps.
    //
    // The second texture is sampled at the **diffuse** coordinate, which is a
    // stated approximation rather than a reading: the cloud plate's own
    // program addresses both units from coordinates it builds out of the same
    // interpolator, and a chunk in this role declares no second coordinate set
    // for us to use instead. A material that tiles its two textures
    // differently would come out wrong here and would show as a mismatched
    // scale rather than as a missing surface.
    // **`slots::FACING_RAMP_SHEEN`: the glass family's facing-ramp combine,
    // fully traced on one material - see `oag_mesh::mesh::rcs::glass_sheen`
    // and `docs/formats/rcsmaterial.md`, "Three sampler roles identified by
    // what they bind, and the glass floor stops painting a ramp".** Where
    // this bit is set, `mesh::rcs::skin::picks` has already pointed `albedo`
    // at the material's facing-ramp texture and `lightmap` at its grid, so
    // the only thing left to do here is address the ramp by `dot(V, N)`
    // rather than the diffuse UV every other material samples it at - the
    // view-angle sheen the traced microcode computes
    // (`DP3 R2.w, -R1, R2` then `TEX H2.xyz, -R2.wwww unit2`).
    let ramp_sheen = (in.slots & 1024u) != 0u;
    let view_dir = normalize(scene.fog.camera - in.world);
    let ramp_facing = dot(view_dir, n);
    let ramp_uv = vec2<f32>(ramp_facing, ramp_facing);
    var first_uv = select(in.texcoord, ramp_uv, ramp_sheen);
    // **The Plasma explosion's ring and halo scroll on the model's own clock**
    // (`slots::CLOCK_SCROLL_RING`/`_HALO`): their programs' `UV_offset` is
    // `node + 0xc0`, the blast's age, not the scene's `time`. Read off the
    // two programs' own microcode (docs/ghidra/functions/ps3-hdfury-eu/
    // plasma.md, 2026-10-05) - ring: `n = tex(u, v + 0.01 t).a`, then
    // `uv + 0.15 n + 0.1 t` on both axes; halo: `n = tex(u, v + 0.4 t).a`,
    // then `(2u + 0.1 t, v + 0.14 t + 0.1 n)`. **Only where the colour tap
    // samples**; the programs' own colour and alpha combine is not
    // reproduced here, the path below stands in for it.
    let clock = uniforms.model_clock;
    if (in.slots & 32768u) != 0u {
        let n = textureSample(
            albedo,
            albedo_sampler,
            vec2<f32>(in.texcoord.x, in.texcoord.y + 0.01 * clock),
        ).a;
        first_uv = in.texcoord + vec2<f32>(0.15 * n + 0.1 * clock);
    } else if (in.slots & 65536u) != 0u {
        let n = textureSample(
            albedo,
            albedo_sampler,
            vec2<f32>(in.texcoord.x, in.texcoord.y + 0.4 * clock),
        ).a;
        first_uv = vec2<f32>(
            2.0 * in.texcoord.x + 0.1 * clock,
            in.texcoord.y + 0.04 * clock + 0.1 * n + 0.1 * clock,
        );
    }
    // **Pulse's slope-mode level selection**: `log2(|z| * slope) + bias` off view
    // depth, clamped to the levels the texture has - the GE's rule, in place of
    // the derivatives `textureSample` would use. On only for a model carrying
    // the disc's own chains; see `mesh_render::PSP_TEXLOD_SLOPE`. The player's
    // TEXTURE DETAIL shifts the level, which is one multiplier on the law.
    var first: vec4<f32>;
    if texlod_slope > 0.0 {
        let lod = max(
            log2(max(in.view_depth * texlod_slope, 1.0e-6)) + texlod_bias + scene.fog.texlod_shift,
            0.0,
        );
        first = sample_at_slope_level(first_uv, lod);
    } else {
        first = textureSample(albedo, albedo_sampler, first_uv);
    }
    let second = textureSample(lightmap, albedo_sampler, in.texcoord);
    let picture = select(first, second, (in.slots & 2u) != 0u);
    let coverage = select(first, second, (in.slots & 4u) != 0u);
    let texel = vec4<f32>(picture.rgb, coverage[(in.slots >> 3u) & 3u]);

    // **The combine itself**, `vertexLight * ramp + ramp`, plus the grid's
    // red as an additive term and as the output alpha - block #7's own
    // `ADD H4.xyz, H6.xxxx, H4` and `MOV H0.w, H6.xxxx END`. Two omissions,
    // both named rather than guessed shut:
    //
    // - **The per-material constant `c`** (`ADD H4.xyz, H2, {c}` -
    //   parameter `0x512f8e65`, measured `0.26562` on `etched_glass_tech`,
    //   not zero) is left out of `ramp + c` - `glass_sheen`'s own doc names
    //   the plumbing this needs and why it did not land in this change.
    // - **`paraboloidReflectionTex`'s reflection tint**, weighted by the same
    //   grid red, is left out entirely: this renderer has no dual-paraboloid
    //   probe and does not invent one, per `CLAUDE.md`.
    //
    // `vertexLight` is `f[TC0] + f[TC1] + f[TC5].x * (sun.colour * N.L)` in
    // the traced microcode (`@0x02`/`@0x05`: `DP3_SAT` against the sun
    // direction, `MUL` by the sun colour - exactly `scene.light.sun * ndl`
    // this file already computes as `sun_diffuse` without the mask, since
    // this program declares no lightmap to mask by). `f[TC5].x` is
    // `in.texcoord`'s own first component (line 830 of `rcsmaterial.md`
    // reads `f[TC5].zw` as this file's `in.texcoord`). `f[TC0]` and `f[TC1]`
    // are a stated approximation, not a reading: both stand in as
    // `in.colour.rgb`, the one per-vertex light term this project already
    // decodes for every HD material, on the same terms this file's own
    // second-texture coordinate above is already a stated approximation
    // rather than a reading.
    let ramp_sun = scene.light.sun * ndl;
    let vertex_light_sheen = in.colour.rgb + in.texcoord.x * ramp_sun;
    let sheen_rgb = vertex_light_sheen * first.rgb + first.rgb + vec3<f32>(second.r);
    let sheen = vec4<f32>(sheen_rgb, second.r * in.colour.a);

    // **Wipeout HD's additive glow.** Its emissive family samples unit 1 at
    // `(u, (v + a) * b + time)`, multiplies by a tint and adds the result to
    // the albedo gated by the diffuse alpha:
    //
    //     MAD H0.xyz, H0.wwww, H1, H0
    //
    // `slots::ADD_SECOND` says a material does this and its index selects the
    // three constants. Every other surface indexes slot 0, whose tint is zero,
    // so this costs one sample and no branch - the same shape the two anim
    // tables take. See `docs/formats/rcsmaterial.md` for the disassembly, and
    // `oag_mesh::mesh::rcs::emissive` for which materials qualify.
    //
    // **The `u` is the diffuse coordinate, which is a stated approximation and
    // not a reading**, exactly as the second texture's own sampling above
    // already is: the program addresses unit 1 from `f[TC3]`, and whether the
    // UV set feeding that interpolator is the one this renderer carries as
    // `texcoord` is unestablished. Only `v` moves, so a mismatch shows as a
    // glow tiled wrongly across the surface rather than as a missing one.
    let glow_slot = in.slots >> 20u;
    let glow_tint_offset = emissives.tint_offset[glow_slot];
    let glow_scale = emissives.scale[glow_slot];
    // **The clock is gated, and `b` defaults to 1 rather than 0.** A material
    // that adds its second texture is not necessarily one that scrolls it:
    // some read `time` while keeping their coordinate constants as the
    // shader's own inline literals, which this reading has not recovered, and
    // some never read it at all. A `b` of 0 with the clock ungated is the
    // worst of both - it annihilates the surface's own `v` and marches one row
    // of the texture across it. See `oag_mesh::mesh::Emissive::rate`.
    let glow_v = (in.texcoord.y + glow_tint_offset.w) * glow_scale.x
        + scene.time.x * glow_scale.y;
    // Fetched only where `ADD_SECOND` is set: everywhere else `glow_gate`
    // below is zero and the sample is multiplied away. A branch on `slots`,
    // which `lit_texel`'s own attribute explains.
    var glow_sample = vec4<f32>(0.0);
    if (in.slots & 256u) != 0u {
        glow_sample = textureSample(
            lightmap,
            albedo_sampler,
            vec2<f32>(in.texcoord.x, glow_v),
        );
    }
    // Gated by the *diffuse* alpha, which is `first.a` rather than the
    // resolved `texel.a`: the microcode's `H0.wwww` is the unit-0 fetch's own
    // fourth channel, before any of the role bits above choose where the
    // output's alpha comes from.
    let glow_gate = first.a * select(0.0, 1.0, (in.slots & 256u) != 0u);
    let glow = glow_sample.rgb * glow_tint_offset.rgb * glow_gate;
    // **The same layer in the linear domain, for the authored path**, which
    // adds it below. The sample is sRGB-decoded and the tint is not, on the
    // terms the albedo already sets on that path: `glow_sample` is a picture -
    // a sign, a tube, a light cone - authored in the space every other texture
    // on the disc is, and `glow_tint_offset.rgb` is a shader constant, an
    // authored magnitude like `light.sun`. `first.a` is an alpha, and alphas
    // never decode.
    //
    // **Neither domain reproduces the RSX**, which multiplies raw 8-bit
    // throughout and sRGB-decodes nothing - renderer.md, "Per-texture
    // sRGB/`GAMMA` decode: settled negative". What a domain can preserve is
    // the glow's magnitude *relative to the albedo it is added to*, and only
    // the decoded form does that: undecoded, a 0.5 sample lands at 0.5 beside
    // an albedo that decoded to 0.22, so the layer arrives at about twice the
    // weight the disc gives it. Both were measured - `scripts/hd-glow-sweep.py`
    // and the table in renderer.md - and the principle rather than the
    // brightness is what chose.
    let glow_linear =
        pow(glow_sample.rgb, vec3<f32>(2.2)) * glow_tint_offset.rgb * glow_gate;
    // **HD's pad light bars**: `_ne.a * colour`, the program's `MAD H.xyz,
    // R0.wwww, C, lit` - added after the light on the far side of the albedo
    // multiply, ungated by the diffuse alpha, and not decoded in either
    // domain (an alpha and an authored magnitude). The colour is the pad's
    // own authored `W_Cycle`/`Colour`, riding in this material's glow-table
    // tint; zero for every other material. Fog follows in `fogged`, which is
    // where the program's own final `MAD` puts it too.
    let pad_glow = pad_ne_texel.a * glow_tint_offset.rgb * select(0.0, 1.0, pad_ne);

    // **HD's magstrip wave** (`slots::MAG_WAVE`, `oag_mesh::mesh::rcs::mag_wave`):
    //
    //     m = lerp(e.rgb, wave(uv * k + time), e.a)
    //     d = e.rgb / (1 - m * Colour)
    //
    // `e` is the emissive picture (the third binding), `k` the glow-table
    // entry's scale, `Colour` its tint, and the clock is added to both axes with
    // no multiplier - read off the microcode, `mag_wave`'s doc has the listing.
    // The wave texture repeats, so the band sweeps once a second.
    //
    // **Chosen, not measured: the denominator is floored at 0.05.** `Colour.g`
    // is 1 on Talon's Junction and the wave's bright band reaches 1, so the
    // quotient diverges, and a black emissive texel makes it `0 / 0`. The RSX
    // writes 8 bits and saturates; this floors the divisor and clamps to 1 so
    // no inf or NaN reaches the bloom's targets. The dodge is also taken on the
    // sampled (sRGB-decoded) texels rather than raw bytes, the same domain
    // choice the glow above makes.
    var mag_glow = vec3<f32>(0.0);
    if (in.slots & 131072u) != 0u {
        let e = textureSample(pad_mask, albedo_sampler, in.texcoord);
        let w = textureSample(
            wave_map,
            albedo_sampler,
            in.texcoord * glow_scale.x + vec2<f32>(scene.time.x * glow_scale.y),
        );
        let m = mix(e.rgb, w.rgb, e.a);
        mag_glow = min(
            e.rgb / max(vec3<f32>(1.0) - m * glow_tint_offset.rgb, vec3<f32>(0.05)),
            vec3<f32>(1.0),
        );
    }
    let mag_glow_gamma = pow(mag_glow, vec3<f32>(1.0 / 2.2));

    // The Zone surface, in both domains, from one sample. See `zone_sample`.
    //
    // `rim` is the microcode's own `1 - dot(N, -V)`, where its `-V` points
    // from the surface back at the eye - so this is 0 dead-on and 1 at the
    // silhouette, and the two summands are silhouette-weighted. The eye comes
    // out of the fog block, the one slot in bind group 2 that carries a
    // position; the specular term below reads it from there too.
    //
    // **Branched, not just weighted.** Every reader below mixes these in by
    // `enabled`, so off a Zone race they contribute nothing - but computing
    // them anyway costs nine texture fetches a pixel, about a fifth of the
    // race pass on an integrated GPU (`OAG_RENDER_GPU_BENCH`). `enabled` is a
    // uniform, so the branch is uniform control flow and the fetches inside it
    // keep their implicit derivatives. Zero is what the mixes already reduced
    // these to, so the picture is unchanged to the byte.
    var zone_linear = vec3<f32>(0.0);
    var zone_gamma = vec3<f32>(0.0);
    var zone_glow_term = vec3<f32>(0.0);
    if scene.zone.enabled != 0.0 {
        let zone_to_eye = normalize(scene.fog.camera - in.world);
        let zone_colours = zone_set(in.slots, in.world);
        let zone_base = zone_base_term(1.0 - dot(n, zone_to_eye), zone_colours);
        let zone = zone_sample(in.texcoord, in.slots, in.world);
        zone_linear = pow(zone, vec3<f32>(2.2)) * zone_colours.effect.rgb + zone_base;
        zone_gamma = zone * zone_colours.effect.rgb + zone_base;
        // The visualiser glow - see `zone_glow`. Gated by `enabled` explicitly
        // rather than trusting `effect.w` to be zero off a Zone race, the same
        // belt-and-braces `surface_linear`/`plain` below already take.
        zone_glow_term =
            zone_glow(n, in.clip.z, in.texcoord, in.slots, in.world) * scene.zone.enabled;
    }

    // The read specular term: half-vector against the sun, raised to
    // `in.specular_exponent` - each material's own inline constant, decoded
    // off its fragment microcode by
    // `oag_formats::rcsmaterial::fragment::Program::specular_exponent`
    // rather than shared, per `oag_mesh::mesh::vertex::GpuVertex::specular_exponent`.
    // `32.0` remains only the *fallback*, for a material the decoder could
    // not resolve - see that field for what a resolved value means and what
    // it does not. Gated by `mask` above - the same sun-occlusion scalar the
    // diffuse term reads, exactly where the disc's own microcode gates its
    // specular by it too - and by the diffuse texture's alpha (gloss lives
    // there; a DXT1 diffuse has alpha 1 everywhere, which is full gloss, as
    // the original samples it too). Zero whenever the authored rig is off:
    // the stand-in never had one.
    let to_eye = normalize(scene.fog.camera - in.world);
    // The authored rig's half of the output, computed only when the rig is
    // on - see the same branch above `atlas`.
    var authored_rgb = vec3<f32>(0.0);
    if scene.light.enabled != 0.0 {
        let half_vector = to_eye + scene.light.direction;
        let ndh = clamp(
            dot(half_vector, n_lit) / max(length(half_vector), 1e-6),
            0.0,
            1.0,
        );
        let specular = scene.light.sun
            * (pow(ndh, in.specular_exponent) * ndl * mask * texel.a * scene.light.specular_scale
                * scene.light.enabled * in.lit * select(1.0, 0.0, emissive));

        // The authored path shades in linear light, as the RSX does: the samples
        // are sRGB-decoded and lit by the authored magnitudes. On a gamma target
        // the result is saturated (the stand-in for HD's exposure stage) and
        // encoded back here; on the linear float target it leaves **unclamped**,
        // because values above 1.0 are exactly what the bloom gate reads, and the
        // saturate-and-encode happens in `post::hd_bloom`'s own pass instead.
        // The vertex colour is **inside** `authored` on this path, not a factor
        // outside it: the microcode's `(prelit + f[TC1]) * albedo` adds the two
        // light terms and multiplies the albedo once. It stays a factor on the
        // stand-in path below, where it is a tint and every other title authors
        // it as one.
        let texel_linear = pow(texel.rgb, vec3<f32>(2.2));
        // `surface = zoneCol`, then `colour = light * surface` - the microcode's
        // own order, and on this path both terms are linear. The Zone surface
        // **replaces** the albedo rather than adding to it, so `enabled` is the
        // mix weight: it is 0.0 for every draw that is not a Zone race with all
        // its inputs resolved, and there this is `texel_linear` unchanged.
        let surface_linear = mix(texel_linear, zone_linear, scene.zone.enabled);
        // `colour = light * surface + glow` - the microcode's own order, and
        // **both glows are on the far side of the multiply because that is where
        // the disc puts them**. Read off `uvanim_diffuse_emissive`'s lit fragment
        // block on Amphiseum, block #3, with `scripts/ps3-microcode.py`:
        //
        //     @0x22  MUL H4.xyz, H0, H4      ; albedo * (prelit + ambient + ndl*sun)
        //     @0x23  MAD H0.xyz, H0.wwww, H1, H4   ; + diffuse alpha * tinted glow
        //
        // Its unlit block #2 (`MUL` at 0x0b, `MAD` at 0x0f) and its second lit
        // block #4 (0x23, 0x25) have the same pair in the same order. Three
        // variants, no exceptions - so the accumulate reads a value the light has
        // already multiplied, and a glow is not itself lit.
        //
        // **`glow_linear` belongs here and used to reach only `plain`**, which is
        // what made HD's emissive layer very nearly invisible: this fragment
        // resolves `mix(plain_rgb, authored_rgb, enabled * in.lit)` and HD's rig is
        // always enabled, so every `in.lit == 1` chunk - the track, the tubes, the
        // signs, everything near the camera - dropped it. Only the far background
        // and the hull ever saw it.
        let lit_linear =
            surface_linear * authored + specular + zone_glow_term + glow_linear + pad_glow + mag_glow;
        let encoded = pow(
            clamp(lit_linear, vec3<f32>(0.0), vec3<f32>(1.0)),
            vec3<f32>(1.0 / 2.2),
        );
        authored_rgb = mix(encoded, lit_linear, linear_out);
    }

    // The stand-in path, byte-for-byte what every other title always drew.
    // Prelit geometry (`lit` 0.0) takes it even under the authored rig: its
    // lighting is baked into its vertex colours, in the gamma space every
    // Pulse-shaped asset authors, so the linear round-trip above would
    // re-shade what is already shaded. On the linear target its gamma result
    // is decoded, so `post::hd_bloom`'s encode returns it byte-for-byte.
    let light = mix(vec3<f32>(1.0), stand_in, in.lit);
    // **The vertex colour is a tint here and a light term on the authored
    // path, so this multiply must not see HD's.** Without the guard, any HD
    // draw that binds `Light::stand_in` - the front end, or a circuit whose
    // `.envsettings` fails to yield a rig - would multiply by a baked light
    // that is zero wherever no colour set exists, and render black.
    //
    // **This used to key on `linear_out` and that was wrong twice.** It missed
    // HD's capture and viewer paths, which draw the same models into a gamma
    // target - `model_probe` found them rendering dark, 1,062 lit pixels of a
    // 1024x1024 frame against 6,988 once fixed. And it over-fired on the race
    // target, forcing *every* model's tint to white there including the sky
    // cube, whose vertex colour genuinely is a tint because `mesh/sky_cube.rs`
    // builds it rather than the HD loader. `colour_is_light` is a property of
    // the model and answers both.
    let tint = mix(in.colour.rgb, vec3<f32>(1.0), colour_is_light);
    // The gamma-domain sum, for the same reason this whole path is gamma: a
    // prelit surface's light is baked into its vertex colours in the space the
    // asset authors, so a linear summand would be the one term shaded twice.
    // The glow is **added after the light**, which is where the microcode puts
    // it: the accumulate reads `H0` once the lightmap and the interpolated
    // term have already multiplied the albedo, so a glow is not itself lit.
    // The visualiser glow rides here too, undecoded on the same terms
    // `zone_gamma` already is on this path - it is a shader parameter and
    // texture-lookup product, not an albedo sample, so it owes no sRGB
    // decode either way.
    let plain = mix(texel.rgb, zone_gamma, scene.zone.enabled) * tint * light
        + glow + zone_glow_term + pad_glow + mag_glow_gamma;
    let plain_rgb = mix(plain, pow(plain, vec3<f32>(2.2)), linear_out);

    // Vertex colour modulates the texture on all four channels, as the GE's
    // texture-env does - RGB and alpha alike, not RGB alone. Dropping the
    // vertex colour's own alpha here is what made the boost plume's baked
    // falloff vanish; see `every_psp_teams_boost_plume_vertex_alpha_is_bimodal`
    // in `crates/game/tests/boost_plume_ground_truth.rs`.

    // **The flame, when this model is one.** `flame_shading` is 1.0 only for a
    // craft's `engineflare` model, and then this replaces everything above
    // rather than modifying it - the program it reproduces has no ambient, no
    // sun, no lightmap and no specular, so lighting the flame and then
    // overwriting it is the honest shape as well as the cheap one.
    //
    // `rim` is the same angle HD's program computes. Its own vertex program
    // dots an *untransformed* normal attribute against
    // `eyePositionWorldSpace - position`, which is only meaningful if the eye
    // reaches it in model space; the angle between the two is the same number
    // under a rigid transform, so it is computed here from the world normal and
    // the real camera.
    //
    // **`texel.a` is deliberately absent.** The program's `TEX H0.xyz` writes
    // three components and the fourth is the vertex ramp from the moment
    // `MOV H0.w, f[TC2]` puts it there, so the sampled alpha never reaches the
    // output. Multiplying it in - which the line below this one does for every
    // other surface - is what held the flame at about a third of its
    // brightness.
    //
    // **Only on a flame pipeline.** `flame_shading` is an `override`, so this
    // branch is a constant the driver deletes on every other pipeline, which
    // otherwise paid the two fetches below on every pixel for a mix weight of
    // zero.
    var flame = vec4<f32>(0.0);
    if flame_shading != 0.0 {
        let rim = clamp(1.0 - dot(to_eye, n), 0.0, 1.0);
        let flame_alpha = flame_alpha_scale
            * (1.0 - (pow(rim, flame_rim_power) * flame_rim_scale + flame_rim_min));
        // **The program's two taps, both of them.** It samples `unit0` twice from
        // one texture: a *noise* tap whose `v` scrolls with the engine clock, and
        // a *colour* tap at doubled coordinates displaced by the noise the first
        // returned. Neither is this file's invention - block #1 and block #2 of
        // `flame_test.rcsmaterial` schedule the identical pair through different
        // registers, and `time`'s provider is engine parameter slot 0. See
        // `oag_mesh::mesh::Flame` and
        // docs/ghidra/functions/ps3-hdfury-eu/engine-flare.md.
        //
        //     @0x0a  MAD R0.y, {Speed}, {time}, Uv1.y   <- the scrolled row
        //     @0x11  TEX R2.w, R0 unit0                 <- its ALPHA is the noise
        //     @0x1e  ADD R1.xy, R1, R2.zwzz             <- (2u, 2v + noise)
        //     @0x1f  TEX H0.xyz, R1 unit0               <- the colour
        //
        // The doubling stays a plain multiply rather than a fract: the sampler
        // repeats, and the authored `Uv1` runs outside 0..1 on one node already.
        let flame_noise = textureSample(
            albedo,
            albedo_sampler,
            vec2<f32>(in.texcoord.x, in.texcoord.y + flame_speed * scene.time.x),
        ).a;
        let flame_texel = textureSample(
            albedo,
            albedo_sampler,
            vec2<f32>(in.texcoord.x * 2.0, in.texcoord.y * 2.0 + flame_noise),
        );
        let flame_rgb = flame_texel.rgb * flame_colour_scale;
        // **Not decoded on the linear target, on purpose - and this is a change
        // of mind recorded in place.** The decode used to sit here on the "what a
        // sampler hands back is authored-space" argument, but the flame's own
        // program is read instruction by instruction and applies no transfer
        // function anywhere: the original multiplies the raw sample into a
        // target the exposure resolve then scales, `ADD_SAT`s and presents with
        // no gamma arithmetic (renderer.md, "no `1/2.2` ... exists anywhere in
        // it"). Decoding here dimmed the flame's mid-tones by up to a third
        // against that arithmetic - the same divergence the HD trail's path had,
        // fixed the same day. See
        // docs/ghidra/functions/ps3-hdfury-eu/engine-trail.md.
        flame = vec4<f32>(flame_rgb, flame_alpha * in.colour.a);
    }

    let shaded = vec4<f32>(
        mix(plain_rgb, authored_rgb, scene.light.enabled * in.lit),
        texel.a * in.colour.a,
    );
    // **The facing-ramp combine replaces the generic rig entirely**, the
    // same shape `flame` below already takes: block #7 computes its own
    // light (the traced `vertexLight`) rather than being multiplied by
    // `authored`'s ambient/prelit/sun sum, so running both would shade it
    // twice.
    let shaded_or_sheen = select(shaded, sheen, ramp_sheen);
    // **The magstrip floor's own combine** (`slots::MAG_LOOP`), traced in
    // `oag_mesh::mesh::rcs::mag_wave`: `vertexLight * grid * (ramp + c) +
    // (grid + ramp) * d`, alpha the grid. `albedo` is the grid and `lightmap`
    // the facing ramp, addressed by `dot(V, N)` as the glass sheen's is. `c`
    // is the glow entry's `offset`. Left out, named: the paraboloid
    // reflection term, which this renderer has no probe for.
    var mag_floor = shaded_or_sheen;
    if (in.slots & 262144u) != 0u {
        let grid = textureSample(albedo, albedo_sampler, in.texcoord).r;
        let ramp = textureSample(lightmap, albedo_sampler, ramp_uv).rgb;
        let loop_light = in.colour.rgb + in.texcoord.x * ramp_sun;
        let loop_rgb = loop_light * grid * (ramp + vec3<f32>(glow_tint_offset.w))
            + (vec3<f32>(grid) + ramp) * mag_glow;
        mag_floor = vec4<f32>(loop_rgb, grid * in.colour.a);
    }

    // **HD's light cone** (`slots::LIGHT_CONE`, `oag_mesh::mesh::rcs::light_cone`):
    // the program's colour is the noise's red times the authored intensity `K`
    // (`MAD R2.xyz, H2.xxxx, K, -fog`, then the fog lerp `fogged` does) and its
    // alpha is `noise * s * H2.x` where `H2.x` is the ramp tap at `dot(N, V)`
    // **only when the program's predicated `TEX` executes** and the noise
    // itself (register `H2.x` is left holding it) when it does not. The
    // predicate is `NE(wwww)` on a condition set from `f[TC1]`, whose `w` is
    // the vertex normal's `y` - zero on every authored cone, whose normal is
    // the constant `(0, 0, 1)` - so on a gated variant the ramp is skipped and
    // the alpha is `s * noise^2`. `glow_scale.y` says the variant is gated.
    // That reading of the condition register is a hypothesis (confidence 60,
    // light_cone's doc): it is what a matched frame favours, not something the
    // instruction set documents. `albedo` is the ramp and `lightmap` the
    // noise, `K` the glow entry's tint, `s` its scale.
    // Raw samples, undecoded: the program applies no transfer function. **The
    // colour is saturated before it blends**, as the 8-bit surface the original
    // draws into does: `K` is 100 on Talon's Junction, so an unclamped colour
    // on this linear float target is `100 * noise` of light times the alpha, a
    // white wall, where the original's is a white of at most 1 at that alpha.
    var cone_or_floor = mag_floor;
    if (in.slots & 524288u) != 0u {
        let noise = textureSample(lightmap, albedo_sampler, in.texcoord).r;
        let cone_facing = clamp(ramp_facing, 0.0, 1.0);
        let ramp = textureSample(albedo, albedo_sampler, vec2<f32>(cone_facing)).r;
        let ramp_skipped = glow_scale.y > 0.5 && in.normal.y == 0.0;
        cone_or_floor = vec4<f32>(
            clamp(vec3<f32>(noise) * glow_tint_offset.rgb, vec3<f32>(0.0), vec3<f32>(1.0)),
            noise * glow_scale.x * select(ramp, noise, ramp_skipped),
        );
    }

    // **The absorb shell, when this model is one.** Fragment block #1 of
    // `hd_absorbinternal.rcsmaterial`, which the fogged blocks repeat before
    // their fog lerp (`fogged` below does that part):
    //
    //     @0x04  TEX R0.w, (u, v) unit0             <- the ALPHA is the offset
    //     @0x05  MAD R1.xy, R0.w, 0.5, (u, v)
    //     @0x07  MAD R1.xy, time, 5.0, R1           <- both axes, 5 per second
    //     @0x09  MOV H0.w, f[TC2]                   <- VertexColour1.w * ShieldColour
    //     @0x0a  TEX H0.xyz, R1 unit0
    //
    // So the colour is the texture sampled at its own alpha-displaced, clock-
    // scrolled coordinate, and the alpha is the vertex ramp times the fade the
    // caller writes into it. `texel.a` does not reach the output. Not decoded
    // on the linear target, for the reason the flame gives: the program
    // applies no transfer function.
    // Branched on its `override` for the flame's reason.
    var absorb = vec4<f32>(0.0);
    if absorb_shading != 0.0 {
        let absorb_offset = textureSample(albedo, albedo_sampler, in.texcoord).a * 0.5
            + 5.0 * scene.time.x;
        let absorb_rgb = textureSample(
            albedo,
            albedo_sampler,
            in.texcoord + vec2<f32>(absorb_offset, absorb_offset),
        ).rgb;
        absorb = vec4<f32>(absorb_rgb, in.colour.a);
    }
    let composed = mix(mix(cone_or_floor, flame, flame_shading), absorb, absorb_shading);

    // **Wipeout HD's two rim-shaded weapon glows**, `slots::RIM_GLOW` (the
    // LeachBall) and `slots::RIM_EDGE` (the Plasma bolt's head), each set only
    // where `mesh::rcs::rim_glow` matched the material's own program - its
    // mnemonics, its literals and its declared parameters. Read instruction by
    // instruction in docs/rendering/hd-unlit-programs.md; every number below
    // is a literal of that program. Neither program reads a light, the vertex
    // colour or the texture's own alpha as coverage, so these replace
    // everything above rather than modifying it, as `flame` does; and like
    // `flame` they are not decoded on the linear target, since the programs
    // apply no transfer function.
    //
    //     TEX R1.w, (u, v + 0.0001 time)            <- the noise is the ALPHA
    //     MAD R3.zw, noise, 0.2, (u, v)
    //     MAD R1.zw, time, 0.4, R3.zw               <- both axes
    //     TEX H1.xyz, R1.zwzz                       <- the colour tap
    //     ADD_SAT rim, -dot(N, V), 1
    //
    // `rim` is `flame`'s own angle, for `flame`'s own reason: the vertex
    // program dots an untransformed normal against `eye - position`, and the
    // angle survives the rigid transform this path applies first.
    //
    // **Branched on the two bits**, so the two fetches run on the two weapon
    // glows that read them and nowhere else. `slots` is a flat varying, which
    // WGSL's uniformity analysis cannot see is constant per quad, hence this
    // function's `derivative_uniformity` diagnostic - see its own attribute.
    let rim_glow_on = (in.slots & 2048u) != 0u;
    let rim_edge_on = (in.slots & 4096u) != 0u;
    if !(rim_glow_on || rim_edge_on) {
        return composed;
    }
    let rim_noise = textureSample(
        albedo,
        albedo_sampler,
        vec2<f32>(in.texcoord.x, in.texcoord.y + 0.0001 * scene.time.x),
    ).a;
    let rim_tap = in.texcoord + vec2<f32>(0.2 * rim_noise + 0.4 * scene.time.x);
    let rim_c = textureSample(albedo, albedo_sampler, rim_tap).rgb;
    let rim_angle = clamp(1.0 - dot(to_eye, n), 0.0, 1.0);
    // `LG2`/`MUL 5`/`EX2` in the program: `rim^5`, multiplied out so `0^5` is
    // `0` rather than whatever `pow(0, 5)` answers on a given backend.
    let rim_5 = rim_angle * rim_angle * rim_angle * rim_angle * rim_angle;
    // RIM_GLOW: `MAD_SAT -x, 0.9, 0.9`, then `^5` again.
    let rim_fade_base = clamp(0.9 - 0.9 * rim_5, 0.0, 1.0);
    let rim_fade = rim_fade_base * rim_fade_base * rim_fade_base * rim_fade_base * rim_fade_base;
    // `c / (1 - c)` through three `RCP`s of half registers. A texel at 255
    // makes that `1 / 0`; **chosen, not measured**: it is held at the largest
    // finite half, 65504, rather than let an infinity into the float target
    // and its bloom.
    let rim_expand = min(vec3<f32>(1.0) / (vec3<f32>(1.0) - rim_c), vec3<f32>(65504.0));
    let rim_glow = vec4<f32>(rim_fade * rim_c * rim_expand, rim_fade);
    // RIM_EDGE: `MUL H1.w, rim^5, 1000`, times the tap; the alpha is the
    // material's `0x7611a2d8`, which `rim_glow::classify` only routes at 1.0.
    let rim_edge = vec4<f32>(1000.0 * rim_5 * rim_c, 1.0);
    return select(select(composed, rim_glow, rim_glow_on), rim_edge, rim_edge_on);
}

// **Whether this model stamps the bloom's glow mask** - 1.0 only for a
// drawable built with `mesh_render::GlowMask::Stamped`. The original writes
// that channel through the GE stencil, never through the blend, so an opaque
// surface stamps a constant the batch names rather than its own alpha - see
// `oag_mesh::mesh::glow` and docs/rendering/glow-mask.md.
override glow_stamp: f32 = 0.0;

// **Whether that stamp is the PS2's rule** - 1.0 only for a drawable built with
// `mesh_render::GlowMask::StampedByTexel`. The PS2 writes the fragment's own
// alpha (texel times vertex colour) from a batch with the glow bits and nothing
// from any other, where the PSP writes a constant the batch names. See
// `GlowMask::StampedByTexel` and docs/rendering/ps2-bloom.md.
override glow_texel: f32 = 0.0;

// `slots::GLOW_BATCH`: the batch carries the glow bits.
const SLOT_GLOW_BATCH: u32 = 8192u;

// What this fragment stamps, before any alpha test: the batch's constant on the
// PSP's rule, the fragment's own alpha on a glow batch on the PS2's.
fn stamp_value(in: VertexOutput, shaded_alpha: f32) -> f32 {
    let ps2 = select(0.0, shaded_alpha, (in.slots & SLOT_GLOW_BATCH) != 0u);
    return mix(in.glow, ps2, glow_texel);
}

// The opaque and cutout pipelines' alpha: `1.0` as it always was, or the
// batch's stamp where the model stamps.
fn stamped_alpha(in: VertexOutput, shaded_alpha: f32) -> f32 {
    return mix(1.0, stamp_value(in, shaded_alpha), glow_stamp);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let shaded = lit_texel(in);
    // This pipeline is opaque (`blend: None`), so the alpha channel is never
    // blended with - but it is still written to whatever render target is
    // bound, so it stays a hardcoded 1.0 here rather than the texture's and
    // vertex colour's combined alpha, matching every prior opaque render
    // exactly. `fs_main_blend` below is the one that actually reads it.
    return vec4<f32>(
        fogged(shadowed(shaded.rgb, in.world), in.world, in.view_depth),
        stamped_alpha(in, shaded.a)
    );
}

// Used only by the blended pipeline - see
// `oag_mesh::mesh::Model::transparent_draws`. Identical to `fs_main` except
// it outputs the texture's alpha times the vertex colour's alpha instead of a
// hardcoded 1.0, which is what lets the blend state built around this entry
// point actually blend rather than replace.
@fragment
fn fs_main_blend(in: VertexOutput) -> @location(0) vec4<f32> {
    let shaded = lit_texel(in);
    return vec4<f32>(
        fogged(shadowed(shaded.rgb, in.world), in.world, in.view_depth),
        shaded.a
    );
}

// **The default, for a draw whose file authors no reference of its own.**
//
// Every PSP and PS2 cutout batch now authors one:
// `oag_vex::vex::Batch::alpha_test_reference` reads the selector out of
// `pass_mask` and `header_flags`, `mesh::DrawCall::alpha_test_ref` carries it,
// and `mesh_render::cutout` builds one pipeline per distinct value - so a
// `.vex` draw overrides this constant rather than falling back to it. What
// still lands here is a PS3 chunk (Wipeout HD authors per *material* instead,
// through `Model::alpha_test_ref` below), a Vita submesh, and every synthetic
// draw this crate makes up.
//
// `0` rather than `0.5` for those, and that is measured rather than
// inherited. `0.5` was this project's own placeholder for the unrecovered
// case and it was wrong on real data: Wipeout Pure's `Speedup Pad` glow
// texture (`speedup_GLOW_KEY.tga`) decodes to an alpha histogram of exactly
// `{0: background, 57..58: glow interior}` - its brightest texel is ~0.23, so
// `0.5` discards the whole batch and the pad renders nothing.
//
// **The pad now gets that reference from its own file, and the value it asks
// for is `0`.** Its 349 batches are the one selector pattern that separates the
// recovered branch from its rival (`pass_mask & 0x80` set, `header_flags &
// 0x20` clear) - see `oag_vex::vex::Batch::alpha_test_reference`. So the pad no
// longer depends on this constant at all, and `pad_alpha_test_ground_truth`
// now guards the recovered value rather than the placeholder.
//
// The constant still stands for the paths that do reach it, on the same
// argument it always had - the most permissive of the recovered references, so
// a draw whose real reference is unknown keeps a few extra near-zero-alpha
// texels rather than this project hiding geometry. Under the `<=` below that
// is `0`, which discards exactly the fully transparent texel; it was written
// `1/255` while the comparison was `<`, and those two say the same thing about
// 8-bit alpha, so the paths that reach this constant keep the picture they
// had.
const ALPHA_TEST_THRESHOLD: f32 = 0.0;

// **Wipeout HD authors its own reference**, and this is where it arrives:
// a `Transparency::Mode2` material carries the `GL_GREATER`/`0.5` pair its
// `Material_ApplyRenderState` programs into `NV4097_SET_ALPHA_FUNC` and
// `SET_ALPHA_REF`, and `mesh_render::build` passes it here off
// `Model::alpha_test_ref`. The default is the constant above. Only
// `GL_GREATER` is reproduced: the comparison below discards at `<=`, and
// `mesh::rcs::cutout` reports a material asking for anything else instead of
// drawing it inverted.
//
// **`<=` and not `<`, and the difference is not cosmetic.** Both `GL_GREATER`
// here and the GE's `GU_ALPHA_GREATER` on the PSP/PS2 path keep what is
// *strictly* above the reference, so the discard is `<=`; `<` is `GEQUAL`.
// While Wipeout HD's `0.5` was the only reference in play the two were
// interchangeable - the alpha is 8-bit and `0.5` sits between `127/255` and
// `128/255`, so no texel could land on the boundary. Every reference recovered
// off a `.vex` batch lands on one exactly (`0`, `0x10`, `0x7f`), and at `0`
// the `<` form discards nothing at all: Pure's `Speedup Pad` painted its
// texture's fully transparent background as a solid plate, a square instead of
// a pad. See `crates/render/tests/pad_alpha_test_ground_truth.rs`.
override alpha_test_ref: f32 = ALPHA_TEST_THRESHOLD;

// Used only by the cutout pipeline - see
// `oag_mesh::mesh::Model::alpha_tested_draws`. Unlike `fs_main_blend`, this
// keeps depth write on (see `mesh_render::build`): a batch tagged
// `is_alpha_tested()` is meant to be treated as opaque wherever its texel
// clears the threshold, and fully absent everywhere else, not smoothly
// blended - `discard` is what lets the pixels below the threshold contribute
// neither colour nor depth, so surfaces behind a cutout's "empty" corners
// still show through and still get occluded correctly by whatever the
// cutout's solid pixels do draw. `shaded.a` is now the texture's alpha times
// the vertex colour's, per `lit_texel` above; measured directly against every
// alpha-tested batch on `01_Track`/`16_Track` that no vertex reference in
// this path carries baked alpha at or below `ALPHA_TEST_THRESHOLD`, so this
// fold-in does not newly discard any fragment that used to pass on real
// track data.
@fragment
fn fs_main_alpha_test(in: VertexOutput) -> @location(0) vec4<f32> {
    let shaded = lit_texel(in);
    if shaded.a <= alpha_test_ref {
        discard;
    }
    return vec4<f32>(
        fogged(shadowed(shaded.rgb, in.world), in.world, in.view_depth),
        stamped_alpha(in, shaded.a)
    );
}

// **The glow mask's stamp for a blended batch** - drawn after the batch's own
// blended draw, through a pipeline that masks colour off and writes alpha only,
// so it reaches the target as the one thing the blend cannot do: replace the
// mask with a constant. The original does this in the same draw with the GE
// stencil (`ALWAYS`, ref = the texture's glow byte, `REPLACE` on depth pass)
// left on under the blend, and the test below is its alpha test
// (`GU_GREATER`, the batch's own reference, on texel alpha times vertex
// alpha - the value the blend reads). A batch without the glow bits carries
// `glow == 0` and never stamps: the original disables the stencil test for it,
// which keeps the mask. See `docs/rendering/glow-mask.md`.
//
// **The additive class also tests colour** - `Gfx_BuildBatchStateList`'s
// `0x200` branch turns the GE colour test on, `NOTEQUAL` against black, and a
// GE dump of Outpost 7 shows it on every additive glow draw and off on the
// alpha-over ones - so a black texel of an additive quad stamps nothing.
// `stamp_colour_test` is that branch's constant, set on the pipeline pair
// that draws the additive batches. Whether the GE tests before or after the
// fog is unread; this tests the lit texel, before the fog.
override stamp_colour_test: f32 = 0.0;

fn stamp_discards(shaded: vec4<f32>, glow: f32) -> bool {
    let black = max(shaded.r, max(shaded.g, shaded.b)) < 0.5 / 255.0;
    return shaded.a <= alpha_test_ref || glow <= 0.0 || (stamp_colour_test > 0.5 && black);
}

@fragment
fn fs_main_stamp(in: VertexOutput) -> @location(0) vec4<f32> {
    let shaded = lit_texel(in);
    let glow = stamp_value(in, shaded.a);
    if stamp_discards(shaded, glow) {
        discard;
    }
    return vec4<f32>(0.0, 0.0, 0.0, glow);
}

@fragment
fn fs_main_stamp_velocity(in: VertexOutput) -> MrtOutput {
    let shaded = lit_texel(in);
    let glow = stamp_value(in, shaded.a);
    if stamp_discards(shaded, glow) {
        discard;
    }
    return MrtOutput(vec4<f32>(0.0, 0.0, 0.0, glow), velocity_of(in));
}

// The velocity-writing twins of `fs_main` and `fs_main_alpha_test`, for the
// pipelines built against the race's two attachments (`mesh_render::Velocity`).
// The blended pipelines have no twin on purpose: they write no depth, so the
// velocity at their pixels belongs to the surface behind them - their second
// target carries an empty write mask instead, keeping velocity and depth
// describing the same surface at every pixel.
@fragment
fn fs_main_velocity(in: VertexOutput) -> MrtOutput {
    let shaded = lit_texel(in);
    return MrtOutput(
        vec4<f32>(
            fogged(shadowed(shaded.rgb, in.world), in.world, in.view_depth),
            stamped_alpha(in, shaded.a)
        ),
        velocity_of(in),
    );
}

@fragment
fn fs_main_alpha_test_velocity(in: VertexOutput) -> MrtOutput {
    let shaded = lit_texel(in);
    if shaded.a <= alpha_test_ref {
        discard;
    }
    return MrtOutput(
        vec4<f32>(
            fogged(shadowed(shaded.rgb, in.world), in.world, in.view_depth),
            stamped_alpha(in, shaded.a)
        ),
        velocity_of(in),
    );
}

// The depth prepass's fragment stage - `mesh_render::prepass`. Every target's
// write mask is empty, so what this returns reaches nothing; it exists only
// because a pass with colour attachments wants a fragment stage that names
// them.
@fragment
fn fs_depth_only() -> @location(0) vec4<f32> {
    return vec4<f32>(0.0);
}

@fragment
fn fs_depth_only_velocity() -> MrtOutput {
    return MrtOutput(vec4<f32>(0.0), vec2<f32>(0.0));
}
