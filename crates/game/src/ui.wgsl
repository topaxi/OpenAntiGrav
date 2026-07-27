// Textured, colour-modulated quads in the PSP's 480x272 screen space.
//
// One pipeline draws both text and solid fills: the glyph atlas carries a single
// opaque texel that solid rectangles sample, so there is no second pipeline and
// no second bind group to keep in step.

struct Uniforms {
    // Multiplies clip space to letterbox 480x272 into a window of any shape.
    viewport: vec2<f32>,
    // The virtual screen, 480x272.
    screen: vec2<f32>,
    // Atlas size in pixels, for normalising the pixel-space UVs.
    atlas: vec2<f32>,
    // Sprite sheet size in pixels, for the same reason.
    sprites: vec2<f32>,
};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var atlas_texture: texture_2d<f32>;
@group(0) @binding(2) var atlas_sampler: sampler;
@group(0) @binding(3) var sprite_texture: texture_2d<f32>;
@group(0) @binding(4) var sprite_sampler: sampler;

struct Instance {
    // x, y, width, height in screen pixels.
    @location(0) rect: vec4<f32>,
    // u, v, width, height in atlas pixels.
    @location(1) uv: vec4<f32>,
    @location(2) color: vec4<f32>,
    // 0 indexes the glyph atlas, 1 the sprite sheet.
    @location(3) mode: f32,
};

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
    // Flat: a quad is one or the other, and interpolating the flag across the
    // triangle would make the middle of a sprite sample somewhere between the
    // two textures.
    @location(2) @interpolate(flat) mode: f32,
};

fn corner_of(index: u32) -> vec2<f32> {
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

fn to_clip(pixels: vec2<f32>) -> vec4<f32> {
    let normalised = pixels / uniforms.screen;
    let clip = vec2<f32>(normalised.x * 2.0 - 1.0, 1.0 - normalised.y * 2.0);
    return vec4<f32>(clip * uniforms.viewport, 0.0, 1.0);
}

@vertex
fn vs_main(@builtin(vertex_index) index: u32, instance: Instance) -> VertexOut {
    let corner = corner_of(index);
    var out: VertexOut;
    out.position = to_clip(instance.rect.xy + corner * instance.rect.zw);
    // Normalised here rather than in the fragment shader, against whichever
    // texture this quad indexes.
    let size = select(uniforms.atlas, uniforms.sprites, instance.mode > 0.5);
    out.uv = (instance.uv.xy + corner * instance.uv.zw) / size;
    out.color = instance.color;
    out.mode = instance.mode;
    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    // Both are sampled unconditionally and one is discarded. `textureSample`
    // needs uniform control flow for its implicit derivatives, and the mode flag
    // is per-instance, so branching around the sample is not allowed here.
    let coverage = textureSample(atlas_texture, atlas_sampler, in.uv).r;
    let sprite = textureSample(sprite_texture, sprite_sampler, in.uv);

    let from_atlas = vec4<f32>(in.color.rgb, in.color.a * coverage);
    let from_sheet = sprite * in.color;
    return select(from_atlas, from_sheet, in.mode > 0.5);
}
