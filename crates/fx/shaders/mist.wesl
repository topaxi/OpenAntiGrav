// The weather's mist overlay: two screen quads of `Mist.mip`, added over the
// frame - see `crate::mist` and `docs/ghidra/functions/psp-pulse-usa/weather.md`.
//
// `WeatherMist_Draw` sets identity matrices, so a vertex's position is already
// clip space. The frame runs under `GU_TFX_MODULATE`/`GU_TCC_RGBA` with a white
// vertex colour, so a layer contributes the texel times its alpha, which the
// pipeline's `(SRC_ALPHA, ONE)` blend then adds.

import oag_shaders::target::{linear_out, gamma_to_target};

@group(0) @binding(0) var mist_texture: texture_2d<f32>;
@group(0) @binding(1) var mist_sampler: sampler;

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) texcoord: vec2<f32>,
    @location(2) alpha: f32,
};

struct VertexOutput {
    @builtin(position) clip: vec4<f32>,
    @location(0) texcoord: vec2<f32>,
    @location(1) alpha: f32,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip = vec4<f32>(in.position, 0.0, 1.0);
    out.texcoord = in.texcoord;
    out.alpha = in.alpha;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let texel = textureSample(mist_texture, mist_sampler, in.texcoord);
    let decoded = gamma_to_target(texel.rgb, linear_out);
    return vec4<f32>(decoded, texel.a * in.alpha);
}
