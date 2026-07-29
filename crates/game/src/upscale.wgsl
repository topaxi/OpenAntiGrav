// Draws the offscreen frame onto the surface.
//
// Three vertices and no vertex buffer: the triangle is generated from the
// index, and it is bigger than the screen so the clipped part costs nothing
// and the seam a two-triangle quad would have down its diagonal does not
// exist.

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> VertexOutput {
    var out: VertexOutput;
    // (-1,-1), (3,-1), (-1,3) in clip space, whose uv is (0,0), (2,0), (0,2).
    let uv = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
    out.uv = uv;
    out.position = vec4<f32>(uv * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0), 0.0, 1.0);
    return out;
}

// Brightness and the gamma *exponent*, which is the reciprocal of the gamma the
// setting names - taken once per frame on the CPU rather than once per pixel
// here. See `display::Gamma::exponent`.
struct Grade {
    brightness: f32,
    exponent: f32,
    // Padding to sixteen bytes, which is the minimum size of a uniform buffer
    // binding and what the layout below is validated against.
    _pad: vec2<f32>,
}

@group(0) @binding(0) var frame: texture_2d<f32>;
@group(0) @binding(1) var frame_sampler: sampler;
@group(0) @binding(2) var<uniform> grade: Grade;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let frame_color = textureSample(frame, frame_sampler, in.uv);
    // Gamma first, then brightness. The other order raises the value the curve
    // is applied to, so turning brightness up would also flatten the curve and
    // the two rows would stop being independent.
    //
    // Clamped before `pow`, which is undefined for a negative base. Nothing
    // this project draws produces one, but an extended-range surface format
    // could, and a NaN here would be a black frame with no message.
    let graded = pow(clamp(frame_color.rgb, vec3<f32>(0.0), vec3<f32>(1.0)), vec3<f32>(grade.exponent));
    return vec4<f32>(graded * grade.brightness, frame_color.a);
}
