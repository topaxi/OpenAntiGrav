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
    /// Where the middle of [`Self::quad_extent`] sits from the model's own origin, in the same
    /// units and in screen orientation (`y` down) - `None` when the quad is centred on the
    /// origin, which every single-quad model measured is. A model drawn as a polygon strip
    /// (Pure's speed and shield fills) is not symmetric about its origin, and a draw that
    /// crops it needs the bounds the crop is a fraction of.
    pub quad_centre: Option<[f32; 2]>,
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
    /// Sheet pixels per pixel of the disc's own picture, on each axis: `[1.0, 1.0]` unless a
    /// texture pack drew the image larger (`oag_texture::replacer`). [`Self::width`],
    /// [`Self::height`] and every layout rect stay in the disc's pixels; a draw that samples
    /// the sheet goes through [`Self::sheet_rect`].
    pub scale: [f32; 2],
}

impl Placed {
    /// `rect`, `[x, y, width, height]` in the disc's pixels of this image, as the sheet's
    /// pixels: the origin offset by where it sits, everything scaled by [`Self::scale`]. A
    /// width or height may be negative (a mirrored patch) or zero.
    #[must_use]
    pub fn sheet_rect(&self, rect: [f32; 4]) -> [f32; 4] {
        let [sx, sy] = self.scale;
        [
            self.x as f32 + rect[0] * sx,
            self.y as f32 + rect[1] * sy,
            rect[2] * sx,
            rect[3] * sy,
        ]
    }
}
