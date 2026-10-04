//! Page kind `2`, the trophy and cup page (`CampaignEventCard_DrawTrophyPage_q`,
//! `0x81052fb4`): an elite trophy or a cup's picture at the left of the body
//! and its heading and callout beside it.
//!
//! Which event gets which art is `oag_2048::campaign::trophy`; the image
//! handles are the nine `trophy/{year}_elite_{n}` and three `trophy/Cup{year}`
//! textures `FUN_8104d1d0` loads. An event whose shape gives it the page but
//! which names no art draws nothing, as in the original.

use super::*;

/// What the trophy page draws, resolved by the caller so this crate opens no
/// file: the image and the two strings.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CardTrophy {
    /// The trophy or cup image's texture name.
    pub texture: String,
    /// The heading (`TROPHY_2048_1_1`, `Cup_Name_2048`), resolved.
    pub header: String,
    /// The callout (`TROPHY_2048_1_2`, `Cup_Callout_2048`), resolved.
    pub callout: String,
}

/// The image is centred here, at its own size (`FUN_81052fb4`: `x+520`,
/// `y+283`, half the texture's width and height either way).
const IMAGE_CENTRE: (f32, f32) = (520.0, 283.0);
/// The text block's left edge and the widths the heading and callout wrap at.
const TEXT_X: f32 = 570.0;
/// `FUN_8129146e(320.0, heading)`: the 320 is read as the width the heading
/// is fitted to. How the original fits a longer one is not decoded, so this
/// build shrinks the scale so an estimated `HEADER_UNIT` units a character
/// fits `HEADER_FIT` (**chosen, not measured**; `2048 ELITE 1 TROPHY` runs
/// about 370 units at scale 1.0 and would cross the page arrow).
const HEADER_FIT: f32 = 320.0;
const HEADER_UNIT: f32 = 19.5;
const HEADER_WRAP: f32 = 400.0;
const CALLOUT_WRAP: f32 = 300.0;
/// Heading scale `1.0`, callout `0.7` (`0x3f333333`), a 30 unit gap between.
const CALLOUT_SCALE: f32 = 0.7;
const CALLOUT_DROP: f32 = 30.0;
/// The block is centred on `y=284` by the callout's wrapped height plus the
/// gap (`284 - (h + 30) / 2`). **Chosen, not measured**: this build has no
/// glyph metrics at card-build time, so the height is `LINE` units a line
/// over an estimated `CHARS_PER_LINE` characters of a 300-wide wrap.
const LINE: f32 = 20.0;
const CHARS_PER_LINE: usize = 24;

impl Frontend {
    pub(super) fn draw_trophy_page(&self, event: &MapEvent, out: &mut Vec<Draw>) {
        let Some(trophy) = &event.card.trophy else {
            return;
        };
        let blue = self.global_colour("Blue2048");
        if let Some(placed) = self.placed(&trophy.texture) {
            let (w, h) = (placed.width as f32, placed.height as f32);
            out.push(Self::sprite_at(
                [IMAGE_CENTRE.0 - w * 0.5, IMAGE_CENTRE.1 - h * 0.5, w, h],
                placed,
                [1.0, 1.0, 1.0, 1.0],
            ));
        }
        let lines = trophy
            .callout
            .chars()
            .count()
            .div_ceil(CHARS_PER_LINE)
            .max(1);
        let top = 284.0 - (lines as f32 * LINE + CALLOUT_DROP) * 0.5;
        let header_scale =
            (HEADER_FIT / (trophy.header.chars().count() as f32 * HEADER_UNIT)).min(1.0);
        out.push(Self::left_text(
            &trophy.header,
            TEXT_X,
            top,
            header_scale,
            blue,
            HEADER_WRAP,
        ));
        out.push(Self::left_text(
            &trophy.callout,
            TEXT_X,
            top + CALLOUT_DROP,
            CALLOUT_SCALE,
            blue,
            CALLOUT_WRAP,
        ));
    }
}
