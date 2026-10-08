//! The corner marks `<Bracket corner="true">` frames each panel of
//! `Cell Selection` with: the four emblem rows, the right column and the
//! three targets.
//!
//! The screen file authors only each rectangle; the mark is
//! `Data\FE\Images\corner2.gtf`, a 16 by 16 mask of one rounded top-left
//! corner (`docs/formats/gtf.md`), and the settled frame of `09_blitzed`
//! draws it at the four corners of every rectangle, turned to each, in the
//! colour the bracket authors. **Mirrored at the other three corners and
//! placed flush with the rectangle's edges: measured on one frame, confidence
//! 65** - the glyph's own margin and where two touching rectangles meet were
//! compared by eye at 3x.

use oag_ui::frontend::{Draw, Placed};
use oag_ui::screen::{Bracket, argb_to_rgba};

/// The mask's `src`.
pub const SRC: &str = r"Data\FE\Images\corner2.gtf";

/// The mask's side, in texels and in authored units.
const SIDE: f32 = 16.0;

/// Every bracket's four marks.
pub(super) fn draws(brackets: &[Bracket], sprites: &dyn Fn(&str) -> Option<Placed>) -> Vec<Draw> {
    let Some(mask) = sprites(SRC) else {
        return Vec::new();
    };
    let (mx, my) = (mask.x as f32, mask.y as f32);
    let mut out = Vec::new();
    for bracket in brackets.iter().filter(|bracket| bracket.corner) {
        let color = argb_to_rgba(bracket.color);
        for (right, bottom) in [(false, false), (true, false), (false, true), (true, true)] {
            let x = if right {
                bracket.x + bracket.width - SIDE
            } else {
                bracket.x
            };
            let y = if bottom {
                bracket.y + bracket.height - SIDE
            } else {
                bracket.y
            };
            // A negative extent turns the mask about that axis.
            let uv = [
                if right { mx + SIDE } else { mx },
                if bottom { my + SIDE } else { my },
                if right { -SIDE } else { SIDE },
                if bottom { -SIDE } else { SIDE },
            ];
            out.push(Draw::Sprite {
                rect: [x, y, SIDE, SIDE],
                uv,
                color,
            });
        }
    }
    out
}

#[cfg(test)]
mod tests;
