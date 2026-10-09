// Wipeout HD's FunkLayerZoom: the history draw and the ring, read out of the
// EBOOT's fragment microcode (FunkLayerBloomDownsample_fp 0x92d780 for the
// history, FunkLayerZoom_fp 0x92b180 for the ring) and the PPU function that
// feeds them (FUN_003b4690) - docs/ghidra/functions/ps3-hdfury-eu/funklayer-zoom.md.

import oag_shaders::fullscreen::{VertexOutput, fullscreen_triangle};

struct Constants {
    // Which sub-rectangle of the quarter-resolution level was drawn, as a UV
    // scale and a clamp half a texel inside its edge - see `post::sub_rectangle`.
    uv_scale: vec2<f32>,
    uv_max: vec2<f32>,
    // Half a texel of the quarter-resolution texture: the ring's own
    // `clampUvs` (1/640, 1/360 on a 320x180 level).
    half_texel: vec2<f32>,
    // The history draw's alpha, min(0.95, 0.25 E).
    weight: f32,
    // The history sample's crop, 0.05 E: uv * (1 - 2c) + c.
    crop: f32,
}

// The quarter-resolution scene for the history draw; the history for the ring.
@group(0) @binding(0) var source_tex: texture_2d<f32>;
@group(0) @binding(1) var source_sampler: sampler;
@group(0) @binding(2) var<uniform> constants: Constants;
// The other ping-pong buffer, the history draw's second input.
@group(0) @binding(3) var history_tex: texture_2d<f32>;

fn drawn(uv: vec2<f32>) -> vec2<f32> {
    return min(uv * constants.uv_scale, constants.uv_max);
}

// What an 8-bit render target does to a value: the ROP clamps on write.
fn surface(texel: vec4<f32>) -> vec4<f32> {
    return clamp(texel, vec4<f32>(0.0), vec4<f32>(1.0));
}

@vertex
fn vs_history(@builtin(vertex_index) index: u32) -> VertexOutput {
    return fullscreen_triangle(index);
}

// The two draws of FunkLayerBloomDownsample_fp, as one: the scene copy into the
// buffer (blend off), then the other buffer over it with SRC_ALPHA,
// ONE_MINUS_SRC_ALPHA at alpha `weight`, its coordinates cropped by `crop`.
// Colour is the plain blend; alpha stays the scene's.
@fragment
fn fs_history(in: VertexOutput) -> @location(0) vec4<f32> {
    let current = surface(textureSample(source_tex, source_sampler, drawn(in.uv)));
    let c = constants.crop;
    let cropped = in.uv * (1.0 - 2.0 * c) + vec2<f32>(c);
    let previous = surface(textureSample(history_tex, source_sampler, drawn(cropped)));
    return vec4<f32>(mix(current.rgb, previous.rgb, constants.weight), current.a);
}

struct RingInput {
    @location(0) position: vec2<f32>,
    @location(1) uv0: vec2<f32>,
    @location(2) uv1: vec2<f32>,
    @location(3) colour: vec4<f32>,
}

struct RingOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv0: vec2<f32>,
    @location(1) uv1: vec2<f32>,
    @location(2) colour: vec4<f32>,
}

// FunkLayerZoom_vp: position and the two texture coordinates pass through
// (the size jitter is already in the positions), colour is passed on.
@vertex
fn vs_ring(in: RingInput) -> RingOutput {
    var out: RingOutput;
    out.position = vec4<f32>(in.position, 0.0, 1.0);
    out.uv0 = in.uv0;
    out.uv1 = in.uv1;
    out.colour = in.colour;
    return out;
}

// FunkLayerZoom_fp (0x92b180), from its microcode:
//
//   tap0 = tex(clamp(TC0.xy))    tap1 = tex(clamp(TC0.zw))
//   rgb  = 1 - col.rgb * (1 - (tap0 + tap1))
//   a    = 1 - (1 - col.a)^2
//
// Both taps are the history; the sum is the original's, doubling a white
// scene before the blend and the saturate of an 8-bit target.
@fragment
fn fs_ring(in: RingOutput) -> @location(0) vec4<f32> {
    let low = constants.half_texel;
    let high = vec2<f32>(1.0) - constants.half_texel;
    let tap0 = textureSample(source_tex, source_sampler, drawn(clamp(in.uv0, low, high))).rgb;
    let tap1 = textureSample(source_tex, source_sampler, drawn(clamp(in.uv1, low, high))).rgb;
    let rgb = vec3<f32>(1.0) - in.colour.rgb * (vec3<f32>(1.0) - (tap0 + tap1));
    let keep = 1.0 - in.colour.a;
    return vec4<f32>(clamp(rgb, vec3<f32>(0.0), vec3<f32>(1.0)), 1.0 - keep * keep);
}
