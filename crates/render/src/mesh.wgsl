// Draws a decoded .vex model.
//
// Lighting is a fixed two-light rig rather than the game's own: the point here
// is to see the geometry clearly and spot decoding errors, not to reproduce
// Pulse's look. A single light leaves faces pointing away from it unreadably
// black.

struct Uniforms {
    view_projection: mat4x4<f32>,
    model: mat4x4<f32>,
    // Unused. Was the phase of a global texture-animation clock, back when
    // every animated surface was assumed to scroll V at a rate chosen from a
    // table of texture names; the authored per-material keyframe blocks
    // replaced it (see `TexAnims` below). Kept as padding rather than removed
    // because `UNIFORMS_SIZE` and this layout are mirrored by the asset
    // viewer and by four other pipelines in this crate.
    //
    // Three trailing f32 fields rather than a vec3: WGSL aligns vec3 to 16
    // bytes, which would silently insert padding this struct's Rust mirror
    // (a flat, tightly packed repr(C)) does not have.
    _unused: f32,
    _pad0: f32,
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
// generous - `oag_render::mesh::ANIM_TRACK_LIMIT` holds the same number and
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
    _pad2: f32,
};

// The light rig a Wipeout HD circuit authors in its own `.envsettings`, or the
// stand-in below when nothing does. `enabled` is 0.0 or 1.0 rather than a
// branch, for the reason `Fog::enabled` is: one pipeline, not two.
//
// The direction and the sun's hue are the disc's. The magnitude is not - see
// `mesh_render::Light`.
struct Light {
    direction: vec3<f32>,
    enabled: f32,
    ambient: vec3<f32>,
    _lpad0: f32,
    sun: vec3<f32>,
    _lpad1: f32,
    // The prelit (baked-lightmap) curve and the specular weight, from the
    // circuit's `.envsettings`, applied exactly where its own fragment
    // microcode applies them - see `lit_texel` and `mesh_render::Light`.
    prelit_scale: vec3<f32>,
    specular_scale: f32,
    prelit_power: vec3<f32>,
    _lpad2: f32,
};

// The Zone effect's per-stage parameters. See `mesh_render::Zone`, which
// carries the microcode this reproduces and, more importantly, the list of
// terms deliberately left out of it for want of a source on the disc.
struct Zone {
    // `zoneColourTint.xy`, the `.effectSettings` keys `Texture U/V scale`.
    uv_scale: vec2<f32>,
    // 1.0 only when every input this path reads resolved off the disc.
    enabled: f32,
    _pad: f32,
    // `zoneEffect<Inner|Outer>`: the showing stage's `Scene.Texture Colour`.
    effect: vec4<f32>,
};

struct Scene {
    fog: Fog,
    light: Light,
    // Engine parameter slot 0, `time`: a global clock in seconds, splatted to
    // four lanes exactly as Wipeout HD's draw-state builders bind it. Only the
    // flame path below reads it. See `mesh_render::Scene::time`.
    time: vec4<f32>,
    zone: Zone,
};

// 1.0 when the render target holds linear light - Wipeout HD's float scene
// target, where the bloom gate reads pre-exposure luminance and the encode
// happens in `post::hd_bloom`'s own pass. 0.0 for every gamma target, where
// this shader encodes (or never decodes) exactly as it always has. Set from
// the target format at pipeline build - see `mesh_render::is_linear_target` -
// so no uniform needs a new field and the sky's zeroed scene buffer cannot
// miss it.
override linear_out: f32 = 0.0;

// **Wipeout HD's engine-flare program**, off by default and on only for a model
// whose material declares the whole parameter set it reads - see
// `oag_render::mesh::Flame`, which carries the arithmetic and the evidence, and
// `docs/ghidra/functions/ps3-hdfury-eu/engine-flare.md`, which carries the
// disassembly the arithmetic came from.
//
// **Every number is the material's, passed in as a pipeline constant**, because
// they are a property of the *model* exactly as `colour_is_light` is, and a
// uniform field would have to be mirrored by every pipeline in this crate and
// by the asset viewer. The defaults are zero rather than the shipped values on
// purpose: a model that reaches this path without its parameters must draw
// nothing recognisable, not a flame with numbers this file invented.
override flame_shading: f32 = 0.0;
override flame_rim_power: f32 = 0.0;
override flame_rim_scale: f32 = 0.0;
override flame_rim_min: f32 = 0.0;
override flame_alpha_scale: f32 = 0.0;
override flame_colour_scale: f32 = 0.0;
// `Speed`, the sixth number the material authors: what multiplies `scene.time`
// into the noise tap's `v`. 2.0 on all fourteen craft.
override flame_speed: f32 = 0.0;


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
@group(2) @binding(0) var<uniform> scene: Scene;
// The Zone stage's own `zoneModeTrack<n>.gtf`, and a sampler of its own
// because it is addressed by a coordinate this shader builds rather than by
// the mesh's own - `scene.zone.uv_scale * (1 - meshUV)` runs outside [0, 1]
// wherever the scale does, and the albedo sampler's `Repeat` is what the
// original's own sampler state says there too. Every draw outside a Zone race
// binds a 1x1 black, which is the identity once `scene.zone.enabled` is 0.
@group(2) @binding(1) var zone_tex: texture_2d<f32>;
@group(2) @binding(2) var zone_sampler: sampler;
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
    transform: array<mat4x4<f32>, 128>,
};
@group(3) @binding(1) var<uniform> node_anims: NodeAnims;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) colour: vec4<f32>,
    @location(3) texcoord: vec2<f32>,
    @location(4) lit: f32,
    @location(5) anim: u32,
    @location(6) lightmap_texcoord: vec2<f32>,
    @location(7) xform: u32,
    // HD's sun-occlusion mask - see `oag_render::mesh::GpuVertex::sun_mask`.
    // Not carried by `colour.a`, which is already the boost plume's baked
    // falloff on other titles and the bloom glow mask on this one.
    @location(8) sun_mask: f32,
    // Which texture this surface's colour and coverage come from - see
    // `oag_render::mesh::slots`, whose bit layout this file decodes and which
    // is changed together with it. Read off the material's own fragment
    // microcode; `slots::DEFAULT` is the first texture's colour and the first
    // texture's alpha, which is what every title but Wipeout HD carries.
    @location(9) slots: u32,
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
};

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
    // Uniform scale only, so the model matrix rotates normals correctly without
    // needing an inverse transpose. **The node matrix is not uniform**: 129 of
    // the 920 authored scale keys are per-axis, and skewing a normal by one of
    // those is wrong. It is harmless on this data and measured rather than
    // assumed - all 37 meshes under a non-uniformly scaled node are prelit
    // (`lit = 0.0`), so their normals never reach the light rig at all. A title
    // that lights one needs the inverse transpose here;
    // `scenery_animation_ground_truth.rs` fails when that day comes.
    let turned = node * vec4<f32>(in.normal, 0.0);
    out.normal = (uniforms.model * turned).xyz;
    out.colour = in.colour;
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
    out.lit = in.lit;
    out.world = world.xyz;
    out.view_depth = out.clip.w;
    out.sun_mask = in.sun_mask;
    out.slots = in.slots;
    out.cur_clip = out.clip;
    out.prev_clip = uniforms.prev_mvp * placed;
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
//     zoneUV    = zoneColourTint.xy * (1 - meshUV)
//     zoneCol   = zoneTex(zoneUV).rgb * zoneEffect.rgb
//     blackMask = saturate((albedo.r + albedo.g + albedo.b) * 100000)
//     surface   = albedo + zoneCol * (1 - blackMask)
//
// `blackMask` is a hand-rolled `step(0, x)` - the `100000` is the literal in
// the microcode - so the recolour lands **only where the material's own
// diffuse is pure black**. That is how the artists chose what lights up in
// Zone mode, and nothing in the palette table or the parameter list says it;
// it is authored in the albedo.
//
// The zone texture is sampled at the *mesh's own* UV, not in screen space and
// not projected through `zoneOrigin` - which is the counter-intuitive half of
// the reading, and is confirmed from the file side by the engine's own schema
// naming the two lanes "Texture U scale" and "Texture V scale".
//
// **This returns the sample and its mask, not the finished term**, because the
// two shading paths below sum in different colour spaces and `pow(a * b, g)`
// is not `pow(a, g) * b`. `zoneTex` is a texture and wants the same decode
// every other sample here gets; `zoneEffect` is a shader parameter and wants
// none - it is authored well past 1.0 (to 3.0 on stage 12), which is a
// multiplier's range and not a colour's.
//
// **Scope, stated because it is an approximation.** This applies to every
// surface `mesh.wgsl` draws, and the original applies it per material: 1,467
// of the disc's 1,590 `.rcsmaterial` files carry a Zone variant, so 123 do
// not. Craft hulls, the sky cube, pads and the collision wireframe all reach
// this path here and would pick up stage colour wherever they happen to carry
// pure-black texels, which the original would leave alone. It is the same
// shape as the shared specular exponent and the blanket sun term above - the
// per-material branch that would tell the families apart does not exist yet -
// and the minority it is wrong for is 123 materials of 1,590.
fn zone_sample(albedo: vec3<f32>, uv: vec2<f32>) -> vec4<f32> {
    let zone_uv = scene.zone.uv_scale * (1.0 - uv);
    let sample = textureSample(zone_tex, zone_sampler, zone_uv).rgb;
    let black_mask = saturate((albedo.r + albedo.g + albedo.b) * 100000.0);
    // `.a` carries the mask *and* the on/off switch, so a draw outside a Zone
    // race multiplies to nothing whatever the sample held.
    return vec4<f32>(sample, (1.0 - black_mask) * scene.zone.enabled);
}

fn lit_texel(in: VertexOutput) -> vec4<f32> {
    let n = normalize(in.normal);

    // The stand-in: two invented directions, kept for every title whose own
    // rig has not been recovered. See this file's header.
    let key = max(dot(n, normalize(vec3<f32>(0.4, 0.8, 0.5))), 0.0);
    let fill = max(dot(n, normalize(vec3<f32>(-0.5, 0.2, -0.7))), 0.0);
    let stand_in = vec3<f32>(0.15 + 0.75 * key + 0.25 * fill);

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
    let ndl = clamp(dot(n, scene.light.direction), 0.0, 1.0);
    let baked_linear = pow(baked.rgb, vec3<f32>(2.2));
    let prelit = scene.light.prelit_scale * pow(baked_linear, scene.light.prelit_power);
    // `select` rather than a multiply by the override, so the value is not
    // touched at all where the flag is set. (Measured: it makes no difference
    // to the frame either way - both forms move the same 9 pixels of 1,175,040
    // by one level, and that movement is the `tint` line below rather than this
    // one. Kept because not multiplying is the clearer statement.)
    let vertex_light = select(vec3<f32>(0.0), in.colour.rgb, colour_is_light > 0.5);

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
    let mask = baked.a * in.sun_mask;
    let sun_diffuse = scene.light.sun * (ndl * mask);
    // **This ambient reaches materials the disc never feeds it to**, and
    // `slots::NO_AMBIENT` says which - 251 of Anulpha Pass's 309 drawn
    // materials, 922 of its 1,101 chunks. Gating on that bit alone was tried
    // on 2026-08-24 and is a regression, not a fix: the materials without an
    // ambient are three families, not one. Most declare `prelitBias`,
    // `prelitScaleSpecular` and `directionalLight0*` and are lit by the
    // lightmap and the sun; but `sign_emissive` and its kin declare only
    // `fogColour` and are **emissive**, so multiplying them by an `authored`
    // with the ambient removed turns them black. Anulpha traded a brown
    // circuit for a black one. The branch this wants is the per-material
    // lighting path HANDOVER has open, not one bit.
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
    let emissive = (in.slots & 192u) == 192u;
    let lit_sum = scene.light.ambient + prelit + vertex_light + sun_diffuse;
    let authored = select(lit_sum, vec3<f32>(1.0), emissive);

    // **Which texture is the picture and which is the coverage, off the
    // material's own microcode** - see `oag_render::mesh::slots` and
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
    let first = textureSample(albedo, albedo_sampler, in.texcoord);
    let second = textureSample(lightmap, albedo_sampler, in.texcoord);
    let picture = select(first, second, (in.slots & 2u) != 0u);
    let coverage = select(first, second, (in.slots & 4u) != 0u);
    let texel = vec4<f32>(picture.rgb, coverage[(in.slots >> 3u) & 3u]);

    // The Zone term, in both domains, from one sample. See `zone_sample`.
    let zone = zone_sample(texel.rgb, in.texcoord);
    let zone_linear = pow(zone.rgb, vec3<f32>(2.2)) * scene.zone.effect.rgb * zone.a;
    let zone_gamma = zone.rgb * scene.zone.effect.rgb * zone.a;

    // The read specular term: half-vector against the sun, and the exponent
    // is a **stand-in**. It is an inline constant of each fragment program,
    // not a shared one: sweeping every material on the disc for the literal a
    // saturated dot is multiplied by between its `LG2` and its `EX2` gives 5
    // on 759 blocks, 10 on 704, 32 on 295, and a tail of 26.156, 40 (the
    // ships) and 300. 32 is the commonest round value and holds the place
    // until the pipeline can pick one per material. Gated by `mask` above -
    // the same sun-occlusion scalar the diffuse term reads, exactly where the
    // disc's own microcode gates its specular by it too - and by the diffuse
    // texture's alpha (gloss lives there; a DXT1 diffuse has alpha 1
    // everywhere, which is full gloss, as the original samples it too). Zero
    // whenever the authored rig is off: the stand-in never had one.
    let to_eye = normalize(scene.fog.camera - in.world);
    let half_vector = to_eye + scene.light.direction;
    let ndh = clamp(
        dot(half_vector, n) / max(length(half_vector), 1e-6),
        0.0,
        1.0,
    );
    let specular = scene.light.sun
        * (pow(ndh, 32.0) * ndl * mask * texel.a * scene.light.specular_scale
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
    // `surface = albedo + zoneCol`, then `colour = light * surface` - the
    // microcode's own order, and on this path both terms are linear.
    let lit_linear = (texel_linear + zone_linear) * authored + specular;
    let encoded = pow(
        clamp(lit_linear, vec3<f32>(0.0), vec3<f32>(1.0)),
        vec3<f32>(1.0 / 2.2),
    );
    let authored_rgb = mix(encoded, lit_linear, linear_out);

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
    let plain = (texel.rgb + zone_gamma) * tint * light;
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
    let rim = clamp(1.0 - dot(to_eye, n), 0.0, 1.0);
    let flame_alpha = flame_alpha_scale
        * (1.0 - (pow(rim, flame_rim_power) * flame_rim_scale + flame_rim_min));
    // **The program's two taps, both of them.** It samples `unit0` twice from
    // one texture: a *noise* tap whose `v` scrolls with the engine clock, and
    // a *colour* tap at doubled coordinates displaced by the noise the first
    // returned. Neither is this file's invention - block #1 and block #2 of
    // `flame_test.rcsmaterial` schedule the identical pair through different
    // registers, and `time`'s provider is engine parameter slot 0. See
    // `oag_render::mesh::Flame` and
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
    let flame = vec4<f32>(flame_rgb, flame_alpha * in.colour.a);

    let shaded = vec4<f32>(
        mix(plain_rgb, authored_rgb, scene.light.enabled * in.lit),
        texel.a * in.colour.a,
    );
    return mix(shaded, flame, flame_shading);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let shaded = lit_texel(in);
    // This pipeline is opaque (`blend: None`), so the alpha channel is never
    // blended with - but it is still written to whatever render target is
    // bound, so it stays a hardcoded 1.0 here rather than the texture's and
    // vertex colour's combined alpha, matching every prior opaque render
    // exactly. `fs_main_blend` below is the one that actually reads it.
    return vec4<f32>(fogged(shaded.rgb, in.world, in.view_depth), 1.0);
}

// Used only by the blended pipeline - see
// `oag_render::mesh::Model::transparent_draws`. Identical to `fs_main` except
// it outputs the texture's alpha times the vertex colour's alpha instead of a
// hardcoded 1.0, which is what lets the blend state built around this entry
// point actually blend rather than replace.
@fragment
fn fs_main_blend(in: VertexOutput) -> @location(0) vec4<f32> {
    let shaded = lit_texel(in);
    return vec4<f32>(fogged(shaded.rgb, in.world, in.view_depth), shaded.a);
}

// The GE's real alpha-test call **is** recovered now:
// `docs/ghidra/functions/psp-pulse-usa/mesh-draw.md` ("Four `_q` names in
// this path are wrong") reads `is_alpha_tested()`'s branch as
// `Gu_AlphaFunc(GU_GREATER, ref, 0xff)` with `ref` one of `0x7f`, `0` or
// `0x10` depending on the batch - a per-batch reference whose selector that
// page leaves unresolved.
//
// `0.5` (~`0x80`) was this project's own placeholder for the unrecovered
// case, and it is wrong on real data: Wipeout Pure's `Speedup Pad` glow
// texture (`speedup_GLOW_KEY.tga`, `Data\Environments\01_Vineta_K\track.vex`)
// decodes to an alpha histogram of exactly `{0: background, 57..58: glow
// interior}` - its brightest texel is ~0.23, so `0.5` discards the whole
// batch and the pad renders nothing. `0` is the value to use instead:
// it is one of the three recovered references, it is the same value the
// same function programs unconditionally on every *transparent* batch, and
// it is the most permissive of the three, so a batch actually authored for
// `0x7f` or `0x10` shows a few extra near-zero-alpha texels rather than this
// project hiding geometry the original drew.
//
// **Pulse's own corpus was not immune either** - the earlier "safe against
// `01_Track`/`16_Track`" claim here only ever measured *vertex* alpha
// (`docs/formats/vex.md`'s census), never the *texture* alpha `shaded.a`
// actually tests. That census itself records alpha-tested textures whose
// range is a partial band like `58-78` - up to 0.306, still under the old
// `0.5`. Measured directly (`crates/render/examples/threshold_probe.rs`, a
// 1024x1024 capture of each full track model, lit pixels = channel sum >
// 100): `01_Track` 16,159 -> 16,156 (noise, at the edge-antialiasing scale),
// `16_Track` 369,534 -> 370,987, **+1,453 pixels (+0.4%)** newly drawn rather
// than discarded. Small, and in the direction the fix predicts: the old
// threshold was already hiding a sliver of real Pulse content, not only
// Pure's pad.
const ALPHA_TEST_THRESHOLD: f32 = 1.0 / 255.0;

// Used only by the cutout pipeline - see
// `oag_render::mesh::Model::alpha_tested_draws`. Unlike `fs_main_blend`, this
// keeps depth write on (see `mesh_render::build`): a batch tagged
// `is_alpha_tested()` is meant to be treated as opaque wherever its texel
// clears the threshold, and fully absent everywhere else, not smoothly
// blended - `discard` is what lets the pixels below the threshold contribute
// neither colour nor depth, so surfaces behind a cutout's "empty" corners
// still show through and still get occluded correctly by whatever the
// cutout's solid pixels do draw. `shaded.a` is now the texture's alpha times
// the vertex colour's, per `lit_texel` above; measured directly against every
// alpha-tested batch on `01_Track`/`16_Track` that no vertex reference in
// this path carries baked alpha below `ALPHA_TEST_THRESHOLD`, so this
// fold-in does not newly discard any fragment that used to pass on real
// track data.
@fragment
fn fs_main_alpha_test(in: VertexOutput) -> @location(0) vec4<f32> {
    let shaded = lit_texel(in);
    if shaded.a < ALPHA_TEST_THRESHOLD {
        discard;
    }
    return vec4<f32>(fogged(shaded.rgb, in.world, in.view_depth), 1.0);
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
        vec4<f32>(fogged(shaded.rgb, in.world, in.view_depth), 1.0),
        velocity_of(in),
    );
}

@fragment
fn fs_main_alpha_test_velocity(in: VertexOutput) -> MrtOutput {
    let shaded = lit_texel(in);
    if shaded.a < ALPHA_TEST_THRESHOLD {
        discard;
    }
    return MrtOutput(
        vec4<f32>(fogged(shaded.rgb, in.world, in.view_depth), 1.0),
        velocity_of(in),
    );
}
