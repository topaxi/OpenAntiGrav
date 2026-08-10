// Draws collision sparks.
//
// Structurally a smaller copy of exhaust.wgsl - see that file's own header
// for why the model matrix in the shared uniform block is unused (the
// vertices already arrive in world space). The one real difference is that
// this pipeline samples no texture: the authored sprite
// (`Data\Psys\Tex\orange_glow2.tga`, named inside the .pob) has no
// locatable WAD entry yet, so its shape is modelled instead. The original
// stretches a single row of that radial glow along the streak's body, with
// the sprite's halves forming the two end caps - so the body keeps a
// constant-width bright core however long the streak grows, instead of
// smearing one radial blob over the whole quad (which is what an earlier
// revision did, and what made every spark a fat soft wedge). The vertex
// stream's `lit` slot carries the caps' share of the half-length; `0.5`
// collapses the profile to the plain radial falloff a round billboard
// wants.

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
    @location(2) cap: f32,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip = uniforms.view_projection * vec4<f32>(in.position, 1.0);
    out.colour = in.colour;
    out.texcoord = in.texcoord;
    out.cap = in.lit;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // `u` runs along the streak, `v` across it. Inside the two caps (the
    // first and last `cap` of the u range) the along-axis term ramps 0 -> 1
    // toward the tip; over the body it is 0, so only the cross-axis
    // profile remains and the core stays a constant-width line however
    // long the quad is. For a billboard `cap` is 0.5 - the caps meet in
    // the middle and the expression reduces exactly to the radial
    // `distance(uv, centre) * 2` this shader used for everything before.
    let tip = min(in.texcoord.x, 1.0 - in.texcoord.x);
    let along = clamp((in.cap - tip) / max(in.cap, 1e-4), 0.0, 1.0);
    let across = abs(in.texcoord.y * 2.0 - 1.0);
    let d = length(vec2<f32>(along, across));
    let shape = clamp(1.0 - d, 0.0, 1.0);
    // The profile is measured off the shipped sprite itself, radially
    // averaged from a PPSSPP texture dump of the 64x64 glow the collision
    // effect binds (orange_glow2.tga - identified live: every psys draw in
    // a crash reads the same texture address, and the dump's pixels carry
    // the authoring-path's promise). Measured: alpha falls essentially
    // linearly (fit (1-r)^0.92, a touch faster mid-range, hence 1.2); the
    // texel colour *also* dims linearly with radius while keeping its
    // saturated hue, whitening only in the innermost ~15 %. Splitting the
    // falloff between rgb and alpha the same way the sprite does is what
    // keeps the bright part of a spark small - a flat-rgb quad reads twice
    // as wide at the same alpha curve, which was the reported "too big,
    // not line-like" look.
    let core = pow(shape, 8.0);
    let rgb = mix(in.colour.rgb, vec3<f32>(1.0, 1.0, 1.0), core * 0.85) * shape;
    return vec4<f32>(rgb, pow(shape, 1.2) * in.colour.a);
}
