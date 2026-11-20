//! The corner marks `<Bracket corner="true">` frames each panel of
//! `Cell Selection` with: the four emblem rows, the right column and the
//! three targets.
//!
//! The screen file authors only each rectangle; the mark is
//! `Data\FE\Images\corner2.gtf`, a 16 by 16 mask of one rounded corner, the
//! **bottom-left** one as stored (`docs/formats/gtf.md`; RPCS3 frames of Cell
//! Selection and Ship Select, 2026-10-27, draw the top-left mark as the
//! texture turned over), and the settled frame of `09_blitzed`
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

/// Every bracket's four marks, `since_entry` seconds after the page became current: none while
/// a bracket's own `delay` runs, then drawn in from the corners ([`Bracket::inset`]). A page
/// that does not play its brackets passes [`oag_ui::screen::EntryClock::finished`]'s seconds.
pub fn draws(
    brackets: &[Bracket],
    sprites: &dyn Fn(&str) -> Option<Placed>,
    since_entry: f32,
) -> Vec<Draw> {
    draws_by(brackets, sprites, &|_| since_entry)
}

/// [`draws`] for a page that arms its brackets on more than one clock: `since` gives the seconds a
/// bracket has counted its `delay` for.
pub fn draws_by(
    brackets: &[Bracket],
    sprites: &dyn Fn(&str) -> Option<Placed>,
    since: &dyn Fn(&Bracket) -> f32,
) -> Vec<Draw> {
    let Some(mask) = sprites(SRC) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for bracket in brackets.iter().filter(|bracket| bracket.corner) {
        let Some([inset_x, inset_y]) = bracket.inset(since(bracket)) else {
            continue;
        };
        let color = argb_to_rgba(bracket.color);
        for (right, bottom) in [(false, false), (true, false), (false, true), (true, true)] {
            let x = if right {
                bracket.x + bracket.width - SIDE - inset_x
            } else {
                bracket.x + inset_x
            };
            let y = if bottom {
                bracket.y + bracket.height - SIDE - inset_y
            } else {
                bracket.y + inset_y
            };
            // A negative extent turns the mask about that axis. The file's
            // own corner is its bottom-left one (an RPCS3 frame of Cell
            // Selection and of Ship Select draws the top-left mark as the
            // texture turned over), so the top marks are the flipped ones.
            let uv = mask.sheet_rect([
                if right { SIDE } else { 0.0 },
                if bottom { 0.0 } else { SIDE },
                if right { -SIDE } else { SIDE },
                if bottom { SIDE } else { -SIDE },
            ]);
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
