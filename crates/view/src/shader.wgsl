// Draws one texture over a checkerboard.
//
// The checkerboard is not decoration. Wipeout's UI textures are white artwork
// whose shape lives entirely in the alpha channel, so on any flat background
// they are either invisible or indistinguishable from a solid rectangle.

struct Uniforms {
    // Scale applied to the quad to preserve the texture's aspect ratio.
    scale: vec2<f32>,
    // Surface size in pixels, for sizing the checkerboard.
    surface: vec2<f32>,
};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var image: texture_2d<f32>;
@group(0) @binding(2) var image_sampler: sampler;

struct VertexOutput {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) pixel: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> VertexOutput {
    // A full-screen triangle rather than a quad: fewer vertices, no seam
    // along the diagonal, and the hardware clips the excess.
    let uv = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
    let ndc = uv * 2.0 - 1.0;

    var out: VertexOutput;
    out.clip = vec4<f32>(ndc * uniforms.scale, 0.0, 1.0);
    // Flip V: textures are stored top-down, clip space is bottom-up.
    out.uv = vec2<f32>(uv.x, 1.0 - uv.y);
    out.pixel = uv * uniforms.surface;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let cell = floor(in.pixel / 8.0);
    let checker = select(0.18, 0.26, (cell.x + cell.y) % 2.0 < 1.0);
    let background = vec3<f32>(checker);

    let texel = textureSample(image, image_sampler, in.uv);

    // Composite manually rather than using blend state, so the checkerboard
    // stays crisp and the result is what the alpha channel actually says.
    return vec4<f32>(mix(background, texel.rgb, texel.a), 1.0);
}
