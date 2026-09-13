//! The quad modes and the two solid-quad pushes, split out of [`super`]
//! under the 1,000-line rule in `scripts/check-file-size.py` - a move, with
//! no behaviour change. A child module rather than a sibling because it
//! reaches the renderer's own private instance list.

use super::{Quad, Renderer};

/// A quad whose `uv` is in glyph-atlas pixels, sampled for coverage.
pub(super) const MODE_ATLAS: f32 = 0.0;
/// A quad whose `uv` is in sprite-sheet pixels, sampled as RGBA.
pub(super) const MODE_SPRITE: f32 = 1.0;
/// [`MODE_SPRITE`], composited **additively** rather than over.
///
/// A third mode rather than a second pipeline, which keeps the draw list's own
/// back-to-front order intact: the pass never splits. The pipeline blends
/// *premultiplied* alpha, so an additive quad is one emitting an alpha of zero
/// (see `ui.wgsl`'s `fs_main` and `Draw::BlendedSprite`). Both this and
/// [`MODE_SPRITE`] index the sheet, so `mode > 0.5` still means "a sheet quad".
pub(super) const MODE_SPRITE_ADDITIVE: f32 = 2.0;
/// A solid quad whose colour runs from `color` at its left edge to `border`
/// at its right - [`Draw::GradientFill`]. The vertex stage interpolates the
/// two and then hands the fragment stage an ordinary [`MODE_ATLAS`] fill, so
/// the pipeline stays one pass with one back-to-front order.
pub(super) const MODE_GRADIENT: f32 = 3.0;
/// A quad whose `uv` is in the **face atlas's** pixels - [`Draw::FacedText`],
/// sampled from a second glyph texture bound alongside the main one rather
/// than replacing it, the way [`MODE_SPRITE`] samples a second texture
/// beside [`MODE_ATLAS`]. Only emitted once the role it names has actually
/// loaded; see `Renderer::push_text`'s `face` parameter.
pub(super) const MODE_FACE_ATLAS: f32 = 4.0;

impl Renderer {
    pub(super) fn push_solid(&mut self, rect: [f32; 4], color: [f32; 4], chamfer: f32) {
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
    /// edge, `right` at its right, and `ui.wgsl` interpolating between.
    pub(super) fn push_gradient(&mut self, rect: [f32; 4], left: [f32; 4], right: [f32; 4]) {
        let solid = self.atlas.solid;
        self.quads.push(Quad {
            rect,
            uv: [solid.x as f32 + 0.5, solid.y as f32 + 0.5, 0.0, 0.0],
            color: left,
            border: right,
            mode: MODE_GRADIENT,
            rotation: 0.0,
            chamfer: 0.0,
            tile: [0.0, 0.0],
        });
    }
}
