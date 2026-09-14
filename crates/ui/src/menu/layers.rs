//! One page's draws split by what a page change may move, and the shape of
//! that change.
//!
//! Moved out of `menu.rs` whole on 2026-09-14, no behaviour change: that
//! file is baselined by `scripts/check-file-size.py` and the blocks' focus
//! state had pushed it past its ceiling. This was the seam it already had -
//! two types and one helper that [`super::draw_list`] hands back and
//! `oag_game`'s menu stage animates, touching nothing else here.

use crate::frontend::Draw;

/// One page's draws, split by what a page change is allowed to move.
///
/// **The split is the original's, not a convenience.** A capture of Pulse
/// changing pages shows the row block scaling up and fading while the top bar
/// and the footer crossfade in place, so a flat list cannot express the
/// transition: something has to say which draws zoom. See
/// `docs/ui/menus-original.md`.
///
/// [`Self::flatten`] puts them back together for a caller that is not
/// animating, in paint order.
#[derive(Debug, Default, Clone)]
pub struct Layers {
    /// The looping movie behind everything. Never moves, never fades.
    pub backdrop: Vec<Draw>,
    /// Framing that stays put across a page change: the screen title.
    pub chrome: Vec<Draw>,
    /// The rows, which are what a page change animates.
    pub body: Vec<Draw>,
}

impl Layers {
    /// Every draw, back to front.
    #[must_use]
    pub fn flatten(self) -> Vec<Draw> {
        let mut out = self.backdrop;
        out.extend(self.chrome);
        out.extend(self.body);
        out
    }

    /// The body scaled about `origin` and faded to `alpha`.
    ///
    /// This is the whole of the page-change effect. A capture of the original
    /// shows the outgoing page growing and fading while the incoming one grows
    /// into place from smaller and fades in, with the chrome crossfading where
    /// it stands - so one function, called twice with different arguments,
    /// covers both halves.
    ///
    /// Only [`Self::body`] moves. The backdrop is a looping movie that runs
    /// across page changes untouched, and the chrome is what the capture shows
    /// staying put.
    #[must_use]
    pub fn zoomed(mut self, origin: (f32, f32), scale: f32, alpha: f32) -> Self {
        for draw in &mut self.body {
            draw.zoom(origin, scale, alpha);
        }
        for draw in &mut self.chrome {
            // Faded but not moved, which is the split the capture shows.
            fade(draw, alpha);
        }
        self
    }
}

/// Multiplies one draw's alpha, leaving it where it is.
fn fade(draw: &mut Draw, alpha: f32) {
    draw.fade(alpha);
}

/// How a page change looks, in the terms [`Layers::zoomed`] takes.
///
/// **Measured, then rounded.** A capture of Pulse going from `Main Menu` to
/// `Grid Selection` puts the outgoing page at scale 1.19 six frames into a
/// thirteen-frame transition and gone by the end, so it is still growing when
/// it disappears; extrapolating the measured steps to the full duration gives
/// roughly 1.5. The incoming page comes *from* smaller by the same argument
/// run backwards. Alpha holds for the first two frames and then falls.
///
/// See `docs/ui/menus-original.md` for the frame-by-frame table.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transition {
    /// Where the zoom is centred, in the 480x272 space.
    ///
    /// Solving the measured positions puts this near the content block's own
    /// centre rather than the screen's - confidence 55, so it is deliberately
    /// approximate. See the doc page.
    pub origin: (f32, f32),
    /// What the outgoing page has grown to by the time it is gone.
    pub out_scale: f32,
    /// What the incoming page grows from.
    pub in_scale: f32,
}

impl Default for Transition {
    fn default() -> Self {
        Self {
            origin: (150.0, 127.0),
            out_scale: 1.5,
            in_scale: 0.7,
        }
    }
}
