//! A front end drawn inside its title's `Safe Area Setting`.
//!
//! HD's Game Options carry `Screen Size` (`Safe Area Setting`, a slider in percent) and the
//! original draws its front-end widgets at `value / 100` about the middle of the screen: on
//! the comparison save (value 96) the Ship Select, Track Select and Campaign Select pages
//! span x 192..1728 where an unscaled 1920 layout spans 160..1760. The picture behind the
//! widgets (the Fury backdrop, the scene backdrop, a movie, a full-screen clear) is not
//! inset. See `docs/formats/hd-frontend.md`, "The front end is drawn inside the safe area".

use oag_display::space::Space;
use oag_hud::{Placement, place};
use oag_ui::frontend::Draw;

/// The placement that draws a `space`-sized grid at `scale` about its centre, or `None` at
/// full size.
#[must_use]
pub fn placement(space: Space, scale: f32) -> Option<Placement> {
    ((scale - 1.0).abs() >= f32::EPSILON).then(|| Placement {
        scale,
        offset: [
            (1.0 - scale) * space.size.0 * 0.5,
            (1.0 - scale) * space.size.1 * 0.5,
        ],
        clip: [-1.0e9, -1.0e9, 2.0e9, 2.0e9],
    })
}

/// `draw` inset by `at`, or `None` when it stays as it was because it is a picture rather
/// than a widget: a video or a backdrop, and a fill that covers the whole `space` (the page's
/// clear colour).
#[must_use]
pub fn inset(draw: &Draw, at: &Placement, space: Space) -> Option<Draw> {
    let whole = match draw {
        Draw::Video { .. } | Draw::FuryBackdrop(_) | Draw::SceneBackdrop(_) => true,
        Draw::Fill { rect, .. } => {
            rect[0] <= 0.0
                && rect[1] <= 0.0
                && rect[0] + rect[2] >= space.size.0
                && rect[1] + rect[3] >= space.size.1
        }
        _ => false,
    };
    if whole {
        return None;
    }
    // The clip is the whole plane, so `place` only returns `None` for the three pictures
    // handled above.
    place(draw, at)
}

/// A point of the screen's grid moved back out of the inset, so a pointer lands on the
/// widget drawn under it.
#[must_use]
pub fn uninset(point: (f32, f32), scale: f32, space: Space) -> (f32, f32) {
    let centre = (space.size.0 * 0.5, space.size.1 * 0.5);
    (
        (point.0 - centre.0) / scale + centre.0,
        (point.1 - centre.1) / scale + centre.1,
    )
}

#[cfg(test)]
mod tests;
