// One movie frame, from three 8-bit planes.
//
// The cache holds yuv420p because that is what an H.264 decoder produces
// natively, so the transcode does no colour conversion and the cache is 1.5
// bytes per pixel rather than 4. The conversion happens here instead.
//
// BT.601 with limited range, which is what the PSP's encoder produced and what
// ffmpeg reports for these files. It is an assumption, not a measurement: a
// wrong choice shows up as washed-out or crushed colour rather than as a
// scrambled picture.

import oag_shaders::quad::{corner_of, pixels_to_clip};

struct Uniforms {
    viewport: vec2<f32>,
    screen: vec2<f32>,
    atlas: vec2<f32>,
    padding: vec2<f32>,
    // Where the movie sits in screen space: [x, y, width, height]. Not always
    // all of `screen` - a PS2 source's own display aspect pillarboxes rather
    // than filling it, unlike a `.PMF`, which always has.
    video_rect: vec4<f32>,
};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var plane_y: texture_2d<f32>;
@group(0) @binding(2) var plane_u: texture_2d<f32>;
@group(0) @binding(3) var plane_v: texture_2d<f32>;
@group(0) @binding(4) var plane_sampler: sampler;

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> VertexOut {
    let corner = corner_of(index);
    let clip = pixels_to_clip(uniforms.video_rect.xy + corner * uniforms.video_rect.zw, uniforms.screen, uniforms.viewport);
    var out: VertexOut;
    out.position = clip;
    out.uv = corner;
    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let luma = (textureSample(plane_y, plane_sampler, in.uv).r - 0.0625) * 1.164383;
    let cb = (textureSample(plane_u, plane_sampler, in.uv).r - 0.5) * 1.138393;
    let cr = (textureSample(plane_v, plane_sampler, in.uv).r - 0.5) * 1.138393;

    let rgb = vec3<f32>(
        luma + 1.596027 * cr,
        luma - 0.391762 * cb - 0.812968 * cr,
        luma + 2.017232 * cb,
    );
    return vec4<f32>(clamp(rgb, vec3<f32>(0.0), vec3<f32>(1.0)), 1.0);
}
