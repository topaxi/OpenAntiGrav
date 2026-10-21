// HD's holographic billboard (`slots::HOLO_BILLBOARD`,
// `oag_mesh::mesh::rcs::holo_billboard`): the advert times a second layer,
// through an overlay combine with a scrolling scanline texture. Read off the
// program's own fragment block (Amphiseum `@0x14e0`); every number below is a
// literal of it except the three numbers that ride in the glow-table entry's
// tint: the scanline drift rate (x), the second layer's scroll per second added
// to its `v` (y, `-5.625` for the inline program's negated `MAD`) and the scale
// of both its coordinates (z, `1` where the program has no multiply). They are
// the program's own inline literals or, for the parameter-driven shape, the
// material's own parameters.
//
//     A     = scan(P)                          TEX R0.yz, f[TC4] unit1
//     lay   = unit2(z Q.x, z (Q.y + y t))      TEX H0.xyz, R1.zwzz
//     base  = advert(S) * lay                  TEX H4.xyz, f[TC5] unit0
//     B     = scan(P.x + r1 t, P.y + r1 t + A.y)
//     out   = 2 base B  where base < 0.5, else 1 - 2 (1 - base)(1 - B)
//     alpha = A.z
//
// `S` is `texcoord`, `Q` the vertex's `lightmap_texcoord.xy` and `P` its
// `lightmap_texcoord.zw`; `albedo` is the advert, `lightmap` the scanlines and
// `pad_mask` the second layer. Samples are raw, as every HD program's are.

import package::bindings::{emissives, scene};
@if(!bindless) import package::bindings::{albedo, albedo_sampler, lightmap, pad_mask};
@if(bindless) import package::bindings::{material, samplers, textures};
import oag_shaders::target::linear_out;
import package::types::{VertexOutput};

// `SLT` against 0.5 per channel, then `MUL x2` where it holds and
// `1 - 2 (1 - a)(1 - b)` where it does not.
fn overlay(base: vec3<f32>, layer: vec3<f32>) -> vec3<f32> {
    let low = 2.0 * base * layer;
    let high = vec3<f32>(1.0) - 2.0 * (vec3<f32>(1.0) - base) * (vec3<f32>(1.0) - layer);
    return select(high, low, base < vec3<f32>(0.5));
}

@diagnostic(off, derivative_uniformity)
fn holo_billboard(in: VertexOutput) -> vec4<f32> {
    // The `bindless` variant's slot - see `bindings.wesl`.
    @if(bindless) {
        let albedo = textures[material.albedo];
        let albedo_sampler = samplers[material.albedo_sampler];
        let lightmap = textures[material.lightmap];
        let pad_mask = textures[material.pad_mask];
    }
    let rates = emissives.tint_offset[(in.slots >> 24u) & 127u];
    let t = scene.time.x;
    let layer_uv = in.lightmap_texcoord.xy;
    let scan_uv = in.lightmap_texcoord.zw;
    let tap = textureSample(lightmap, albedo_sampler, scan_uv);
    let layer = textureSample(
        pad_mask,
        albedo_sampler,
        rates.z * vec2<f32>(layer_uv.x, layer_uv.y + rates.y * t),
    ).rgb;
    let advert = textureSample(albedo, albedo_sampler, in.texcoord).rgb;
    let base = advert * layer;
    let drift = rates.x * t;
    let scan = textureSample(
        lightmap,
        albedo_sampler,
        vec2<f32>(scan_uv.x + drift, scan_uv.y + drift + tap.y),
    ).rgb;
    // The program's closing `/2`, then the encode a gamma target needs; the
    // race's linear target is encoded by the ROP, as `lit_texel` does.
    let lit = overlay(base, scan) * scene.light.output_scale;
    let encoded = pow(clamp(lit, vec3<f32>(0.0), vec3<f32>(1.0)), vec3<f32>(1.0 / 2.2));
    return vec4<f32>(mix(encoded, lit, linear_out), tap.z);
}
