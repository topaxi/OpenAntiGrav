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
//
// There is a second, smaller glyph texture too - `face_texture`, for
// `Draw::FacedText` - bound alongside the first exactly the way the sprite
// sheet is, rather than replacing it: a menu screen can need both the body
// face and a named role's face (Wipeout HD's bold chrome title) in the same
// frame, and one bound atlas at a time cannot draw that. It carries a 1x1
// placeholder whenever no title package has loaded a face atlas, which no
// quad samples in that case - see `MODE_FACE_ATLAS`.
//
// And a third, `buttons_texture`, for `Draw::FacedText { role: "Buttons" }` -
// Wipeout HD/Fury's own footer button glyphs, which need to be on screen
// alongside `face_texture`'s `Title`-role screen title in the same frame, so
// they cannot share its one slot. See `MODE_BUTTONS_ATLAS`.

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
    // The face atlas's own size in pixels - `Draw::FacedText`'s glyphs, which
    // are not always the same font as `atlas` and so not always the same
    // size. A 1x1 placeholder whenever no face atlas is loaded, which no quad
    // samples in that case anyway.
    face_atlas: vec2<f32>,
    // The buttons atlas's own size in pixels, the same idiom as `face_atlas`
    // one field up - HD/Fury's `ps_buttons.fnt`/`PS_BUTTONS.fnt` once loaded,
    // a 1x1 placeholder otherwise. Occupies the slot a plain `_padding` field
    // held before this existed: WGSL still rounds `Uniforms` to 64 bytes
    // either way, so this is a real field rather than appended bytes.
    buttons_atlas: vec2<f32>,
    // The HUD stretch: x is output pixels per grid unit, y is 1 when the HUD
    // is drawn sharp-bilinear and 0 for everything else.
    hud: vec4<f32>,
};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var atlas_texture: texture_2d<f32>;
@group(0) @binding(2) var atlas_sampler: sampler;
@group(0) @binding(3) var sprite_texture: texture_2d<f32>;
@group(0) @binding(4) var sprite_sampler: sampler;
@group(0) @binding(5) var face_texture: texture_2d<f32>;
@group(0) @binding(6) var face_sampler: sampler;
@group(0) @binding(7) var buttons_texture: texture_2d<f32>;
@group(0) @binding(8) var buttons_sampler: sampler;

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
    // into a plain mode-0 fill - 4 the **face** atlas, a second glyph
    // texture for `Draw::FacedText`, and 5 the **buttons** atlas, a third,
    // for `Draw::FacedText { role: "Buttons" }`. Every mode is tested by
    // range rather than by an open-ended `>`, because a higher mode sorts
    // above every earlier one and an open-ended test would silently catch
    // it too - the trap `is_face` used to have before `MODE_BUTTONS_ATLAS`
    // existed to catch it: `mode > 3.5` alone would have counted every
    // buttons-atlas quad as a face-atlas one.
    @location(4) mode: f32,
    // Clockwise turn about the quad's own centre, in radians. Zero for
    // everything but the lock-on reticle's corner brackets, which are four
    // instances of one model at four quarter turns.
    @location(5) rotation: f32,
    // How far the top-left corner is pulled right and the top-right corner
    // pulled left, in screen units. Zero for everything but HD's menu
    // blocks' top band.
    @location(6) chamfer: vec2<f32>,
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
    // Output pixels per texel on each axis, and the size of the texture this
    // quad samples, for the sharp-bilinear blend.
    @location(6) @interpolate(flat) texel_pixels: vec2<f32>,
    @location(7) @interpolate(flat) texture_size: vec2<f32>,
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
    // The chamfers, before anything else touches the corners: they are a
    // change to where this vertex *is* on the quad, not a transform of the
    // quad. Only the two top corners move, each inward - the two triangles
    // are `TL, TR, BL` and `BL, TR, BR`, so a shorter top edge leaves the
    // second's `TR`-to-`BR` edge and the first's `TL`-to-`BL` edge as the
    // diagonals joining the two lengths. Clamped so the two cuts together
    // collapse the top edge to a point rather than folding it back past
    // each other: the right cut is bounded by the width, the left by what
    // the right cut leaves.
    let right_cut = min(instance.chamfer.y, instance.rect.z);
    let left_cut = min(instance.chamfer.x, instance.rect.z - right_cut);
    let is_top = 1.0 - corner.y;
    let is_top_right = corner.x * is_top;
    let is_top_left = (1.0 - corner.x) * is_top;
    let corner_px = corner * instance.rect.zw
        + vec2<f32>(is_top_left * left_cut - is_top_right * right_cut, 0.0);
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
    let is_gradient = instance.mode > 2.5 && instance.mode < 3.5;
    let is_sheet = instance.mode > 0.5 && instance.mode < 2.5;
    // Both range-tested, not open-ended - see `Instance::mode`'s own doc:
    // an untested `is_face = mode > 3.5` would also be true for a mode-5
    // (buttons-atlas) quad.
    let is_face = instance.mode > 3.5 && instance.mode < 4.5;
    let is_buttons = instance.mode > 4.5 && instance.mode < 5.5;
    var size = select(uniforms.atlas, uniforms.sprites, is_sheet);
    size = select(size, uniforms.face_atlas, is_face);
    size = select(size, uniforms.buttons_atlas, is_buttons);
    out.uv = (instance.uv.xy + corner * instance.uv.zw) / size;
    out.texture_size = size;
    out.texel_pixels = instance.rect.zw * uniforms.hud.x / max(instance.uv.zw, vec2<f32>(1.0));
    let tiled = instance.tile.x > 0.0;
    out.tile_rect = select(vec4<f32>(0.0), instance.uv / vec4<f32>(size, size), tiled);
    out.tiles = corner * instance.tile;
    let graded = mix(instance.color, instance.border, corner.x);
    out.color = select(instance.color, graded, is_gradient);
    out.border = select(instance.border, graded, is_gradient);
    out.mode = select(instance.mode, 0.0, is_gradient);
    return out;
}

// Nearest across most of a texel and a linear blend over its last output
// pixel: `scale` is how many pixels one texel covers, so the blend band is
// `1 / scale` of a texel wide. A texel shown under one pixel is left linear.
fn sharp_uv(uv: vec2<f32>, size: vec2<f32>, scale: vec2<f32>) -> vec2<f32> {
    let s = max(scale, vec2<f32>(1.0));
    let texel = uv * size;
    let base = floor(texel);
    let centred = texel - base - 0.5;
    let band = 0.5 - 0.5 / s;
    let blended = (centred - clamp(centred, -band, band)) * s + 0.5;
    return (base + blended) / size;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    // All three are sampled unconditionally and two are discarded.
    // `textureSample` needs uniform control flow for its implicit derivatives,
    // and the mode flag is per-instance, so branching around the sample is
    // not allowed here.
    let sharp = uniforms.hud.y > 0.5 && in.tile_rect.z <= 0.0;
    let uv = select(in.uv, sharp_uv(in.uv, in.texture_size, in.texel_pixels), sharp);
    let glyph = textureSample(atlas_texture, atlas_sampler, uv);
    // `r` is the body/outline mask, `g` the silhouette's coverage.
    let mask = glyph.r;
    let coverage = glyph.g;
    // A tiled quad samples its one tile again every whole unit of `tiles`;
    // an ordinary one samples where the vertex stage said. `textureSample`
    // stays unconditional, and the sheet has one mip level, so the seam in
    // the wrapped coordinate's derivative costs nothing.
    let wrapped = in.tile_rect.xy + fract(in.tiles) * in.tile_rect.zw;
    let sprite_uv = select(uv, wrapped, in.tile_rect.z > 0.0);
    let sprite = textureSample(sprite_texture, sprite_sampler, sprite_uv);
    let face = textureSample(face_texture, face_sampler, uv);
    let face_mask = face.r;
    let face_coverage = face.g;
    let buttons = textureSample(buttons_texture, buttons_sampler, uv);
    let buttons_mask = buttons.r;
    let buttons_coverage = buttons.g;

    // The vertex stage collapses a gradient (mode 3) to mode 0 before this
    // runs, so `in.mode` only ever reaches here as 0, 1, 2, 4 or 5 - tested
    // by range, for the same reason the vertex stage now is (see
    // `Instance::mode`'s own doc for the `is_face`/`is_buttons` trap this
    // guards against).
    let is_sheet = in.mode > 0.5 && in.mode < 2.5;
    let is_face = in.mode > 3.5 && in.mode < 4.5;
    let is_buttons = in.mode > 4.5 && in.mode < 5.5;

    // Body toward `color`, outline toward `border`. The alpha is mixed too, so a
    // translucent border colour - which is what the HUD authors, 0x40000000 -
    // leaves the outline fainter than the glyph rather than opaque black.
    let ink = mix(in.border, in.color, mask);
    let from_atlas = vec4<f32>(ink.rgb, ink.a * coverage);
    let from_sheet = sprite * in.color;
    // The face atlas mixes the same way the main one does - the menu faces
    // this build has loaded through it all carry a constant mask, same as
    // `from_atlas`'s own menu-font case - so this is not a third formula, only
    // a third sample. The buttons atlas is a fourth sample of the identical
    // formula, for the identical reason: `ps_buttons.fnt`'s own mask is
    // constant too (`oag-tools --example hd_buttons_font_probe`'s own atlas
    // dump shows plain white glyph art, no baked outline).
    let face_ink = mix(in.border, in.color, face_mask);
    let from_face = vec4<f32>(face_ink.rgb, face_ink.a * face_coverage);
    let buttons_ink = mix(in.border, in.color, buttons_mask);
    let from_buttons = vec4<f32>(buttons_ink.rgb, buttons_ink.a * buttons_coverage);
    let with_face = select(select(from_atlas, from_sheet, is_sheet), from_face, is_face);
    let straight = select(with_face, from_buttons, is_buttons);

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
    let is_additive = in.mode > 1.5 && in.mode < 2.5;
    let alpha = select(straight.a, 0.0, is_additive);
    return vec4<f32>(straight.rgb * straight.a, alpha);
}
