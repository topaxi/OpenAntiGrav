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
//! - Both rows draw in the `Default` font (`FUN_088cd3b4(widget, NULL)`, `pulse_text.fnt`,
//!   about half the HUD glyph height), as the original does.
//! - `Position` keeps its authored anchor and shows the **leader's** row,
//!   `IG_HUD_1ST`.
//! - `Position Outof` becomes the **second** row, `IG_HUD_2ND`, at scale 0.8,
//!   pushed down the screen to `y = clamp(gap / 2, 60, 180)` - `[40, 85]` in the top and
//!   bottom cut, whose leader row also sits on the bar's top (`Split ScreenH`, PS2
//!   `FUN_001371c8`, confidence 85).
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
//! - Each row is `"%s %s"`, the ordinal then a name: in a split both are the craft's team
//!   name (`1st Qirex` live); with one person the opponent's is the team's display name
//!   (`Goteki 45` live) and the player's the profile tag (`AAA` live). This engine has
//!   no profile tag, so that one row shows its ordinal alone.
//! - The player's row throbs between its colour and white (observed live; `0x200` is inferred to be the cause), period
//!   about 3.1 s. The law is unread, so it is a steady colour here.

use super::draw::{Context, Frame, top_edge};
use super::{Draw, Readout, VertAlign, argb_to_rgba};

/// The gap to the one opponent.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct HeadToHead {
    /// `|progress_player - progress_opponent|`, in the units of the race's own
    /// arc length - the number the original prints before its `m`.
    pub gap: f32,
    /// Whether the player is in first place: `*(hud+0x3c)+0x10 == 1`.
    pub player_leads: bool,
    /// The team id this view's own craft flies (`Feisar`, which the string table turns into
    /// its display name), `None` where the original draws the profile tag instead (one
    /// person, `AAA` live) and this build has none: that row then shows its ordinal alone.
    pub own_team: Option<String>,
    /// The team id of the other craft, `None` when the race records no teams (a hand-built one).
    pub other_team: Option<String>,
    /// Whether this view is one of two cut top and bottom (`Split ScreenH`), which the
    /// original gives a shorter bar and the leader row on the bar's top; set by the caller
    /// that knows the cut.
    pub stacked: bool,
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
/// The clamp on `gap * 0.5`, `0x42700000`/`0x43340000`: one view, and the left and right cut.
const HALF_MIN: f32 = 60.0;
const HALF_MAX: f32 = 180.0;
/// The same clamp in the top and bottom cut, `0x42200000`/`0x42aa0000` (`FUN_001371c8`'s
/// `g_split_mode == 1` branch).
const STACKED_HALF_MIN: f32 = 40.0;
const STACKED_HALF_MAX: f32 = 85.0;

/// The widgets this module owns while a Head2Head readout is up.
pub(super) fn owns(name: &str) -> bool {
    matches!(
        name,
        "Position" | "Position Outof" | "PositionOf" | "DistanceApart" | "PositionTxt"
    )
}

/// HD's gap label, where Pulse's is `PositionOf`: the same widget under another name.
const HD_GAP_LABEL: &str = "DistanceApart";
/// HD's second row sits at `clamp(2 * gap, 240, 720)` (`Hud_UpdatePositionCounter`, `0x00096088`),
/// in the layout's 1920-wide space: exactly 4x Pulse's `clamp(gap / 2, 60, 180)`, with no
/// separate top-and-bottom clamp.
const HD_ROW_MIN: f32 = 240.0;
const HD_ROW_MAX: f32 = 720.0;

/// Whether the layout is HD's Head to Head shape (an authored `DistanceApart`).
fn is_hd_shape(layout: &super::Layout) -> bool {
    layout.label("PositionOf").is_none() && layout.label(HD_GAP_LABEL).is_some()
}

/// Whether the place panel and its arc are hidden: HD's bind clears the visible flags of
/// `PositionTxt` and `PositionPanel` in mode 9 (`Hud_BindWidgets`, `0x0009e0ec`).
pub(super) fn hides_place_panel(cx: &Context<'_>, readout: &Readout) -> bool {
    readout.head_to_head.is_some() && is_hd_shape(cx.layout)
}

/// The gap label's scale where it stands under an authored place: a little over the
/// 0.6 of `Position Outof`, which 2048's `Hud` font makes too small to read at a view.
const BESIDE_GAP_SCALE: f32 = 0.8;
/// A place that sits in a panel (`vertalign="middle"`, HD's hex) gets the gap this many of
/// its own lines below its centre, which clears the panel; a bare number gets a quarter line.
const PANEL_CLEARANCE_LINES: f32 = 0.5;
const BARE_CLEARANCE_LINES: f32 = 0.25;

/// The Head2Head widgets, or nothing when the layout lacks any of them.
///
/// Three shapes, by what the layout authors:
///
/// - HD's and Omega's: `DistanceApart` for the gap label. The place panel is hidden, `Position` and
///   `Position Outof` carry `1ST`/`2ND` (`clamp(2 * gap, 240, 720)`), the bar runs from its
///   authored `y` to the second row. `Hud_UpdatePositionCounter`, confidence 75-80; the ordinal-only rows are chosen.
///
/// - Pulse's own: `HeadToHeadBar`, `Position`, `Position Outof` and `PositionOf` all there
///   (Pulse PSP and PS2). Measured.
/// - Every other: Pure authors no Head to Head and 2048 a bare `Position`. The place stands as authored (`2/2`, a hex
///   panel) and the gap sits under it ([`gap_under_place`]). **Chosen, not measured**: it
///   borrows Pulse's gap label and colours, not its rows, whose ordinals only Pulse has.
///
/// Returns whether the place widgets are replaced ([`owns`]).
pub(super) fn draws(cx: &Context<'_>, readout: &Readout, frame: &mut Frame) -> bool {
    let Some(state) = readout.head_to_head.as_ref() else {
        return false;
    };
    let layout = cx.layout;
    let hd = is_hd_shape(layout);
    let (Some(first), Some(second), Some(of), Some(bar)) = (
        layout.label("Position"),
        layout.label("Position Outof"),
        layout
            .label("PositionOf")
            .or_else(|| layout.label(HD_GAP_LABEL)),
        layout.fill("HeadToHeadBar"),
    ) else {
        gap_under_place(cx, state, frame);
        return false;
    };
    let y_authored = bar.rect[1] - bar.origin_y;
    let (low, high) = if state.stacked {
        (STACKED_HALF_MIN, STACKED_HALF_MAX)
    } else {
        (HALF_MIN, HALF_MAX)
    };
    let half = if hd {
        (state.gap * 2.0).clamp(HD_ROW_MIN, HD_ROW_MAX)
    } else {
        (state.gap * 0.5).clamp(low, high)
    };
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

    // HD's bar is `Hud_UpdatePositionCounter`'s `bar+0xb0 = D - GetY()`: `+0xb0` is an `<Image>`'s
    // Height (`Hud_UpdateShieldReadout` writes `DamageBar`'s crop height there), so the bar
    // runs from its authored `y` to the second row, Pulse's relation, with the authored
    // `x` and `width` (`docs/ghidra/functions/ps3-hdfury-eu/head2head.md`, confidence 78).
    frame.sprites.push(Draw::Fill {
        rect: [bar.rect[0], bar.rect[1], bar.rect[2], half - y_authored],
        color: bar.color,
    });
    // The bind copies `Position`'s `x` (`+0x9c`) and align (`+0x168`) onto
    // `Position Outof`, so the two rows share one right edge (measured live
    // 2026-09-29: within 3 px). `PositionOf` keeps its own `x`: the gap label's
    // right edge sat about 8 px past the rows'.
    let second = &super::Label {
        x: if hd { second.x } else { first.x },
        ..second.clone()
    };
    // `"%s %s"`: the ordinal, then the craft's team name where there is one.
    // HD's rows are the ordinal alone, **chosen, not measured**: the original's `"%s %s"` puts a team
    // name after it, and at the layout's own size (`Position` scale 1.0 in the HUD font, a
    // hex-panel number's size) `1ST FEISAR` runs out of its view.
    let row = |id: &str, team: Option<&String>| match team.filter(|_| !hd) {
        Some(team) => format!(
            "{} {}",
            cx.strings.get_or_id(id),
            cx.strings.get_or_id(team)
        ),
        None => cx.strings.get_or_id(id).to_string(),
    };
    let (first_team, second_team) = if state.player_leads {
        (state.own_team.as_ref(), state.other_team.as_ref())
    } else {
        (state.other_team.as_ref(), state.own_team.as_ref())
    };
    // The leader row stands on the bar's top: `Position` and the bar share the authored `y`
    // (30), and the row is bottom-aligned. That is the same place in both cuts of this
    // layout, so the top and bottom cut changes the bar's length and not this row.
    // HD keeps both rows in the layout's authored font and anchors; the second is
    // bottom-aligned at `y = half`.
    let (rows, row_line, second_scale) = if hd {
        (&mut frame.hud_text, cx.hud_line_height, second.scale)
    } else {
        (
            &mut frame.default_text,
            cx.default_line_height,
            SECOND_ROW_SCALE,
        )
    };
    let first_y = top_edge(first, row_line);
    rows.push(text(
        first,
        first_y,
        first.scale,
        first.align,
        first_colour,
        row("IG_HUD_1ST", first_team),
    ));
    let second_y = if hd {
        bottom - row_line * second_scale
    } else {
        bottom
    };
    let second_align = if hd { second.align } else { first.align };
    rows.push(text(
        second,
        second_y,
        second_scale,
        second_align,
        second_colour,
        row("IG_HUD_2ND", second_team),
    ));
    let middle = bar.origin_y + (y_authored + half) * 0.5;
    let line = cx.hud_line_height * of.scale;
    frame.hud_text.push(text(
        of,
        middle - line / 2.0,
        of.scale,
        first.align,
        tint,
        gap_text(state),
    ));
    true
}

/// The gap alone, one line under the layout's own `Position`, in its font, alignment and
/// border, tinted as Pulse's gap is. The place stays.
fn gap_under_place(cx: &Context<'_>, state: &HeadToHead, frame: &mut Frame) {
    let Some(place) = cx.layout.label("Position") else {
        return;
    };
    let line = cx.hud_line_height * place.scale;
    let lines = if place.vertalign == VertAlign::Middle {
        PANEL_CLEARANCE_LINES
    } else {
        BARE_CLEARANCE_LINES
    };
    frame.hud_text.push(Draw::Text {
        x: place.x,
        y: top_edge(place, cx.hud_line_height) + line + line * lines,
        scale: BESIDE_GAP_SCALE,
        color: argb_to_rgba(if state.player_leads {
            LEADING
        } else {
            TRAILING
        }),
        border: Some(place.border.unwrap_or(cx.default_border)),
        align: place.align,
        text: gap_text(state),
        wrap_width: None,
    });
}

fn gap_text(state: &HeadToHead) -> String {
    format!(
        "{}{:3.0}m",
        if state.player_leads { "-" } else { "+" },
        state.gap
    )
}

#[cfg(test)]
mod tests;
