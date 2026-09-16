// Draws `05_Track`'s cloud puffs: a textured, camera-facing, per-instance
// rotating billboard - see `crate::cloud` and
// `docs/ghidra/functions/psp-pulse-usa/clouds.md`.
//
// Unlike `exhaust.wgsl`, this is a **standard alpha blend**
// (`GU_SRC_ALPHA`/`GU_ONE_MINUS_SRC_ALPHA`), not additive - see
// `CloudGroup_ApplyDrawState` on the doc page. The pipeline's `BlendState`
// does the weighting; this shader returns the unpremultiplied colour and the
// alpha to weight it by, the same split `exhaust.wgsl` uses for its own
// additive blend.

struct Uniforms {
    view_projection: mat4x4<f32>,
    model: mat4x4<f32>,
};

// 1.0 on Wipeout HD's linear float scene target, 0.0 on every gamma target -
// see `mesh_render::is_linear_target` and `exhaust.wgsl`'s own copy of this
// override for why the decode exists at all.
override linear_out: f32 = 0.0;

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(1) @binding(0) var cloud_texture: texture_2d<f32>;
@group(1) @binding(1) var cloud_sampler: sampler;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) colour: vec4<f32>,
    @location(3) texcoord: vec2<f32>,
    @location(4) lit: f32,
};

struct VertexOutput {
    @builtin(position) clip: vec4<f32>,
    @location(0) colour: vec4<f32>,
    @location(1) texcoord: vec2<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip = uniforms.view_projection * vec4<f32>(in.position, 1.0);
    out.colour = in.colour;
    out.texcoord = in.texcoord;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let texel = textureSample(cloud_texture, cloud_sampler, in.texcoord);
    let rgb = texel.rgb * in.colour.rgb;
    let decoded = mix(rgb, pow(rgb, vec3<f32>(2.2)), linear_out);
    return vec4<f32>(decoded, texel.a * in.colour.a);
}
