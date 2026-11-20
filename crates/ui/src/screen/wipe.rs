//! The `Image` widget's `transitiontype="vwipe"`: a slab sweeps down the quad and back up, then the
//! quad grows from its top edge (`docs/ghidra/functions/ps3-hdfury-eu/image-widget.md`,
//! "What is drawn: `Image_Render`").
//!
//! The geometry is one function of the widget's alpha `s`, a linear ramp over the in-time
//! (`Widget_UpdateTransition`). A caller scales [`Window`] by the quad's height and the texture's
//! rows and draws the quad at full opacity: `vwipe` replaces the alpha-multiply mode.
//!
//! A page arms every `vwipe` image's countdown with its authored `delay` the frame the page
//! becomes current (`Widget_ArmTransition`, "Page entry"), so [`Vwipe::entry_alpha`] reads an
//! [`EntryClock`] and draws nothing until the delay is out; a restart that does not re-arm the
//! delay (a team step) reads [`Vwipe::alpha`] from the restart.

use super::{Node, Screens};
use crate::frontend::Draw;

/// One front-end frame, the unit the countdown and the ramp are counted in: the first ramp frame
/// is the frame the countdown crosses 0 and already holds one `dt`.
pub const FRAME_SECONDS: f32 = 1.0 / 60.0;

/// Seconds an `Image` takes to ramp its alpha when the XML authors neither `EnableTransition` nor
/// `Transition` (the class default stored by the constructor, `docs/ghidra/functions/ps3-hdfury-eu/image-widget.md`,
/// "How the XML selects the transition"). Confidence 80.
pub const IMAGE_RAMP_SECONDS: f32 = 0.7;

/// A `vwipe` image: how long its alpha takes to run 0 to 1, and how long the page waits first.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vwipe {
    /// `EnableTransition`, else `Transition`, else [`IMAGE_RAMP_SECONDS`]. An authored `0` is set,
    /// and means the quad is plain from the first frame.
    pub seconds: f32,
    /// The XML's `delay`: seconds after the page becomes current before the ramp starts. Unset is 0.
    pub delay: f32,
}

/// Seconds since a page became current, which every `vwipe` image on it counts its `delay` from.
///
/// Starts at zero where the page is built; a still that wants the finished page uses
/// [`Self::finished`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EntryClock {
    seconds: f32,
}

impl Default for EntryClock {
    fn default() -> Self {
        Self { seconds: 0.0 }
    }
}

impl EntryClock {
    /// A clock at `seconds`, for a still drawn a stated time after the page opened.
    #[must_use]
    pub fn at(seconds: f32) -> Self {
        Self {
            seconds: seconds.max(0.0),
        }
    }

    /// A clock past every wipe: what a still with no stated time draws.
    #[must_use]
    pub const fn finished() -> Self {
        Self {
            seconds: f32::INFINITY,
        }
    }

    /// Advances by one tick.
    pub fn tick(&mut self, dt: f32) {
        self.seconds += dt.max(0.0);
    }

    /// Seconds since the page became current.
    #[must_use]
    pub fn seconds(self) -> f32 {
        self.seconds
    }
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

impl Window {
    /// `rect` and `uv` (each `[x, y, width, height]`) cut to this window: the quad's rows and the
    /// texture's rows pass through at the same fractions of their height.
    #[must_use]
    pub fn cut(self, rect: [f32; 4], uv: [f32; 4]) -> ([f32; 4], [f32; 4]) {
        let [x, y, width, height] = rect;
        let [u, v, texels_wide, texels_high] = uv;
        (
            [x, y + height * self.top, width, height * self.height],
            [
                u,
                v + texels_high * self.top,
                texels_wide,
                texels_high * self.height,
            ],
        )
    }
}

impl Vwipe {
    /// The alpha `since_entry` seconds after the page became current, `None` while the widget
    /// waits out its `delay` (`Image_Render` returns while the countdown is above 0).
    ///
    /// The countdown starts at the delay on the entry frame and falls a frame at a time; the first
    /// ramp frame is the one it crosses 0 on and holds one frame already. Half a frame of slack
    /// keeps an accumulated clock off the wrong side of that frame.
    #[must_use]
    pub fn entry_alpha(self, since_entry: f32) -> Option<f32> {
        let ramp = since_entry - self.delay + FRAME_SECONDS;
        (ramp > FRAME_SECONDS * 0.5).then(|| self.alpha(ramp))
    }

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
            delay: self.number(node.value("delay")).unwrap_or(0.0).max(0.0),
        })
    }
}

/// `draw`, the plain quad of an image that may author a `vwipe`, as the page shows it
/// `since_entry` seconds after it became current: nothing while the delay runs, the window of the
/// ramp, then the quad whole. A draw that is not a plain sprite is shown whole once the delay is
/// out.
#[must_use]
pub fn entry_draw(wipe: Option<Vwipe>, since_entry: f32, draw: Draw) -> Option<Draw> {
    let Some(wipe) = wipe else {
        return Some(draw);
    };
    let alpha = wipe.entry_alpha(since_entry)?;
    let (Some(window), Draw::Sprite { rect, uv, color }) = (Vwipe::window(alpha), &draw) else {
        return Some(draw);
    };
    let (rect, uv) = window.cut(*rect, *uv);
    Some(Draw::Sprite { rect, uv, color: *color })
}

#[cfg(test)]
mod tests;
