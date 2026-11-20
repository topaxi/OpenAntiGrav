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
    /// The widget's `transitiontype="vwipe"`, when it authors one.
    pub wipe: Option<super::Vwipe>,
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
    /// The XML's `delay`: seconds after the page becomes current before the marks appear.
    /// Unset is 0.
    pub delay: f32,
    /// `EnableTransition`, else `Transition`, else [`super::BRACKET_RAMP_SECONDS`]: how long the
    /// marks take to move out to the corners. An authored `0` means they are there at once.
    pub seconds: f32,
}

impl Bracket {
    /// How far in from its corner each mark sits `since_entry` seconds after the page became
    /// current, as `[along the width, along the height]`; `None` while the `delay` runs.
    ///
    /// `Bracket_Update` skips the draw while the countdown is above 0, and `Bracket_Render` pulls
    /// each mark in by `(1 - alpha) * extent / 3` on each axis, so the marks open out from a
    /// third of the way in to the corners over the in-time (`docs/ghidra/functions/ps3-hdfury-eu/image-widget.md`,
    /// "The corner brackets"). The bracket is its own widget: the image it is written inside
    /// does not wipe it.
    #[must_use]
    pub fn inset(&self, since_entry: f32) -> Option<[f32; 2]> {
        let ramp = super::ramp_seconds(self.delay, since_entry)?;
        let alpha = if self.seconds <= 0.0 {
            1.0
        } else {
            (ramp / self.seconds).clamp(0.0, 1.0)
        };
        let closed = (1.0 - alpha) / 3.0;
        Some([self.width * closed, self.height * closed])
    }
}

#[cfg(test)]
mod tests;
