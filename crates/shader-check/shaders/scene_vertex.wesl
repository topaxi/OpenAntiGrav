// The vertex layout and uniform block every scene-space draw outside the mesh
// pipeline shares: effects, the weapon quads, the blob shadow. Sharing the
// block rather than trimming it per draw is what lets each reuse the Rust
// layout and the size assertion the mesh path already has; a draw that
// pre-transforms its vertices on the CPU simply ignores `model`.

public struct Uniforms {
    view_projection: mat4x4<f32>,
    model: mat4x4<f32>,
};

public struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) colour: vec4<f32>,
    @location(3) texcoord: vec2<f32>,
    @location(4) lit: f32,
};

// A clip position, the vertex colour and one texture coordinate: all a plain
// textured draw interpolates.
public struct VertexOutput {
    @builtin(position) clip: vec4<f32>,
    @location(0) colour: vec4<f32>,
    @location(1) texcoord: vec2<f32>,
};
