// The fullscreen triangle every post pass draws: three vertices, no vertex
// buffer, and no seam down a quad's diagonal. `(-1,-1), (3,-1), (-1,3)` in clip
// space, whose uv is `(0,0), (2,0), (0,2)`; the hardware clips the excess.

public struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

public fn fullscreen_uv(index: u32) -> vec2<f32> {
    return vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
}

// V runs down the picture, clip-space Y runs up, hence the flip.
public fn fullscreen_clip(index: u32) -> vec4<f32> {
    let uv = fullscreen_uv(index);
    return vec4<f32>(uv * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0), 0.0, 1.0);
}

public fn fullscreen_triangle(index: u32) -> VertexOutput {
    var out: VertexOutput;
    out.uv = fullscreen_uv(index);
    out.position = fullscreen_clip(index);
    return out;
}
