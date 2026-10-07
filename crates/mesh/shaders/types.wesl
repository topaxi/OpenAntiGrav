// Every struct the mesh shader shares: uniform blocks, vertex in/out, the MRT output.

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
    // A second per-draw scalar: `ColourAnim` on the Bomb's fireball
    // (`slots::BOMB_FIRE`), a float the blast owns. Zero for every other draw.
    // Was padding until 2026-10-07.
    model_colour: f32,
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
    // World to the behind-the-glass target's clip space, the original's
    // `refractProject`; read by the tunnel glass alone. See
    // `mesh_render::Scene::refraction`.
    refraction: mat4x4<f32>,
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
    // The second diffuse coordinate set - see `oag_mesh::mesh::GpuVertex::texcoord2`.
    @location(12) texcoord2: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) colour: vec4<f32>,
    @location(2) texcoord: vec2<f32>,
    @location(3) lit: f32,
    @location(4) world: vec3<f32>,
    // `xy` the lightmap's coordinate, `zw` the second diffuse set
    // (`GpuVertex::texcoord2`): carried in the one location rather than a
    // fifteenth, which the GL backend has little room for.
    @location(5) lightmap_texcoord: vec4<f32>,
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

// Colour and screen-space velocity together, for the pipelines built against
// the race's two attachments - see `mesh_render::Velocity`.
struct MrtOutput {
    @location(0) colour: vec4<f32>,
    @location(1) velocity: vec2<f32>,
}
