//! **`Race End Photo`**: the front-end state a Pulse race sits in between the
//! flag and `EndRace Results`.
//!
//! The screen is `Race End Photo` in `InGame_Definition.xml`
//! ([`oag_pulse::names::INGAME_DEFINITION`]). It authors no panel at all: two
//! `Stats`-font text widgets, `PhotoMessage` (`ER_PRESS_SELECT_PHOTO`) over
//! `ProceedMessage` (`FE_PRESS_TO_CONT`), both left-aligned at `x = 20`, and a
//! `SELECT` redirect to photo mode. The race keeps running behind it (the
//! screen's `BackendController` task is `run`, where `InGame Race End Photo`'s
//! is `Pause`).
//!
//! What was **measured** on the original (Pulse PSP, PPSSPP, four captures of
//! a line finish; `docs/gameplay/after-the-finish.md`): the front end enters
//! the state `61` frames after the player's finish is registered, and the
//! legend fades in from there, linearly, to full ink about `42` frames
//! (`0.7 s`) later. Until then the view is clean - the HUD went at the finish
//! plus one frame, and nothing is drawn over the race.
//!
//! What is **not** here: photo mode. `SELECT` does nothing in this build, so
//! the line `PRESS SELECT BUTTON FOR PHOTO MODE` promises a mode that does not
//! exist yet; it is drawn anyway because the disc authors it and leaving it
//! out would be a second invention, in the other direction.

use oag_ui::frontend::{Align, Draw};
use oag_ui::screen::argb_to_rgba;

use super::Layout;

/// The screen's own name in `InGame_Definition.xml`.
pub const SCREEN: &str = "Race End Photo";

/// Ticks from the finish to the state being entered: `61` frames on all four
/// measured line finishes. The legend starts to fade in on this tick.
pub const ENTER_TICKS: u32 = 61;

/// Ticks the legend takes to reach full ink: `42` (`0.7 s`), read off a
/// photograph every second frame from `F+56` to `F+110`, two lines of text,
/// with the photograph's own two-frame lag allowed for. The ramp is straight
/// to within the noise of a moving background. **Why `0.7`** is unread - the
/// screen authors no `Transition` - though it is the value most screens
/// author for `enabletransition` (`docs/formats/fe-menu-definitions.md`).
pub const FADE_TICKS: u32 = 42;

/// How much narrower this build draws a `Stats` line than its `small`
/// stand-in does. `Stats` is `Pulse_14.fnt`; no atlas of that file is loaded,
/// so the text is the `menu` face scaled down, and a uniform scale cannot get
/// both the width and the height of the original's glyphs: the first line,
/// `PRESS SELECT BUTTON FOR PHOTO MODE`, was 315 px wide at the plain `small`
/// scale against the capture's 283 (`0.90`), and 9 pixel rows tall against 8
/// (the second line was 8 against 8). **Chosen, not measured**: the width is
/// matched, and the height is then near rather than exact. Not re-photographed
/// in English after the change. The real atlas retires it.
const STATS_OVER_SMALL: f32 = 0.9;

/// The legend's ink at `ticks` after the finish: `0` until [`ENTER_TICKS`],
/// then a straight ramp to `1` over [`FADE_TICKS`].
#[must_use]
pub fn legend_alpha(ticks: u32) -> f32 {
    let into = ticks.saturating_sub(ENTER_TICKS);
    (into as f32 / FADE_TICKS as f32).min(1.0)
}

/// Whether the state has been entered - the tick from which the original
/// reads the continue press. Before it the front end is still on `InGame`,
/// where a thrust button held across the line means nothing to it.
#[must_use]
pub fn entered(ticks: u32) -> bool {
    ticks >= ENTER_TICKS
}

/// The legend's draws at `ticks` after the finish, in the order the screen
/// authors its widgets. Empty before the state is entered, and for a widget
/// whose string id does not resolve: a raw id on screen would be an
/// invention of its own.
///
/// `Stats` is `Pulse_14.fnt`, the file the `Small` role names too
/// (`oag_ui::language`), so it scales as `small` does against the loaded
/// `menu` face. The `ε` in `FE_PRESS_TO_CONT` is the cross button's glyph in
/// that file; whether the face this build draws it in carries it is the
/// capture's business, not this function's.
#[must_use]
pub fn photo_draw_list(layout: &Layout, ticks: u32) -> Vec<Draw> {
    let alpha = legend_alpha(ticks);
    if alpha <= 0.0 {
        return Vec::new();
    }
    layout
        .screen
        .texts
        .iter()
        .filter_map(|text| {
            let content = text.string.clone()?;
            let mut color = argb_to_rgba(text.color);
            color[3] *= alpha;
            let face = match text.font.to_ascii_lowercase().as_str() {
                "stats" => layout.faces.small * STATS_OVER_SMALL,
                "small" => layout.faces.small,
                _ => 1.0,
            };
            Some(Draw::Text {
                x: text.x,
                y: text.y,
                scale: text.scale * face,
                color,
                border: None,
                align: match text.align.to_ascii_lowercase().as_str() {
                    "right" => Align::Right,
                    "centre" | "center" => Align::Centre,
                    _ => Align::Left,
                },
                text: content,
                wrap_width: text.wrap_width,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests;
