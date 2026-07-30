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

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let n = normalize(in.normal);

    let key = max(dot(n, normalize(vec3<f32>(0.4, 0.8, 0.5))), 0.0);
    let fill = max(dot(n, normalize(vec3<f32>(-0.5, 0.2, -0.7))), 0.0);
    let rig = 0.15 + 0.75 * key + 0.25 * fill;
    // Prelit geometry already carries its lighting in the vertex colour.
    let light = mix(1.0, rig, in.lit);

    let texel = textureSample(albedo, albedo_sampler, in.texcoord);
    // Vertex colour modulates the texture, as the GE's texture-env does.
    return vec4<f32>(texel.rgb * in.colour.rgb * light, 1.0);
}
