//! The quad instance, its modes and the two solid-quad pushes, split out of
//! [`super`] under the 1,000-line rule in `scripts/check-file-size.py` - a
//! move, with no behaviour change. A child module rather than a sibling
//! because it reaches the renderer's own private instance list.

use super::Renderer;

/// One quad.
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct Quad {
    pub(super) rect: [f32; 4],
    pub(super) uv: [f32; 4],
    pub(super) color: [f32; 4],
    /// The colour the glyph's baked outline takes, RGBA.
    ///
    /// Only the atlas path reads it, and only for a font that bakes an outline -
    /// the two HUD ones. Everywhere else the atlas's mask is a constant 255, so
    /// the mix collapses to `color` and this is never visible. See
    /// `oag_ui::font::Atlas::luma`.
    pub(super) border: [f32; 4],
    /// Which texture `uv` indexes: [`MODE_ATLAS`] or [`MODE_SPRITE`].
    ///
    /// Per-quad rather than per-pipeline, so images, text and fills stay in one
    /// instance stream and the draw list's own back-to-front order is honoured
    /// without splitting the pass. The movie still needs a split because it is a
    /// genuinely different pipeline; a sprite is not.
    pub(super) mode: f32,
    /// Clockwise turn about the quad's own centre, in radians.
    ///
    /// `0.0` for everything but [`super::Draw::RotatedSprite`]. The **geometry** spins
    /// and the `uv` does not, which is what makes this a rotated model rather
    /// than a rotated texture lookup: the four corner brackets of the lock-on
    /// reticle are one model drawn four times.
    ///
    /// A trailing attribute, so adding it changed no existing site's meaning -
    /// see [`super::Draw::RotatedSprite`] on why the variant is separate too.
    pub(super) rotation: f32,
    /// How far the quad's top-left corner is pulled right and its top-right
    /// corner pulled left, in screen units.
    ///
    /// `[0.0, 0.0]` for everything but [`super::Draw::ChamferedFill`], HD's menu
    /// blocks' top band. Moving the two corners rather than adding vertices
    /// keeps this a trailing field on the six-vertex quad: the triangles are
    /// `TL, TR, BL` and `BL, TR, BR`, so a pulled `TR` turns the second's
    /// shared edge into a diagonal and a pulled `TL` does the same to the
    /// first's `TL`-`BL` edge. See [`super::Draw::ChamferedFill`].
    pub(super) chamfer: [f32; 2],
    /// How many times `uv` repeats across and down the quad - `[0, 0]` for
    /// every quad but a [`super::Draw::TiledSprite`], whose `uv` is one tile of the
    /// sheet rather than the whole patch. Read by the fragment stage, which
    /// wraps its own texture coordinate rather than relying on a sampler
    /// address mode: the tile is a patch *inside* the sheet, so the sampler's
    /// repeat would wrap the sheet, not the patch.
    pub(super) tile: [f32; 2],
}

/// A quad whose `uv` is in glyph-atlas pixels, sampled for coverage.
pub(super) const MODE_ATLAS: f32 = 0.0;
/// A quad whose `uv` is in sprite-sheet pixels, sampled as RGBA.
pub(super) const MODE_SPRITE: f32 = 1.0;
/// [`MODE_SPRITE`], composited **additively** rather than over.
///
/// A third mode rather than a second pipeline, which keeps the draw list's own
/// back-to-front order intact: the pass never splits. The pipeline blends
/// *premultiplied* alpha, so an additive quad is one emitting an alpha of zero
/// (see `ui.wesl`'s `fs_main` and `Draw::BlendedSprite`). Both this and
/// [`MODE_SPRITE`] index the sheet, so `mode > 0.5` still means "a sheet quad".
pub(super) const MODE_SPRITE_ADDITIVE: f32 = 2.0;
/// A solid quad whose colour runs from `color` at its left edge to `border`
/// at its right - [`super::Draw::GradientFill`]. The vertex stage interpolates the
/// two and then hands the fragment stage an ordinary [`MODE_ATLAS`] fill, so
/// the pipeline stays one pass with one back-to-front order.
pub(super) const MODE_GRADIENT: f32 = 3.0;
/// A quad whose `uv` is in the **face atlas's** pixels - [`super::Draw::FacedText`],
/// sampled from a second glyph texture bound alongside the main one rather
/// than replacing it, the way [`MODE_SPRITE`] samples a second texture
/// beside [`MODE_ATLAS`]. Only emitted once the role it names has actually
/// loaded; see `Renderer::push_text`'s `face` parameter.
pub(super) const MODE_FACE_ATLAS: f32 = 4.0;
/// A quad whose `uv` is in the **buttons atlas's** pixels -
/// [`super::Draw::FacedText`] with `role: "Buttons"`, sampled from a
/// *third* glyph texture (`Renderer::buttons_view`) rather than either of
/// the other two - HD's own screen title (`Title`-role, [`MODE_FACE_ATLAS`])
/// and its footer's button glyphs (`Buttons`-role, this) are on screen in
/// the same frame, so one secondary slot cannot serve both. Only emitted
/// once `Renderer::buttons_atlas` has actually loaded; see
/// `render::text::atlas_for`'s own doc for why this mode has **no**
/// fallback to [`MODE_ATLAS`] the way [`MODE_FACE_ATLAS`] does.
pub(super) const MODE_BUTTONS_ATLAS: f32 = 5.0;

impl Renderer {
    pub(super) fn push_solid(&mut self, rect: [f32; 4], color: [f32; 4], chamfer: [f32; 2]) {
        let solid = self.atlas.solid;
        self.quads.push(Quad {
            rect,
            // A single texel, sampled with nearest filtering, so the whole quad
            // reads full coverage.
            uv: [solid.x as f32 + 0.5, solid.y as f32 + 0.5, 0.0, 0.0],
            color,
            // A fill samples the solid patch, whose mask is all body, so the mix
            // is a no-op and this only has to be a real value.
            border: color,
            mode: MODE_ATLAS,
            rotation: 0.0,
            chamfer,
            tile: [0.0, 0.0],
        });
    }

    /// [`Self::push_solid`] with a second colour: `left` at the quad's left
    /// edge, `right` at its right, and `ui.wesl` interpolating between.
    pub(super) fn push_gradient(&mut self, rect: [f32; 4], left: [f32; 4], right: [f32; 4]) {
        let solid = self.atlas.solid;
        self.quads.push(Quad {
            rect,
            uv: [solid.x as f32 + 0.5, solid.y as f32 + 0.5, 0.0, 0.0],
            color: left,
            border: right,
            mode: MODE_GRADIENT,
            rotation: 0.0,
            chamfer: [0.0, 0.0],
            tile: [0.0, 0.0],
        });
    }
}

/// Shared with both shaders.
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct Uniforms {
    pub(super) viewport: [f32; 2],
    pub(super) screen: [f32; 2],
    pub(super) atlas: [f32; 2],
    /// The sprite sheet's size, for normalising its pixel-space UVs. This took
    /// the slot a padding pair held, so the struct is still 32 bytes.
    pub(super) sprites: [f32; 2],
    /// Where the movie sits in screen space: `[x, y, width, height]`. Only
    /// `video.wesl` reads this; `ui.wesl` still declares the field so the two
    /// shaders agree on the buffer's layout.
    pub(super) video_rect: [f32; 4],
    /// The face atlas's size, for normalising `Draw::FacedText`'s pixel-space
    /// UVs - `ui.wesl` alone reads this; `video.wesl` does not declare the
    /// field at all, the same way it already stops short of `sprites`.
    pub(super) face_atlas: [f32; 2],
    /// The buttons atlas's own size, the same idiom one field up -
    /// `ui.wesl` alone reads this too. Occupies the 8 bytes a `_padding`
    /// field held before `Draw::FacedText { role: "Buttons" }` existed:
    /// WGSL still rounds `Uniforms` to 64 bytes either way (`vec4`
    /// alignment), so this has to be *this* field and not one appended
    /// after it - appending would leave `ui.wesl`'s own `buttons_atlas`
    /// reading whatever the real padding held instead.
    pub(super) buttons_atlas: [f32; 2],
    /// `[pixels per grid unit, sharp-bilinear flag, 0, 0]`: the HUD stretch
    /// `ui.wesl` reads to blend only the last pixel of each texel. Zero flag
    /// for everything but a raster HUD asked for `sharp-bilinear`.
    pub(super) hud: [f32; 4],
}

impl Renderer {
    /// Gives the next [`Self::render_with`] a quad buffer of its own.
    ///
    /// Each call fills the buffer with `queue.write_buffer`, which lands before
    /// the encoder's passes run, so two calls into one encoder share one
    /// buffer and both passes draw the *second* call's quads - the first
    /// call's rules and bullet vanished from `Grid Selection` that way, and a
    /// flyer card between two calls is the only place this crate records two.
    /// The old buffer stays alive in the encoder until it is submitted.
    pub fn renew_quad_buffer(&mut self, device: &wgpu::Device) {
        self.quad_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("oag-game quads"),
            size: (self.quad_capacity * std::mem::size_of::<Quad>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
    }
}
