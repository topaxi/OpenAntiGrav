// Draws a decoded .vex model.
//
// Lighting is a fixed two-light rig rather than the game's own: the point here
// is to see the geometry clearly and spot decoding errors, not to reproduce
// Pulse's look. A single light leaves faces pointing away from it unreadably
// black.

struct Uniforms {
    view_projection: mat4x4<f32>,
    model: mat4x4<f32>,
    // Blink-light palette scroll, in the texture's own V (row) units - see
    // `oag_render::mesh::GpuVertex::glow`. Only vertices with `glow == 1.0`
    // are affected; every other surface ignores this value.
    //
    // Three trailing f32 fields rather than a vec3: WGSL aligns vec3 to 16
    // bytes, which would silently insert padding this struct's Rust mirror
    // (a flat, tightly packed repr(C)) does not have.
    glow_scroll: f32,
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(1) @binding(0) var albedo: texture_2d<f32>;
@group(1) @binding(1) var albedo_sampler: sampler;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) colour: vec4<f32>,
    @location(3) texcoord: vec2<f32>,
    @location(4) lit: f32,
    @location(5) glow: f32,
};

struct VertexOutput {
    @builtin(position) clip: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) colour: vec4<f32>,
    @location(2) texcoord: vec2<f32>,
    @location(3) lit: f32,
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
    out.texcoord = in.texcoord - vec2<f32>(0.0, uniforms.glow_scroll * in.glow);
    out.lit = in.lit;
    return out;
}

fn lit_texel(in: VertexOutput) -> vec4<f32> {
    let n = normalize(in.normal);

    let key = max(dot(n, normalize(vec3<f32>(0.4, 0.8, 0.5))), 0.0);
    let fill = max(dot(n, normalize(vec3<f32>(-0.5, 0.2, -0.7))), 0.0);
    let rig = 0.15 + 0.75 * key + 0.25 * fill;
    // Prelit geometry already carries its lighting in the vertex colour.
    let light = mix(1.0, rig, in.lit);

    let texel = textureSample(albedo, albedo_sampler, in.texcoord);
    // Vertex colour modulates the texture, as the GE's texture-env does.
    return vec4<f32>(texel.rgb * in.colour.rgb * light, texel.a);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let shaded = lit_texel(in);
    // This pipeline is opaque (`blend: None`), so the alpha channel is never
    // blended with - but it is still written to whatever render target is
    // bound, so it stays a hardcoded 1.0 here rather than the texture's own
    // alpha, matching every prior opaque render exactly. `fs_main_blend`
    // below is the one that actually reads the texture's alpha.
    return vec4<f32>(shaded.rgb, 1.0);
}

// Used only by the blended pipeline - see
// `oag_render::mesh::Model::transparent_draws`. Identical to `fs_main` except
// it outputs the texture's own alpha instead of a hardcoded 1.0, which is what
// lets the blend state built around this entry point actually blend rather
// than replace.
@fragment
fn fs_main_blend(in: VertexOutput) -> @location(0) vec4<f32> {
    return lit_texel(in);
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
// cutout's solid pixels do draw.
@fragment
fn fs_main_alpha_test(in: VertexOutput) -> @location(0) vec4<f32> {
    let shaded = lit_texel(in);
    if shaded.a < ALPHA_TEST_THRESHOLD {
        discard;
    }
    return vec4<f32>(shaded.rgb, 1.0);
}
