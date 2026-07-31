// Draws collision sparks.
//
// Structurally a smaller copy of exhaust.wgsl - see that file's own header
// for why the model matrix in the shared uniform block is unused (the
// vertices already arrive in world space). The one real difference is that
// this pipeline samples no texture at all: no authored spark texture is
// available to decode (see sparks.rs's module doc comment), so the shape is
// a plain radial falloff computed from the quad's own texture coordinates.

struct Uniforms {
    view_projection: mat4x4<f32>,
    model: mat4x4<f32>,
};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;

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
    out.clip = uniforms.view_projection * vec4<f32>(in.position, 1.0);
    out.colour = in.colour;
    out.texcoord = in.texcoord;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Distance from the quad's centre, 0 there and 1 at the inscribed circle
    // touching the edges - texcoord spans [0, 1] per axis, so 0.5 is centre.
    let d = distance(in.texcoord, vec2<f32>(0.5, 0.5)) * 2.0;
    let shape = clamp(1.0 - d, 0.0, 1.0);
    let shape2 = shape * shape;
    // The shape belongs on alpha only, the same split exhaust.wgsl uses
    // (`shape * in.colour.a`, rgb untouched): the additive blend already
    // multiplies rgb by alpha (`src.rgb * src.a`), so folding the same
    // falloff into rgb here as well squares it a second time. That second
    // squaring is what made a spark's already-small quad (a few pixels at
    // any sensible camera distance) collapse to a sub-pixel bright dot no
    // rasterizer sample ever landed on - it looked like nothing drew at all
    // rather than like a dim one.
    return vec4<f32>(in.colour.rgb, shape2 * in.colour.a);
}
