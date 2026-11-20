//! The `Image` widget's `transitiontype="vwipe"`: a slab sweeps down the quad and back up, then the
//! quad grows from its top edge (`docs/ghidra/functions/ps3-hdfury-eu/image-widget.md`,
//! "What is drawn: `Image_Render`").
//!
//! The geometry is one function of the widget's alpha `s`, a linear ramp over the in-time
//! (`Widget_UpdateTransition`). A caller scales [`Window`] by the quad's height and the texture's
//! rows and draws the quad at full opacity: `vwipe` replaces the alpha-multiply mode.

use super::{Node, Screens};

/// Seconds an `Image` takes to ramp its alpha when the XML authors neither `EnableTransition` nor
/// `Transition` (the class default stored by the constructor, `docs/ghidra/functions/ps3-hdfury-eu/image-widget.md`,
/// "How the XML selects the transition"). Confidence 80.
pub const IMAGE_RAMP_SECONDS: f32 = 0.7;

/// A `vwipe` image: how long its alpha takes to run 0 to 1.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vwipe {
    /// `EnableTransition`, else `Transition`, else [`IMAGE_RAMP_SECONDS`]. An authored `0` is set,
    /// and means the quad is plain from the first frame.
    pub seconds: f32,
}

/// The part of the quad a `vwipe` draws, as fractions of its height. The texture's rows follow the
/// same fractions: the rows pass through unscaled.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Window {
    /// Distance of the window's top edge below the quad's top.
    pub top: f32,
    /// The window's height.
    pub height: f32,
}

impl Vwipe {
    /// The alpha after `elapsed` seconds of the ramp: `clamp(elapsed, 0, T) / T`.
    #[must_use]
    pub fn alpha(self, elapsed: f32) -> f32 {
        if self.seconds <= 0.0 {
            1.0
        } else {
            (elapsed / self.seconds).clamp(0.0, 1.0)
        }
    }

    /// The window `Image_Render` draws at alpha `s`, `None` at `s == 1` where it draws the plain quad.
    #[must_use]
    pub fn window(s: f32) -> Option<Window> {
        const SLAB: f32 = 0.1;
        if s >= 1.0 {
            return None;
        }
        let s = s.max(0.0);
        Some(if s < 0.4 {
            Window {
                top: 2.25 * s,
                height: SLAB,
            }
        } else if s < 0.8 {
            Window {
                top: 0.9 + (s - 0.4) * -2.25,
                height: SLAB,
            }
        } else {
            Window {
                top: 0.0,
                height: SLAB + (s - 0.8) * 4.5,
            }
        })
    }
}

impl Screens {
    /// `node`'s `vwipe`, when it authors `transitiontype="vwipe"` (case-insensitive, as the
    /// executable compares it) and its in-time.
    #[must_use]
    pub fn vwipe_from_node(&self, node: &Node) -> Option<Vwipe> {
        if !node
            .value("transitiontype")
            .is_some_and(|kind| kind.eq_ignore_ascii_case("vwipe"))
        {
            return None;
        }
        let own = self
            .number(node.value("EnableTransition"))
            .or_else(|| self.number(node.value("Transition")));
        Some(Vwipe {
            seconds: own.unwrap_or(IMAGE_RAMP_SECONDS),
        })
    }
}

#[cfg(test)]
mod tests;
