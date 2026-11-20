//! Multiplayer tags: `PlrTag0`-`PlrTag7` of `MPTag_HUD.xml`, a player's name over each other
//! person's craft in a multiplayer race.
//!
//! The law is `Hud_UpdateMultiplayerTags` (`docs/ghidra/functions/psp-pulse-usa/hud-game-options.md`,
//! confidence 80):
//!
//! - a tag's fade rises by `dt * 1.2` up to 1 ([`step_fade`]);
//! - the anchor is the craft raised by `fade * 2.0` units ([`raise`]), projected to the screen;
//! - a craft is tagged only where the view's track-section row holds its section ([`in_view_row`],
//!   `FUN_0883e37c`);
//! - alpha is `min(fade, f(distance))`, `f` being 1 up to 20 units and falling linearly to 0 at
//!   60 ([`alpha`]); the option off forces it to 0;
//! - the colour is white, the scale and font are the `PlrTag` label's own.
//!
//! The layout authors no `x` or `y` for the eight labels: the position is the projected anchor,
//! worked out where the camera is ([`Readout::player_tags`]), and the label only supplies the
//! face, the scale and the alignment.
//!
//! The raise runs along the world's Y axis, not the craft's own up: the original adds it to the
//! second float of the craft's translation row. The local craft has no tag (its kind byte is 0,
//! and a tag is active only for a non-zero one).
//!
//! **Chosen, not measured:** only the other person-flown craft are tagged (the original tags
//! every craft built with a non-zero kind, AI included; whether a multiplayer race has any is
//! unread). The name is the lobby's where the race has one and `Player N` otherwise.

use crate::draw::{Context, Frame};
use crate::{Readout, VertAlign};
use oag_ui::frontend::Draw;

/// Whether a craft in track section `section` is tagged in a view whose visible-section row is
/// `row` (`FUN_0883e37c`): the row's bit for the section is set. A section outside `0..64`, or
/// one the track does not name (`None`), is tagged unconditionally.
#[must_use]
pub fn in_view_row(section: Option<u8>, row: u64) -> bool {
    match section {
        Some(section) if section < 64 => row & (1u64 << section) != 0,
        _ => true,
    }
}

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

/// Where the tag anchors for a craft at `position` (world space): raised along the world's Y by
/// [`raise`], whatever way the craft banks (`Hud_UpdateMultiplayerTags` adds it to the
/// translation row's second float).
#[must_use]
pub fn anchor(position: [f32; 3], fade: f32) -> [f32; 3] {
    [position[0], position[1] + raise(fade), position[2]]
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
