//! Which races go through the ship picker.

use oag_race::Mode;

use super::skips_team_step;

/// Each title and whether its Zone race launches without the ship picker.
const TITLES: [(&oag_title::Title, bool); 5] = [
    (oag_pulse::TITLE, false),
    (oag_pure::TITLE, true),
    (oag_hd::TITLE, false),
    (oag_omega::TITLE, false),
    (oag_2048::TITLE, false),
];

/// **The maintainer's report, 2026-10-26**: "when doing a zone race, I'm no longer able to select
/// my team". Pulse's `Team Selection` runs in Zone (`zone-mode.md`), so its Zone race opens Ship
/// Select after Track Select, as the three titles that were not read in Zone do; only Pure, whose
/// own chain has no team step in Zone, launches on the stored team. Fails on a build that skips
/// the screen for Zone again.
#[test]
fn a_zone_race_opens_ship_select_on_every_title_but_pure() {
    for (title, skipped) in TITLES {
        assert_eq!(skips_team_step(title, Mode::Zone), skipped, "{}", title.name);
    }
}

/// Only Zone is ever skipped: every other mode takes the team step on every title.
#[test]
fn no_other_mode_skips_the_ship_picker() {
    for (title, _) in TITLES {
        for mode in Mode::ALL.into_iter().filter(|mode| *mode != Mode::Zone) {
            assert!(!skips_team_step(title, mode), "{} {mode:?}", title.name);
        }
    }
}
