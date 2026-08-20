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

struct Scene {
    fog: Fog,
    light: Light,
};

// 1.0 when the render target holds linear light - Wipeout HD's float scene
// target, where the bloom gate reads pre-exposure luminance and the encode
// happens in `post::hd_bloom`'s own pass. 0.0 for every gamma target, where
// this shader encodes (or never decodes) exactly as it always has. Set from
// the target format at pipeline build - see `mesh_render::is_linear_target` -
// so no uniform needs a new field and the sky's zeroed scene buffer cannot
// miss it.
override linear_out: f32 = 0.0;

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(1) @binding(0) var albedo: texture_2d<f32>;
@group(1) @binding(1) var albedo_sampler: sampler;
// The circuit's baked lighting atlas, on the surfaces whose material names one.
// Every other draw binds a white 1x1, so the multiply below is the identity and
// no branch is needed - the same arrangement `albedo` already uses.
@group(1) @binding(2) var lightmap: texture_2d<f32>;
@group(2) @binding(0) var<uniform> scene: Scene;
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
    return out;
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
    let baked = textureSample(lightmap, albedo_sampler, in.lightmap_texcoord);

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
    // and that sum multiplies the albedo. **Neither has an `N.L` or a sun
    // colour**, which is why no sun term appears here: the disc's sun reaches
    // this material only through the specular path below. A `sun * ndl *
    // baked.a` summand used to stand where the comment ends, and with the
    // lightmap-less placeholder's alpha of 1 it fired at full strength on the
    // two thirds of a circuit that has no lightmap - measured as 14.3 % of
    // the frame clipped to white against the reference's 5.8 %.
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
    let authored = scene.light.ambient + prelit + in.colour.rgb;

    let texel = textureSample(albedo, albedo_sampler, in.texcoord);

    // The read specular term: half-vector against the sun, and the exponent
    // is a **stand-in**. It is an inline constant of each fragment program,
    // not a shared one: sweeping every material on the disc for the literal a
    // saturated dot is multiplied by between its `LG2` and its `EX2` gives 5
    // on 759 blocks, 10 on 704, 32 on 295, and a tail of 26.156, 40 (the
    // ships) and 300. 32 is the commonest round value and holds the place
    // until the pipeline can pick one per material. Masked by
    // the sun's own incidence, the lightmap's shadow alpha and the diffuse
    // texture's alpha (gloss lives there; a DXT1 diffuse has alpha 1
    // everywhere, which is full gloss, as the original samples it too). Zero
    // whenever the authored rig is off: the stand-in never had one.
    //
    // This term is the **only** way directional light reaches an HD surface.
    // Both materials read so far agree: `track_surface` block #7 and
    // `detonator_ship_rich_iridescent` block #2 each build a half vector from
    // the same patched light direction and gate the result by `N.L`, and
    // neither has a Lambert diffuse. See renderer.md, "Ships have no Lambert
    // diffuse either".
    let to_eye = normalize(scene.fog.camera - in.world);
    let half_vector = to_eye + scene.light.direction;
    let ndh = clamp(
        dot(half_vector, n) / max(length(half_vector), 1e-6),
        0.0,
        1.0,
    );
    let specular = scene.light.sun
        * (pow(ndh, 32.0) * ndl * baked.a * texel.a * scene.light.specular_scale
            * scene.light.enabled * in.lit);

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
    let lit_linear = texel_linear * authored + specular;
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
    // path, so this multiply must not see HD's.** `linear_out` is the
    // discriminator and it is exact: it is set from `is_linear_target`, and
    // the float scene target belongs to HD's bloom chain alone. Without this,
    // any HD draw that binds `Light::stand_in` - the front end, or a circuit
    // whose `.envsettings` fails to yield a rig - would multiply by a baked
    // light that is zero wherever no colour set exists, and render black.
    let tint = mix(in.colour.rgb, vec3<f32>(1.0), linear_out);
    let plain = texel.rgb * tint * light;
    let plain_rgb = mix(plain, pow(plain, vec3<f32>(2.2)), linear_out);

    // Vertex colour modulates the texture on all four channels, as the GE's
    // texture-env does - RGB and alpha alike, not RGB alone. Dropping the
    // vertex colour's own alpha here is what made the boost plume's baked
    // falloff vanish; see `every_psp_teams_boost_plume_vertex_alpha_is_bimodal`
    // in `crates/game/tests/boost_plume_ground_truth.rs`.
    return vec4<f32>(
        mix(plain_rgb, authored_rgb, scene.light.enabled * in.lit),
        texel.a * in.colour.a,
    );
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
