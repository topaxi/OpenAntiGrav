// Draws the LeachBeam's own ribbon - see `beam.rs`'s module doc comment for
// the recovered evidence. Deliberately the smallest shader in this crate:
// the ribbon carries no lighting, no fog and no second texture, only a
// texture sample tinted by the vertex colour the geometry half already
// computed (white, faded by the disconnect linger, zeroed at both ends).

import oag_shaders::scene_vertex::{Uniforms, VertexInput};
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

// 1.0 for Wipeout HD's Rocket smoke ribbon, whose fragment program
// (`hd_rockettrail.rcsmaterial`, read on
// `docs/ghidra/functions/ps3-hdfury-eu/rocket-trail.md`) shades
//
//     facing = saturate(max(min(abs(dot(n, v)), 0.32), 0) * 3.12433)
//     rgb    = tex.rgb * vertex.rgb
//     alpha  = saturate(14 * tex.a * facing * vertex.a) * saturate(window_z * 0.75)
//
// and is drawn `SRC_ALPHA, ONE_MINUS_SRC_ALPHA`, so the alpha leaves here
// unweighted for the blend to apply. The draw's own `(0.8, 0.8, 0.8)` ambient is
// already in the vertex colour (`oag_fx::rocket_smoke::AMBIENT`).
override smoke: f32 = 0.0;

// 1.0 for Wipeout HD's LeachBeam strip, whose fragment program
// (`hd_leachbeam.rcsmaterial`, `docs/ghidra/functions/ps3-hdfury-eu/leach-beam-strips.md`)
// shades
//
//     n     = 2 * noise(u + 2 time, v).r                 unit 0, 128x128
//     glow  = glow(u + time + n, v)                      unit 1, 256x256
//     rgb   = glow.rgb * vertex.rgb
//     alpha = glow.a * abs(dot(n, v)) * vertex.a * saturate(window_z * 0.75)
//
// and is drawn `SRC_ALPHA, ONE`. `time` rides the uniform block's eye column.
override leach: f32 = 0.0;

@group(0) @binding(0) var<uniform> uniforms: Uniforms;

struct VertexOutput {
    @builtin(position) clip: vec4<f32>,
    @location(0) colour: vec4<f32>,
    @location(1) texcoord: vec2<f32>,
    // The smoke's `TC2` and `TC1`: the world normal and eye minus position.
    @location(2) normal: vec3<f32>,
    @location(3) view: vec3<f32>,
    @location(4) clock: f32,
};
@group(1) @binding(0) var beam_texture: texture_2d<f32>;
@group(1) @binding(1) var beam_sampler: sampler;
@group(1) @binding(2) var noise_texture: texture_2d<f32>;

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
    out.normal = in.normal;
    // `model`'s fourth column carries the eye for the smoke's facing fade -
    // `Pipeline::upload_with_eye`; the identity's zero elsewhere, unused.
    out.view = uniforms.model[3].xyz - in.position;
    out.clock = uniforms.model[3].w;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    if leach > 0.5 {
        let u = in.texcoord.x;
        let v = in.texcoord.y;
        let n = 2.0 * textureSample(noise_texture, beam_sampler, vec2<f32>(u + 2.0 * in.clock, v)).r;
        let glow = textureSample(beam_texture, beam_sampler, vec2<f32>(u + in.clock + n, v));
        let facing = abs(dot(normalize(in.normal), normalize(in.view)));
        let alpha = glow.a * facing * in.colour.a * saturate(in.clip.z * 0.75);
        return vec4<f32>(gamma_to_target(glow.rgb * in.colour.rgb, linear_out * decode_source), alpha);
    }
    let texel = textureSample(beam_texture, beam_sampler, in.texcoord);
    let rgb = texel.rgb * in.colour.rgb;
    let decoded = gamma_to_target(rgb, linear_out * decode_source);
    // `GU_ADD, GU_SRC_ALPHA, GU_FIX(0xffffff)`: the colour is weighted by the
    // fragment's alpha here, so the alpha channel is free to carry the glow
    // stamp the stencil writes - `beam::pipeline::BLEND` adds the colour and
    // replaces the alpha.
    let weight = texel.a * mix(1.0, in.colour.a, vertex_alpha_weight);
    let alpha = mix(glow_mask, in.colour.a * texel.a, alpha_is_fragment);
    if smoke > 0.5 {
        let facing_dot = dot(normalize(in.normal), normalize(in.view));
        // `abs`: the `MIN`'s source carries NV40's `SRC0_ABS` bit (word 1 bit
        // 29), so a fin shows from either side - the original's own eye and
        // vertex buffer leave every fin back-facing from the chase camera
        // otherwise, against a film full of smoke.
        let facing = saturate(max(min(abs(facing_dot), 0.32), 0.0) * 3.12433);
        let depth_fade = saturate(in.clip.z * 0.75);
        let coverage = saturate(14.0 * texel.a * facing * in.colour.a) * depth_fade;
        return vec4<f32>(decoded, coverage);
    }
    return vec4<f32>(decoded * weight, alpha);
}
