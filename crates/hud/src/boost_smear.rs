//! A speed pad's boost on the HUD: every widget drawn several times, and a glow under the speed bar.
//!
//! Pulse PSP's `Hud_Update` (`0x0881bf50`, `0x0881c2cc`-`0x0881c3b0`) reads the player's exhaust
//! flare, whose `+0xb8` is the boost timer (`0.8` s from a pad hit) and `+0xb4` its length, and
//! counts `n = trunc(10 * w)` with `w` a triangle of `timer / length` - up from `0` at the hit to
//! `1` halfway, back to `0` at the end - or `1` while no boost runs. Both widget draw functions
//! (`0x088a7a5c` for an image, `0x088ce668` for text) read that count and draw their widget `n`
//! times at `alpha / n`, copy `i` shifted by `i` steps of the widget's direction away from the
//! screen's centre (the GE's screen offset, truncated to whole pixels, so a copy lands `ceil` or
//! `floor` of a fraction of a step away). A text widget draws once more before its `n` copies.
//! `Hud_UpdateSpeedBar` (`0x0881a3e0`) hands the speed bar's fill `trunc(n / 10 * 255)` when `n` is
//! at least `2`, which its draw writes into the framebuffer's alpha - the glow mask the bloom's
//! bright pass reads - through a stencil replace. That is the whole of the boost's HUD "glow": the
//! bloom the frame has anyway, seeing a bar that now has a mask. See
//! `docs/ghidra/functions/psp-pulse-usa/hud-boost-smear.md`.
//!
//! **Measured**: the count, the stamp, the shifts and the alpha division, on a PPSSPP frame at
//! `n = 2` (a GE list) and at `n = 3` and `2` (the framebuffer's alpha, `76` and `51`), and the
//! directions of three images. **Chosen, not measured**: a text widget's direction is the line
//! through its anchor where the original quantises it to one of 32 table entries, and every draw
//! of the frame is smeared, the lock-on reticle included.

use oag_ui::frontend::Draw;

use super::Readout;
use super::draw::Frame;

/// The copies a boost draws: `1` with no boost, else `max(1, n)`.
#[must_use]
pub fn copies(art: &oag_title::HudArt, readout: &Readout) -> u32 {
    let Some(law) = art.boost_smear else {
        return 1;
    };
    count(law, readout.boost_phase).max(1)
}

fn count(law: oag_title::hud::BoostSmear, phase: f32) -> u32 {
    if phase <= 0.0 {
        return 1;
    }
    let r = phase.min(1.0);
    let w = if r <= 0.5 { r * 2.0 } else { (1.0 - r) * 2.0 };
    (w * f32::from(law.peak)) as u32
}

/// What the speed bar's fill writes into the glow mask: `0` below two copies.
#[must_use]
pub fn stamp(art: &oag_title::HudArt, readout: &Readout) -> u8 {
    let Some(law) = art.boost_smear else {
        return 0;
    };
    let n = count(law, readout.boost_phase);
    if n < 2 {
        return 0;
    }
    (n as f32 / f32::from(law.peak) * 255.0) as u8
}

/// A rectangle of the frame whose pixels the bloom's bright pass reads at `value`, not at the
/// scene's own glow mask: `[x, y, width, height]` in the layout's grid.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GlowStamp {
    /// The covered rectangle.
    pub rect: [f32; 4],
    /// The mask byte written there.
    pub value: u8,
}

/// Copy `i`'s shift along `dir`: the GE offset `base - d * i` truncated, moved to whole pixels.
fn shift(dir: [f32; 2], i: u32, base: [f32; 2]) -> [f32; 2] {
    let step = i as f32;
    [
        base[0] - (base[0] - dir[0] * step).trunc(),
        base[1] - (base[1] - dir[1] * step).trunc(),
    ]
}

fn direction(from: [f32; 2], centre: [f32; 2]) -> [f32; 2] {
    let d = [from[0] - centre[0], from[1] - centre[1]];
    let length = d[0].hypot(d[1]);
    if length <= f32::EPSILON {
        [0.0, 0.0]
    } else {
        [d[0] / length, d[1] / length]
    }
}

fn dimmed(draw: &Draw, copies: u32) -> Draw {
    let k = 1.0 / copies as f32;
    let alpha = |c: [f32; 4]| [c[0], c[1], c[2], ((c[3] * 255.0 * k).trunc()) / 255.0];
    let mut out = draw.clone();
    match &mut out {
        Draw::Sprite { color, .. } | Draw::RotatedSprite { color, .. } => *color = alpha(*color),
        Draw::GradientSprite { left, right, .. } => {
            *left = alpha(*left);
            *right = alpha(*right);
        }
        Draw::Text { color, border, .. } => {
            *color = alpha(*color);
            *border = border.map(alpha);
        }
        _ => {}
    }
    out
}

fn anchor(draw: &Draw) -> Option<[f32; 2]> {
    match draw {
        Draw::Sprite { rect, .. }
        | Draw::RotatedSprite { rect, .. }
        | Draw::GradientSprite { rect, .. } => Some([rect[0], rect[1]]),
        Draw::Text { x, y, .. } => Some([*x, *y]),
        _ => None,
    }
}

fn moved(draw: &Draw, by: [f32; 2]) -> Draw {
    let mut out = draw.clone();
    match &mut out {
        Draw::Sprite { rect, .. }
        | Draw::RotatedSprite { rect, .. }
        | Draw::GradientSprite { rect, .. } => {
            rect[0] += by[0];
            rect[1] += by[1];
        }
        Draw::Text { x, y, .. } => {
            *x += by[0];
            *y += by[1];
        }
        _ => {}
    }
    out
}

fn expand(list: &mut Vec<Draw>, copies: u32, centre: [f32; 2], text: bool) {
    let base = [1808.0, 1912.0];
    let mut out = Vec::with_capacity(list.len() * copies as usize);
    for draw in list.iter() {
        let Some(at) = anchor(draw) else {
            out.push(draw.clone());
            continue;
        };
        let dir = direction(at, centre);
        let dim = dimmed(draw, copies);
        if text {
            out.push(dim.clone());
        }
        for i in 0..copies {
            out.push(moved(&dim, shift(dir, i, base)));
        }
    }
    *list = out;
}

/// Draws every widget of `frame` `n` times for this boost and records the speed bar fill's glow
/// stamp in [`Frame::glow`], or leaves the frame alone with no boost.
pub(super) fn smear(
    frame: &mut Frame,
    art: &oag_title::HudArt,
    readout: &Readout,
    fill: &[[f32; 4]],
) {
    let Some(law) = art.boost_smear else {
        return;
    };
    let n = copies(art, readout);
    if n < 2 {
        return;
    }
    let centre = [f32::from(law.centre[0]), f32::from(law.centre[1])];
    let value = stamp(art, readout);
    if value > 0 {
        let base = [1808.0, 1912.0];
        for rect in fill {
            let dir = direction([rect[0], rect[1]], centre);
            for i in 0..n {
                let by = shift(dir, i, base);
                frame.glow.push(GlowStamp {
                    rect: [rect[0] + by[0], rect[1] + by[1], rect[2], rect[3]],
                    value,
                });
            }
        }
    }
    expand(&mut frame.sprites, n, centre, false);
    expand(&mut frame.hud_text, n, centre, true);
    expand(&mut frame.small_text, n, centre, true);
    expand(&mut frame.default_text, n, centre, true);
    expand(&mut frame.skin_text, n, centre, true);
}

#[cfg(test)]
mod tests;
