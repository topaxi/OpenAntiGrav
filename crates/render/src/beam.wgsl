// Draws the LeachBeam's own ribbon - see `beam.rs`'s module doc comment for
// the recovered evidence. Deliberately the smallest shader in this crate:
// the ribbon carries no lighting, no fog and no second texture, only a
// texture sample tinted by the vertex colour the geometry half already
// computed (white, faded by the disconnect linger, zeroed at both ends).

struct Uniforms {
    view_projection: mat4x4<f32>,
    model: mat4x4<f32>,
};

// 1.0 on a linear float scene target, 0.0 on a gamma one - see
// `exhaust.wgsl`'s own copy of this override for why the decode exists.
override linear_out: f32 = 0.0;

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(1) @binding(0) var beam_texture: texture_2d<f32>;
@group(1) @binding(1) var beam_sampler: sampler;

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
    // The geometry half already builds world-space positions - `LeachBeam_BuildStrip`
    // pre-transforms the chain on the CPU and draws with the GE's own matrices
    // reset to identity, which this pipeline mirrors by carrying no model
    // matrix of its own.
    out.clip = uniforms.view_projection * vec4<f32>(in.position, 1.0);
    out.colour = in.colour;
    out.texcoord = in.texcoord;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let texel = textureSample(beam_texture, beam_sampler, in.texcoord);
    let rgb = texel.rgb * in.colour.rgb;
    let decoded = mix(rgb, pow(rgb, vec3<f32>(2.2)), linear_out);
    // The pipeline's blend state does the `* alpha` and the additive `+ dst` -
    // see `Pipeline::new`'s citation of `exhaust::BLEND`. This returns the
    // unpremultiplied colour and the alpha it should be weighted by, the same
    // contract `exhaust.wgsl`'s `fs_main` follows.
    return vec4<f32>(decoded, texel.a * in.colour.a);
}
