//! Where a widget's image sits in the front end's sprite sheet.
//!
//! Split out of `crate::sprite` rather than defined there: [`Placed`] is the
//! pure-data half of that module - a rectangle and two read-off-the-model
//! extras - and this crate's screens, frames and drawing all need to name it
//! without needing `crate::sprite::Sheet`'s decode step, which stays behind.

/// Where one image sits in the sheet, in pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placed {
    /// Left edge in the sheet.
    pub x: u32,
    /// Top edge in the sheet.
    pub y: u32,
    /// The image's own width.
    pub width: u32,
    /// The image's own height.
    pub height: u32,
    /// A `.vex` model's own quad, in the `<Mode3D>` overlay's own units - `None`
    /// for anything that is not a model, and `None` for a model whose caller
    /// did not measure one.
    ///
    /// Read off the model's own vertex positions rather than authored
    /// anywhere, since a `.vex` carries no "this is N units wide" attribute,
    /// only the vertices themselves - so a caller that wants a screen size at
    /// draw time reads it here instead of a hand-measured constant per mesh.
    /// The lock-on reticle predates this field and still carries its own
    /// hand-measured `SIGHT_SIZE` rather than reading it - see that
    /// constant's own doc for why unifying the two was left alone rather
    /// than guessed at.
    pub quad_extent: Option<[f32; 2]>,
    /// How a `.vex` model's own batch asked to be composited, straight off its
    /// `pass_mask` - `None` for anything that is not a model, and `None` for a
    /// model whose batch is not in the transparent class at all.
    ///
    /// **Read, never chosen.** All three of Pulse's lock-on sight models
    /// declare `pass_mask 0x120e`, whose `0x0200` bit is
    /// [`oag_vex::vex::BlendClass::Additive`], and their embedded textures
    /// carry a black background with alpha pinned at 250/255 - so drawing them
    /// with an ordinary alpha blend puts each bracket on an opaque black tile.
    /// Carrying the declared class here is what lets the draw honour it
    /// without any per-model table; Pure's ten icon models get the same
    /// treatment from the same reading the day a Pure disc is present to read
    /// them from. See `docs/ui/hud.md` and
    /// `docs/ghidra/functions/psp-pulse-usa/mesh-draw.md`.
    pub blend: Option<oag_vex::vex::BlendClass>,
}
