// Omega's bloom chain below its bright pass, read out of the PS4 executable's GCN
// microcode: docs/ghidra/functions/ps4-omega-eu/tonemap.md, "The bloom chain". Each level
// is blurred along one axis into a second target and back along the other; a level after
// the first takes its horizontal pass from the level above, halving as it goes; then the
// levels add back up, coarse to fine. `law.rs` holds the same numbers.

import oag_shaders::fullscreen::{VertexOutput, fullscreen_triangle};

struct Step {
    // The share of the source texture this view covers.
    uv_scale: vec2<f32>,
    // The last coordinate of that share: the taps of a split view stop at its edge.
    uv_max: vec2<f32>,
    // One source texel along the pass's axis.
    texel: vec2<f32>,
    // The level's `Blend factors` entry, for the pass that adds it up.
    scale: f32,
    _pad: f32,
}

@group(0) @binding(0) var source_tex: texture_2d<f32>;
@group(0) @binding(1) var source_sampler: sampler;
@group(0) @binding(2) var<uniform> step: Step;

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> VertexOutput {
    return fullscreen_triangle(index);
}

fn tap(uv: vec2<f32>, distance: f32) -> vec4<f32> {
    return textureSampleLevel(source_tex, source_sampler, min(uv + distance * step.texel, step.uv_max), 0.0);
}

// `Offsets`, `Weights`: `Shdr` 0x961894 and its vertical twin 0x961ea4. Seven bilinear
// taps, both axes the same.
@fragment
fn fs_blur(in: VertexOutput) -> @location(0) vec4<f32> {
    const OFFSETS = array<f32, 4>(0.0, 1.3846154, 3.2307692, 5.0769229);
    const WEIGHTS = array<f32, 4>(0.22558594, 0.31420898, 0.06982422, 0.003173828);
    let uv = in.uv * step.uv_scale;
    var sum = tap(uv, 0.0) * WEIGHTS[0];
    for (var i = 1; i < 4; i = i + 1) {
        sum = sum + (tap(uv, OFFSETS[i]) + tap(uv, -OFFSETS[i])) * WEIGHTS[i];
    }
    return sum;
}

// `aWeights`: `Shdr` 0x962534. Eleven taps a whole source texel apart, the pass that
// halves a level.
@fragment
fn fs_down(in: VertexOutput) -> @location(0) vec4<f32> {
    const WEIGHTS = array<f32, 6>(0.22569615, 0.19345383, 0.12090865, 0.05373717, 0.01612115, 0.00293112);
    let uv = in.uv * step.uv_scale;
    var sum = tap(uv, 0.0) * WEIGHTS[0];
    for (var i = 1; i < 6; i = i + 1) {
        sum = sum + (tap(uv, f32(i)) + tap(uv, -f32(i))) * WEIGHTS[i];
    }
    return sum;
}

// `Scale`: `Shdr` 0x95fa14, drawn with blend state one + one onto the level above.
@fragment
fn fs_add(in: VertexOutput) -> @location(0) vec4<f32> {
    return tap(in.uv * step.uv_scale, 0.0) * step.scale;
}
