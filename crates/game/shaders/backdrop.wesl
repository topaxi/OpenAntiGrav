// The Fury menu backdrop: a point cloud drawn as camera-facing sprites, its
// trail fed back frame to frame, and the two composited over the page.
//
// Four entry-point pairs, one per pass. The point pass is `RadioHead2_vp`/
// `_fp`'s arithmetic as docs/ghidra/functions/ps3-hdfury-eu/menu-backdrop.md
// reads it; the three post passes are `FEBackgroundAnimFuryBlend`, `..Wave`
// and `FEBackgroundAnimFury` as the same page reads their fragment programs.
// Every constant comes in through `Points`/`Post`, filled from
// `oag_ui::backdrop::Frame`; nothing is authored here.

import oag_shaders::quad::{ClipUv, covering_triangle};

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
    return in.colour * textureSample(sprite, sprite_sampler, in.uv);
}

// The post passes: one full-screen triangle over whichever target is bound.

struct Post {
    // rgb: the trail's survival per frame.
    feedback: vec4<f32>,
    // rgb: the cap on the fresh particles.
    source_max: vec4<f32>,
    // rgb: the wave's colour scale.
    wave_scale: vec4<f32>,
    // x: the wave's bias, y: its bias factor.
    wave_bias: vec4<f32>,
    // rgb: the screen's tint, which the composite applies itself.
    tint: vec4<f32>,
}

@group(0) @binding(0) var<uniform> post: Post;
@group(0) @binding(1) var source: texture_2d<f32>;
@group(0) @binding(2) var trail: texture_2d<f32>;
@group(0) @binding(3) var post_sampler: sampler;
// The half-size trail is read back up to the viewport bilinearly, as the
// half targets' linear views are; the full-size particles never are.
@group(0) @binding(4) var trail_sampler: sampler;

@vertex
fn vs_post(@builtin(vertex_index) index: u32) -> ClipUv {
    return covering_triangle(index);
}

// Blend: the fresh particles, capped, over the decayed trail. The program
// scales the fresh sample by `srcScale` before the cap; which of `Render`'s
// binds fills it is unread, so it is dropped here as one.
@fragment
fn fs_blend(in: ClipUv) -> @location(0) vec4<f32> {
    let fresh = textureSample(source, post_sampler, in.uv).rgb;
    let previous = textureSample(trail, trail_sampler, in.uv).rgb;
    return vec4<f32>(min(fresh, post.source_max.rgb) + previous * post.feedback.rgb, 1.0);
}

// Wave: the accumulation dimmed and pulled down, so a trail dies out rather
// than settling.
@fragment
fn fs_wave(in: ClipUv) -> @location(0) vec4<f32> {
    let h = textureSample(source, post_sampler, in.uv).rgb * post.wave_scale.rgb;
    let bias = post.wave_bias.x;
    return vec4<f32>(h + saturate(h * post.wave_bias.y) * bias + bias, 1.0);
}

// Fury: fresh particles plus their trail, stretched so a full byte reads as
// one, then scaled by the screen's tint. The original adds `tint * sum` through
// `BlendFunc(CONSTANT_COLOR, ONE)` and its buffer behaves as an sRGB one: a tint
// of 0.125 reads about 0.4 on screen, not 0.125 (RPCS3, tint edited live;
// menu-backdrop.md, "The tint is applied in linear light"). A tint of 1 is left
// exactly as the byte is, so Main Menu is unchanged and only a dimmer tint is
// lifted: `encode(tint * decode(sum))`. Whether tint 1 is itself encoded cannot
// be told from the ratios measured, so it is not. The curve is the standard
// piecewise sRGB, **chosen, not measured**. The page is gamma-space in this
// build (ADR-0020), so this is done here and the blend just adds; exact where
// the page behind the backdrop is black.
fn srgb_decode(encoded: vec3<f32>) -> vec3<f32> {
    let c = clamp(encoded, vec3<f32>(0.0), vec3<f32>(1.0));
    return select(
        pow((c + 0.055) / 1.055, vec3<f32>(2.4)),
        c / 12.92,
        c <= vec3<f32>(0.04045),
    );
}

fn srgb_encode(linear: vec3<f32>) -> vec3<f32> {
    let c = clamp(linear, vec3<f32>(0.0), vec3<f32>(1.0));
    return select(
        1.055 * pow(c, vec3<f32>(1.0 / 2.4)) - 0.055,
        12.92 * c,
        c <= vec3<f32>(0.0031308),
    );
}

@fragment
fn fs_composite(in: ClipUv) -> @location(0) vec4<f32> {
    let fresh = textureSample(source, post_sampler, in.uv).rgb;
    let trailed = textureSample(trail, trail_sampler, in.uv).rgb;
    let sum = (fresh + trailed) * (257.0 / 256.0) - 1.0 / 257.0;
    return vec4<f32>(srgb_encode(srgb_decode(sum) * post.tint.rgb), 1.0);
}
