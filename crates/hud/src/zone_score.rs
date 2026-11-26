//! Where a Zone run's `Score` value hangs: a title fact
//! ([`oag_title::ZoneScore`]), since the layout authors the widget left-anchored at its
//! frame's origin and the original moves it at run time.

use oag_title::ZoneScore;
use oag_ui::font::{Cell, cells_ink_width};
use oag_ui::frontend::Align;

use super::{Context, Font, Label, Readout};

/// The `HUD` face's `3` and the atlas's texel scale: the glyph the original's template
/// string repeats.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TemplateDigit {
    /// The glyph's metrics.
    pub cell: Cell,
    /// [`oag_ui::font::Atlas::texel_scale`] of the face it came from.
    pub texel_scale: f32,
}

impl TemplateDigit {
    /// The `3` of `atlas`, or `None` for a face without one.
    #[must_use]
    pub fn of(atlas: &oag_ui::font::Atlas) -> Option<Self> {
        Some(Self {
            cell: atlas.cell('3')?,
            texel_scale: atlas.texel_scale,
        })
    }

    /// The width of `digits` of them at `scale`, as the original measures it
    /// (`Font_MeasureString`, pens rounded up).
    fn width(self, digits: usize, scale: f32) -> f32 {
        cells_ink_width(&vec![self.cell; digits], scale * self.texel_scale, true)
    }
}

/// What `label`'s authored anchor becomes: the shift along x and the alignment.
///
/// A Zone run's left-anchored `Score` alone: an Eliminator's keeps the authored anchor, and
/// the retro skins author theirs right-aligned.
pub(super) fn place(
    cx: &Context<'_>,
    readout: &Readout,
    label: &Label,
    text: &str,
) -> (f32, Align) {
    if readout.zone == 0 || label.name != "Score" || label.align != Align::Left {
        return (0.0, label.align);
    }
    match cx.art.zone_score {
        ZoneScore::Authored => (0.0, label.align),
        ZoneScore::CentredOnValue => (0.0, Align::Centre),
        ZoneScore::CentredOnThrees => {
            // The width is measured at the widget's horizontal scale.
            let width = cx
                .template_digit
                .filter(|_| label.font == Font::Hud)
                .map_or(0.0, |digit| {
                    digit.width(text.chars().count(), label.scale * label.stretch)
                });
            (-0.5 * width, label.align)
        }
    }
}

#[cfg(test)]
mod tests;
