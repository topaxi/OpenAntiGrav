// FXAA: a lightweight, contrast-adaptive edge blur.
//
// Unlike `fsr1.wgsl` and `smaa.wgsl`, which are line-for-line transliterations
// of a specific MIT-licensed upstream, this is written from the published
// description of the FXAA technique - luma-based edge detection, then a
// directional blend across the detected edge - rather than copied from any
// one author's shader text. See `post::fxaa` for why. It is deliberately a
// simpler, single-quality-tier shape than NVIDIA's own FXAA 3.11: one 5-tap
// edge test, one directional blend, no iterative search along the edge. See
// `post::fxaa` for what that trades away.
//
// **Works in perceptual space** - sRGB-encoded values, the same convention
// `fsr1.wgsl` and `smaa.wgsl` use. See `post`'s module docs for the table of
// who wants what.

struct Constants {
    inv_size: vec2<f32>,
    // The minimum local contrast that counts as an edge at all, and the same
    // threshold as a fraction of the local brightness - a shadow needs less
    // absolute contrast to register than a highlight does. See `fs_main`.
    threshold: f32,
    relative_threshold: f32,
    // The rectangle of `scene_tex` that was actually drawn, as a UV scale and
    // a clamp half a texel inside its edge. Both are exactly 1.0 whenever the
    // drawn rectangle is the whole texture, which is every frame until a
    // resolution controller moves it - see `post::sub_rectangle`.
    uv_scale: vec2<f32>,
    uv_max: vec2<f32>,
}

@group(0) @binding(0) var scene_tex: texture_2d<f32>;
@group(0) @binding(1) var scene_sampler: sampler;
@group(0) @binding(2) var<uniform> constants: Constants;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

// The same fullscreen triangle `fsr1.wgsl` and the blit use: no vertex
// buffer, and no seam down a quad's diagonal.
@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> VertexOutput {
    var out: VertexOutput;
    let uv = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
    out.uv = uv;
    out.position = vec4<f32>(uv * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0), 0.0, 1.0);
    return out;
}

// Rec. 709 luma weights - the same ones `smaa.wgsl`'s luma edge detection
// uses, and the correct ones for sRGB primaries (Rec. 601's are the older
// analogue-TV standard).
fn luma(c: vec3<f32>) -> f32 {
    return dot(c, vec3<f32>(0.2126, 0.7152, 0.0722));
}

// A tap `texel_offset` texels from `uv`, kept inside the drawn rectangle.
//
// The offset is in texels of the *resource*, which is what `inv_size` holds -
// a 5-tap cross has to step one real texel whatever fraction of the texture
// is being drawn into, or the edge test would widen as the render scale fell.
fn sample_at(uv: vec2<f32>, texel_offset: vec2<f32>) -> vec4<f32> {
    let at = uv * constants.uv_scale + texel_offset * constants.inv_size;
    return textureSampleLevel(scene_tex, scene_sampler, min(at, constants.uv_max), 0.0);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let center = sample_at(in.uv, vec2<f32>(0.0, 0.0));
    let north = sample_at(in.uv, vec2<f32>(0.0, -1.0));
    let south = sample_at(in.uv, vec2<f32>(0.0, 1.0));
    let east = sample_at(in.uv, vec2<f32>(1.0, 0.0));
    let west = sample_at(in.uv, vec2<f32>(-1.0, 0.0));

    let luma_m = luma(center.rgb);
    let luma_n = luma(north.rgb);
    let luma_s = luma(south.rgb);
    let luma_e = luma(east.rgb);
    let luma_w = luma(west.rgb);

    let luma_min = min(luma_m, min(min(luma_n, luma_s), min(luma_e, luma_w)));
    let luma_max = max(luma_m, max(max(luma_n, luma_s), max(luma_e, luma_w)));
    let range = luma_max - luma_min;

    // No visible edge here: skip the blend outright, which is both cheaper
    // and what keeps flat areas untouched rather than softened for nothing.
    if range < max(constants.threshold, luma_max * constants.relative_threshold) {
        return center;
    }

    // Blend across whichever axis carries the stronger gradient: a vertical
    // edge (colour changing left to right, so east and west disagree more
    // than north and south do) is softened by averaging east and west *into*
    // the seam, and a horizontal edge the same way with north and south. The
    // weaker axis is left alone - its neighbours already agree, so averaging
    // them would soften flat areas along the edge for nothing.
    let horizontal_gradient = abs(luma_e - luma_w);
    let vertical_gradient = abs(luma_n - luma_s);
    var blended: vec4<f32>;
    if horizontal_gradient > vertical_gradient {
        blended = (east + west) * 0.5;
    } else {
        blended = (north + south) * 0.5;
    }

    // How far into the blend to go: proportional to how far the local
    // contrast sits past the threshold, capped at an even mix. Past that the
    // edge is already soft enough that leaning further toward the neighbours
    // stops smoothing it and starts smearing it.
    let blend_factor = clamp(range / max(luma_max, 0.0001), 0.0, 0.5);
    return mix(center, blended, blend_factor);
}
