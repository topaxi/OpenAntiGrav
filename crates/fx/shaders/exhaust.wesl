// Draws the ship's engine flare.
//
// Separate from `mesh` (the mesh pipeline) for one reason that matters: that shader returns
// `alpha = 1.0` unconditionally, because everything it draws is opaque. The
// flare's whole appearance is its alpha.
//
// The vertices arrive already in world space - `exhaust::Exhaust::vertices`
// builds them from the camera basis - so there is no model matrix here and the
// `model` field of the shared uniform block is ignored. The block is shared
// rather than trimmed so this pipeline can reuse the layout and the size
// assertion the mesh path already has.

import oag_shaders::scene_vertex::{Uniforms, VertexInput};
import oag_shaders::target::{linear_out, gamma_to_target};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(1) @binding(0) var flare: texture_2d<f32>;
@group(1) @binding(1) var flare_sampler: sampler;
// The ribbon's second texture. Bound always so the layout is satisfied, and
// read only when `trail_shape` is 1.0 - see the override below.
@group(1) @binding(2) var trail_shape_texture: texture_2d<f32>;

// **1.0 only for a title whose ribbon material names two textures**, which is
// Wipeout HD's and no other's: `hd_enginetrail_red_alphaisnoise.gtf` in slot 0
// and `hd_enginetrail_blue_alphaistrail.gtf` in slot 2, the two named by the
// material the executable's own `Trail_ModelPath` points at.
//
// **What is implemented is the sample HD's program takes and nothing else.**
// That program also lerps the two textures' colours by an authored factor (0.0
// on the disc, so the identity), fades by the angle between two interpolated
// vectors this ribbon does not carry, fades by window depth, and multiplies a
// per-vertex scalar the engine writes. Each is read and none is invented here.
// See `docs/ghidra/functions/ps3-hdfury-eu/engine-flare.md`.
override trail_shape: f32 = 0.0;

struct VertexOutput {
    @builtin(position) clip: vec4<f32>,
    @location(0) colour: vec4<f32>,
    @location(1) texcoord: vec2<f32>,
    // The three interpolants Wipeout HD's own ribbon program carries and the
    // one-texture path ignores: the world normal (its `TC2`), the view vector
    // (`TC1` = eyePositionWorldSpace - position), and the blue-to-red colour
    // mix the engine patches per craft, riding the vertex `lit` slot.
    @location(2) normal: vec3<f32>,
    @location(3) view: vec3<f32>,
    @location(4) red_mix: f32,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip = uniforms.view_projection * vec4<f32>(in.position, 1.0);
    out.colour = in.colour;
    out.texcoord = in.texcoord;
    out.normal = in.normal;
    // `model` is otherwise unused by this pipeline; its fourth column carries
    // the camera's world position so the ribbon's facing fade has an eye to
    // face - see `Pipeline::upload`.
    out.view = uniforms.model[3].xyz - in.position;
    out.red_mix = in.lit;
    return out;
}

fn shade(in: VertexOutput) -> vec4<f32> {
    let texel = textureSample(flare, flare_sampler, in.texcoord);

    // The recovered blend is additive weighted by source alpha:
    //
    //     result = src.rgb * src.a + dst.rgb * 1.0
    //
    // from sceGuBlendFunc(GU_ADD, GU_SRC_ALPHA, GU_FIX, 0, 0xffffff) in
    // ExhaustFlare_BuildDisplayList. The pipeline's BlendState does the
    // `* src.a` and the `+ dst`, so this returns the unpremultiplied colour and
    // the alpha it should be weighted by.
    //
    // The texture is a single-channel glow authored as a full RGBA image, so its
    // own alpha carries the shape and its rgb the colour. Both are used: a flare
    // whose texel is transparent must not brighten the framebuffer.
    // **Wipeout HD's ribbon draws from the *other* texture**, and getting that
    // backwards is what made it nearly vanish: the noise map's alpha averages
    // 17 of 255, so using it as the ribbon's coverage - which is what the
    // one-texture path does, correctly, for a title whose one texture *is* the
    // coverage - leaves almost nothing on screen.
    //
    // HD's own program says which is which. It samples unit 0's **alpha**
    // (`hd_enginetrail_red_alphaisnoise.gtf`), adds it to `u`, and samples unit
    // 1 (`hd_enginetrail_blue_alphaistrail.gtf`) at that displaced coordinate
    // for the colour *and* the output alpha:
    //
    //     @0x06  ADD R2.z, u + TrailSpeed, red.a      <- the noise displaces
    //     @0x12  TEX H0, R2.zwzz unit1                <- blue, all four
    //     @0x1a  MUL H0.w, H0, ...                    <- alpha is blue's
    //     @0x1c  MUL H0.xyz, H0, f[TC0] END           <- colour is blue's
    //
    // See `docs/ghidra/functions/ps3-hdfury-eu/engine-flare.md`.
    //
    // **`TrailSpeed` is already in `in.texcoord.x`**, baked there by
    // `exhaust::hd::Tube`. The original keeps it out of the vertex entirely -
    // its SPU job writes `u_head * (1 - k * 4/54)` and nothing else - and
    // applies it **once on each of the two lookup paths**: the vertex program
    // writes `u + c[210]` into `TC3.z` for the noise sample, and the fragment
    // program's `@0x00` keeps the *raw* `u` in `R0.z` so `@0x02` can add the
    // same constant again before the colour sample. One CPU-side addition
    // therefore reproduces both coordinates exactly, not because the original
    // adds it twice but because it adds it once per path to two coordinates
    // that differ only by the noise. The value is
    // `EngineFlare_PlaceShapes`' wrapping phase accumulator, which the trail
    // manager's constructor binds to the material parameter by address.
    // Read 2026-08-24; docs/ghidra/functions/ps3-hdfury-eu/engine-trail.md.
    let displaced = vec2<f32>(in.texcoord.x + texel.a, in.texcoord.y);
    let authored = textureSample(trail_shape_texture, flare_sampler, displaced);
    // HD's program reads the *red* texture's colour at the same displaced
    // coordinate and lerps the blue one toward it by the per-craft mix the
    // engine patches - 1.0 exactly when the craft wears a Fury skin
    // (`concept1`, `nitro`, `detonator`, `chrome_c1`), which is what makes a
    // Fury craft's trail red where a classic craft's is blue.
    let red = textureSample(flare, flare_sampler, displaced);
    // `mix` rather than a branch, for the reason every other override in this
    // tree uses one: a pipeline built without it takes `texel` unchanged and
    // compiles to the picture it always drew.
    let picture = mix(texel, authored, trail_shape);
    let shape = picture.a;
    // The two fades HD's fragment program applies and no other title's path
    // computes: `saturate(clamp(|dot(n, v)|, 0, 0.15) / 0.15)` kills a fin
    // seen edge-on (the `MIN` at @0x0b carries NV40's `SRC0_ABS` bit, so a
    // fin seen from behind is as visible as from the front), and `saturate(window_z * 0.75)` fades the
    // ribbon out as it nears the camera plane. Both collapse to 1.0 when
    // `trail_shape` is 0.0.
    let facing_dot = dot(normalize(in.normal), normalize(in.view));
    let facing = saturate(clamp(abs(facing_dot), 0.0, 0.15) / 0.15);
    let depth_fade = saturate(in.clip.z * 0.75);
    let hd_fade = mix(1.0, facing * depth_fade, trail_shape);
    let rgb = mix(picture.rgb, red.rgb, in.red_mix * trail_shape) * in.colour.rgb;
    // On the linear float target the flare's gamma-authored colour is decoded
    // so the additive blend and the encode after it round-trip; on a gamma
    // target this is the identity.
    //
    // **The HD tube skips that decode on purpose.** Its own fragment program
    // multiplies the raw texture samples into the output with no transfer
    // function anywhere in its 29 instructions, so the original adds
    // gamma-space values into its linear scene target as they are - decoding
    // them here cut the trail's green and blue channels to a third and was
    // most of why it read too faint. The sampler's sRGB-remap state is the
    // one unread bit; if it turns out to be on, this line is where that
    // lands. See docs/ghidra/functions/ps3-hdfury-eu/engine-trail.md.
    let decoded = gamma_to_target(rgb, linear_out);
    let out_rgb = mix(decoded, rgb, trail_shape);
    return vec4<f32>(out_rgb, shape * in.colour.a * hd_fade);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return shade(in);
}

// The twin for the race's two attachments. Colour as `fs_main`; the velocity
// target takes zero motion weighted by the same coverage (its blend is
// source-alpha over), so the engine's glow and trail are not smeared by the
// motion blur and the temporal upscaler along the track's screen motion behind
// them - which is large exactly when the craft turns hard under a chase camera.
// **Chosen, not measured**: the original has no motion blur to compare against.
struct VelocityOutput {
    @location(0) colour: vec4<f32>,
    @location(1) velocity: vec4<f32>,
};

@fragment
fn fs_main_velocity(in: VertexOutput) -> VelocityOutput {
    let colour = shade(in);
    return VelocityOutput(colour, vec4<f32>(0.0, 0.0, 0.0, colour.a));
}
