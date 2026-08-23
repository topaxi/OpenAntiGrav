// Draws the ship's engine flare.
//
// Separate from mesh.wgsl for one reason that matters: that shader returns
// `alpha = 1.0` unconditionally, because everything it draws is opaque. The
// flare's whole appearance is its alpha.
//
// The vertices arrive already in world space - `exhaust::Exhaust::vertices`
// builds them from the camera basis - so there is no model matrix here and the
// `model` field of the shared uniform block is ignored. The block is shared
// rather than trimmed so this pipeline can reuse the layout and the size
// assertion the mesh path already has.

struct Uniforms {
    view_projection: mat4x4<f32>,
    model: mat4x4<f32>,
};

// 1.0 when the render target holds linear light (Wipeout HD's float scene
// target), 0.0 on every gamma target. Set from the target format at pipeline
// build - `mesh_render::is_linear_target` - so the additive blend below runs
// in the same colour space as everything else drawn into that target.
override linear_out: f32 = 0.0;

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

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) colour: vec4<f32>,
    @location(3) texcoord: vec2<f32>,
    @location(4) lit: f32,
};

struct VertexOutput {
    @builtin(position) clip: vec4<f32>,
    @location(0) colour: vec4<f32>,
    @location(1) texcoord: vec2<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip = uniforms.view_projection * vec4<f32>(in.position, 1.0);
    out.colour = in.colour;
    out.texcoord = in.texcoord;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
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
    // **`TrailSpeed` is not added here.** The material authors 1.0, and adding
    // a whole 1.0 to a coordinate this sampler repeats is the identity - so the
    // engine must patch that slot per frame, and what it patches it with is
    // unread. Adding a scroll of this project's own choosing would be the
    // fitted coefficient this tree keeps out.
    let displaced = vec2<f32>(in.texcoord.x + texel.a, in.texcoord.y);
    let authored = textureSample(trail_shape_texture, flare_sampler, displaced);
    // `mix` rather than a branch, for the reason every other override in this
    // tree uses one: a pipeline built without it takes `texel` unchanged and
    // compiles to the picture it always drew.
    let picture = mix(texel, authored, trail_shape);
    let shape = picture.a;
    let rgb = picture.rgb * in.colour.rgb;
    // On the linear float target the flare's gamma-authored colour is decoded
    // so the additive blend and the encode after it round-trip; on a gamma
    // target this is the identity.
    let out_rgb = mix(rgb, pow(rgb, vec3<f32>(2.2)), linear_out);
    return vec4<f32>(out_rgb, shape * in.colour.a);
}
