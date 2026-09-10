// Textured, colour-modulated quads in the PSP's 480x272 screen space.
//
// One pipeline draws both text and solid fills: the glyph atlas carries a single
// opaque texel that solid rectangles sample, so there is no second pipeline and
// no second bind group to keep in step.
//
// The glyph atlas is two channels, not one. `r` is a body/outline mask and `g` is
// coverage, because the disc's two HUD fonts bake an outline into their atlas and
// distinguish it only by grey level - see `font.rs`. Text is composited as
// `mix(border, color, mask)` at `coverage`, which for the three menu fonts (mask
// constant 1) is exactly the plain `color` it always was.

struct Uniforms {
    // Multiplies clip space to letterbox 480x272 into a window of any shape.
    viewport: vec2<f32>,
    // The virtual screen, 480x272.
    screen: vec2<f32>,
    // Atlas size in pixels, for normalising the pixel-space UVs.
    atlas: vec2<f32>,
    // Sprite sheet size in pixels, for the same reason.
    sprites: vec2<f32>,
    // Where the movie sits in screen space. Unused here - only video.wgsl
    // reads it - but both shaders bind the same buffer, so its layout must
    // agree.
    video_rect: vec4<f32>,
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
    // What the glyph's baked outline is drawn in. Only the atlas path reads it.
    @location(3) border: vec4<f32>,
    // 0 indexes the glyph atlas, 1 the sprite sheet, 2 the sprite sheet added
    // rather than blended over, 3 a solid fill whose colour runs from `color`
    // on the left to `border` on the right - which the vertex stage resolves
    // into a plain mode-0 fill, so every `> 0.5` test in the fragment stage
    // is still exactly "is this a sheet quad".
    @location(4) mode: f32,
    // Clockwise turn about the quad's own centre, in radians. Zero for
    // everything but the lock-on reticle's corner brackets, which are four
    // instances of one model at four quarter turns.
    @location(5) rotation: f32,
    // How far the top-right corner is pulled left, in screen units. Zero for
    // everything but HD's main-menu tab corner.
    @location(6) chamfer: f32,
    // How many times `uv` - then one tile of the sheet, not a patch - repeats
    // across and down the quad. Zero for everything but a tiled sprite: the
    // selection screens' hex grid, a 32x16 tile drawn 340x120.
    @location(7) tile: vec2<f32>,
};

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) border: vec4<f32>,
    // Flat: a quad is one or the other, and interpolating the flag across the
    // triangle would make the middle of a sprite sample somewhere between the
    // two textures.
    @location(3) @interpolate(flat) mode: f32,
    // The tile's own rect in the sheet, normalised, for a tiled sprite; a
    // zero width for everything else, which is the flag the fragment stage
    // reads.
    @location(4) @interpolate(flat) tile_rect: vec4<f32>,
    // Where this corner is in tiles: `corner * tile`, so `fract` of it walks
    // the tile again every whole unit.
    @location(5) tiles: vec2<f32>,
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
    // The **geometry** turns and the `uv` below does not, so this spins the
    // quad rather than the texture lookup - the difference between drawing one
    // model four ways and sampling one atlas patch four ways. About the middle
    // of the rectangle, so a rotated sprite keeps the centre an unrotated one
    // would have had.
    //
    // **In pixels, not in the unit square.** Turning the corner first and
    // scaling by `rect.zw` afterwards is `scale . rotate`, which shears any quad
    // that is not square: HD's `ZoneBG` is a 676x153 bar authored at a quarter
    // turn, and that order left it 676 wide and 153 tall with its art lying on
    // its side instead of standing it up as the 153x676 column it is. A square
    // quad comes out the same either way, which is why the lock-on reticle -
    // the only caller until 2026-08-31, and 8 pixels square - never showed it.
    //
    // The chamfer, before anything else touches the corner: it is a change to
    // where this vertex *is* on the quad, not a transform of the quad. Only
    // the top-right corner moves, and only left - the two triangles are
    // `TL, TR, BL` and `BL, TR, BR`, so a shorter top edge in the first leaves
    // the second's `TR`-to-`BR` edge as the diagonal joining the two lengths.
    // Clamped at the quad's own width so an over-wide chamfer collapses the
    // top edge to a point rather than folding it back past the left edge.
    let cut = min(instance.chamfer, instance.rect.z);
    let is_top_right = corner.x * (1.0 - corner.y);
    let corner_px = corner * instance.rect.zw - vec2<f32>(is_top_right * cut, 0.0);
    let centred = corner_px - instance.rect.zw * 0.5;
    let turn = mat2x2<f32>(
        vec2<f32>(cos(instance.rotation), sin(instance.rotation)),
        vec2<f32>(-sin(instance.rotation), cos(instance.rotation)),
    );
    let placed = turn * centred + instance.rect.zw * 0.5;
    var out: VertexOut;
    out.position = to_clip(instance.rect.xy + placed);
    // Normalised here rather than in the fragment shader, against whichever
    // texture this quad indexes.
    // A gradient fill is resolved here: its colour at this corner is the mix
    // of its two edge colours by how far across the quad the corner sits, and
    // what reaches the fragment stage is an ordinary atlas fill of that colour
    // - both `color` and `border` carry it, so the solid texel's own mask mixes
    // to the same value whichever way it reads.
    let is_gradient = instance.mode > 2.5;
    let is_sheet = instance.mode > 0.5 && !is_gradient;
    let size = select(uniforms.atlas, uniforms.sprites, is_sheet);
    out.uv = (instance.uv.xy + corner * instance.uv.zw) / size;
    let tiled = instance.tile.x > 0.0;
    out.tile_rect = select(vec4<f32>(0.0), instance.uv / vec4<f32>(size, size), tiled);
    out.tiles = corner * instance.tile;
    let graded = mix(instance.color, instance.border, corner.x);
    out.color = select(instance.color, graded, is_gradient);
    out.border = select(instance.border, graded, is_gradient);
    out.mode = select(instance.mode, 0.0, is_gradient);
    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    // Both are sampled unconditionally and one is discarded. `textureSample`
    // needs uniform control flow for its implicit derivatives, and the mode flag
    // is per-instance, so branching around the sample is not allowed here.
    let glyph = textureSample(atlas_texture, atlas_sampler, in.uv);
    // `r` is the body/outline mask, `g` the silhouette's coverage.
    let mask = glyph.r;
    let coverage = glyph.g;
    // A tiled quad samples its one tile again every whole unit of `tiles`;
    // an ordinary one samples where the vertex stage said. `textureSample`
    // stays unconditional, and the sheet has one mip level, so the seam in
    // the wrapped coordinate's derivative costs nothing.
    let wrapped = in.tile_rect.xy + fract(in.tiles) * in.tile_rect.zw;
    let sprite_uv = select(in.uv, wrapped, in.tile_rect.z > 0.0);
    let sprite = textureSample(sprite_texture, sprite_sampler, sprite_uv);

    // Body toward `color`, outline toward `border`. The alpha is mixed too, so a
    // translucent border colour - which is what the HUD authors, 0x40000000 -
    // leaves the outline fainter than the glyph rather than opaque black.
    let ink = mix(in.border, in.color, mask);
    let from_atlas = vec4<f32>(ink.rgb, ink.a * coverage);
    let from_sheet = sprite * in.color;
    let straight = select(from_atlas, from_sheet, in.mode > 0.5);

    // **Premultiplied on the way out**, because the pipeline blends
    // premultiplied alpha - see `render.rs`'s colour target. `src.rgb * src.a`
    // here is exactly the `SrcAlpha` factor the ROP used to apply, so every
    // quad that was drawn before this existed composites identically.
    //
    // An **additive** quad returns an alpha of zero instead, which leaves the
    // destination factor `1 - 0` at one: `src.rgb * src.a + dst.rgb`. That is
    // the `GU_ADD, GU_SRC_ALPHA / GU_FIX 0xffffff` the `pass_mask & 0x200`
    // class programs on the original, and it is the whole difference between
    // the lock-on reticle's brackets glowing on the track and sitting on
    // opaque black tiles: their textures carry the shape in the colour
    // channels over a black field, with alpha pinned at 250/255.
    let is_additive = in.mode > 1.5;
    let alpha = select(straight.a, 0.0, is_additive);
    return vec4<f32>(straight.rgb * straight.a, alpha);
}
