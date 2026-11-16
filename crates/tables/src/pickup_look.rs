//! The look of a simulation mod's own pickup: the icon it shows and the colour of the
//! backdrop behind it, each naming its source. Declared by a mod's `[pickup]` and
//! overridden per tick by its `pickup-look` command; read by the HUD.
//!
//! Data only: the title's fade, overshoot and size pulse are the HUD's, and play on any
//! look.

use crate::weapons::Weapon;

/// The icon of a mod's own pickup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Icon {
    /// The title's own icon for a weapon; none where the title has no icon for it.
    Title(Weapon),
    /// A mod's own PNG: its `[icons]` key in a manifest, and the HUD sheet's key once the
    /// race has resolved it.
    Mod(String),
}

/// The backdrop colour of a mod's own pickup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Colour {
    /// The title's own backdrop row for a weapon. A title with no row draws the backdrop as
    /// it draws a weapon with none, and one with no backdrop colours (Pure, 2048) draws it
    /// as authored.
    Title(Weapon),
    /// A colour of the mod's own, `[r, g, b, a]`. Only a title that colours its backdrop
    /// (Pulse, HD, Omega) shows it.
    Rgba([u8; 4]),
}

/// Icon and colour together.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PickupLook {
    /// The icon.
    pub icon: Icon,
    /// The backdrop colour.
    pub colour: Colour,
}

/// One row of a mod's own pickup odds (`[[pickup.odds]]` of its `oag-mod.toml`): the
/// weights the weapon pads draw the mod's pickup with, beside the title's own weapons,
/// for a title (its lowercased hyphenated name) and a class (a `<Pickupodds class>` name,
/// any case), each `None` for every one.
#[derive(Debug, Clone, PartialEq)]
pub struct OddsRow {
    /// The title it is for, or every title.
    pub title: Option<String>,
    /// The class it is for, or every class.
    pub class: Option<String>,
    /// The weights.
    pub odds: crate::weapons::PickupOdds,
}

/// The odds the most specific row gives a race of `title` in `class`: title and class
/// over title alone over class alone over neither; between equals, the first row.
#[must_use]
pub fn odds_for(rows: &[OddsRow], title: &str, class: &str) -> Option<crate::weapons::PickupOdds> {
    let fits = |want: &Option<String>, have: &str| {
        want.as_deref()
            .is_none_or(|want| want.eq_ignore_ascii_case(have))
    };
    let mut best: Option<(u8, &OddsRow)> = None;
    for row in rows
        .iter()
        .filter(|row| fits(&row.title, title) && fits(&row.class, class))
    {
        let rank = u8::from(row.title.is_some()) * 2 + u8::from(row.class.is_some());
        if best.is_none_or(|(held, _)| rank > held) {
            best = Some((rank, row));
        }
    }
    best.map(|(_, row)| row.odds)
}

#[cfg(test)]
mod odds_tests {
    use super::*;
    use crate::weapons::PickupOdds;

    fn row(title: Option<&str>, class: Option<&str>, ai: f32) -> OddsRow {
        OddsRow {
            title: title.map(str::to_string),
            class: class.map(str::to_string),
            odds: PickupOdds {
                ai,
                ..PickupOdds::default()
            },
        }
    }

    /// The most specific row wins, so 2048's per-class tables each take their own row.
    #[test]
    fn the_most_specific_row_wins_and_the_first_breaks_a_tie() {
        let rows = [
            row(None, None, 1.0),
            row(None, Some("venom"), 2.0),
            row(Some("2048"), None, 3.0),
            row(Some("2048"), Some("Speed"), 4.0),
            row(Some("2048"), Some("speed"), 5.0),
        ];
        let ai = |title, class| odds_for(&rows, title, class).map(|o| o.ai);
        assert_eq!(ai("pulse", "FLASH"), Some(1.0));
        assert_eq!(ai("pulse", "VENOM"), Some(2.0));
        assert_eq!(ai("2048", "Fighter"), Some(3.0));
        assert_eq!(ai("2048", "SPEED"), Some(4.0));
        assert_eq!(odds_for(&rows[2..], "pulse", "VENOM"), None);
    }
}
