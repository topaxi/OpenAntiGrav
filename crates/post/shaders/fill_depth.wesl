// Fills a `Depth32Float` attachment from a buffer of per-texel values.
//
// **A test fixture, not part of the port** - nothing in `ffx_fsr3upscaler`
// corresponds to it. It exists because WebGPU has no buffer-to-texture copy
// into a depth format, so the only way to hand FSR 3.1's first pass a
// hand-built depth buffer is to rasterise one.
//
// A fullscreen triangle with `DepthCompare::Always`, writing `frag_depth`
// directly, so every texel ends up holding exactly the value the buffer had.

import oag_shaders::fullscreen::fullscreen_clip;

@group(0) @binding(0) var<storage, read> values: array<f32>;
@group(0) @binding(1) var<uniform> extent: vec4<u32>;

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    return fullscreen_clip(index);
}

@fragment
fn fs_main(@builtin(position) position: vec4<f32>) -> @builtin(frag_depth) f32 {
    let p = vec2<u32>(position.xy);
    return values[p.y * extent.x + p.x];
}
