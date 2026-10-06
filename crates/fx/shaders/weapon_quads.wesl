// Draws the Cannon's two hand-built quads: the bolt streak and the muzzle
// flash. Structurally a smaller copy of exhaust.wesl - see that file's own
// header for why the model matrix in the shared uniform block is unused.
// Unlike psys.wesl, this pipeline does sample a texture, because both
// `Cannon_bolt.mip` and `Cannon_muzzle_flash.mip` are real, located WAD
// entries - see `crates/fx/src/weapon_quads.rs` and
// `docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md`.

import oag_shaders::scene_vertex::{Uniforms, VertexInput, VertexOutput};
import oag_shaders::target::{linear_out, gamma_to_target};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(1) @binding(0) var quad_texture: texture_2d<f32>;
@group(1) @binding(1) var quad_sampler: sampler;

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
    // Same linear-target decode `exhaust.wesl` applies, for the same
    // reason: an additive blend into a linear scene target needs its
    // gamma-authored input decoded first so the add and the later encode
    // round-trip.
    let decoded = gamma_to_target(rgb, linear_out);
    return vec4<f32>(decoded, texel.a * in.colour.a);
}
