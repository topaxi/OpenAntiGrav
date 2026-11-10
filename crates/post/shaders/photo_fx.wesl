// Photo mode's exposure, saturation and depth of field, on a finished frame.
//
// The frame is read with `textureLoad` at the pixel being written: the pass draws into a
// scratch target of the same size and rectangle, and `fs_copy` carries the result home. Taps
// are integer pixels clamped to the drawn rectangle, so nothing here depends on a sampler.
//
// **Works on the frame as the player sees it**: display-encoded values. The exposure and the
// saturation are applied in linear light and encoded again, and only when the colour flag is
// on, so a depth-of-field-only frame is never round-tripped through the transfer curve.

import oag_shaders::colour::{luma_rec709, srgb_decode, srgb_encode};
import oag_shaders::fullscreen::{VertexOutput, fullscreen_triangle};

struct Constants {
    // The drawn rectangle: first and last pixel, inclusive.
    rect: vec4<i32>,
    // The projection's near and far planes, the largest blur radius in pixels, and 1 when
    // depth of field runs.
    depth: vec4<f32>,
    // Near edge, near focus, far focus and far edge, in world units from the eye.
    edges: vec4<f32>,
    // Exposure gain on linear light, saturation, and 1 when the colour grade runs.
    grade: vec4<f32>,
}

@group(0) @binding(0) var scene_tex: texture_2d<f32>;
@group(0) @binding(1) var<uniform> constants: Constants;
@if(msaa) @group(0) @binding(2) var depth_tex: texture_multisampled_2d<f32>;
@if(!msaa) @group(0) @binding(2) var depth_tex: texture_2d<f32>;

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> VertexOutput {
    return fullscreen_triangle(index);
}

const TAPS: i32 = 56;
const GOLDEN_ANGLE: f32 = 2.39996323;

fn inside(p: vec2<i32>) -> vec2<i32> {
    return clamp(p, constants.rect.xy, constants.rect.zw);
}

// Eye distance of the surface at `p`, from the 0..1 depth of a perspective
// projection with the plane distances in `constants.depth`.
fn eye_distance(p: vec2<i32>) -> f32 {
    let z = textureLoad(depth_tex, inside(p), 0).x;
    let near = constants.depth.x;
    let far = constants.depth.y;
    return near * far / max(far - z * (far - near), 1.0e-4);
}

// How far, 0..1, a surface at eye distance `d` is from being in focus: 0 between the near and
// far focus, 1 at or beyond either edge.
fn blur_of(d: f32) -> f32 {
    let e = constants.edges;
    if d < e.y {
        return clamp((e.y - d) / max(e.y - e.x, 1.0e-4), 0.0, 1.0);
    }
    if d > e.z {
        return clamp((d - e.z) / max(e.w - e.z, 1.0e-4), 0.0, 1.0);
    }
    return 0.0;
}

fn radius_of(d: f32) -> f32 {
    return blur_of(d) * constants.depth.z;
}

// A gather over the blur circle. A tap lends its colour to this pixel when its own circle of
// confusion reaches it; a tap behind this pixel is held to this pixel's own radius, so a sharp
// surface keeps a hard edge instead of taking a halo of the blurred background behind it.
fn gather(pixel: vec2<i32>, own_d: f32, own_radius: f32) -> vec4<f32> {
    let centre = textureLoad(scene_tex, pixel, 0);
    var sum = centre;
    var weight = 1.0;
    let reach = constants.depth.z;
    for (var i = 0; i < TAPS; i = i + 1) {
        let t = (f32(i) + 0.5) / f32(TAPS);
        let r = sqrt(t) * reach;
        let a = f32(i) * GOLDEN_ANGLE;
        let offset = vec2<i32>(round(vec2<f32>(cos(a), sin(a)) * r));
        if offset.x == 0 && offset.y == 0 {
            continue;
        }
        let p = pixel + offset;
        let d = eye_distance(p);
        var tap_radius = radius_of(d);
        if d > own_d {
            tap_radius = min(tap_radius, own_radius);
        }
        let w = clamp(tap_radius - length(vec2<f32>(offset)) + 0.5, 0.0, 1.0);
        if w > 0.0 {
            sum = sum + textureLoad(scene_tex, inside(p), 0) * w;
            weight = weight + w;
        }
    }
    // The alpha is the scene's own (the glow mask the bloom reads), not an average.
    return vec4<f32>(sum.rgb / weight, centre.a);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let pixel = vec2<i32>(in.position.xy);
    var colour = textureLoad(scene_tex, pixel, 0);
    if constants.depth.w > 0.5 {
        let d = eye_distance(pixel);
        colour = gather(pixel, d, radius_of(d));
    }
    if constants.grade.w > 0.5 {
        var linear = srgb_decode(colour.rgb) * constants.grade.x;
        let grey = vec3<f32>(luma_rec709(linear));
        linear = max(mix(grey, linear, constants.grade.y), vec3<f32>(0.0));
        colour = vec4<f32>(srgb_encode(clamp(linear, vec3<f32>(0.0), vec3<f32>(1.0))), colour.a);
    }
    return colour;
}

// The result, carried home exactly: `scene_tex` is bound to the scratch target for this pass.
@fragment
fn fs_copy(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureLoad(scene_tex, vec2<i32>(in.position.xy), 0);
}
