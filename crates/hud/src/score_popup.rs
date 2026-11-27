//! The floating score: the `+N` that rises off a scored object and fades.
//!
//! The sixteen `ScoreFeedBack0`-`15` widgets the layout authors, driven by
//! `Hud_RaiseScoreFeedback` and `Hud_UpdateScoreFeedback`
//! (`docs/ghidra/functions/ps3-hdfury-eu/detonator-hud.md`, "The floating score").
//! The font, alignment and colours are the widgets' own; the alpha, size and place are
//! written here as the original writes them.

use oag_core::math::Vec3;
use oag_ui::frontend::Draw;

use super::Readout;
use super::draw::{Context, lit_colour, outline_colour, text_shadow, top_edge};
use super::{Font, Frame, Label};

/// The slots, and so the widgets.
pub const SCORE_POPUPS: usize = 16;

/// A busy slot at this alpha or more is dimmed when another popup is raised.
const DIM_FROM: u8 = 180;
/// What it is dimmed to.
const DIM_FACTOR: f32 = 0.7;
/// The fade per update is `(1 - dt)` times this.
const FADE: f32 = 0.995;
/// World units the popup has risen by the time it has faded out.
const RISE: f32 = 5.0;
/// The widget's scale under [`BIG_FROM`] points, and from it.
const SCALE: f32 = 1.2;
const BIG_SCALE: f32 = 2.0;
const BIG_FROM: i32 = 1000;

/// One busy slot.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Slot {
    /// What the score gained.
    pub points: i32,
    /// Where the scored object was. The original keeps a pointer to it; **chosen**: ours
    /// copies it at the raise, the destroyed mine having no position left to follow.
    pub at: Vec3,
    /// The text's and border's alpha, the byte the original stores.
    pub alpha: u8,
    /// World units above [`Self::at`] along the camera's up this update.
    pub rise: f32,
}

/// The sixteen slots between updates. Display only, never simulation state.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ScoreFeedback {
    slots: [Option<Slot>; SCORE_POPUPS],
}

impl ScoreFeedback {
    /// Takes the first free slot for `points` at `at`, dimming the others still bright. With
    /// none free the popup is dropped.
    pub fn raise(&mut self, points: i32, at: Vec3) {
        let Some(free) = self.slots.iter().position(Option::is_none) else {
            return;
        };
        for slot in self.slots.iter_mut().flatten() {
            if slot.alpha >= DIM_FROM {
                slot.alpha = (f32::from(slot.alpha) * DIM_FACTOR) as u8;
            }
        }
        self.slots[free] = Some(Slot {
            points,
            at,
            alpha: 0xff,
            rise: 0.0,
        });
    }

    /// One update: a spent slot is freed, every other fades and rises. The alpha is truncated
    /// to a byte each time, the rise read off the untruncated product.
    pub fn advance(&mut self, dt: f32) {
        let k = (1.0 - dt) * FADE;
        for slot in &mut self.slots {
            let Some(popup) = slot else { continue };
            if popup.alpha < 1 {
                *slot = None;
                continue;
            }
            let faded = f32::from(popup.alpha) * k;
            popup.rise = RISE - RISE * faded / 255.0;
            popup.alpha = faded as u8;
        }
    }

    /// The slots, widget `ScoreFeedBack{i}` being entry `i`.
    #[must_use]
    pub fn slots(&self) -> &[Option<Slot>; SCORE_POPUPS] {
        &self.slots
    }
}

/// One popup placed on the HUD's own screen, for the widget of the same index.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScorePopup {
    /// What the score gained.
    pub points: i32,
    /// The anchor's x in the layout's space.
    pub x: f32,
    /// The anchor's y in the layout's space.
    pub y: f32,
    /// The text's and border's alpha.
    pub alpha: u8,
}

/// `+%3.0f`: three characters wide, so `50` reads `+ 50`. Under one point, `+%3.2f`.
#[must_use]
pub fn text(points: i32) -> String {
    if points < 1 {
        format!("+{:3.2}", points as f32)
    } else {
        format!("+{points:>3}")
    }
}

/// `ScoreFeedBack%d`: which slot `name` is.
fn slot_of(name: &str) -> Option<usize> {
    let digits = name.strip_prefix("ScoreFeedBack")?;
    digits
        .parse::<usize>()
        .ok()
        .filter(|&index| index < SCORE_POPUPS)
}

/// Draws the popups `readout` places through the layout's own widgets.
pub(super) fn append(frame: &mut Frame, cx: &Context<'_>, readout: &Readout) {
    for label in &cx.layout.labels {
        let Some(popup) = slot_of(&label.name).and_then(|slot| readout.score_popups[slot]) else {
            continue;
        };
        let draw = draw_of(cx, label, popup);
        let shadow = text_shadow(cx, &draw);
        let list = match label.font {
            Font::Small => &mut frame.small_text,
            Font::Default => &mut frame.default_text,
            Font::Skin => &mut frame.skin_text,
            Font::Hud => &mut frame.hud_text,
        };
        list.extend(shadow.map(|draw| draw.stretched(label.stretch)));
        list.push(draw.stretched(label.stretch));
    }
}

fn draw_of(cx: &Context<'_>, label: &Label, popup: ScorePopup) -> Draw {
    let line_height = match label.font {
        Font::Small => cx.small_line_height,
        Font::Default => cx.default_line_height,
        Font::Skin => cx.skin_line_height,
        Font::Hud => cx.hud_line_height,
    };
    let scale = if popup.points < BIG_FROM {
        SCALE
    } else {
        BIG_SCALE
    };
    let placed = Label {
        y: popup.y,
        scale,
        ..label.clone()
    };
    let alpha = f32::from(popup.alpha) / 255.0;
    let mut colour = label.color;
    colour[3] = alpha;
    let mut border = outline_colour(cx, label.border.unwrap_or(cx.default_border));
    border[3] = alpha;
    Draw::Text {
        x: popup.x,
        y: top_edge(&placed, line_height),
        scale,
        color: lit_colour(cx, colour),
        border: Some(border),
        align: label.align,
        text: text(popup.points),
        wrap_width: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DT: f32 = 1.0 / 60.0;

    fn alpha(feedback: &ScoreFeedback, slot: usize) -> Option<u8> {
        feedback.slots()[slot].map(|popup| popup.alpha)
    }

    #[test]
    fn a_popup_fades_by_the_truncated_byte_and_rises_as_it_goes() {
        let mut feedback = ScoreFeedback::default();
        feedback.raise(100, Vec3::ZERO);
        assert_eq!(alpha(&feedback, 0), Some(255));
        let mut seen = Vec::new();
        for _ in 0..4 {
            feedback.advance(DT);
            seen.push(alpha(&feedback, 0).unwrap());
        }
        assert_eq!(seen, [249, 243, 237, 231]);
        let rise = feedback.slots()[0].unwrap().rise;
        let expected = 5.0 - 5.0 * (237.0 * (1.0 - DT) * 0.995) / 255.0;
        assert!((rise - expected).abs() < 1e-5, "{rise} {expected}");
        for _ in 0..400 {
            feedback.advance(DT);
        }
        assert_eq!(feedback, ScoreFeedback::default(), "spent and freed");
    }

    #[test]
    fn a_raise_takes_the_first_free_slot_and_dims_the_bright_ones_only() {
        let mut feedback = ScoreFeedback::default();
        feedback.raise(100, Vec3::ZERO);
        feedback.raise(200, Vec3::ZERO);
        assert_eq!(alpha(&feedback, 0), Some(178), "255 * 0.7, truncated");
        assert_eq!(alpha(&feedback, 1), Some(255));
        feedback.raise(300, Vec3::ZERO);
        assert_eq!(alpha(&feedback, 0), Some(178), "178 is under 180");
        assert_eq!(alpha(&feedback, 1), Some(178));
    }

    #[test]
    fn a_full_set_drops_the_popup_without_dimming() {
        let mut feedback = ScoreFeedback::default();
        for n in 0..SCORE_POPUPS {
            feedback.raise(n as i32 + 1, Vec3::ZERO);
        }
        let before = feedback;
        feedback.raise(999, Vec3::ZERO);
        assert_eq!(feedback, before);
    }

    #[test]
    fn the_number_is_three_wide() {
        assert_eq!(text(50), "+ 50");
        assert_eq!(text(100), "+100");
        assert_eq!(text(2500), "+2500");
        assert_eq!(text(0), "+0.00");
    }
}
