//! What the race load adds to the event card: the game tip that replaces the circuit's photo, and
//! the mode page the race manager appends.
//!
//! Operands read in `docs/ghidra/functions/vita-2048-eu-v104/race-load.md`, "What
//! `CampaignEventCard_EnterLoad` does: the game tip" (`GameTip_DrawPanel`, `0x8106dd36`) and
//! "What the race manager does: the mode page and the slide"
//! (`CampaignEventCard_DrawModePage`, `0x8104fc5a`). Every string is resolved by the caller from
//! the disc's table; this crate opens no file.

use super::*;

/// A game tip, resolved to the words and the picture the panel draws.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GameTip {
    /// `TUT_TIP_HEADER`, resolved.
    pub header: String,
    /// The `?` mark's texture name.
    pub question_mark: String,
    /// The picture's texture name.
    pub texture: String,
    /// The row's title, resolved.
    pub title: String,
    /// The row's body, resolved; empty on the controls rows, which draw no body.
    pub body: String,
}

/// The panel's rectangle: where the photo was, 398 by 334 at (16, 92).
const PANEL: [f32; 4] = [16.0, 92.0, 398.0, 334.0];
/// The body's wrap and line pitch, and its scale in `NEOSANS_BOLD`.
const TIP_WRAP: f32 = 360.0;
const TIP_PITCH: f32 = 20.0;
const TIP_BODY_SCALE: f32 = 0.7;
/// What the bold face's scale is multiplied by here. This build draws `NEOSANS_BOLD` as the
/// `Default` face at the line-height ratio, which is 8 % wider than the original's own bold
/// face: the original's `Out-speed, out-gun and out-smart your` spans 370 units (a Vita3K
/// frame of the mode page), this build's 399 at the bare ratio. Measured on that one line.
const BOLD_FIT: f32 = 0.927;
/// The mode page's body wrap and pitch.
const MODE_WRAP: f32 = 375.0;
const MODE_PITCH: f32 = 21.0;

impl CardPaint<'_> {
    pub(super) fn wrapped_lines(&self, text: &str, scale: f32, width: f32) -> usize {
        self.atlas.map_or(1, |atlas| {
            crate::font::wrap(atlas, text, scale, width).len()
        })
    }

    /// The width `text` draws at, `None` without an atlas.
    pub(super) fn text_width(&self, text: &str, scale: f32) -> Option<f32> {
        self.atlas
            .map(|atlas| crate::font::measure(atlas, text) * scale)
    }

    /// `GameTip_DrawPanel (x = 16, y = 92)`.
    pub(super) fn draw_tip_panel(&self, tip: &GameTip, out: &mut Vec<Draw>) {
        let [x, y, w, h_panel] = PANEL;
        let blue = self.global_colour("Blue2048");
        let orange = self.global_colour("Orange2048");
        out.push(Draw::Fill {
            rect: PANEL,
            color: self.global_colour("Transparent2048"),
        });
        self.card_sprite(&tip.question_mark, [x, y, 64.0, 64.0], orange, out);
        out.push(Self::left_text(
            &tip.header,
            x + 64.0,
            y + 10.0,
            1.0,
            orange,
            w,
        ));
        let bold = self.bold * BOLD_FIT;
        let body_scale = TIP_BODY_SCALE * bold;
        let h = if tip.body.is_empty() {
            0.0
        } else {
            self.wrapped_lines(&tip.body, body_scale, TIP_WRAP) as f32 * TIP_PITCH
        };
        if !tip.body.is_empty() {
            out.push(Self::left_text(
                &tip.body,
                x + 12.0,
                y + h_panel - 4.0 - h,
                body_scale,
                blue,
                TIP_WRAP,
            ));
        }
        out.push(Self::left_text(
            &tip.title,
            x + 12.0,
            y + 304.0 - h,
            bold,
            blue,
            383.0,
        ));
        if let Some(picture) = self.placed(&tip.texture) {
            let fit = if 258.0 - h < 163.0 {
                (258.0 - h) / 163.0
            } else {
                1.0
            };
            let (pw, ph) = (picture.width as f32 * fit, picture.height as f32 * fit);
            let (cx, cy) = (x + 199.0, y + 50.0 + (284.0 - h - 25.0) / 2.0);
            out.push(Self::sprite_at(
                [cx - pw * 0.5, cy - ph * 0.5, pw, ph],
                picture,
                [1.0, 1.0, 1.0, 1.0],
            ));
        }
    }

    /// Page kind `6`: the mode's name, class icon and one paragraph (the body is
    /// [`EventCard::mode_body`], resolved from `Tut_Event_*_1`).
    pub(super) fn draw_mode_page(&self, event: &MapEvent, out: &mut Vec<Draw>) {
        let blue = self.global_colour("Blue2048");
        let card = &event.card;
        let Some(body) = card.mode_body.as_deref() else {
            return;
        };
        let scale = self.bold * BOLD_FIT;
        let h = self.wrapped_lines(body, scale, MODE_WRAP) as f32 * MODE_PITCH;
        let y = (284.0 - (h + 30.0) / 2.0).max(198.0);
        out.push(Self::left_text(
            body,
            500.0,
            y + 30.0,
            scale,
            blue,
            MODE_WRAP,
        ));
        if let Some(kind) = &card.kind_label {
            out.push(Self::left_text(kind, 558.0, y - 10.0, 0.82, blue, 400.0));
        }
        self.card_sprite(
            event.kind.texture_name(),
            // `0x81061040 (x + 524, y' + 8, 0.7, ..)` is the icon's centre (a frame puts its ink
            // 22 units left of and above the corner this would be).
            [524.0 - 22.4, y + 8.0 - 22.4, 44.8, 44.8],
            blue,
            out,
        );
    }
}
