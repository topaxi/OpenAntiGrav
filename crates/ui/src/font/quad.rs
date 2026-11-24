//! The quad of one glyph on Pure's snapped pen.

use super::Cell;

/// How far, in atlas texels, the texture coordinates of a glyph on Pure's snapped pen start before
/// its box, on both axes: the original's sprite samples `[u0 - 0.5, u1 + 0.5]` by `[v0 - 0.5,
/// v1 + 0.5]` (`docs/ghidra/functions/psp-pure-usa/menu-text-centring.md`, "The quad").
pub const SNAPPED_UV_SHIFT: f32 = 0.5;

/// The quad of one glyph on Pure's snapped pen, in screen units: whole pixels, as a PSP sprite
/// is, and the texels it samples.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SnappedQuad {
    /// The pen rounded up (the quad starts at the pen, not at the ink: the box's bearing is in
    /// the texture).
    pub left: f32,
    /// The box's width at `scale`, rounded down, at least one pixel for a box with ink.
    pub width: f32,
    /// The box's height at `scale`, rounded down.
    pub height: f32,
    /// Texels the quad's width samples, from `u0 -` [`SNAPPED_UV_SHIFT`]: the box's width plus
    /// one, spread over the fractional width the sprite was built with, then cut to
    /// [`Self::width`] pixels. `0` for an empty box.
    pub u_span: f32,
    /// [`Self::u_span`] for the height, from `v0 -` [`SNAPPED_UV_SHIFT`].
    pub v_span: f32,
}

/// [`SnappedQuad`] of `cell` with the pen before it at `pen` and `scale` screen units per atlas
/// texel. Whole-pixel edges measured on every Pure face (`docs/ui/menus-original.md`, "Pure's
/// glyph quads are whole pixels"); the texture span is the executable's own ratio
/// (`docs/ghidra/functions/psp-pure-usa/menu-text-centring.md`, "The quad").
#[must_use]
pub fn snapped_quad(cell: &Cell, scale: f32, pen: f32) -> SnappedQuad {
    // A box with ink is at least one pixel: a one-texel glyph at a scale under 1 would vanish.
    // No glyph of Pure's two faces is under two texels, so the floor alone decides on every
    // measured glyph.
    let whole = |texels: u32| {
        if texels == 0 {
            0.0
        } else {
            (texels as f32 * scale).floor().max(1.0)
        }
    };
    // The sprite's corners are `x` and `x + texels * scale`, sampling `texels + 1` texels; the
    // rasteriser keeps whole pixels of it and the slope of the fractional span.
    let span = |texels: u32| {
        if texels == 0 {
            0.0
        } else {
            (texels as f32 + 1.0) * whole(texels) / (texels as f32 * scale)
        }
    };
    SnappedQuad {
        left: pen.ceil(),
        width: whole(cell.width),
        height: whole(cell.height),
        u_span: span(cell.width),
        v_span: span(cell.height),
    }
}
