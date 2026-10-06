// Draws the `blob` shadow tier: one ground-aligned quad per craft.
//
// The vertices arrive already in world space - `shadow::quad` lays them on the
// surface the caller's own raycast found - so there is no model matrix here and
// the `model` field of the shared uniform block is ignored. The block is shared
// rather than trimmed so this pipeline reuses the layout and the size assertion
// the mesh path already has.
//
// **No `linear_out` override, unlike every other blended pipeline here.** This
// shader outputs pure black, and black is black in both a gamma target and
// Wipeout HD's linear float one - there is nothing to convert. The alpha-over
// blend then computes `dst * (1 - a)`, a darkening, in whichever space the
// target holds.

import oag_shaders::scene_vertex::{Uniforms, VertexInput};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(1) @binding(0) var silhouette: texture_2d<f32>;
@group(1) @binding(1) var silhouette_sampler: sampler;

struct VertexOutput {
    @builtin(position) clip: vec4<f32>,
    @location(0) texcoord: vec2<f32>,
    // How dark this craft's shadow is this frame, in the vertex colour's alpha:
    // the height fade `shadow::fade` computes. The rgb is unused - see below.
    @location(1) strength: f32,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip = uniforms.view_projection * vec4<f32>(in.position, 1.0);
    out.texcoord = in.texcoord;
    out.strength = in.colour.a;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // **The red channel is the coverage, and that is measured rather than
    // assumed.** Wipeout HD's `ambient_shadow.gtf` is a one-channel `B8`
    // texture whose descriptor's own `remap` broadcasts the stored byte to
    // rgb and forces alpha opaque - so the alpha carries nothing and every
    // colour channel carries the same value. Reading `.a` here would draw a
    // full-strength rectangle. See `docs/formats/gtf.md` and
    // `gtf_ground_truth.rs`, which asserts exactly that shape.
    //
    // High is shadow: measured on the disc's own nine, the corner texel is 0
    // and the craft's silhouette runs to 212 of 255.
    let coverage = textureSample(silhouette, silhouette_sampler, in.texcoord).r;
    return vec4<f32>(0.0, 0.0, 0.0, coverage * in.strength);
}
