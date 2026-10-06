// Draws `05_Track`'s cloud puffs: a textured, camera-facing, per-instance
// rotating billboard - see `crate::cloud` and
// `docs/ghidra/functions/psp-pulse-usa/clouds.md`.
//
// Unlike `exhaust.wesl`, this is a **standard alpha blend**
// (`GU_SRC_ALPHA`/`GU_ONE_MINUS_SRC_ALPHA`), not additive - see
// `CloudGroup_ApplyDrawState` on the doc page. The pipeline's `BlendState`
// does the weighting; this shader returns the unpremultiplied colour and the
// alpha to weight it by, the same split `exhaust.wesl` uses for its own
// additive blend.

import oag_shaders::scene_vertex::{Uniforms, VertexInput, VertexOutput};
import oag_shaders::target::{linear_out, gamma_to_target};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(1) @binding(0) var cloud_texture: texture_2d<f32>;
@group(1) @binding(1) var cloud_sampler: sampler;

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
    let decoded = gamma_to_target(rgb, linear_out);
    return vec4<f32>(decoded, texel.a * in.colour.a);
}
