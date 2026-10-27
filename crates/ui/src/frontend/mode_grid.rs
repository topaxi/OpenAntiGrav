//! Where `GameModeChoice`'s tiles sit, and which of them show.
//!
//! **Read, confidence 85** (`docs/ghidra/functions/vita-2048-eu-v104/game-mode-choice-layout.md`):
//! the screen's constructor, `FUN_81123a26`, calls `FUN_811237c0` with
//! "either downloadable campaign mounted" and that function overwrites every
//! tile's `x` and `y` after the file was read, so `Definition.xml`'s own
//! coordinates (the v1.04 patch authors the top row at `y=110`) are never the
//! ones drawn. The enter handler, `FUN_811241c2`, also clears the visible bits
//! of `confirmButton` every time and of `offlinehd`/`offlinefury` when their
//! package is not mounted.
//!
//! The rule, by the number of top-row tiles `n = 1 + dlc1 + dlc2`:
//!
//! | mounted | top row (`y`) | bottom row (`y`) |
//! | --- | --- | --- |
//! | none | one row of four, `y=190`: `x` 167, 329, 491, 653 (single player, then the three network modes) | - |
//! | one | single player `x=320`, the one tile `x=500`, `y=95` | `x` 228, 410, 592, `y=320` |
//! | both | `x` 228, 410, 592, `y=95` | `x` 228, 410, 592, `y=320` |
//!
//! Which package is mounted is read here from the maps the caller gave
//! ([`Frontend::has_campaign_tier`]), the nearest thing this build has to the
//! executable's `g_bDlc1Mounted`/`g_bDlc2Mounted` (**chosen proxy**).

use super::*;
use crate::screen::TouchButton;

/// What the original does with one `GameModeChoice` button, by its widget name.
pub(super) enum Place {
    /// Hidden: the tick, and a campaign tile whose package is not mounted.
    Hidden,
    /// Drawn with its top-left at `(x, y)`.
    At(f32, f32),
    /// Not one this rule names: the file's own position stands.
    Authored,
}

/// The place of button `name` given which downloadable campaigns are mounted.
pub(super) fn place(name: &str, dlc1: bool, dlc2: bool) -> Place {
    let top = ["offline", "offlinehd", "offlinefury"];
    let network = ["multiplayer", "adhoc", "crossplay"];
    let shown = [true, dlc1, dlc2];
    if name == "confirmButton" {
        return Place::Hidden;
    }
    if let Some(at) = top.iter().position(|n| *n == name) {
        if !shown[at] {
            return Place::Hidden;
        }
        if !(dlc1 || dlc2) {
            return Place::At(167.0, 190.0);
        }
        let rank = shown[..at].iter().filter(|s| **s).count();
        let xs: &[f32] = match shown.iter().filter(|s| **s).count() {
            2 => &[320.0, 500.0],
            _ => &[228.0, 410.0, 592.0],
        };
        return Place::At(xs[rank], 95.0);
    }
    if let Some(at) = network.iter().position(|n| *n == name) {
        return if dlc1 || dlc2 {
            Place::At([228.0, 410.0, 592.0][at], 320.0)
        } else {
            Place::At([329.0, 491.0, 653.0][at], 190.0)
        };
    }
    Place::Authored
}

impl Frontend {
    /// Which of the downloadable campaigns' tiles the original would show.
    pub(super) fn mounted_campaigns(&self) -> (bool, bool) {
        (
            self.has_campaign_tier("FE_RC_HD"),
            self.has_campaign_tier("FE_RC_FURY"),
        )
    }

    /// `button`'s rect on `GameModeChoice`, or `None` when it is hidden.
    pub(super) fn mode_button_rect(&self, button: &TouchButton) -> Option<[f32; 4]> {
        let (dlc1, dlc2) = self.mounted_campaigns();
        let authored = [button.x, button.y, button.width, button.height];
        match place(button.name.as_deref().unwrap_or(""), dlc1, dlc2) {
            Place::Hidden => None,
            Place::At(x, y) => Some([x, y, button.width, button.height]),
            Place::Authored => Some(authored),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(name: &str, dlc: (bool, bool)) -> Option<(f32, f32)> {
        match place(name, dlc.0, dlc.1) {
            Place::At(x, y) => Some((x, y)),
            _ => None,
        }
    }

    #[test]
    fn both_packages_make_two_rows_of_three_with_the_top_row_at_95() {
        let both = (true, true);
        assert_eq!(at("offline", both), Some((228.0, 95.0)));
        assert_eq!(at("offlinehd", both), Some((410.0, 95.0)));
        assert_eq!(at("offlinefury", both), Some((592.0, 95.0)));
        assert_eq!(at("multiplayer", both), Some((228.0, 320.0)));
        assert_eq!(at("adhoc", both), Some((410.0, 320.0)));
        assert_eq!(at("crossplay", both), Some((592.0, 320.0)));
    }

    #[test]
    fn one_package_centres_a_pair_on_the_top_row() {
        assert_eq!(at("offline", (true, false)), Some((320.0, 95.0)));
        assert_eq!(at("offlinehd", (true, false)), Some((500.0, 95.0)));
        assert!(matches!(place("offlinefury", true, false), Place::Hidden));
        assert_eq!(at("offline", (false, true)), Some((320.0, 95.0)));
        assert_eq!(at("offlinefury", (false, true)), Some((500.0, 95.0)));
        assert!(matches!(place("offlinehd", false, true), Place::Hidden));
        assert_eq!(at("adhoc", (true, false)), Some((410.0, 320.0)));
    }

    #[test]
    fn no_package_is_one_row_of_four_at_190_and_the_tick_is_always_hidden() {
        let none = (false, false);
        assert_eq!(at("offline", none), Some((167.0, 190.0)));
        assert_eq!(at("multiplayer", none), Some((329.0, 190.0)));
        assert_eq!(at("adhoc", none), Some((491.0, 190.0)));
        assert_eq!(at("crossplay", none), Some((653.0, 190.0)));
        assert!(matches!(place("offlinehd", false, false), Place::Hidden));
        assert!(matches!(place("confirmButton", true, true), Place::Hidden));
        assert!(matches!(
            place("somethingElse", true, true),
            Place::Authored
        ));
    }
}
