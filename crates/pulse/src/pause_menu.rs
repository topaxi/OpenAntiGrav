//! Wipeout Pulse's own pause screens, as its disc authors them.

use oag_title::{
    Origin,
    PauseAction::{Options, Photo, Quit, Restart, Resume},
    PauseMenu, PauseScreen,
    pause_menu::row,
};

/// `Data\Plugins\PI001\GUI\InGame_Definition.xml`'s four single-player pause screens, identical
/// on the EU and USA PSP discs (`crates/raceplay/tests/pause_menu_ground_truth.rs`). The
/// executable picks the screen from the game mode (`PauseMenu_BindRedirectForMode` on the USA binary: mode 4
/// `Tournament`, 5 `Time Trial`, 10 `Speed Lap`, every other single-player mode the redirect's
/// default `InGame Pause SP`; the multiplayer modes 13 to 18 take `InGame Pause MP`, which a
/// race here never opens). `docs/ui/photo-mode.md`, "Entry".
///
/// `GAME OPTIONS` opens the OPTIONS page here; the disc's own `InGame Settings` is a
/// save-or-cancel page of the same settings. The game manual, ending a Speed Lap session and
/// skipping a tournament race have nothing behind them in this build and are not drawn.
pub const PAUSE_MENU: &PauseMenu = &PauseMenu {
    heading: "IG_PAUSE_GPAUSED",
    screens: &[
        PauseScreen {
            name: "InGame Pause SP",
            rows: &[
                row("IG_PAUSE_CONT", Some(Resume)),
                row("OPT_GAME_OPT", Some(Options)),
                row("EXT_PHOTO", Some(Photo)),
                row("IG_PAUSE_GMAN", None),
                row("IG_PAUSE_REST", Some(Restart)),
                row("IG_PAUSE_QUIT", Some(Quit)),
            ],
        },
        PauseScreen {
            name: "InGame Pause SP Time Trial",
            rows: &[
                row("IG_PAUSE_CONT", Some(Resume)),
                row("OPT_GAME_OPT", Some(Options)),
                row("EXT_PHOTO", Some(Photo)),
                row("IG_PAUSE_GMAN", None),
                row("IG_PAUSE_REST", Some(Restart)),
                row("IG_PAUSE_QUIT", Some(Quit)),
            ],
        },
        PauseScreen {
            name: "InGame Pause SP Speed Lap",
            rows: &[
                row("IG_PAUSE_CONT", Some(Resume)),
                row("OPT_GAME_OPT", Some(Options)),
                row("EXT_PHOTO", Some(Photo)),
                row("IG_PAUSE_ENDSES", None),
                row("IG_PAUSE_GMAN", None),
                row("IG_PAUSE_REST", Some(Restart)),
                row("IG_PAUSE_QUIT", Some(Quit)),
            ],
        },
        PauseScreen {
            name: "InGame Pause SP Tournament",
            rows: &[
                row("IG_PAUSE_CONT", Some(Resume)),
                row("OPT_GAME_OPT", Some(Options)),
                row("EXT_PHOTO", Some(Photo)),
                row("IG_PAUSE_GMAN", None),
                row("IG_PAUSE_SKIP", None),
                row("IG_PAUSE_QUIT", Some(Quit)),
            ],
        },
    ],
    default_screen: "InGame Pause SP",
    by_mode: &[
        ("time_trial", "InGame Pause SP Time Trial"),
        ("speed_lap", "InGame Pause SP Speed Lap"),
        ("tournament", "InGame Pause SP Tournament"),
    ],
    scrim_argb: 0xe800_0000,
    layout: None,
    hud_under_scrim: false,
    offers_photo: true,
    origin: Origin::Measured,
};

#[cfg(test)]
mod tests {
    use super::PAUSE_MENU;

    /// The executable's own switch on the game mode (`PauseMenu_BindRedirectForMode`): Tournament, Time Trial
    /// and Speed Lap have screens, every other mode the redirect's default.
    #[test]
    fn a_mode_opens_the_screen_the_executable_picks_for_it() {
        let screen = |mode| PAUSE_MENU.screen_for(mode).name;
        assert_eq!(screen("time_trial"), "InGame Pause SP Time Trial");
        assert_eq!(screen("speed_lap"), "InGame Pause SP Speed Lap");
        assert_eq!(screen("tournament"), "InGame Pause SP Tournament");
        for mode in ["single_race", "head_to_head", "zone", "eliminator"] {
            assert_eq!(screen(mode), "InGame Pause SP", "{mode}");
        }
    }

    #[test]
    fn every_screen_a_mode_names_is_in_the_table() {
        for (mode, screen) in PAUSE_MENU.by_mode {
            assert!(PAUSE_MENU.screen(screen).is_some(), "{mode}: {screen}");
        }
        assert!(PAUSE_MENU.screen(PAUSE_MENU.default_screen).is_some());
    }
}
