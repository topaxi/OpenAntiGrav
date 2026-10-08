//! Widgets that are only a rectangle: what a screen file says about where a
//! picture or a frame goes, kept as data for the screen that knows what to put
//! there. Nothing here draws.

/// An `Image` widget with a rect and nothing to draw it with - see
/// [`super::Screen::slots`].
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Slot {
    /// Widget name.
    pub name: Option<String>,
    /// Left edge, its container's offsets folded in.
    pub x: f32,
    /// Top edge.
    pub y: f32,
    /// Authored width.
    pub width: Option<f32>,
    /// Authored height.
    pub height: Option<f32>,
}

/// A `<Bracket corner="true">`: a rectangle framed by a mark at each of its
/// four corners. The mark's art is `Data\FE\Images\corner2.gtf`, a 16 by 16
/// mask the screen file does not name (`docs/formats/gtf.md`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Bracket {
    /// Left edge, its container's offsets folded in.
    pub x: f32,
    /// Top edge.
    pub y: f32,
    /// Authored width.
    pub width: f32,
    /// Authored height.
    pub height: f32,
    /// ARGB of `Colour`, resolved through `FEGlobals->`; white when absent.
    pub color: u32,
    /// The `corner` flag: the corner-mark form, which is the only one the
    /// disc authors.
    pub corner: bool,
}

#[cfg(test)]
mod tests;
