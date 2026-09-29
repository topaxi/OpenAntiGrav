//! Head2Head's own HUD swap: the `HeadToHeadBar` gap readout.
//!
//! `Arcade_HUD.xml` is shared with the ordinary race, and `Hud_BindWidgets`
//! (`0x0881fbec`) + `FUN_0881d458` (`0x0881d458`) rework four of its widgets
//! when the game mode is 9. Every rule below is read off that pair; see
//! `docs/ghidra/functions/psp-pulse-usa/head2head.md` for the decompile, the
//! field semantics and the confidence of each claim.
//!
//! What draws, from the layout's own widgets:
//!
//! - `Position` keeps its authored anchor and shows the **leader's** row,
//!   `IG_HUD_1ST`.
//! - `Position Outof` becomes the **second** row, `IG_HUD_2ND`, at scale 0.8,
//!   pushed down the screen to `y = clamp(gap / 2, 60, 180)`.
//! - `HeadToHeadBar` is the vertical connector: its authored `y` is the top,
//!   the second row's `y` the bottom, so its runtime height is
//!   `y_second - y_authored` (`widget+0xa0` is `Height` on an `<Image>`, and
//!   slot `+0xec` of its vtable is `GetY` - `docs/ghidra/functions/psp-pulse-usa/head2head.md`).
//! - `PositionOf` becomes the gap label, `"%s%3.0fm"`, centred between the
//!   bar's top and the second row.
//! - `PositionTxt` (`POS`) is hidden.
//!
//! **Measured live 2026-09-29** (PPSSPP, `data/reference/psp-head2head/`): the bar's
//! size, x, width and both clamps, the `"%3.0fm"` label, both colours and the
//! leading swap all match. See `head2head.md` "Live measurement".
//!
//! **Not drawn, and why.**
//!
//! - The rows draw in the HUD font; the original switches both to the `Default`
//!   font (`FUN_088cd3b4(widget, NULL)`, `pulse_text.fnt`, about half the glyph
//!   height). This engine has no `Default` text bucket yet.
//! - Each row is `"%s %s"`, the ordinal then a name: the opponent's is the
//!   team's display name (`Goteki 45` live), the player's is the profile tag
//!   (`AAA` live). This engine carries neither into the readout, so a row
//!   shows its ordinal alone.
//! - The player's row throbs between its colour and white (observed live; `0x200` is inferred to be the cause), period
//!   about 3.1 s. The law is unread, so it is a steady colour here.

use super::draw::{Context, Frame, top_edge};
use super::{Draw, Readout, argb_to_rgba};

/// The gap to the one opponent.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HeadToHead {
    /// `|progress_player - progress_opponent|`, in the units of the race's own
    /// arc length - the number the original prints before its `m`.
    pub gap: f32,
    /// Whether the player is in first place: `*(hud+0x3c)+0x10 == 1`.
    pub player_leads: bool,
}

/// `0xff30ff30`, the leading player's row and gap label. ARGB, the same
/// packing as an authored `Color`.
const LEADING: u32 = 0xFF30_FF30;
/// `0xffff3030`, the trailing player's row and gap label.
const TRAILING: u32 = 0xFFFF_3030;
/// White, the other row (`0xffffffff`, confirmed live).
const WHITE: [f32; 4] = [1.0; 4];
/// `0x3f4ccccd`, the second row's scale.
const SECOND_ROW_SCALE: f32 = 0.8;
/// The clamp on `gap * 0.5`, `0x42700000`/`0x43340000`.
const HALF_MIN: f32 = 60.0;
const HALF_MAX: f32 = 180.0;

/// The widgets this module owns while a Head2Head readout is up.
pub(super) fn owns(name: &str) -> bool {
    matches!(
        name,
        "Position" | "Position Outof" | "PositionOf" | "PositionTxt"
    )
}

/// The Head2Head widgets, or nothing when the layout lacks any of them.
pub(super) fn draws(cx: &Context<'_>, readout: &Readout, frame: &mut Frame) -> bool {
    let Some(state) = readout.head_to_head else {
        return false;
    };
    let layout = cx.layout;
    let (Some(first), Some(second), Some(of), Some(bar)) = (
        layout.label("Position"),
        layout.label("Position Outof"),
        layout.label("PositionOf"),
        layout.fill("HeadToHeadBar"),
    ) else {
        return false;
    };
    let y_authored = bar.rect[1] - bar.origin_y;
    let half = (state.gap * 0.5).clamp(HALF_MIN, HALF_MAX);
    let bottom = bar.origin_y + half;
    let tint = argb_to_rgba(if state.player_leads {
        LEADING
    } else {
        TRAILING
    });
    let (first_colour, second_colour) = if state.player_leads {
        (tint, WHITE)
    } else {
        (WHITE, tint)
    };
    let border = |label: &super::Label| Some(label.border.unwrap_or(cx.default_border));
    let text = |label: &super::Label, y: f32, scale: f32, align, color, text: String| Draw::Text {
        x: label.x,
        y,
        scale,
        color,
        border: border(label),
        align,
        text,
        wrap_width: None,
    };

    frame.sprites.push(Draw::Fill {
        rect: [bar.rect[0], bar.rect[1], bar.rect[2], half - y_authored],
        color: bar.color,
    });
    // The bind copies `Position`'s `x` (`+0x9c`) and align (`+0x168`) onto
    // `Position Outof`, so the two rows share one right edge (measured live
    // 2026-09-29: within 3 px). `PositionOf` keeps its own `x`: the gap label's
    // right edge sat about 8 px past the rows'.
    let second = &super::Label {
        x: first.x,
        ..second.clone()
    };
    let ordinal = |id: &str| cx.strings.get_or_id(id).to_string();
    frame.hud_text.push(text(
        first,
        top_edge(first, cx.hud_line_height),
        first.scale,
        first.align,
        first_colour,
        ordinal("IG_HUD_1ST"),
    ));
    frame.hud_text.push(text(
        second,
        bottom,
        SECOND_ROW_SCALE,
        first.align,
        second_colour,
        ordinal("IG_HUD_2ND"),
    ));
    let middle = bar.origin_y + (y_authored + half) * 0.5;
    let line = cx.hud_line_height * of.scale;
    frame.hud_text.push(text(
        of,
        middle - line / 2.0,
        of.scale,
        first.align,
        tint,
        format!(
            "{}{:3.0}m",
            if state.player_leads { "-" } else { "+" },
            state.gap
        ),
    ));
    true
}

#[cfg(test)]
mod tests;
