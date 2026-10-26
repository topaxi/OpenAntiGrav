// Wipeout 2048's race post chain, the `wo_*` programs its executable registers
// (PostFx_RegisterPrograms, 0x8103d2ae) and runs in PostFx_RenderChain
// (0x810387b4), read off Vita3K's translation of each program -
// docs/ghidra/functions/vita-2048-eu-v104/exposure.md. Four passes and the
// exposure step, the original's own order: the gate into a quarter-resolution
// 8-bit target (wo_bloom_gate_fp), two diagonal blurs (wo_blur_fp), the
// composite (wo_composite_nocc_fp), then the step the CPU takes over the gate
// target's alpha (Exposure_AdaptFromReadback, 0x8103d148). `law.rs` is the same
// arithmetic in Rust; its tests hold this file to it.

import oag_shaders::fullscreen::{VertexOutput, fullscreen_triangle};

struct Params {
    // The drawn rectangle of the scene target (`post::sub_rectangle_at`).
    scene_offset: vec2<f32>,
    scene_scale: vec2<f32>,
    scene_max: vec2<f32>,
    // One texel of the scene target, in uv.
    scene_texel: vec2<f32>,
    // The drawn rectangle of the quarter targets, which start at their corner.
    quarter_scale: vec2<f32>,
    quarter_max: vec2<f32>,
    // One texel of a quarter target, in uv.
    quarter_texel: vec2<f32>,
    // The drawn rectangle of a quarter target, in pixels, for the readback.
    quarter_pixels: vec2<u32>,
    bloom_factor: f32,
    bloom_gate: f32,
    exposure_scale: f32,
    exposure_max: f32,
    exposure_speed: f32,
    _pad: f32,
    _pad2: vec2<f32>,
}

// One picture's exposure, kept from frame to frame.
struct State {
    exposure: f32,
    // 0 until the first frame has set the exposure straight to its target.
    primed: u32,
    // The last readback, `a`, for a test to read.
    readback: f32,
    _pad: f32,
}

// `luminanceFactor`, seeded at 0x816af030/0x816af040.
const LUMINANCE: vec3<f32> = vec3<f32>(0.3, 0.59, 0.11);

@group(0) @binding(0) var source_tex: texture_2d<f32>;
@group(0) @binding(1) var source_sampler: sampler;
@group(0) @binding(2) var<uniform> params: Params;

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> VertexOutput {
    return fullscreen_triangle(index);
}

// A coordinate inside the drawn rectangle of the scene.
fn scene_uv(uv: vec2<f32>) -> vec2<f32> {
    return min(uv * params.scene_scale + params.scene_offset, params.scene_max);
}

// A gate tap kept inside the drawn rectangle of the scene, half a texel in from
// either edge, so a split view's glow never reads across the cut. At the
// frame's own edges it reads what the sampler's clamp would.
fn scene_tap(uv: vec2<f32>) -> vec2<f32> {
    return clamp(uv, params.scene_offset + params.scene_texel * 0.5, params.scene_max);
}

// A coordinate inside the drawn rectangle of a quarter target.
fn quarter_uv(uv: vec2<f32>) -> vec2<f32> {
    return min(uv * params.quarter_scale, params.quarter_max);
}

@group(1) @binding(0) var<storage, read> exposed: State;

// wo_bloom_gate_fp over the four taps wo_bloom_gate_vp places one scene texel
// out on each diagonal (`offset0..3` = (+-1/w, +-1/h)): the taps' summed colour
// `s`, their mean luminance `Lg`, and
//   rgb = (E * clamp(Lg - Gate, 0, 1))^2 * s,   alpha = Lg * 0.25
// (`exposure = half2(E, Gate)`; the 0.25 is the program's literal 0x3400).
@fragment
fn fs_gate(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = scene_uv(in.uv);
    let d = params.scene_texel;
    let s = textureSampleLevel(source_tex, source_sampler, scene_tap(uv + vec2<f32>(-d.x, -d.y)), 0.0).rgb
        + textureSampleLevel(source_tex, source_sampler, scene_tap(uv + vec2<f32>(d.x, -d.y)), 0.0).rgb
        + textureSampleLevel(source_tex, source_sampler, scene_tap(uv + vec2<f32>(-d.x, d.y)), 0.0).rgb
        + textureSampleLevel(source_tex, source_sampler, scene_tap(uv + d), 0.0).rgb;
    let lg = dot(s, LUMINANCE * 0.25);
    let k = exposed.exposure * clamp(lg - params.bloom_gate, 0.0, 1.0);
    return vec4<f32>(k * k * s, lg * 0.25);
}

// wo_blur_fp over wo_blur_vp's six taps at +-1, +-3 and +-5 `offset`s, where
// `offset` is half a quarter-target texel along one diagonal: weights 1.0,
// 0.7998 and 0.5 a pair (the program's halves 0x3c00, 0x3a66, 0x3800), the sum
// times 0.21704 (0x32f2, 1 / 4.6), alpha 0.
fn blur(uv: vec2<f32>, direction: vec2<f32>) -> vec4<f32> {
    let step = direction * params.quarter_texel * 0.5;
    let at = quarter_uv(uv);
    var sum = vec3<f32>(0.0);
    let weights = array<f32, 3>(1.0, 0.7998046875, 0.5);
    for (var k = 0u; k < 3u; k = k + 1u) {
        let reach = step * f32(2u * k + 1u);
        let pair = textureSampleLevel(source_tex, source_sampler, min(at - reach, params.quarter_max), 0.0).rgb
            + textureSampleLevel(source_tex, source_sampler, min(at + reach, params.quarter_max), 0.0).rgb;
        sum = sum + pair * weights[k];
    }
    return vec4<f32>(sum * 0.217041015625, 0.0);
}

// The first blur: `offset = (0.5/w, 0.5/h)`.
@fragment
fn fs_blur_first(in: VertexOutput) -> @location(0) vec4<f32> {
    return blur(in.uv, vec2<f32>(1.0, 1.0));
}

// The second, over the first's output: `offset = (-0.5/w, 0.5/h)`.
@fragment
fn fs_blur_second(in: VertexOutput) -> @location(0) vec4<f32> {
    return blur(in.uv, vec2<f32>(-1.0, 1.0));
}

@group(2) @binding(0) var bloom_tex: texture_2d<f32>;

// wo_composite_nocc_fp:
//   e = E * main,  L = dot(e, LUMINANCE),
//   out = sqrt(e * L / (1 + L) + BloomFactor * bloom) * v + screenTint
// with `c = alpha.rgb + (1 - alpha.a) * main` read as `main` (nothing here
// draws into the original's separate `alphaTex`), the vignette `v` at 1 and the
// tint at 0: both are unread (exposure.md, "Open").
@fragment
fn fs_composite(in: VertexOutput) -> @location(0) vec4<f32> {
    let main = textureSampleLevel(source_tex, source_sampler, scene_uv(in.uv), 0.0).rgb;
    let bloom = textureSampleLevel(bloom_tex, source_sampler, quarter_uv(in.uv), 0.0).rgb;
    let e = exposed.exposure * max(main, vec3<f32>(0.0));
    let l = dot(e, LUMINANCE);
    return vec4<f32>(sqrt(e * l / (1.0 + l) + params.bloom_factor * bloom), 1.0);
}

@group(1) @binding(0) var<storage, read_write> state: State;

var<workgroup> partial: array<vec2<f32>, 64>;

// The mean of every fourth pixel of every fourth row of the gate target's alpha
// byte, times 1/64 (0x3c800000), as Exposure_AdaptFromReadback reads it. The
// 64 invocations each take every 64th sampled row and then add up.
fn readback(lane: u32) -> f32 {
    var sum = 0.0;
    var count = 0.0;
    for (var y = lane * 4u; y < params.quarter_pixels.y; y = y + 256u) {
        for (var x = 0u; x < params.quarter_pixels.x; x = x + 4u) {
            sum = sum + round(textureLoad(source_tex, vec2<u32>(x, y), 0).a * 255.0);
            count = count + 1.0;
        }
    }
    partial[lane] = vec2<f32>(sum, count);
    workgroupBarrier();
    var total = vec2<f32>(0.0);
    for (var k = 0u; k < 64u; k = k + 1u) {
        total = total + partial[k];
    }
    return total.x / max(total.y, 1.0) * (1.0 / 64.0);
}

// Exposure_AdaptFromReadback: above a readback of 0.01,
// `E += (ExposureScale / a - E) * ExposureSpeed`, then `E = min(E, ExposureMax)`.
// Once a frame, not scaled by the frame's length.
@compute @workgroup_size(64)
fn cs_adapt(@builtin(local_invocation_index) lane: u32) {
    let a = readback(lane);
    if lane != 0u {
        return;
    }
    state.readback = a;
    if a > 0.01 {
        let e = state.exposure + (params.exposure_scale / a - state.exposure) * params.exposure_speed;
        state.exposure = min(e, params.exposure_max);
    }
}

// The first frame's exposure, set straight to the step's fixed point
// `min(ExposureScale / a, ExposureMax)` rather than walked there from the reset
// value: a single captured frame would otherwise show an exposure no player
// sees once the race is a second old. Chosen, not measured.
@compute @workgroup_size(64)
fn cs_prime(@builtin(local_invocation_index) lane: u32) {
    let a = readback(lane);
    if lane != 0u {
        return;
    }
    state.readback = a;
    if a > 0.01 {
        state.exposure = min(params.exposure_scale / a, params.exposure_max);
    }
    state.primed = 1u;
}
