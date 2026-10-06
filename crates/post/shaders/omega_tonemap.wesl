// Wipeout: Omega Collection's tone map, read out of the PS4 executable's own
// GCN microcode - docs/ghidra/functions/ps4-omega-eu/tonemap.md. Three
// stages, the original's own: a Rec.601 luma ladder over the linear scene
// (pal_LuminanceFilter_fp and its 2x bilinear downsamples), one coefficient
// step (pal_ToneMapCoefficientsFilter_fp, here a one-thread compute pass
// holding the luminance history the executable keeps on its CPU), and the
// per-channel cubic the MSAA resolve applies (blob 0x0196aa70). `law.rs` is
// the same arithmetic in Rust; its tests hold this file to it.

import oag_shaders::fullscreen::{VertexOutput, fullscreen_triangle};

struct Params {
    luminance_a: f32,
    luminance_b: f32,
    exposure_minimum: f32,
    exposure_maximum: f32,
    // `Exposure response * 0.016666668`: the most LAvg moves in a frame.
    step: f32,
    // `round(Exposure time * 60)` as a byte, at least 1.
    history: u32,
    source_end_a: f32,
    source_end_b: f32,
    // Radians.
    start_angle: f32,
    end_angle: f32,
    output_end: f32,
    brightness: f32,
    uv_scale: vec2<f32>,
    uv_max: vec2<f32>,
    // `0.0100021 * (16/9, 1)`: the composite's offset per unit of the
    // distortion texture, heat-haze.md.
    distortion: vec2<f32>,
    _pad: vec2<f32>,
}

struct State {
    lavg: f32,
    // Frames written so far, modulo the ring.
    cursor: u32,
    primed: u32,
    _pad: u32,
    abcd: vec4<f32>,
    // (t0, t1, exposure, last frame's luminance)
    window: vec4<f32>,
    ring: array<f32, 256>,
}

@group(0) @binding(0) var source_tex: texture_2d<f32>;
@group(0) @binding(1) var source_sampler: sampler;
@group(0) @binding(2) var<uniform> params: Params;

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> VertexOutput {
    return fullscreen_triangle(index);
}

// A coordinate inside the drawn rectangle of the scene - see
// `post::sub_rectangle`.
fn drawn(uv: vec2<f32>) -> vec2<f32> {
    return min(uv * params.uv_scale, params.uv_max);
}

// pal_LuminanceFilter_fp: one sample, Rec.601 luma, no log.
@fragment
fn fs_luma(in: VertexOutput) -> @location(0) vec4<f32> {
    let c = textureSampleLevel(source_tex, source_sampler, drawn(in.uv), 0.0).rgb;
    let l = 0.299 * c.r + 0.587 * c.g + 0.114 * c.b;
    return vec4<f32>(l, l, l, 1.0);
}

// The 2x bilinear copy that halves each level: at a 2x2 block's centre the
// sampler returns the block's mean.
@fragment
fn fs_halve(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSampleLevel(source_tex, source_sampler, in.uv, 0.0);
}

@group(1) @binding(0) var<storage, read_write> state: State;

// (cos + sin) / (cos - sin), with the microcode's +-1e-6 guard.
fn slope_factor(phi: f32) -> f32 {
    let s = sin(phi);
    let c = cos(phi);
    var den = c - s;
    if abs(den) <= 1e-6 {
        den = select(-1e-6, 1e-6, den > 0.0);
    }
    return (c + s) / den;
}

// pal_ToneMapCoefficientsFilter_fp plus the history the executable keeps on
// its CPU (ToneMap_UpdateCoefficients): target = the mean of the last
// `history` frames' luminance; LAvg steps toward it by at most `step`; then
// the Hermite cubic and the exposure.
@compute @workgroup_size(1)
fn cs_coefficients() {
    let l = clamp(textureLoad(source_tex, vec2<i32>(0, 0), 0).x, 0.0, 1000.0);
    let n = max(params.history, 1u);
    if state.primed == 0u {
        // First frame: the history and LAvg start settled on this frame,
        // rather than at the zero the executable clears them to - a single
        // captured frame would otherwise show an exposure no player sees.
        for (var i = 0u; i < 256u; i = i + 1u) {
            state.ring[i] = l;
        }
        state.lavg = l;
        state.cursor = 0u;
        state.primed = 1u;
    }
    state.cursor = (state.cursor + 1u) % 256u;
    state.ring[state.cursor] = l;
    var sum = 0.0;
    for (var k = 0u; k < n; k = k + 1u) {
        sum = sum + state.ring[(state.cursor + 256u - k) % 256u];
    }
    let goal = min(sum / f32(n), 1000.0);
    let previous = min(state.lavg, 1000.0);
    let lavg = previous + clamp(goal - previous, -params.step, params.step);
    state.lavg = lavg;

    let t0 = 0.0;
    let t1 = max(params.source_end_a + params.source_end_b * lavg, t0 + 0.01);
    let s0 = 0.0;
    let s1 = max(params.output_end, s0 + 0.01);
    let h = t1 - t0;
    let k_ = (s1 - s0) / h;
    let m0 = k_ * slope_factor(params.start_angle);
    let m1 = k_ * slope_factor(params.end_angle);
    let c = (3.0 * (s1 - s0) - h * (2.0 * m0 + m1)) / (h * h);
    let d = (2.0 * (s0 - s1) + h * (m0 + m1)) / (h * h * h);
    let exposure = clamp(
        (params.luminance_a + params.luminance_b * lavg) / max(lavg, 1e-4),
        params.exposure_minimum,
        params.exposure_maximum,
    ) * params.brightness;
    state.abcd = vec4<f32>(s0, m0, c, d);
    state.window = vec4<f32>(t0, t1, exposure, l);
}

@group(1) @binding(0) var<storage, read> curve: State;
@group(2) @binding(0) var distortion_tex: texture_2d<f32>;

// The curve resolve, per channel, then the display encode every linear
// target in this crate ends on (ADR-0026's stand-in): the curve's [0, 1] is
// taken as linear light, which is what its HDR twin's 0..40 range implies.
@fragment
fn fs_apply(in: VertexOutput) -> @location(0) vec4<f32> {
    // wo_composite*: `uv' = uv + 0.0100021 * (Aspect * d.x, d.y)` with `d` the
    // distortion texture's `rg`, and the scene sampled at `uv'`. The original
    // texture is SNORM, so a read is already inside [-1, 1]; the half-float
    // stand-in is clamped to match.
    let d = clamp(
        textureSampleLevel(distortion_tex, source_sampler, drawn(in.uv), 0.0).rg,
        vec2<f32>(-1.0),
        vec2<f32>(1.0),
    );
    let c = textureSampleLevel(source_tex, source_sampler, drawn(in.uv + params.distortion * d), 0.0).rgb;
    let w = curve.window;
    let x = clamp(c * w.z, vec3<f32>(w.x), vec3<f32>(w.y));
    let k = curve.abcd;
    let y = k.x + x * (k.y + x * (k.z + k.w * x));
    return vec4<f32>(pow(clamp(y, vec3<f32>(0.0), vec3<f32>(1.0)), vec3<f32>(1.0 / 2.2)), 1.0);
}
