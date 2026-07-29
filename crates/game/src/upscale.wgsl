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

@group(0) @binding(0) var frame: texture_2d<f32>;
@group(0) @binding(1) var frame_sampler: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(frame, frame_sampler, in.uv);
}
