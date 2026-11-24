// Photo mode's exposure, saturation and depth of field, on a finished frame.
//
// The frame is read with `textureLoad` at the pixel being written: the pass draws into a
// scratch target of the same size and rectangle, and `fs_copy` carries the result home. Taps
// are integer pixels clamped to the drawn rectangle, so nothing here depends on a sampler.
//
// Depth of field runs in three steps. `fs_tile` writes each 8x8 tile's widest blur radius,
// `fs_dilate` widens that to every tile a blur could reach from, and `fs_main` gathers over
// that reach: dense where the blur nearby is small, out to the widest only where something
// that blurred is close.
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
// `fs_main`: the widest radius that can reach each tile, from `fs_dilate`.
@group(0) @binding(3) var reach_tex: texture_2d<f32>;
// `fs_dilate`: each tile's own widest radius, from `fs_tile`.
@group(0) @binding(4) var tile_tex: texture_2d<f32>;

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> VertexOutput {
    return fullscreen_triangle(index);
}

// Quads of 2x2 pixels a gather reads, on a spiral: a quad holds both parities of each axis,
// so one-pixel detail averages out under a blur instead of aliasing into stripes.
const QUADS: i32 = 32;
const GOLDEN_ANGLE: f32 = 2.39996323;
// Pixels a side of a tile of `fs_tile` and `fs_dilate`; `photo_fx.rs` sizes their targets.
const TILE: i32 = 8;

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

// Corner of quad `i` of `QUADS` spread over a disc of `radius` pixels, as an offset from the
// pixel: the spiral point rounded to the nearest half pixel, so the quad sits around it.
fn quad(i: i32, radius: f32) -> vec2<i32> {
    let t = (f32(i) + 0.5) / f32(QUADS);
    let a = f32(i) * GOLDEN_ANGLE;
    let p = vec2<f32>(cos(a), sin(a)) * sqrt(t) * radius;
    return vec2<i32>(round(p - vec2<f32>(0.5)));
}

// Depth of field at `pixel` as two layers.
//
// - **This surface and what lies behind it**: the average over the pixel's own circle of
//   confusion of every pixel at or behind it. A sharp pixel reads itself alone, so it keeps a
//   hard edge and takes no halo of a blurred background.
// - **What lies in front**: nearer pixels whose own circle reaches this one. Each stands for
//   its share of the gather's area, over the area its blur spreads it across, which sums to
//   the part of this pixel it covers. That layer is laid over the first by that coverage, so
//   a blurred near edge thins out over what is behind it instead of covering it.
fn gather(pixel: vec2<i32>, own_d: f32, own_radius: f32) -> vec4<f32> {
    let centre = textureLoad(scene_tex, pixel, 0);
    let reach = max(own_radius, textureLoad(reach_tex, pixel / TILE, 0).x);
    if reach < 0.5 {
        return centre;
    }
    var rest = centre.rgb;
    var rest_weight = 1.0;
    if own_radius >= 0.5 {
        for (var i = 0; i < QUADS; i = i + 1) {
            let corner = quad(i, own_radius);
            for (var k = 0; k < 4; k = k + 1) {
                let offset = corner + vec2<i32>(k & 1, k >> 1u);
                let p = pixel + offset;
                if eye_distance(p) >= own_d {
                    let w = clamp(own_radius - length(vec2<f32>(offset)) + 0.5, 0.0, 1.0);
                    rest = rest + textureLoad(scene_tex, inside(p), 0).rgb * w;
                    rest_weight = rest_weight + w;
                }
            }
        }
    }
    // The area one pixel read stands for, over pi: the quads spread the spiral by half a pixel.
    let share = (reach + 0.5) * (reach + 0.5) / f32(4 * QUADS);
    var front = vec3<f32>(0.0);
    var front_weight = 0.0;
    var cover = 0.0;
    for (var i = 0; i < QUADS; i = i + 1) {
        let corner = quad(i, reach);
        for (var k = 0; k < 4; k = k + 1) {
            let offset = corner + vec2<i32>(k & 1, k >> 1u);
            let p = pixel + offset;
            let d = eye_distance(p);
            if d < own_d {
                let r = radius_of(d);
                let w = clamp(r - length(vec2<f32>(offset)) + 0.5, 0.0, 1.0);
                if w > 0.0 {
                    front = front + textureLoad(scene_tex, inside(p), 0).rgb * w;
                    front_weight = front_weight + w;
                    cover = cover + w * share / max(r * r, 0.25);
                }
            }
        }
    }
    var colour = rest / rest_weight;
    if front_weight > 0.0 {
        colour = mix(colour, front / front_weight, clamp(cover, 0.0, 1.0));
    }
    // The alpha is the scene's own (the glow mask the bloom reads), not an average.
    return vec4<f32>(colour, centre.a);
}

// The first tile and the last of the drawn rectangle.
fn tiles() -> vec4<i32> {
    return constants.rect / TILE;
}

// Each tile's widest blur radius, in pixels.
@fragment
fn fs_tile(in: VertexOutput) -> @location(0) vec4<f32> {
    let origin = vec2<i32>(in.position.xy) * TILE;
    var widest = 0.0;
    for (var y = 0; y < TILE; y = y + 1) {
        for (var x = 0; x < TILE; x = x + 1) {
            widest = max(widest, radius_of(eye_distance(origin + vec2<i32>(x, y))));
        }
    }
    return vec4<f32>(widest, 0.0, 0.0, 1.0);
}

// The widest radius among the tiles whose blur can reach some pixel of this one.
@fragment
fn fs_dilate(in: VertexOutput) -> @location(0) vec4<f32> {
    let tile = vec2<i32>(in.position.xy);
    let span = i32(ceil(constants.depth.z / f32(TILE))) + 1;
    let bounds = tiles();
    var reach = 0.0;
    for (var dy = -span; dy <= span; dy = dy + 1) {
        for (var dx = -span; dx <= span; dx = dx + 1) {
            let other = tile + vec2<i32>(dx, dy);
            if any(other != clamp(other, bounds.xy, bounds.zw)) {
                continue;
            }
            let r = textureLoad(tile_tex, other, 0).x;
            // The nearest two pixels of the two tiles are this far apart.
            let gap = vec2<f32>(max(abs(vec2<i32>(dx, dy)) - vec2<i32>(1), vec2<i32>(0)) * TILE);
            if r > reach && length(gap) <= r {
                reach = r;
            }
        }
    }
    return vec4<f32>(reach, 0.0, 0.0, 1.0);
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
