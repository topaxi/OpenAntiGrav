//! Detonator's HUD: the score, the stage, the clip and the volley pips.
//!
//! The layout is `detonator_hud.xml` with `hud_detonatorweapons.xml` (HD and Omega) or
//! `2048_hud\Detonator_HUD_HD.xml` (2048); which widget reads which value is
//! `docs/ghidra/functions/ps3-hdfury-eu/detonator-hud.md`, "The answer in one table".
//!
//! Four widgets the layout authors are never drawn, because the original's controller binds
//! nothing to them (`PulseScore`, `PulseMultiplier`, `EmpScore`, `PulseBankedScore`, confidence
//! 65-75). The EMP widgets, the bomb-ahead warning and the floating score feedback wait on
//! mechanics this build does not run, so they draw nothing either.

use super::draw::{Context, lit_draw, sprite_draw};
use super::{Draw, Readout};

/// Volleys in a clip, and so `Bullet00`-`Bullet14` and `Multi00`-`Multi14`.
pub const VOLLEYS: usize = 15;

/// The constant the stage counter is read against (`DetonatorStage`, `%d/%d`).
pub const STAGES: u32 = 14;

/// The streak line `PulseBonus` shows: `Hud_RaiseChainBonus` sets the localised `HUD_CHAIN_n`
/// text and the amount, holds it 2.0 s and fades it over 1.0 s.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bonus {
    /// The streak: 5, 10 or 15 volleys.
    pub streak: u8,
    /// What the award paid.
    pub amount: i32,
    /// The text's alpha, `1.0` while held and falling to `0.0` over the fade.
    pub alpha: f32,
}

/// What the Detonator HUD shows this frame.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Detonator {
    /// The score, `DetonatorScore` (`%07d`).
    pub score: i32,
    /// Volleys left in the clip, `PulseBullets` (`%02d`).
    pub clip: u8,
    /// The stage, `DetonatorStage` (`%d/%d`).
    pub stage: u32,
    /// Bit `i` set once volley `i` has hit: `Multi(14 - i)` is lit.
    pub hit: u16,
    /// Bit `i` set where a streak award recoloured volley `i`'s pip.
    pub tinted: u16,
    /// The streak line, while it is up.
    pub bonus: Option<Bonus>,
}

/// A pip's colour after a streak award recoloured it, `0xff00eeff`.
const AWARD_PIP: [f32; 4] = [0.0, 238.0 / 255.0, 1.0, 1.0];

/// The widgets the original's controller never binds or never writes, so they stay empty.
const DEAD: [&str; 4] = [
    "PulseScore",
    "PulseMultiplier",
    "EmpScore",
    "PulseBankedScore",
];

/// The language id of a streak's line.
#[must_use]
pub fn chain_id(streak: u8) -> String {
    format!("HUD_CHAIN_{streak}")
}

/// `Bullet00`-`Bullet14` and `Multi00`-`Multi14`: which of the numbered family `name` is.
fn numbered(name: &str, prefix: &str) -> Option<usize> {
    let digits = name.strip_prefix(prefix)?;
    (digits.len() == 2)
        .then(|| digits.parse::<usize>().ok())
        .flatten()
        .filter(|&index| index < VOLLEYS)
}

/// The text of a Detonator widget, or `None` when `name` is not one of them.
/// `Some(None)` is a widget that draws nothing this frame.
pub(super) fn text(
    name: &str,
    readout: &Readout,
    strings: &oag_ui::language::StringTable,
) -> Option<Option<String>> {
    let shown = readout.detonator.as_ref();
    Some(match name {
        "DetonatorScore" => shown.map(|d| format!("{:07}", d.score)),
        "PulseBullets" => shown.map(|d| format!("{:02}", d.clip)),
        "DetonatorStage" => shown.map(|d| format!("{}/{STAGES}", d.stage)),
        // A language table without the id (HD's carries none) leaves the line empty: the text is
        // the data's, and the raw id is not drawn in its place.
        "PulseBonus" => shown.and_then(|d| d.bonus).and_then(|bonus| {
            let id = chain_id(bonus.streak);
            strings
                .get(&id)
                .map(|text| format!("{text} - {}", bonus.amount))
        }),
        name if DEAD.contains(&name) => None,
        _ => return None,
    })
}

/// `PulseBonus`'s colour: opaque white, its alpha running down through the fade.
pub(super) fn colour(readout: &Readout, name: &str) -> Option<[f32; 4]> {
    let bonus = readout.detonator.as_ref()?.bonus?;
    (name == "PulseBonus").then_some([1.0, 1.0, 1.0, bonus.alpha])
}

/// The clip's icons and the volley pips.
///
/// **Chosen, not measured:** a spent bullet icon is hidden and an unlit pip is hidden. The
/// original's per-state art (`0x0007f0a0`) is unread; the layout authors one source rectangle
/// for each icon and each pip, so these are the only states it can show.
pub(super) fn sprites(cx: &Context<'_>, readout: &Readout) -> Vec<Draw> {
    let Some(shown) = readout.detonator.as_ref() else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for sprite in &cx.layout.sprites {
        if let Some(index) = numbered(&sprite.name, "Bullet") {
            if index < usize::from(shown.clip) {
                out.extend(sprite_draw(sprite, cx.sheet));
            }
        } else if let Some(index) = numbered(&sprite.name, "Multi") {
            let volley = VOLLEYS - 1 - index;
            if shown.hit & (1 << volley) != 0 {
                let mut pip = sprite.clone();
                if shown.tinted & (1 << volley) != 0 {
                    pip.color = AWARD_PIP;
                }
                out.extend(sprite_draw(&pip, cx.sheet));
            }
        }
    }
    out.into_iter().map(|draw| lit_draw(cx, draw)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_numbered_family_is_two_digits_and_in_range() {
        assert_eq!(numbered("Bullet00", "Bullet"), Some(0));
        assert_eq!(numbered("Bullet14", "Bullet"), Some(14));
        assert_eq!(numbered("Bullet15", "Bullet"), None);
        assert_eq!(numbered("BulletBack", "Bullet"), None);
        assert_eq!(numbered("Multi07", "Multi"), Some(7));
        assert_eq!(numbered("Multi7", "Multi"), None);
    }

    #[test]
    fn a_dead_widget_is_ours_and_draws_nothing() {
        let readout = Readout {
            detonator: Some(Detonator::default()),
            ..Readout::default()
        };
        let strings = oag_ui::language::StringTable::default();
        for name in DEAD {
            assert_eq!(text(name, &readout, &strings), Some(None), "{name}");
        }
        assert_eq!(text("Lap", &readout, &strings), None);
    }
}
