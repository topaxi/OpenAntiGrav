// The cockpit hit static: the noise texture on two gradient strips along the
// view's side edges, alpha-blended by the pipeline - `CockpitHitFx_Draw`'s
// textured triangle strips. See `cockpit_hit.rs`.

import oag_shaders::target::{linear_out, gamma_to_target, target_out};

@group(0) @binding(0) var noise: texture_2d<f32>;
@group(0) @binding(1) var noise_sampler: sampler;

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) texcoord: vec2<f32>,
    @location(2) colour: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip: vec4<f32>,
    @location(0) texcoord: vec2<f32>,
    @location(1) colour: vec4<f32>,
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip = vec4<f32>(in.position, 0.0, 1.0);
    out.texcoord = in.texcoord;
    out.colour = in.colour;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // The GE's default texture function modulates the noise by the vertex
    // colour, alpha included.
    let texel = textureSample(noise, noise_sampler, in.texcoord);
    let rgb = gamma_to_target(texel.rgb * in.colour.rgb, linear_out);
    return vec4<f32>(target_out(rgb), texel.a * in.colour.a);
}
