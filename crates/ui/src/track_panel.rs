//! The track-description panel the original lays over the pre-race flyby:
//! `InGameTrackDescriptionScreen`, authored in `InGame_Definition.xml`.
//!
//! The layout is the disc's (two bars with a hexagon tile and a gradient rule each, the
//! circuit's name at the top, its paragraph at the bottom); the two strings are filled in by
//! the caller. What this module owns is the **timing**, measured on PPSSPP (2026-10-02, a
//! breakpoint on `Widget_UpdateTransitionFraction` read every frame of one flyby and its end):
//!
//! | Widget | Enter | Leave |
//! | --- | --- | --- |
//! | the bars, tile, rules and both texts | alpha `t / 0.7`, linear | alpha falls `1 / 0.7` a second from where it was |
//! | the `Viewport` holding the texts | its width wipes open over `2.0` s (its authored `enabletransition`) | closes over `0.7` s, starting from full width once the panel had been up `0.7` s |
//!
//! The 0.7 s is the `+0x68`/`+0x6c` pair the generic widget class carries (read on every widget
//! of the screen but the viewport's own enter time, which the XML authors as `2`); the law is
//! [`Widget_UpdateTransitionFraction`](../../../docs/ghidra/functions/psp-pulse-usa/race-box-screens.md).
//! The wipe is the viewport's fraction times its authored `deltaWidth` of 470: the clip edge of
//! the paragraph followed `470 * fraction` to within a glyph on one circuit (checked against
//! per-frame screenshots, not fitted).
//!
//! The panel has no input of its own: a held Cross skips the flyby it sits on, and a pointer
//! press does the same here - see `oag_game::main::session`.

use crate::campaign::Layout;
use crate::frontend::{Align, Draw, Placed};
use crate::screen::argb_to_rgba;

/// Seconds a widget takes to fade in or out: `+0x68`/`+0x6c` as read on the screen's widgets.
pub const FADE_SECONDS: f32 = 0.7;

/// Seconds the viewport takes to wipe open: the `enabletransition="2"` the XML authors on it.
pub const WIPE_SECONDS: f32 = 2.0;

/// The viewport's `deltaWidth`, the width its wipe opens to.
pub const WIPE_WIDTH: f32 = 470.0;

// The paragraph's `widthlimited` wrap is the viewport less the text's own inset on both sides
// (`WIPE_WIDTH - 2 * x`, 450). **Chosen, not measured**: the XML names no width, and 450 is the one
// round value that reproduces the original's breaks on the two circuits compared (`16_Track`
// breaks before "Concordia", which fits at 460; line one still takes "when public").

/// Where the panel is in its life, in seconds of the original's 60 Hz update.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Progress {
    /// Seconds since the screen came up.
    pub entered: f32,
    /// Seconds since the flyby ended and the screen began to leave, once it has.
    pub left: Option<f32>,
}

impl Progress {
    /// The alpha every widget draws at.
    #[must_use]
    pub fn alpha(&self) -> f32 {
        let reached = (self.entered / FADE_SECONDS).clamp(0.0, 1.0);
        match self.left {
            None => reached,
            Some(left) => (reached - left / FADE_SECONDS).max(0.0),
        }
    }

    /// How much of its width the texts' viewport shows, `0` to `1`.
    ///
    /// Leaving, the viewport's clock is clamped to its disable time, so it starts from the same
    /// fraction the other widgets do.
    #[must_use]
    pub fn wipe(&self) -> f32 {
        match self.left {
            None => (self.entered / WIPE_SECONDS).clamp(0.0, 1.0),
            Some(_) => self.alpha(),
        }
    }

    /// Whether anything is left to draw.
    #[must_use]
    pub fn visible(&self) -> bool {
        self.alpha() > 0.0
    }
}

/// What to draw, split by the face each part is drawn in.
#[derive(Debug, Clone, PartialEq)]
pub struct DrawLists {
    /// The bars, tiles and rules, then the circuit's name last, in the `Menu` face.
    pub frame: Vec<Draw>,
    /// The paragraph, in the `Default` face.
    pub body: Vec<Draw>,
    /// The right edge of the texts' viewport, in screen units: what both texts are clipped to.
    pub right: f32,
}

/// The panel at `progress`, with the circuit's `name` and `description`.
///
/// `sprites` places the tile's texture in the caller's sheet. A tile it cannot place is left
/// out, never replaced.
#[must_use]
pub fn draw_lists(
    layout: &Layout,
    sprites: &dyn Fn(&str) -> Option<Placed>,
    name: &str,
    description: &str,
    progress: Progress,
) -> DrawLists {
    let alpha = progress.alpha();
    let mut frame = Vec::new();
    for fill in &layout.screen.fills {
        let mut draw = crate::picker::fill_draw(fill);
        draw.fade(alpha);
        frame.push(draw);
    }
    for image in &layout.screen.images {
        let Some(placed) = sprites(&image.src) else {
            continue;
        };
        let mut draw = crate::campaign::draw::sprite_draw(image, placed, image.x, image.y, image.color);
        draw.fade(alpha);
        frame.push(draw);
    }
    let mut body = Vec::new();
    for text in &layout.screen.texts {
        let content = match text.name.as_deref() {
            Some(oag_pulse::race::TRACK_NAME_WIDGET) => name,
            Some(oag_pulse::race::TRACK_TEXT_WIDGET) => description,
            _ => continue,
        };
        let mut draw = Draw::Text {
            x: text.x,
            y: text.y,
            scale: text.scale,
            color: argb_to_rgba(text.color),
            border: None,
            align: Align::Left,
            text: content.to_string(),
            wrap_width: text.wrap_width.or_else(|| {
                text.name
                    .as_deref()
                    .filter(|n| *n == oag_pulse::race::TRACK_TEXT_WIDGET)
                    .map(|_| WIPE_WIDTH - 2.0 * text.x)
            }),
        };
        draw.fade(alpha);
        if text.name.as_deref() == Some(oag_pulse::race::TRACK_NAME_WIDGET) {
            frame.push(draw);
        } else {
            body.push(draw);
        }
    }
    DrawLists {
        frame,
        body,
        right: WIPE_WIDTH * progress.wipe(),
    }
}

#[cfg(test)]
mod tests;
