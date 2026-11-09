//! Wipeout Pure's own pause screens, as its disc authors them.

use oag_title::{
    Origin,
    PauseAction::{Quit, Restart, Resume},
    PauseLayout, PauseMenu, PauseMenuRows, PauseScreen,
    pause_menu::row,
};

/// `Data\Plugins\PI001\GUI\InGame_Definition.xml`'s three pause screens with rows, identical on the
/// EU and USA PSP discs (`crates/raceplay/tests/main_menu_ground_truth.rs`). START opens
/// `InGame`'s `Redirect` (`forward="start"`), whose `Entry` rows the file spells in terms of the
/// front end's own state: a Tournament or a multiplayer race gets `InGame NoRestart Main Menu`,
/// Free Play `InGame Free Play Menu`, every other race the default `InGame Main Menu`. The mode
/// is read from the file, not from the executable (Pulse's and HD's is the binary's own switch).
///
/// `InGame Pause` is a different screen: a `Paused` text on a band with no rows.
///
/// `End Session` leaves the race the way QUIT does here; the original goes to
/// `InGame Void` and its results (`docs/ui/pause-menu.md`, "Pure"). **Chosen, not measured.**
pub const PAUSE_MENU: &PauseMenu = &PauseMenu {
    heading: "Paused",
    screens: &[
        PauseScreen {
            name: "InGame Main Menu",
            rows: &[
                row("Continue", Some(Resume)),
                row("Restart", Some(Restart)),
                row("Quit", Some(Quit)),
            ],
        },
        PauseScreen {
            name: "InGame NoRestart Main Menu",
            rows: &[row("Continue", Some(Resume)), row("Quit", Some(Quit))],
        },
        PauseScreen {
            name: "InGame Free Play Menu",
            rows: &[
                row("Continue", Some(Resume)),
                row("Restart", Some(Restart)),
                row("End Session", Some(Quit)),
            ],
        },
    ],
    default_screen: "InGame Main Menu",
    by_mode: &[
        ("tournament", "InGame NoRestart Main Menu"),
        ("free_play", "InGame Free Play Menu"),
    ],
    scrim_argb: 0x0000_0000,
    layout: Some(PauseLayout {
        definition: r"Data\Plugins\PI001\GUI\InGame_Definition.xml",
        chrome: None,
        grid: [480, 272],
        menu_rows: Some(PauseMenuRows {
            pitch: 15.0,
            unfocused_alpha: 0.50,
        }),
    }),
    hud_under_scrim: true,
    offers_photo: true,
    origin: Origin::Measured,
};

#[cfg(test)]
mod tests {
    use super::PAUSE_MENU;

    #[test]
    fn a_mode_opens_the_screen_the_disc_s_redirect_names_for_it() {
        let screen = |mode| PAUSE_MENU.screen_for(mode).name;
        assert_eq!(screen("tournament"), "InGame NoRestart Main Menu");
        assert_eq!(screen("free_play"), "InGame Free Play Menu");
        for mode in ["single_race", "time_trial", "zone", "speed_lap"] {
            assert_eq!(screen(mode), "InGame Main Menu", "{mode}");
        }
    }

    #[test]
    fn every_screen_a_mode_names_is_in_the_table_and_can_be_left() {
        use oag_title::PauseAction::{Quit, Resume};
        for (mode, screen) in PAUSE_MENU.by_mode {
            assert!(PAUSE_MENU.screen(screen).is_some(), "{mode}: {screen}");
        }
        for screen in PAUSE_MENU.screens {
            let builds: Vec<_> = screen.rows.iter().filter_map(|row| row.builds).collect();
            assert!(builds.contains(&Resume), "{}", screen.name);
            assert!(builds.contains(&Quit), "{}", screen.name);
        }
    }
}
