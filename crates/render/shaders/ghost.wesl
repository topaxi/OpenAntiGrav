// The ghost ship's three passes - see `ghost.rs`'s module doc comment and
// `docs/ghidra/functions/psp-pulse-usa/ghost.md` for the recovered state each
// entry point below stands for. Unlit throughout: `MeshNode_Ghost_Draw`
// disables lighting at entry for every pass.

import oag_shaders::target::{gamma_to_target, linear_out};

struct Uniforms {
    view_projection: mat4x4<f32>,
    model: mat4x4<f32>,
    prev_mvp: mat4x4<f32>,
    // offset_u, offset_v, glow_ref / 255, the base stamp / 255.
    params: vec4<f32>,
    // scale_u, scale_v, the static alpha reference / 255, unused.
    scale: vec4<f32>,
};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(1) @binding(0) var albedo: texture_2d<f32>;
@group(1) @binding(1) var albedo_sampler: sampler;
@group(2) @binding(0) var static_texture: texture_2d<f32>;
@group(2) @binding(1) var static_sampler: sampler;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(3) texcoord: vec2<f32>,
};

struct VertexOutput {
    @invariant @builtin(position) clip: vec4<f32>,
    @location(0) texcoord: vec2<f32>,
    @location(1) cur_clip: vec4<f32>,
    @location(2) prev_clip: vec4<f32>,
};

struct MrtOutput {
    @location(0) colour: vec4<f32>,
    @location(1) velocity: vec2<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    let world = uniforms.model * vec4<f32>(in.position, 1.0);
    out.clip = uniforms.view_projection * world;
    out.cur_clip = out.clip;
    out.prev_clip = uniforms.prev_mvp * vec4<f32>(in.position, 1.0);
    out.texcoord = in.texcoord;
    return out;
}

// `mesh.wgsl`'s `velocity_of`, the same convention.
fn velocity_of(in: VertexOutput) -> vec2<f32> {
    if in.prev_clip.w <= 0.0 {
        return vec2<f32>(0.0);
    }
    let cur = in.cur_clip.xy / in.cur_clip.w;
    let prev = in.prev_clip.xy / in.prev_clip.w;
    return (cur - prev) * vec2<f32>(0.5, -0.5);
}

// Pass 0: depth, no colour, and the ordinary stamp in the glow mask (the
// write mask keeps only alpha, or nothing on a source whose mask this is not).
@fragment
fn fs_depth(in: VertexOutput) -> MrtOutput {
    var out: MrtOutput;
    out.colour = vec4<f32>(0.0, 0.0, 0.0, uniforms.params.w);
    out.velocity = velocity_of(in);
    return out;
}

// Pass 1: the batch's own texture, unlit, alpha test GREATER 0. The blend
// state is the cross-fade; the write mask keeps the glow mask untouched.
@fragment
fn fs_hull(in: VertexOutput) -> MrtOutput {
    let texel = textureSample(albedo, albedo_sampler, in.texcoord);
    if texel.a <= 0.0 {
        discard;
    }
    let rgb = gamma_to_target(texel.rgb, linear_out);
    var out: MrtOutput;
    out.colour = vec4<f32>(rgb, 1.0);
    out.velocity = vec2<f32>(0.0);
    return out;
}

// Pass 2: the static, projected in screen space - `u = 4 * x_ndc + off_u`,
// `v = 1.8 * y_ndc + off_v` (`ghost.md`, "the static texture and its matrix",
// confidence 60) - and where its alpha clears GREATER 0x80, `glow_ref`
// stamped into the glow mask. No colour: the write mask keeps only alpha.
@fragment
fn fs_static(in: VertexOutput) -> MrtOutput {
    let ndc = in.cur_clip.xy / in.cur_clip.w;
    let uv = ndc * uniforms.scale.xy + uniforms.params.xy;
    let texel = textureSampleLevel(static_texture, static_sampler, uv, 0.0);
    if texel.a <= uniforms.scale.z {
        discard;
    }
    var out: MrtOutput;
    out.colour = vec4<f32>(0.0, 0.0, 0.0, uniforms.params.z);
    out.velocity = vec2<f32>(0.0);
    return out;
}
