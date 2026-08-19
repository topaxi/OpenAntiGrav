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
    let shape = texel.a;
    let rgb = texel.rgb * in.colour.rgb;
    // On the linear float target the flare's gamma-authored colour is decoded
    // so the additive blend and the encode after it round-trip; on a gamma
    // target this is the identity.
    let out_rgb = mix(rgb, pow(rgb, vec3<f32>(2.2)), linear_out);
    return vec4<f32>(out_rgb, shape * in.colour.a);
}
