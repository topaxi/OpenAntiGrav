// Draws the LeachBeam's own ribbon - see `beam.rs`'s module doc comment for
// the recovered evidence. Deliberately the smallest shader in this crate:
// the ribbon carries no lighting, no fog and no second texture, only a
// texture sample tinted by the vertex colour the geometry half already
// computed (white, faded by the disconnect linger, zeroed at both ends).

import oag_shaders::scene_vertex::{Uniforms, VertexInput, VertexOutput};
import oag_shaders::target::{linear_out, gamma_to_target};

// What the ribbon stamps into the glow mask - `beam::GLOW_MASK`, the stencil
// reference `LeachBeam_SubmitStrip` writes with `REPLACE`.
override glow_mask: f32 = 0.0;

// 1.0 weights the colour by the vertex alpha as well as the texel's (the
// LeachBeam's `SRC_ALPHA` blend); 0.0 leaves the vertex alpha out of the colour
// - `MagStripArc_fp` computes `rgb = vertex.rgb * tex.rgb * tex.a` and sends the
// vertex alpha to the alpha output alone. See `magstrip.rs`.
override vertex_alpha_weight: f32 = 1.0;

// 1.0 decodes the gamma-authored colour on a linear target, as every additive
// draw here does; 0.0 adds it as it is - what `MagStripArc_fp` does: its five
// instructions carry no transfer function, so the original adds gamma-space
// values into its scene target. The HD engine tube makes the same choice, see
// `exhaust.wesl`.
override decode_source: f32 = 1.0;

// 1.0 writes `vertex.a * tex.a` to the alpha output (`MagStripArc_fp`'s own
// `a = vertex.a * tex.a`, added into the frame's alpha by its `ONE, ONE` blend);
// 0.0 writes the constant `glow_mask`.
override alpha_is_fragment: f32 = 0.0;

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(1) @binding(0) var beam_texture: texture_2d<f32>;
@group(1) @binding(1) var beam_sampler: sampler;

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    // The geometry half already builds world-space positions - `LeachBeam_BuildStrip`
    // pre-transforms the chain on the CPU and draws with the GE's own matrices
    // reset to identity, which this pipeline mirrors by carrying no model
    // matrix of its own.
    out.clip = uniforms.view_projection * vec4<f32>(in.position, 1.0);
    out.colour = in.colour;
    out.texcoord = in.texcoord;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let texel = textureSample(beam_texture, beam_sampler, in.texcoord);
    let rgb = texel.rgb * in.colour.rgb;
    let decoded = gamma_to_target(rgb, linear_out * decode_source);
    // `GU_ADD, GU_SRC_ALPHA, GU_FIX(0xffffff)`: the colour is weighted by the
    // fragment's alpha here, so the alpha channel is free to carry the glow
    // stamp the stencil writes - `beam::pipeline::BLEND` adds the colour and
    // replaces the alpha.
    let weight = texel.a * mix(1.0, in.colour.a, vertex_alpha_weight);
    let alpha = mix(glow_mask, in.colour.a * texel.a, alpha_is_fragment);
    return vec4<f32>(decoded * weight, alpha);
}
