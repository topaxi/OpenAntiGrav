// Draws the loading screen's procedural wave.
//
// A 2D overlay, so there is no camera here at all: `loading.rs` already emits
// quads in normalised screen space and this only maps that space onto clip
// space. That is the one structural difference from exhaust.wgsl and
// sparks.wgsl, which both spend their uniform block on a view-projection.

struct Uniforms {
    // rgb tints the whole overlay, a fades it. Both are caller-side knobs, not
    // recovered terms - see `Pipeline::upload`.
    tint: vec4<f32>,
};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;

@group(1) @binding(0) var strip: texture_2d<f32>;
@group(1) @binding(1) var strip_sampler: sampler;

struct VertexInput {
    // Normalised screen space: origin top left, 0..1 on both axes.
    @location(0) position: vec2<f32>,
    @location(1) texcoord: vec2<f32>,
    @location(2) alpha: f32,
};

struct VertexOutput {
    @builtin(position) clip: vec4<f32>,
    @location(0) texcoord: vec2<f32>,
    @location(1) alpha: f32,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    // Y flips: screen space grows downward, clip space upward.
    out.clip = vec4<f32>(in.position.x * 2.0 - 1.0, 1.0 - in.position.y * 2.0, 0.0, 1.0);
    out.texcoord = in.texcoord;
    out.alpha = in.alpha;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let texel = textureSample(strip, strip_sampler, in.texcoord);
    // The texel's own alpha is deliberately dropped. Every one of the glow
    // strip's 256 palette entries carries alpha 255 - measured on the decoded
    // blob, `Data.wad` entry 68 - so the shape lives entirely in the palette's
    // black-to-cyan colour ramp, and the alpha channel would only multiply by
    // a constant 1.0. Folding it in anyway would make a future texture with a
    // meaningful alpha channel silently change the blend's meaning.
    return vec4<f32>(texel.rgb * uniforms.tint.rgb, in.alpha * uniforms.tint.a);
}
