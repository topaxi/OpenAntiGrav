// Draws the Cannon's two hand-built quads: the bolt streak and the muzzle
// flash. Structurally a smaller copy of exhaust.wgsl - see that file's own
// header for why the model matrix in the shared uniform block is unused.
// Unlike psys.wgsl, this pipeline does sample a texture, because both
// `Cannon_bolt.mip` and `Cannon_muzzle_flash.mip` are real, located WAD
// entries - see `crates/fx/src/weapon_quads.rs` and
// `docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md`.

struct Uniforms {
    view_projection: mat4x4<f32>,
    model: mat4x4<f32>,
};

// 1.0 when the render target holds linear light - set from the target
// format at pipeline build, see `mesh_render::is_linear_target`.
override linear_out: f32 = 0.0;

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(1) @binding(0) var quad_texture: texture_2d<f32>;
@group(1) @binding(1) var quad_sampler: sampler;

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
    let texel = textureSample(quad_texture, quad_sampler, in.texcoord);
    // The recovered blend, identical to the exhaust flare's own
    // (`sceGuBlendFunc(GU_ADD, GU_SRC_ALPHA, GU_FIX, 0, 0xffffff)` at both
    // `Cannon_BuildBoltList` and `Cannon_BuildMuzzleFlashList`):
    //
    //     result = src.rgb * src.a + dst.rgb * 1.0
    //
    // the pipeline's BlendState does the `* src.a` and the `+ dst`; this
    // returns the unpremultiplied colour and the alpha to weight it by.
    let rgb = texel.rgb * in.colour.rgb;
    // Same linear-target decode `exhaust.wgsl` applies, for the same
    // reason: an additive blend into a linear scene target needs its
    // gamma-authored input decoded first so the add and the later encode
    // round-trip.
    let decoded = mix(rgb, pow(rgb, vec3<f32>(2.2)), linear_out);
    return vec4<f32>(decoded, texel.a * in.colour.a);
}
