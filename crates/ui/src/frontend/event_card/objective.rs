//! Page kind `0`, the pass objective (`CampaignEventCard_DrawObjectivePage`,
//! `0x81055150`): the `PASS` line with its medal, the elite line once the
//! event is passed, and the glyph row under the rule.

use super::*;

/// One glyph cell of the row: 56 units, the glyph 44.8 (`0.7 * 64`) in it.
const CELL: f32 = 56.0;
const GLYPH: f32 = 44.8;
const ROW_Y: f32 = 370.0;
/// `FUN_81055006`'s width for a forced craft, and where `FUN_81061808`'s logo
/// sits in it (`x + 7`).
const CRAFT_CELL: f32 = 108.0;

impl Frontend {
    /// The width of the glyph row (`FUN_81055006`), in the executable's own
    /// terms: 56 per glyph, the callout's `n * 56`, 108 for a forced craft.
    fn glyph_row_width(card: &EventCard) -> f32 {
        let cells = usize::from(card.class_icon.is_some())
            + usize::from(card.has_trophy_page)
            + usize::from(card.laps.is_some())
            + card.weapons.as_ref().map_or(0, |w| w.icons.len())
            + card.allowed_classes.len();
        cells as f32 * CELL
            + if card.forced_craft.is_some() {
                CRAFT_CELL
            } else {
                0.0
            }
    }

    pub(super) fn draw_objective_page(&self, event: &MapEvent, out: &mut Vec<Draw>) {
        let blue = self.global_colour("Blue2048");
        let white = [1.0, 1.0, 1.0, 1.0];
        let card = &event.card;
        let state = self.campaign.state_of(self.campaign.selected);
        // `param_3[0xb6]` is `3` once passed and `4` once elite: from then on
        // the page shifts up 40 units and adds the elite line under the pass
        // line, each with its own medal (`FUN_81061344`).
        let two_rows = matches!(state, ProgressState::Passed | ProgressState::Elite);
        let top = if two_rows { 189.0 } else { 229.0 };
        let medal = match state {
            ProgressState::Locked | ProgressState::Open => MEDAL_NONE,
            ProgressState::Passed | ProgressState::Elite => MEDAL_PASS,
        };
        self.card_sprite(medal, [488.0, top + 6.0, 52.0, 52.0], white, out);
        // Both lines start at `x=552` in frame 14 (bounding boxes 552 and
        // 553). The original centres a block of the text's own width plus the
        // medal on `x=682`; the frontend has no glyph metrics, so this
        // left-aligns at the frame's own edge and a wider or narrower wording
        // sits off by half the difference (open).
        if let Some(label) = &card.pass_label {
            out.push(Self::left_text(label, 552.0, top, 0.66, blue, 345.0));
        }
        if let Some(text) = &card.objective {
            out.push(Self::left_text(text, 552.0, top + 27.0, 0.8, blue, 345.0));
        }
        if two_rows {
            let elite_medal = if state == ProgressState::Elite {
                MEDAL_ELITE
            } else {
                MEDAL_NONE
            };
            self.card_sprite(elite_medal, [488.0, 265.0, 52.0, 52.0], white, out);
            if let Some(label) = &card.elite_label {
                out.push(Self::left_text(label, 552.0, 259.0, 0.66, blue, 345.0));
            }
            if let Some(text) = &card.elite_objective {
                out.push(Self::left_text(text, 552.0, 287.0, 0.8, blue, 345.0));
            }
        }
        out.push(Draw::Fill {
            rect: [482.0, 329.0, 400.0, 1.0],
            color: blue,
        });
        self.draw_glyph_row(card, blue, out);
    }

    fn draw_glyph_row(&self, card: &EventCard, blue: [f32; 4], out: &mut Vec<Draw>) {
        let glyph = |at: f32| [at - GLYPH * 0.5, ROW_Y - GLYPH * 0.5, GLYPH, GLYPH];
        let mut x = CENTRE_X + CELL * 0.5 - Self::glyph_row_width(card) * 0.5;
        if let Some(class) = card.class_icon.as_deref() {
            self.card_sprite(class, glyph(x), blue, out);
            x += CELL;
        }
        if card.has_trophy_page {
            self.card_sprite(TROPHY_GLYPH, glyph(x), blue, out);
            x += CELL;
        }
        if let Some(laps) = card.laps {
            self.card_sprite(NUM_LAPS, glyph(x), blue, out);
            out.push(Self::card_text(
                &laps.to_string(),
                x,
                358.0,
                0.6,
                blue,
                None,
            ));
            x += CELL;
        }
        if let Some(weapons) = &card.weapons {
            self.draw_callout_row(weapons, x - 22.0, ROW_Y, out);
            x += weapons.icons.len() as f32 * CELL;
        }
        if let Some(craft) = &card.forced_craft {
            // **Chosen, not measured**: `FUN_81061808` is called with the
            // glyph scale of this row's other cells, which is not decoded;
            // the pair keeps the rules page's measured offsets (logo at
            // `x + 7`, the class icon 40 to its left) scaled by `0.7`.
            let white = [1.0, 1.0, 1.0, 1.0];
            let left = x + 7.0;
            if let Some(logo) = craft.logo.as_deref() {
                self.card_sprite(logo, [left, ROW_Y - GLYPH * 0.5, GLYPH, GLYPH], white, out);
            }
            if let Some(icon) = craft.type_icon.as_deref() {
                self.card_sprite(
                    icon,
                    [left - 28.0, ROW_Y - GLYPH * 0.5, GLYPH, GLYPH],
                    white,
                    out,
                );
            }
            x += CRAFT_CELL;
        }
        for allowed in &card.allowed_classes {
            self.card_sprite(&allowed.icon, glyph(x), blue, out);
            x += CELL;
        }
    }
}
