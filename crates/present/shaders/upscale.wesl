// Draws the offscreen frame onto the surface.
//
// Three vertices and no vertex buffer: the triangle is generated from the
// index, and it is bigger than the screen so the clipped part costs nothing
// and the seam a two-triangle quad would have down its diagonal does not
// exist.

import oag_shaders::fullscreen::{VertexOutput, fullscreen_triangle};

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> VertexOutput {
    return fullscreen_triangle(index);
}

// Brightness and the gamma *exponent*, which is the reciprocal of the gamma the
// setting names - taken once per frame on the CPU rather than once per pixel
// here. See `display::Gamma::exponent`.
struct Grade {
    brightness: f32,
    exponent: f32,
    // 1.0 when the bound source holds sRGB-encoded values this shader has to
    // decode itself, 0.0 when the sampler already did it. See the Rust mirror.
    decode: f32,
    // Padding to sixteen bytes, which is the minimum size of a uniform buffer
    // binding and what the layout below is validated against.
    _pad: f32,
    // How much of the bound source to read, and how far into it the sampler may
    // be asked to go. Both `1.0` whenever the source was drawn in full, which
    // is every frame until a dynamic-resolution controller exists - see the
    // Rust mirror's `Source` and ADR-0037.
    uv_scale: vec2<f32>,
    uv_max: vec2<f32>,
}

@group(0) @binding(0) var frame: texture_2d<f32>;
@group(0) @binding(1) var frame_sampler: sampler;
@group(0) @binding(2) var<uniform> grade: Grade;

// The sRGB EOTF, exactly - the piecewise curve, not a `pow(c, 2.2)` standing in
// for it. The values being decoded here were encoded by the hardware using the
// exact curve on the way into the offscreen target, so anything else would not
// round-trip and would tint the darks, which is where the two curves differ
// most and where a racing game spends much of its frame.
fn srgb_decode(c: vec3<f32>) -> vec3<f32> {
    let v = clamp(c, vec3<f32>(0.0), vec3<f32>(1.0));
    let lo = v / 12.92;
    let hi = pow((v + vec3<f32>(0.055)) / 1.055, vec3<f32>(2.4));
    return select(hi, lo, v <= vec3<f32>(0.04045));
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // The drawn rectangle, not the whole texture. At `uv_scale == 1` and
    // `uv_max == 1` this is the identity over the `0..1` the triangle covers -
    // the sampler's own clamp-to-edge already behaved this way - so a frame
    // drawn in full comes out byte-for-byte as it did before the source
    // rectangle existed. Below full, `uv_max` keeps the linear tap half a texel
    // inside the edge, off whatever a larger earlier frame left outside it.
    let uv = min(in.uv * grade.uv_scale, grade.uv_max);
    let frame_color = textureSample(frame, frame_sampler, uv);
    // An upscaler hands over perceptual values through a non-sRGB view, because
    // it works in that space throughout; the offscreen target arrives through an
    // sRGB view the sampler has already decoded. Everything below wants linear,
    // so the two are brought together here rather than in two shader variants.
    let linear = select(frame_color.rgb, srgb_decode(frame_color.rgb), grade.decode > 0.5);
    // Gamma first, then brightness. The other order raises the value the curve
    // is applied to, so turning brightness up would also flatten the curve and
    // the two rows would stop being independent.
    //
    // Clamped before `pow`, which is undefined for a negative base. Nothing
    // this project draws produces one, but an extended-range surface format
    // could, and a NaN here would be a black frame with no message.
    let graded = pow(clamp(linear, vec3<f32>(0.0), vec3<f32>(1.0)), vec3<f32>(grade.exponent));
    return vec4<f32>(graded * grade.brightness, frame_color.a);
}
