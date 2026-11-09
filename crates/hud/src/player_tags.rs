//! Multiplayer tags: `PlrTag0`-`PlrTag7` of `MPTag_HUD.xml`, a player's name over each other
//! person's craft in a multiplayer race.
//!
//! The law is `Hud_UpdateMultiplayerTags` (`docs/ghidra/functions/psp-pulse-usa/hud-game-options.md`,
//! confidence 80):
//!
//! - a tag's fade rises by `dt * 1.2` up to 1 ([`step_fade`]);
//! - the anchor is the craft raised by `fade * 2.0` units ([`raise`]), projected to the screen;
//! - alpha is `min(fade, f(distance))`, `f` being 1 up to 20 units and falling linearly to 0 at
//!   60 ([`alpha`]); the option off forces it to 0;
//! - the colour is white, the scale and font are the `PlrTag` label's own.
//!
//! The layout authors no `x` or `y` for the eight labels: the position is the projected anchor,
//! worked out where the camera is ([`Readout::player_tags`]), and the label only supplies the
//! face, the scale and the alignment.
//!
//! **Chosen, not measured:** the raise runs along the craft's own up (the original's axis is
//! unread), and only the other person-flown craft are tagged (the original tags every active
//! slot of a multiplayer race, which on this engine's grid are the people; an AI craft's slot
//! is not offered a name). The name is the lobby's where the race has one and `Player N`
//! otherwise.

use crate::draw::{Context, Frame};
use crate::{Readout, VertAlign};
use oag_ui::frontend::Draw;

/// How fast a tag fades in, per second (`dt * 1.2`).
pub const FADE_IN_PER_SECOND: f32 = 1.2;
/// How far above the craft a fully faded-in tag sits, in world units.
pub const RAISE: f32 = 2.0;
/// Up to this distance a tag is fully opaque.
pub const FULL_RANGE: f32 = 20.0;
/// Past [`FULL_RANGE`], the distance over which it falls to nothing (gone at 60).
pub const FADE_RANGE: f32 = 40.0;

/// One tag, already projected into the grid the view's reticle uses.
#[derive(Debug, Clone, PartialEq)]
pub struct PlayerTag {
    /// Where the raised anchor lands, in the view's sight grid, `y` down.
    pub screen: [f32; 2],
    /// The law's alpha, `(0, 1]`.
    pub alpha: f32,
    /// What the tag says.
    pub name: String,
}

/// The fade after one more step of `dt` seconds.
#[must_use]
pub fn step_fade(fade: f32, dt: f32) -> f32 {
    (fade + dt * FADE_IN_PER_SECOND).min(1.0)
}

/// How far above the craft the anchor sits at `fade`.
#[must_use]
pub fn raise(fade: f32) -> f32 {
    fade * RAISE
}

/// The tag's alpha at `distance` world units from the camera. `on` is the option: off forces 0.
#[must_use]
pub fn alpha(fade: f32, distance: f32, on: bool) -> f32 {
    if !on {
        return 0.0;
    }
    let range = if distance <= FULL_RANGE {
        1.0
    } else {
        ((FADE_RANGE - (distance - FULL_RANGE)) / FADE_RANGE).max(0.0)
    };
    fade.min(range).clamp(0.0, 1.0)
}

/// Draws `readout`'s tags into `frame` in the `PlrTag` label's face, in the grid they were
/// projected in. Nothing when the context carries no label (the layout did not load) or no tag
/// is visible.
pub fn append(frame: &mut Frame, cx: &Context<'_>, readout: &Readout) {
    let Some(label) = cx.tag_label else {
        return;
    };
    let line_height = match label.font {
        crate::Font::Small => cx.small_line_height,
        crate::Font::Default => cx.default_line_height,
        crate::Font::Skin => cx.skin_line_height,
        crate::Font::Hud => cx.hud_line_height,
    };
    for tag in readout.player_tags.iter().filter(|tag| tag.alpha > 0.0) {
        // The text sits on the anchor, as the original's `PlrTag` does at its raised point.
        let sitting = crate::Label {
            vertalign: VertAlign::Bottom,
            y: tag.screen[1],
            x: tag.screen[0],
            ..label.clone()
        };
        let mut border = crate::draw::outline_colour(cx, label.border.unwrap_or(cx.default_border));
        border[3] *= tag.alpha;
        let draw = Draw::Text {
            x: sitting.x,
            y: crate::draw::top_edge(&sitting, line_height),
            scale: label.scale,
            color: crate::draw::lit_colour(cx, [1.0, 1.0, 1.0, tag.alpha]),
            border: Some(border),
            align: label.align,
            text: tag.name.clone(),
            wrap_width: None,
        };
        let shadow = crate::draw::text_shadow(cx, &draw);
        let list = match label.font {
            crate::Font::Small => &mut frame.small_text,
            crate::Font::Default => &mut frame.default_text,
            crate::Font::Skin => &mut frame.skin_text,
            crate::Font::Hud => &mut frame.hud_text,
        };
        list.extend(shadow);
        list.push(draw);
    }
}

#[cfg(test)]
mod tests;
