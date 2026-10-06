// Blend class 8: Wipeout: Omega Collection's `psys_normal_heathaze` program,
// read instruction by instruction out of the PS4 executable's GCN microcode
// (docs/ghidra/functions/ps4-omega-eu/heat-haze.md, patch eboot blobs
// 0x9d147c and 0x9d116c, confidence 80).
//
// It does not draw a colour. It writes a signed screen-space offset into a
// separate target, and the composite (`oag_post::omega_tonemap`) moves the
// scene sample by it.
//
//   vertex:   colour = (kColourScale, kColourScale, kColourScale, alpha)
//             fade   = 2 / |w|^0.75
//   fragment: k   = colour.a * fade * sprite.a
//             out = colour.rg * k * (sprite.rg - 0.5)
//
// The executable's fragment first rejects a pixel whose scene depth is nearer
// than the particle (`depth >= w` passes). That is the hardware depth test of
// the pipeline that draws this - `LessEqual` against the scene's own depth,
// written nothing - so the gate is not repeated here.

import oag_shaders::scene_vertex::Uniforms;

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var sheet: texture_2d<f32>;
@group(0) @binding(2) var sheet_sampler: sampler;

// The three attributes the program reads of `GpuVertex`: position, colour and
// texture coordinate. The normal and the rest are not consumed, and a vertex
// state that declares an attribute its shader ignores is a validation warning.
struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(2) colour: vec4<f32>,
    @location(3) texcoord: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip: vec4<f32>,
    @location(0) colour: vec4<f32>,
    @location(1) texcoord: vec2<f32>,
    @location(2) fade: f32,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip = uniforms.view_projection * vec4<f32>(in.position, 1.0);
    out.colour = in.colour;
    out.texcoord = in.texcoord;
    out.fade = 2.0 / pow(max(abs(out.clip.w), 1e-4), 0.75);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let texel = textureSample(sheet, sheet_sampler, in.texcoord);
    let k = in.colour.a * in.fade * texel.a;
    return vec4<f32>(in.colour.rg * k * (texel.rg - vec2<f32>(0.5)), 0.0, 1.0);
}
