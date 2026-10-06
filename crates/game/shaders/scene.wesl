// The HD-style menu backdrop's two filters over a scene drawn on white:
// `FEBackgroundAnim_fp`'s edge drawing, and a separable blur.
//
// The edge pass is the program's arithmetic as
// docs/ghidra/functions/ps3-hdfury-eu/menu-backdrop-scene.md reads it. The blur
// is the engine's global blur pass in intent only: its kernel is unread, so the
// weights here are chosen, not measured.

import oag_shaders::quad::{ClipUv, covering_triangle};

struct Params {
    // Edge: (dx, dy, -dx/2, -dy/2), the tap spacing in uv. Blur: (0, 0, sigma,
    // taps each side), both in taps.
    taps: vec4<f32>,
    // Edge: (edge_level, 1 - fill_level, fill_level, 0). Blur: (dx, dy) of one
    // tap step in uv along the pass's axis.
    levels: vec4<f32>,
}

@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var source: texture_2d<f32>;
@group(0) @binding(2) var source_sampler: sampler;

@vertex
fn vs_quad(@builtin(vertex_index) index: u32) -> ClipUv {
    return covering_triangle(index);
}

fn tap(uv: vec2<f32>) -> vec3<f32> {
    return textureSample(source, source_sampler, uv).rgb;
}

// A Roberts cross over the scene's colours, four taps centred on the pixel.
// `e` is how much of an edge there is, `a` is one where the scene left the
// white alone; the grey is one minus the edge darkening minus the fill
// darkening on what is not an edge.
@fragment
fn fs_edge(in: ClipUv) -> @location(0) vec4<f32> {
    let base = in.uv + params.taps.zw;
    let t0 = tap(base);
    let t1 = tap(base + vec2<f32>(params.taps.x, 0.0));
    let t2 = tap(base + params.taps.xy);
    let t3 = tap(base + vec2<f32>(0.0, params.taps.y));
    let d = abs(t0 - t2) + abs(t1 - t3);
    let e = min(6.0 * (d.x + d.y + d.z), 1.0);
    let a = floor(0.340088 * (t0.x + t0.y + t0.z));
    let grey = 1.0 - e * params.levels.x - (1.0 - e) * (1.0 - a) * params.levels.z;
    return vec4<f32>(vec3<f32>(grey), 1.0);
}

// One axis of a gaussian: `levels.xy` is the step between taps in uv, `taps.z`
// the sigma in taps and `taps.w` how many taps each side.
@fragment
fn fs_blur(in: ClipUv) -> @location(0) vec4<f32> {
    var sum = tap(in.uv);
    var weight = 1.0;
    let reach = i32(params.taps.w);
    for (var i = 1; i <= reach; i = i + 1) {
        let offset = f32(i);
        let w = exp(-0.5 * offset * offset / (params.taps.z * params.taps.z));
        let step = vec2<f32>(params.levels.x, params.levels.y) * offset;
        sum = sum + (tap(in.uv + step) + tap(in.uv - step)) * w;
        weight = weight + 2.0 * w;
    }
    return vec4<f32>(sum / weight, 1.0);
}

// The page gets the result as it is: opaque, where the original's final copy
// (`FEBackgroundAnimCopy_fp`) is a plain texture read.
@fragment
fn fs_copy(in: ClipUv) -> @location(0) vec4<f32> {
    return vec4<f32>(tap(in.uv), 1.0);
}
