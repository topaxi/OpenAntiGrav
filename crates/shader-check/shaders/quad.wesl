// The two ways the front end puts a quad or a triangle on the screen.

public struct ClipUv {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

// One triangle covering the viewport, uv `(0,0)` top left: `(-1,-1)`, `(3,-1)`,
// `(-1,3)` in clip space. Unlike `fullscreen::fullscreen_triangle` this one
// derives uv from the clip position, and it winds the other way.
public fn covering_triangle(index: u32) -> ClipUv {
    let x = f32(i32(index & 1u) * 4 - 1);
    let y = f32(i32(index >> 1u) * 4 - 1);
    var out: ClipUv;
    out.clip = vec4<f32>(x, y, 0.0, 1.0);
    out.uv = vec2<f32>(x * 0.5 + 0.5, 0.5 - y * 0.5);
    return out;
}

// The six corners of a unit square as two triangles.
public fn corner_of(index: u32) -> vec2<f32> {
    var corners = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(1.0, 1.0),
    );
    return corners[index];
}

// A point in the virtual `screen` (origin top left, y down) as clip space,
// scaled by `viewport` to letterbox the screen into a window of any shape.
public fn pixels_to_clip(pixels: vec2<f32>, screen: vec2<f32>, viewport: vec2<f32>) -> vec4<f32> {
    let normalised = pixels / screen;
    let clip = vec2<f32>(normalised.x * 2.0 - 1.0, 1.0 - normalised.y * 2.0);
    return vec4<f32>(clip * viewport, 0.0, 1.0);
}
