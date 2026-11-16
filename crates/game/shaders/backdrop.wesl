// The Fury menu backdrop: a point cloud drawn as camera-facing sprites, its
// trail and glow fed back frame to frame, the smear band and scanlines over the
// sum, and the finished field copied onto the page.
//
// One entry-point pair per pass of docs/ghidra/functions/ps3-hdfury-eu/
// menu-backdrop.md, "The frame, measured". The point pass is `RadioHead2_vp`/
// `_fp`'s arithmetic; the post passes are `FEBackgroundAnimFuryBlend`, `..Wave`
// and `FEBackgroundAnimFury` and `FunkLayer_DrawScanBand`'s two as read back
// live. Every constant comes in through `Points`/`Post`, filled from
// `oag_ui::backdrop::Frame`; nothing is authored here.
//
// The original reads every target decoded and packs every fragment to sRGB
// before the blend unit sees it, so a program's arithmetic is linear and what
// it blends with is stored bytes (`docs/rendering/hd-track-tone.md`, "HD's
// blends combine encoded values"). Here a pass samples a target through an
// sRGB view, computes in linear light and writes `stored(..)` to a plain one.

import oag_shaders::colour::srgb_encode;
import oag_shaders::quad::{ClipUv, covering_triangle};

// What a fragment hands the blend unit: its colour packed, alpha as it is.
fn stored(colour: vec4<f32>) -> vec4<f32> {
    return vec4<f32>(srgb_encode(saturate(colour.rgb)), colour.a);
}

struct Points {
    world_view: mat4x4<f32>,
    proj: mat4x4<f32>,
    // (offset, scale, jitter, 0): t = ((p.z - offset) + rand * jitter) * scale.
    colour_ramp_factors: vec4<f32>,
    // (4, -3, 6, 0): ramp = max(0, 4 * (1 - frac(t)) * 0.4 * d - 3) ^ 6.
    colour_ramp_factors2: vec4<f32>,
    // (-start, -1/strength, 24, 1/factor).
    dof: vec4<f32>,
    // (-start, -1/length, -start-length, exponent).
    fog: vec4<f32>,
    // rgb, and the sprite's half-size in w.
    particle_colour: vec4<f32>,
    // rgb, and the music multiplier in w.
    colour_ramp: vec4<f32>,
    // (a, b, 0, 0): fade = d * a + b.
    depth_fade: vec4<f32>,
}

@group(0) @binding(0) var<uniform> points: Points;
@group(0) @binding(1) var sprite: texture_2d<f32>;
@group(0) @binding(2) var sprite_sampler: sampler;

struct Point {
    @location(0) position: vec3<f32>,
    @location(1) rand: f32,
    @location(2) normal: vec3<f32>,
}

struct PointOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) colour: vec4<f32>,
    @location(1) uv: vec2<f32>,
}

// The renderer's shared four-corner stream, (-1,0) (1,0) (1,1) (-1,1), as two
// triangles; the program turns it into corners on -1..1 and uvs on 0..1.
fn quad_offset(index: u32) -> vec2<f32> {
    switch index {
        case 0u, 3u: { return vec2<f32>(-1.0, 0.0); }
        case 1u: { return vec2<f32>(1.0, 0.0); }
        case 2u, 4u: { return vec2<f32>(1.0, 1.0); }
        default: { return vec2<f32>(-1.0, 1.0); }
    }
}

@vertex
fn vs_points(@builtin(vertex_index) index: u32, point: Point) -> PointOut {
    let offset = quad_offset(index);
    let corner = offset * vec2<f32>(1.0, 2.0) + vec2<f32>(0.0, -1.0);
    let uv = offset * vec2<f32>(0.5, 1.0) + vec2<f32>(0.5, 0.0);
    let r = point.rand * 2.0 - 1.0;
    let p = point.position + point.normal * points.colour_ramp.w;
    var view = points.world_view * vec4<f32>(p, 1.0);
    let crf = points.colour_ramp_factors;
    // The ramp reads the attribute's own z, before the normal displacement.
    let t = ((point.position.z - crf.x) + r * crf.z) * crf.y;
    // Beyond the fog's end the point is put behind the eye, so the clipper
    // culls it - `view.z = 1` is what the program writes.
    if view.z < points.fog.z {
        view.z = 1.0;
    }
    let d = -view.z;
    // Four of the program's instructions carry the saturate flag, so their
    // results are clamped to 0..1 on write: the fade, the distance term, the
    // ramp base and the fog base. The ramp is therefore never more than one.
    let fade = saturate(d * points.depth_fade.x + points.depth_fade.y);
    // `(view.z - (-start)) * (-1/strength)` is `(d - start) / strength`.
    let dof = 1.0 + clamp((view.z - points.dof.x) * points.dof.y, 0.0, points.dof.z);
    let fog_base = saturate((view.z - points.fog.x) * points.fog.y);
    let fog = pow(fog_base, points.fog.w);
    let crf2 = points.colour_ramp_factors2;
    // `MUL_SAT` by the literal `-0.4`: the distance term is `min(0.4 * d, 1)`,
    // one for everything past two and a half units.
    let distance = saturate(0.4 * d);
    let ramp_base = saturate((1.0 - fract(t)) * distance * crf2.x + crf2.y);
    let ramp = pow(ramp_base, crf2.z);
    let size = dof * points.particle_colour.w;
    let placed = view + vec4<f32>(corner * size, 0.0, 0.0);
    var out: PointOut;
    out.clip = points.proj * placed;
    // `f[COL0]` is read clamped to 0..1 by the fragment program, which is
    // where RPCS3's decoder puts the `_saturate` too.
    out.colour = saturate(vec4<f32>(
        points.particle_colour.rgb + ramp * points.colour_ramp.rgb,
        fade * (1.0 - fog),
    ));
    out.uv = uv;
    return out;
}

@fragment
fn fs_points(in: PointOut) -> @location(0) vec4<f32> {
    return stored(in.colour * textureSample(sprite, sprite_sampler, in.uv));
}

// The post passes: one full-screen triangle over whichever target is bound,
// but for the wave's grid and the band's quads.

struct Post {
    // rgb: the trail's survival per frame.
    feedback: vec4<f32>,
    // rgb: the wave's colour scale.
    wave_scale: vec4<f32>,
    // x: the wave's bias, y: its bias factor, z: its pinch, w: its drift.
    wave_bias: vec4<f32>,
    // The screen's tint: the program's `srcScale`.
    tint: vec4<f32>,
    // xy: the wave's pivot in clip units, z: the band's top edge in clip y,
    // w: the band's half height.
    geometry: vec4<f32>,
    // rgb: what every fourth row is multiplied by, w: the band's bend.
    scanline: vec4<f32>,
    // x: what the band's middle is multiplied by.
    band: vec4<f32>,
}

@group(0) @binding(0) var<uniform> post: Post;
@group(0) @binding(1) var source: texture_2d<f32>;
@group(0) @binding(2) var trail: texture_2d<f32>;
@group(0) @binding(3) var post_sampler: sampler;
@group(0) @binding(4) var trail_sampler: sampler;

@vertex
fn vs_post(@builtin(vertex_index) index: u32) -> ClipUv {
    return covering_triangle(index);
}

// One of a quad's two triangles' corners, on 0..1 both ways.
fn cell_corner(index: u32) -> vec2<u32> {
    switch index {
        case 0u, 3u: { return vec2<u32>(0u, 0u); }
        case 1u: { return vec2<u32>(1u, 0u); }
        case 2u, 4u: { return vec2<u32>(1u, 1u); }
        default: { return vec2<u32>(0u, 1u); }
    }
}

// Blend: the fresh particles scaled by the screen's tint over the decayed
// accumulation, so a low tint also shortens what accumulates.
@fragment
fn fs_blend(in: ClipUv) -> @location(0) vec4<f32> {
    let fresh = textureSample(source, post_sampler, in.uv) * post.tint;
    let previous = textureSample(trail, post_sampler, in.uv);
    return stored(min(fresh, vec4<f32>(1.0)) + previous * vec4<f32>(post.feedback.rgb, 0.0));
}

// Glow's copy: the pass's constant blend colour is what makes it a gain.
@fragment
fn fs_copy(in: ClipUv) -> @location(0) vec4<f32> {
    return stored(textureSample(source, post_sampler, in.uv));
}

// The wave's grid, four quads meeting at the pivot. The outer edge samples
// pulled in by the pinch and everything drifts down the screen, so the glow
// spreads away from the pivot; the pivot's own uv is its screen position plus
// half the drift, so nothing but the drift moves there.
@vertex
fn vs_wave(@builtin(vertex_index) index: u32) -> ClipUv {
    let cell = vec2<u32>((index / 6u) % 2u, index / 12u);
    let line = cell + cell_corner(index % 6u);
    let pivot = post.geometry.xy;
    let pinch = post.wave_bias.z;
    let drift = post.wave_bias.w;
    var xs = array<f32, 3>(-1.0, pivot.x, 1.0);
    var ys = array<f32, 3>(1.0, pivot.y, -1.0);
    var us = array<f32, 3>(pinch, (pivot.x + 1.0) * 0.5, 1.0 - pinch);
    var vs = array<f32, 3>(drift, (1.0 - pivot.y + drift) * 0.5, 1.0 + drift);
    var out: ClipUv;
    out.clip = vec4<f32>(xs[line.x], ys[line.y], 0.0, 1.0);
    out.uv = vec2<f32>(us[line.x], vs[line.y]);
    return out;
}

// Wave: the glow dimmed and pulled down, so it dies out rather than settling.
@fragment
fn fs_wave(in: ClipUv) -> @location(0) vec4<f32> {
    let h = textureSample(source, trail_sampler, in.uv).rgb * post.wave_scale.rgb;
    let bias = post.wave_bias.x;
    return stored(vec4<f32>(h + saturate(h * post.wave_bias.y) * bias + bias, 0.0));
}

// Fury: the accumulation plus the glow, stretched so a full byte reads as one.
// The tint is long since applied.
@fragment
fn fs_composite(in: ClipUv) -> @location(0) vec4<f32> {
    let accumulation = textureSample(source, post_sampler, in.uv);
    let glow = textureSample(trail, trail_sampler, in.uv);
    return stored((accumulation + glow) * (257.0 / 256.0) - vec4<f32>(1.0 / 257.0));
}

struct BandOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    // One at the band's outer edges, nothing along its middle.
    @location(1) edge: f32,
}

// Two quads, the band's upper and lower half, meeting along its middle.
@vertex
fn vs_band(@builtin(vertex_index) index: u32) -> BandOut {
    let half_band = index / 6u;
    let corner = cell_corner(index % 6u);
    let height = post.geometry.w;
    let y = post.geometry.z - height * f32(half_band + corner.y);
    let x = f32(corner.x) * 2.0 - 1.0;
    var out: BandOut;
    out.clip = vec4<f32>(x, y, 0.0, 1.0);
    out.uv = vec2<f32>(f32(corner.x), (1.0 - y) * 0.5);
    out.edge = abs(f32(half_band + corner.y) - 1.0);
    return out;
}

// The smear: the picture read sideways by a spike along the band's middle
// and dimmed there. The source is the field as the passes before left it.
@fragment
fn fs_band(in: BandOut) -> @location(0) vec4<f32> {
    let bend = pow(1.0 - in.edge, 10.0) * post.scanline.w;
    let dim = mix(post.band.x, 1.0, in.edge);
    let texel = textureSample(source, post_sampler, in.uv + vec2<f32>(bend, 0.0));
    return stored(vec4<f32>(texel.rgb * dim, texel.a * in.edge));
}

// One row in four, from the top: the other three are left alone.
@fragment
fn fs_scanlines(in: ClipUv) -> @location(0) vec4<f32> {
    if (u32(in.clip.y) % 4u) != 0u {
        discard;
    }
    return stored(vec4<f32>(post.scanline.rgb, 0.0));
}

// The finished field into the bloom chain's float scene target, as light and with no
// offset: the original's ladder reads the field directly, and the half texel the final
// composite's quad carries is the chain's own resolve (`Params::final_half_texel`), so a
// scanline reaches the gate as one dark row and the screen as two half-dark ones
// (menu-backdrop.md, "A race's final composite is the same quad").
@fragment
fn fs_present(in: ClipUv) -> @location(0) vec4<f32> {
    return textureSample(source, trail_sampler, in.uv);
}

// The finished picture onto the page, as the bytes it holds: the page is
// gamma-space, and the target's texture is read through a plain view here.
@fragment
fn fs_page(in: ClipUv) -> @location(0) vec4<f32> {
    return textureSample(source, post_sampler, in.uv);
}
