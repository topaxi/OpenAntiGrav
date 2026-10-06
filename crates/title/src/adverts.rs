//! How a title draws the adverts its circuits' billboard slots show.
//!
//! A circuit's `trackstartup.xml` names up to eight slots; on Pulse and Wipeout HD
//! the engine draws each slot's model through the model's own `Camera` node into
//! a small offscreen target, and the track's `billboardN` quad samples that
//! target. What differs per title is the target, the clip planes and whether the
//! colour-slot pool is read, and each is a measurement, so each lives here rather
//! than as a constant in the renderer. `None` on [`crate::Title::adverts`] is a
//! title whose adverts are not drawn - not measured, or not shipped - and its
//! placeholder quads draw nothing.

/// What one title's advert pass measured.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Adverts {
    /// The offscreen target every advert is drawn into, in pixels.
    ///
    /// Pulse: 128 x 128 (`FUN_0891fe14(object + 0x98, 0x80, 0x80)`, and the
    /// `FBPTR` width of a live GE dump). Wipeout HD: 512 x 256 (the literal
    /// viewport in `Billboard_UpdateAndRender`, and the `0x200` / `0x100` of the
    /// target object read live on RPCS3).
    pub target: (u32, u32),
    /// The near plane of the advert's projection.
    pub near: f32,
    /// The far plane of the advert's projection.
    pub far: f32,
    /// Whether a slot that names a colour draws an advert from the engine's pool
    /// (`oag_tables::billboard_pool`). `false` is a title whose pool order is not
    /// measured: those slots stay undrawn and the load report names them.
    pub colour_pool: bool,
    /// Whether the track draws that show a target sample it with their V
    /// coordinate negated.
    ///
    /// Pulse does: a live PPSSPP dump of Talon's Junction puts `TEXSCALE (1, -1)`
    /// and `TEXOFFSET (0, 0)` on the prims that read the 128 x 128 target and
    /// `(1, 1)` or an authored animation scale on every other textured prim
    /// (547 prims). The target holds the picture top row first and the track's
    /// UVs run V up, so the engine flips it on exactly those draws. HD authors
    /// its billboard quads with V running down (read off the track's vertices), so
    /// it needs none.
    pub flip_v: bool,
    /// Whether a Zone race swaps the per-slot adverts for one shared texture.
    ///
    /// Wipeout HD's `Billboard_LoadModelAndBind` takes a second branch while the
    /// zone effects are on: it binds a single `"billboard"` texture to every slot
    /// but 8 instead of building a target per slot. What that texture is has not
    /// been read, so on such a title a Zone race draws no advert and says so.
    pub zone_shares_one_texture: bool,
    /// Where the numbers came from.
    pub origin: crate::Origin,
}
