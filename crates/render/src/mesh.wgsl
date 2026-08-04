// Draws a decoded .vex model.
//
// Lighting is a fixed two-light rig rather than the game's own: the point here
// is to see the geometry clearly and spot decoding errors, not to reproduce
// Pulse's look. A single light leaves faces pointing away from it unreadably
// black.

struct Uniforms {
    view_projection: mat4x4<f32>,
    model: mat4x4<f32>,
    // Phase of the global texture-animation clock, 0.0 to 1.0 over
    // `ANIM_PERIOD_TICKS`. Scaled per vertex by `v_cycles`, so a surface with
    // `v_cycles == 0.0` - which is nearly all of them - ignores it entirely.
    // See `oag_render::mesh::GpuVertex::v_cycles`.
    //
    // Three trailing f32 fields rather than a vec3: WGSL aligns vec3 to 16
    // bytes, which would silently insert padding this struct's Rust mirror
    // (a flat, tightly packed repr(C)) does not have.
    anim_phase: f32,
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
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
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(1) @binding(0) var albedo: texture_2d<f32>;
@group(1) @binding(1) var albedo_sampler: sampler;
@group(2) @binding(0) var<uniform> fog: Fog;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) colour: vec4<f32>,
    @location(3) texcoord: vec2<f32>,
    @location(4) lit: f32,
    @location(5) v_cycles: f32,
};

struct VertexOutput {
    @builtin(position) clip: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) colour: vec4<f32>,
    @location(2) texcoord: vec2<f32>,
    @location(3) lit: f32,
    @location(4) world: vec3<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    let world = uniforms.model * vec4<f32>(in.position, 1.0);
    out.clip = uniforms.view_projection * world;
    // Uniform scale only, so the model matrix rotates normals correctly
    // without needing an inverse transpose.
    out.normal = (uniforms.model * vec4<f32>(in.normal, 0.0)).xyz;
    out.colour = in.colour;
    // Scrolls backward through the palette's rows (decreasing V), not
    // forward. Confirmed against a real ship: the white column's authored
    // row sequence is asymmetric (a fast rise then a slower fade, not a
    // symmetric pulse like red's), so playing it in the wrong direction
    // reads as backwards - a fade-then-flash instead of a flash-then-fade -
    // while red's near-symmetric curve looks the same either way. See
    // `docs/formats/vex.md`.
    out.texcoord = in.texcoord - vec2<f32>(0.0, uniforms.anim_phase * in.v_cycles);
    out.lit = in.lit;
    out.world = world.xyz;
    return out;
}

// The GE's fog is a linear ramp between `near` and `far` - `Gu_Fog`
// (`0x08811748`) sends `far` and `1/(far - near)` and nothing else, so there is
// no curve to reproduce here. What the original varies is the *parameters*,
// re-sampled every frame from the camera's position inside the fog volume;
// that happens on the CPU before this uniform is written. See
// `docs/ghidra/functions/psp-pulse/fog.md`.
//
// **Radial distance from the eye, where the hardware uses view-space depth.**
// The two differ towards the screen edges, by up to `1 / cos(fov / 2)` - about
// 15 % at the corners of Pulse's authored field of view. Reproducing view-space
// z needs the view matrix separately, which this uniform block does not carry;
// recorded as a known divergence rather than silently accepted.
fn fogged(colour: vec3<f32>, world: vec3<f32>) -> vec3<f32> {
    let distance = length(world - fog.camera);
    let span = max(fog.far - fog.near, 1e-6);
    // 1.0 is clear, 0.0 is fully fogged, matching the GE's own sense.
    let factor = clamp((fog.far - distance) / span, 0.0, 1.0);
    return mix(fog.colour, colour, mix(1.0, factor, fog.enabled));
}

fn lit_texel(in: VertexOutput) -> vec4<f32> {
    let n = normalize(in.normal);

    let key = max(dot(n, normalize(vec3<f32>(0.4, 0.8, 0.5))), 0.0);
    let fill = max(dot(n, normalize(vec3<f32>(-0.5, 0.2, -0.7))), 0.0);
    let rig = 0.15 + 0.75 * key + 0.25 * fill;
    // Prelit geometry already carries its lighting in the vertex colour.
    let light = mix(1.0, rig, in.lit);

    let texel = textureSample(albedo, albedo_sampler, in.texcoord);
    // Vertex colour modulates the texture on all four channels, as the GE's
    // texture-env does - RGB and alpha alike, not RGB alone. See
    // MESH_BLEND_ALPHA_PLAN.md finding 1: dropping the vertex colour's own
    // alpha here is what made the boost plume's baked falloff vanish.
    return vec4<f32>(texel.rgb * in.colour.rgb * light, texel.a * in.colour.a);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let shaded = lit_texel(in);
    // This pipeline is opaque (`blend: None`), so the alpha channel is never
    // blended with - but it is still written to whatever render target is
    // bound, so it stays a hardcoded 1.0 here rather than the texture's and
    // vertex colour's combined alpha, matching every prior opaque render
    // exactly. `fs_main_blend` below is the one that actually reads it.
    return vec4<f32>(fogged(shaded.rgb, in.world), 1.0);
}

// Used only by the blended pipeline - see
// `oag_render::mesh::Model::transparent_draws`. Identical to `fs_main` except
// it outputs the texture's alpha times the vertex colour's alpha instead of a
// hardcoded 1.0, which is what lets the blend state built around this entry
// point actually blend rather than replace.
@fragment
fn fs_main_blend(in: VertexOutput) -> @location(0) vec4<f32> {
    let shaded = lit_texel(in);
    return vec4<f32>(fogged(shaded.rgb, in.world), shaded.a);
}

// The GE's real alpha-test reference value has not been recovered from a
// decompile; 0.5 is an invented placeholder, the same status as
// `mesh_render::TRANSPARENT_BLEND` and `race::GLOW_SCROLL_PERIOD_TICKS` -
// revise the moment the real threshold is known.
const ALPHA_TEST_THRESHOLD: f32 = 0.5;

// Used only by the cutout pipeline - see
// `oag_render::mesh::Model::alpha_tested_draws`. Unlike `fs_main_blend`, this
// keeps depth write on (see `mesh_render::build`): a batch tagged
// `is_alpha_tested()` is meant to be treated as opaque wherever its texel
// clears the threshold, and fully absent everywhere else, not smoothly
// blended - `discard` is what lets the pixels below the threshold contribute
// neither colour nor depth, so surfaces behind a cutout's "empty" corners
// still show through and still get occluded correctly by whatever the
// cutout's solid pixels do draw. `shaded.a` is now the texture's alpha times
// the vertex colour's, per `lit_texel` above; confirmed against every
// alpha-tested batch on `01_Track`/`16_Track` that no vertex reference in
// this path carries baked alpha below `ALPHA_TEST_THRESHOLD` (see
// MESH_BLEND_ALPHA_PLAN.md), so this fold-in does not newly discard any
// fragment that used to pass on real track data.
@fragment
fn fs_main_alpha_test(in: VertexOutput) -> @location(0) vec4<f32> {
    let shaded = lit_texel(in);
    if shaded.a < ALPHA_TEST_THRESHOLD {
        discard;
    }
    return vec4<f32>(fogged(shaded.rgb, in.world), 1.0);
}
