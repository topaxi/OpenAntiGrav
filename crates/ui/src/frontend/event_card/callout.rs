//! The weapon callout of page kinds `0` and `4` (`FUN_810626ce`): a row of
//! weapon icons, or the one "weapons off" style icon, with a caption on the
//! rules page. Which icons is `oag_2048::campaign::callout`; this draws them.

use super::*;

/// What the callout says, resolved by the caller so this crate opens no file.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CardWeapons {
    /// One texture name per icon, left to right.
    pub icons: Vec<String>,
    /// The caption the rules page prints under the row: the weapon names
    /// joined with ` + `, or the single `Event_Variable_*` wording.
    pub caption: String,
}

/// The gap between icons, in every call (`0x41400000`).
const GAP: f32 = 12.0;
/// The drawn icon edge on the rules page (`0x427c0000`) and the edge it is
/// centred as (`0x42700000`) - the executable measures at 60 and draws at 63.
const RULES_DRAWN: f32 = 63.0;
const RULES_MEASURED: f32 = 60.0;
/// The objective page and `FUN_81055006` measure and draw at 44.
const ROW_SIZE: f32 = 44.0;
/// The caption's `y` offset below the item centre, as every rules caption.
const CAPTION_DROP: f32 = 38.0;
/// The widest the callout may be before the rules page stacks a two-item row
/// (`FUN_810535fe` compares the 44 unit measure with `140.0`).
pub(super) const STACK_ABOVE: f32 = 140.0;

/// `n` icons of `size` with [`GAP`] between them, as `FUN_810626ce` returns.
pub(super) fn row_width(n: usize, size: f32) -> f32 {
    if n == 0 {
        return 0.0;
    }
    n as f32 * size + (n - 1) as f32 * GAP
}

impl Frontend {
    /// The icons on the objective page's glyph row: `left` is the slot's left
    /// edge, the row is centred on `y`, and nothing is captioned.
    pub(super) fn draw_callout_row(
        &self,
        weapons: &CardWeapons,
        left: f32,
        y: f32,
        out: &mut Vec<Draw>,
    ) {
        let blue = self.global_colour("Blue2048");
        for (i, icon) in weapons.icons.iter().enumerate() {
            let x = left + i as f32 * (ROW_SIZE + GAP);
            self.card_sprite(icon, [x, y - ROW_SIZE * 0.5, ROW_SIZE, ROW_SIZE], blue, out);
        }
    }

    /// The callout as an item of the rules page, centred on `at`.
    pub(super) fn draw_callout_item(
        &self,
        weapons: &CardWeapons,
        at: (f32, f32),
        out: &mut Vec<Draw>,
    ) {
        let blue = self.global_colour("Blue2048");
        let n = weapons.icons.len();
        let left = at.0 - row_width(n, RULES_MEASURED) * 0.5;
        for (i, icon) in weapons.icons.iter().enumerate() {
            let x = left + i as f32 * (RULES_DRAWN + GAP);
            self.card_sprite(icon, [x, at.1 - 32.0, RULES_DRAWN, RULES_DRAWN], blue, out);
        }
        let wrap = STACK_ABOVE.max(n as f32 * RULES_DRAWN);
        out.push(Self::card_text(
            &weapons.caption,
            at.0,
            at.1 + CAPTION_DROP,
            0.6,
            blue,
            Some(wrap),
        ));
    }
}
