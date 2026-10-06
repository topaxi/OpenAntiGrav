// AMD FidelityFX Super Resolution 1 - EASU and RCAS, ported to WGSL.
//
// Ported from `ffx_fsr1.h` and `ffx_a.h` of AMD's FidelityFX-FSR
// (https://github.com/GPUOpen-Effects/FidelityFX-FSR), which are
// Copyright (c) 2021 Advanced Micro Devices, Inc. and MIT-licensed. The
// licence text travels with this project in `licences/AMD-FidelityFX-MIT.txt`.
//
// This is a transliteration, not a reinterpretation: the arithmetic, the tap
// pattern, the magic constants and the bit-trick reciprocals are all upstream's
// and are deliberately left exactly as they are, comments included, so that a
// reader can diff this against `ffx_fsr1.h` line for line. Where upstream uses a
// macro (`AF1`, `AU1_AF1`) this uses the plain WGSL type or `bitcast`; that is
// the only systematic change.
//
// **Both passes work in perceptual space** - that is, on sRGB-encoded values,
// not on linear light. See `post::fsr1` for who is responsible for arranging
// that.

import oag_shaders::fullscreen::{VertexOutput, fullscreen_triangle};

struct Constants {
    // FsrEasuCon's four output vectors, already unpacked to floats on the CPU
    // rather than bit-packed into uint4s: upstream packs them only so that the
    // 16-bit path can unpack two halves out of one uint.
    con0: vec4<f32>,
    con1: vec4<f32>,
    con2: vec4<f32>,
    con3: vec4<f32>,
    // FsrRcasCon's `sharpness`, already through exp2(-sharpness).
    sharpness: f32,
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
}

@group(0) @binding(0) var source: texture_2d<f32>;
@group(0) @binding(1) var source_sampler: sampler;
@group(0) @binding(2) var<uniform> constants: Constants;

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> VertexOutput {
    return fullscreen_triangle(index);
}

// ffx_a.h's approximate reciprocals and reciprocal square root. These are
// Newton-free bit tricks: subtract the float's bit pattern from a magic
// constant. They are not interchangeable with `1.0 / x` - upstream's comment on
// the RCAS resolve is explicit that the *medium* precision one is needed "to
// avoid visible tonality changes", so the precision of each call site is part of
// the algorithm rather than an optimisation to be tidied away.
fn prx_lo_rcp(a: f32) -> f32 {
    return bitcast<f32>(0x7ef07ebbu - bitcast<u32>(a));
}

fn prx_med_rcp(a: f32) -> f32 {
    let b = bitcast<f32>(0x7ef19fffu - bitcast<u32>(a));
    return b * (-b * a + 2.0);
}

fn prx_lo_rsq(a: f32) -> f32 {
    return bitcast<f32>(0x5f347d74u - (bitcast<u32>(a) >> 1u));
}

fn min3(x: f32, y: f32, z: f32) -> f32 { return min(x, min(y, z)); }
fn max3(x: f32, y: f32, z: f32) -> f32 { return max(x, max(y, z)); }
fn min3v(x: vec3<f32>, y: vec3<f32>, z: vec3<f32>) -> vec3<f32> { return min(x, min(y, z)); }
fn max3v(x: vec3<f32>, y: vec3<f32>, z: vec3<f32>) -> vec3<f32> { return max(x, max(y, z)); }

// FsrEasuTapF: one weighted tap into the accumulator.
//
// `acc.xyz` is the accumulated colour and `acc.w` the accumulated weight,
// packed into one vec4 because WGSL has no `inout` parameters.
fn easu_tap(
    acc: vec4<f32>,
    offset: vec2<f32>,
    dir: vec2<f32>,
    len: vec2<f32>,
    lob: f32,
    clp: f32,
    colour: vec3<f32>,
) -> vec4<f32> {
    // Rotate offset by direction.
    var v = vec2<f32>(
        (offset.x * dir.x) + (offset.y * dir.y),
        (offset.x * -dir.y) + (offset.y * dir.x),
    );
    // Anisotropy.
    v *= len;
    // Compute distance^2, limited to the window: at a corner two taps can
    // easily fall outside it.
    let d2 = min(v.x * v.x + v.y * v.y, clp);
    // Approximation of lanczos2 without sin(), rcp() or sqrt():
    //  (25/16 * (2/5 * x^2 - 1)^2 - (25/16 - 1)) * (1/4 * x^2 - 1)^2
    //  |_______________________________________|   |_______________|
    //                   base                             window
    var w_b = (2.0 / 5.0) * d2 + -1.0;
    var w_a = lob * d2 + -1.0;
    w_b *= w_b;
    w_a *= w_a;
    w_b = (25.0 / 16.0) * w_b + -(25.0 / 16.0 - 1.0);
    let w = w_b * w_a;
    return acc + vec4<f32>(colour * w, w);
}

// FsrEasuSetF: accumulate gradient direction and length for one of the four
// bilinear corners.
//
// Upstream passes four compile-time booleans and relies on the compiler folding
// away three of the four weight expressions. Here the caller passes the weight
// it already knows, which is the same arithmetic with the folding done by hand -
// WGSL has no predicate-immediate idiom to lean on.
//
// `acc` packs dir in `.xy` and len in `.z`.
fn easu_set(
    acc: vec3<f32>,
    w: f32,
    l_a: f32,
    l_b: f32,
    l_c: f32,
    l_d: f32,
    l_e: f32,
) -> vec3<f32> {
    var out = acc;
    // Direction is the '+' diff:
    //    a
    //  b c d
    //    e
    // Magnitude comes from the absolute average of both sides of 'c'. Length
    // converts a gradient reversal to 0, smoothly to non-reversal at 1, shaped,
    // then adds the horizontal and vertical terms.
    let dc = l_d - l_c;
    let cb = l_c - l_b;
    var len_x = max(abs(dc), abs(cb));
    len_x = prx_lo_rcp(len_x);
    let dir_x = l_d - l_b;
    out.x += dir_x * w;
    len_x = clamp(abs(dir_x) * len_x, 0.0, 1.0);
    len_x *= len_x;
    out.z += len_x * w;

    // Repeat for the y axis.
    let ec = l_e - l_c;
    let ca = l_c - l_a;
    var len_y = max(abs(ec), abs(ca));
    len_y = prx_lo_rcp(len_y);
    let dir_y = l_e - l_a;
    out.y += dir_y * w;
    len_y = clamp(abs(dir_y) * len_y, 0.0, 1.0);
    len_y *= len_y;
    out.z += len_y * w;
    return out;
}

// FsrEasuF: edge-adaptive spatial upsampling.
@fragment
fn fs_easu(in: VertexOutput) -> @location(0) vec4<f32> {
    // Get position of 'f'.
    let ip = floor(in.position.xy);
    var pp = ip * constants.con0.xy + constants.con0.zw;
    let fp = floor(pp);
    pp -= fp;

    // 12-tap kernel.
    //    b c
    //  e f g h
    //  i j k l
    //    n o
    //
    // Gather ordering is (lower-left, lower-right, upper-right, upper-left),
    // which WGSL's `textureGather` shares with HLSL's `Gather` and Vulkan's
    // `OpImageGather`, so upstream's component naming carries over unchanged.
    // The 'z's below are upstream's unused lanes, kept so the diff stays honest.
    let p0 = fp * constants.con1.xy + constants.con1.zw;
    let p1 = p0 + constants.con2.xy;
    let p2 = p0 + constants.con2.zw;
    let p3 = p0 + constants.con3.xy;

    let bczz_r = textureGather(0, source, source_sampler, p0);
    let bczz_g = textureGather(1, source, source_sampler, p0);
    let bczz_b = textureGather(2, source, source_sampler, p0);
    let ijfe_r = textureGather(0, source, source_sampler, p1);
    let ijfe_g = textureGather(1, source, source_sampler, p1);
    let ijfe_b = textureGather(2, source, source_sampler, p1);
    let klhg_r = textureGather(0, source, source_sampler, p2);
    let klhg_g = textureGather(1, source, source_sampler, p2);
    let klhg_b = textureGather(2, source, source_sampler, p2);
    let zzon_r = textureGather(0, source, source_sampler, p3);
    let zzon_g = textureGather(1, source, source_sampler, p3);
    let zzon_b = textureGather(2, source, source_sampler, p3);

    // Simplest multi-channel approximate luma possible (luma times 2).
    let bczz_l = bczz_b * 0.5 + (bczz_r * 0.5 + bczz_g);
    let ijfe_l = ijfe_b * 0.5 + (ijfe_r * 0.5 + ijfe_g);
    let klhg_l = klhg_b * 0.5 + (klhg_r * 0.5 + klhg_g);
    let zzon_l = zzon_b * 0.5 + (zzon_r * 0.5 + zzon_g);

    let b_l = bczz_l.x;
    let c_l = bczz_l.y;
    let i_l = ijfe_l.x;
    let j_l = ijfe_l.y;
    let f_l = ijfe_l.z;
    let e_l = ijfe_l.w;
    let k_l = klhg_l.x;
    let l_l = klhg_l.y;
    let h_l = klhg_l.z;
    let g_l = klhg_l.w;
    let o_l = zzon_l.z;
    let n_l = zzon_l.w;

    // Accumulate for bilinear interpolation. The four weights are the bilinear
    // corners s, t, u, v of `pp`.
    var dir_len = vec3<f32>(0.0);
    dir_len = easu_set(dir_len, (1.0 - pp.x) * (1.0 - pp.y), b_l, e_l, f_l, g_l, j_l);
    dir_len = easu_set(dir_len, pp.x * (1.0 - pp.y), c_l, f_l, g_l, h_l, k_l);
    dir_len = easu_set(dir_len, (1.0 - pp.x) * pp.y, f_l, i_l, j_l, k_l, n_l);
    dir_len = easu_set(dir_len, pp.x * pp.y, g_l, j_l, k_l, l_l, o_l);
    var dir = dir_len.xy;
    var len = dir_len.z;

    // Normalize with approximation, and cleanup close to zero.
    let dir2 = dir * dir;
    var dir_r = dir2.x + dir2.y;
    let zero = dir_r < (1.0 / 32768.0);
    dir_r = prx_lo_rsq(dir_r);
    dir_r = select(dir_r, 1.0, zero);
    dir.x = select(dir.x, 1.0, zero);
    dir *= vec2<f32>(dir_r);
    // Transform from {0 to 2} to {0 to 1} range, and shape with square.
    len = len * 0.5;
    len *= len;
    // Stretch kernel {1.0 vert|horz, to sqrt(2.0) on diagonal}.
    let stretch = (dir.x * dir.x + dir.y * dir.y) * prx_lo_rcp(max(abs(dir.x), abs(dir.y)));
    // Anisotropic length after rotation:
    //  x := 1.0 lerp to 'stretch' on edges
    //  y := 1.0 lerp to 2x on edges
    let len2 = vec2<f32>(1.0 + (stretch - 1.0) * len, 1.0 + -0.5 * len);
    // Based on the amount of 'edge', the window shifts from +/-{sqrt(2.0) to
    // slightly beyond 2.0}.
    let lob = 0.5 + ((1.0 / 4.0 - 0.04) - 0.5) * len;
    // Set distance^2 clipping point to the end of the adjustable window.
    let clp = prx_lo_rcp(lob);

    // Accumulation mixed with min/max of the 4 nearest.
    let f_rgb = vec3<f32>(ijfe_r.z, ijfe_g.z, ijfe_b.z);
    let g_rgb = vec3<f32>(klhg_r.w, klhg_g.w, klhg_b.w);
    let j_rgb = vec3<f32>(ijfe_r.y, ijfe_g.y, ijfe_b.y);
    let k_rgb = vec3<f32>(klhg_r.x, klhg_g.x, klhg_b.x);
    let min4 = min(min3v(f_rgb, g_rgb, j_rgb), k_rgb);
    let max4 = max(max3v(f_rgb, g_rgb, j_rgb), k_rgb);

    var acc = vec4<f32>(0.0);
    acc = easu_tap(acc, vec2<f32>(0.0, -1.0) - pp, dir, len2, lob, clp, vec3<f32>(bczz_r.x, bczz_g.x, bczz_b.x)); // b
    acc = easu_tap(acc, vec2<f32>(1.0, -1.0) - pp, dir, len2, lob, clp, vec3<f32>(bczz_r.y, bczz_g.y, bczz_b.y)); // c
    acc = easu_tap(acc, vec2<f32>(-1.0, 1.0) - pp, dir, len2, lob, clp, vec3<f32>(ijfe_r.x, ijfe_g.x, ijfe_b.x)); // i
    acc = easu_tap(acc, vec2<f32>(0.0, 1.0) - pp, dir, len2, lob, clp, j_rgb); // j
    acc = easu_tap(acc, vec2<f32>(0.0, 0.0) - pp, dir, len2, lob, clp, f_rgb); // f
    acc = easu_tap(acc, vec2<f32>(-1.0, 0.0) - pp, dir, len2, lob, clp, vec3<f32>(ijfe_r.w, ijfe_g.w, ijfe_b.w)); // e
    acc = easu_tap(acc, vec2<f32>(1.0, 1.0) - pp, dir, len2, lob, clp, k_rgb); // k
    acc = easu_tap(acc, vec2<f32>(2.0, 1.0) - pp, dir, len2, lob, clp, vec3<f32>(klhg_r.y, klhg_g.y, klhg_b.y)); // l
    acc = easu_tap(acc, vec2<f32>(2.0, 0.0) - pp, dir, len2, lob, clp, vec3<f32>(klhg_r.z, klhg_g.z, klhg_b.z)); // h
    acc = easu_tap(acc, vec2<f32>(1.0, 0.0) - pp, dir, len2, lob, clp, g_rgb); // g
    acc = easu_tap(acc, vec2<f32>(1.0, 2.0) - pp, dir, len2, lob, clp, vec3<f32>(zzon_r.z, zzon_g.z, zzon_b.z)); // o
    acc = easu_tap(acc, vec2<f32>(0.0, 2.0) - pp, dir, len2, lob, clp, vec3<f32>(zzon_r.w, zzon_g.w, zzon_b.w)); // n

    // Normalize and dering.
    let pix = min(max4, max(min4, acc.rgb * vec3<f32>(1.0 / acc.w)));
    return vec4<f32>(pix, 1.0);
}

// This is set at the limit of providing unnatural results for sharpening.
const RCAS_LIMIT: f32 = 0.25 - (1.0 / 16.0);

// FsrRcasF: robust contrast-adaptive sharpening.
//
// Input and output are the same size, and the pass covers its whole target with
// no viewport offset, so `position.xy` indexes texels directly and this reads
// through `textureLoad` rather than a sampler - which is also what upstream's
// `FsrRcasLoadF` does.
@fragment
fn fs_rcas(in: VertexOutput) -> @location(0) vec4<f32> {
    let sp = vec2<i32>(in.position.xy);
    let limit = vec2<i32>(textureDimensions(source)) - vec2<i32>(1);
    // Upstream leaves edge addressing to the resource's own clamp; `textureLoad`
    // has no address mode, so the clamp is explicit here.
    let b = textureLoad(source, clamp(sp + vec2<i32>(0, -1), vec2<i32>(0), limit), 0).rgb;
    let d = textureLoad(source, clamp(sp + vec2<i32>(-1, 0), vec2<i32>(0), limit), 0).rgb;
    let e = textureLoad(source, clamp(sp, vec2<i32>(0), limit), 0).rgb;
    let f = textureLoad(source, clamp(sp + vec2<i32>(1, 0), vec2<i32>(0), limit), 0).rgb;
    let h = textureLoad(source, clamp(sp + vec2<i32>(0, 1), vec2<i32>(0), limit), 0).rgb;

    // Upstream computes the five lumas here and folds them into a noise-
    // detection term `nz` that multiplies `lobe`. That is gated behind
    // FSR_RCAS_DENOISE, which is off by default, so neither the lumas nor `nz`
    // are computed here - upstream's own comment recommends applying film grain
    // after RCAS instead of enabling it.

    // Min and max of the ring.
    let mn4 = min(min3v(b, d, f), h);
    let mx4 = max(max3v(b, d, f), h);
    // Immediate constants for peak range.
    let peak_c = vec2<f32>(1.0, -1.0 * 4.0);
    // Limiters. These need high precision reciprocals.
    let hit_min = min(mn4, e) / (4.0 * mx4);
    let hit_max = (vec3<f32>(peak_c.x) - max(mx4, e)) / (4.0 * mn4 + vec3<f32>(peak_c.y));
    let lobe_rgb = max(-hit_min, hit_max);
    let lobe = max(-RCAS_LIMIT, min(max3(lobe_rgb.r, lobe_rgb.g, lobe_rgb.b), 0.0)) * constants.sharpness;

    // Resolve, which needs the medium precision rcp approximation to avoid
    // visible tonality changes.
    let rcp_l = prx_med_rcp(4.0 * lobe + 1.0);
    let pix = (lobe * b + lobe * d + lobe * h + lobe * f + e) * rcp_l;
    return vec4<f32>(pix, 1.0);
}
