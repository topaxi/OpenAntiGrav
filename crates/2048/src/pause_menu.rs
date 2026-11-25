//! Wipeout 2048's own pause screens, as its disc authors them
//! (`docs/ui/pause-menu.md`, "2048").

use oag_title::{
    Origin,
    PauseAction::{Options, Photo, Quit, Restart, Resume},
    PauseLayout, PauseMenu, PauseScreen, PauseTouch,
    pause_menu::{named, row},
};

/// `Data\Plugins\Frontend\NEWGUI\InGame_Definition.xml`'s three single-player pause screens, in
/// the same file in the base package and in the v1.04 patch. Each is a page of `<TouchButton>`
/// icon tiles, not HD's and Omega's `Block` and `List` widgets (checked, differs): the rows are
/// the tiles, keyed by the widget's `idstring` and, for the three icon-only tiles at the foot,
/// by its `name` (`quitButton`, `restartButton`, `continueButton`), in the file's order.
/// QUIT and RESTART open the confirm screen nested in the page, whose two tiles (a tick and a
/// cross, with no `idstring` or `name`) are keyed by their `redirect`.
///
/// The executable's binder (`FUN_8112e050`, `docs/ghidra/functions/vita-2048-eu-v104/pause-menu.md`)
/// writes `PauseMenuTournament` for mode 4 and `PauseMenuSpeed` for mode 10, and leaves every
/// other single-view mode on the redirect's default `PauseMenuRace`. `PauseMenuSpeed` is nested
/// in `PauseMenuSpeedTop`, which draws its panel and heading.
///
/// `IG_PAUSE_OPT` opens this build's OPTIONS page (the disc's is the touch `pause_options` tree).
/// The icon-only tiles are worded by the HD pause strings `2048` also ships, for the page's rows
/// and the pad's focus; the tile draws its icon alone. `CHOOSE_GHOST` opens a four-entry touch
/// list of which ghost to race; nothing here builds it.
pub const PAUSE_MENU: &PauseMenu = &PauseMenu {
    heading: "IG_PAUSE_GPAUSED",
    screens: &[
        PauseScreen {
            name: "PauseMenuRace",
            rows: &[
                row("IG_PAUSE_OPT", Some(Options)),
                row("MSC_PHOTO_PHOTOMODE", Some(Photo)),
                named("quitButton", "IG_PAUSE_QUIT", Some(Quit)).confirmed_on("PauseQuitConfirm"),
                named("restartButton", "IG_PAUSE_REST", Some(Restart))
                    .confirmed_on("PauseRetryConfirm"),
                named("continueButton", "IG_PAUSE_CONT", Some(Resume)),
            ],
        },
        PauseScreen {
            name: "PauseMenuTournament",
            rows: &[
                row("IG_PAUSE_OPT", Some(Options)),
                row("MSC_PHOTO_PHOTOMODE", Some(Photo)),
                named("quitButton", "IG_PAUSE_QUIT", Some(Quit))
                    .confirmed_on("TournamentPauseQuitConfirm"),
                named("continueButton", "IG_PAUSE_CONT", Some(Resume)),
            ],
        },
        PauseScreen {
            name: "PauseMenuSpeed",
            rows: &[
                row("IG_PAUSE_OPT", Some(Options)),
                row("MSC_PHOTO_PHOTOMODE", Some(Photo)),
                row("CHOOSE_GHOST", None),
                named("quitButton", "IG_PAUSE_ENDSES", Some(Quit))
                    .confirmed_on("SpeedPauseQuitConfirm"),
                named("restartButton", "IG_PAUSE_REST", Some(Restart))
                    .confirmed_on("SpeedPauseRetryConfirm"),
                named("continueButton", "IG_PAUSE_CONT", Some(Resume)),
            ],
        },
        PauseScreen {
            name: "PauseQuitConfirm",
            rows: &[
                named("KillGameVita", "FE_QUIT", Some(Quit)),
                named("PreviousScreen", "FE_CANCEL", Some(Resume)),
            ],
        },
        PauseScreen {
            name: "PauseRetryConfirm",
            rows: &[
                named("Retry", "FE_RETRY", Some(Restart)),
                named("PreviousScreen", "FE_CANCEL", Some(Resume)),
            ],
        },
        PauseScreen {
            name: "TournamentPauseQuitConfirm",
            rows: &[
                named("KillGameVita", "FE_QUIT", Some(Quit)),
                named("PreviousScreen", "FE_CANCEL", Some(Resume)),
            ],
        },
        PauseScreen {
            name: "SpeedPauseQuitConfirm",
            rows: &[
                named("End Session 2048", "IG_PAUSE_ENDSES", Some(Quit)),
                named("PreviousScreen", "FE_CANCEL", Some(Resume)),
            ],
        },
        PauseScreen {
            name: "SpeedPauseRetryConfirm",
            rows: &[
                named("Retry", "FE_RETRY", Some(Restart)),
                named("PreviousScreen", "FE_CANCEL", Some(Resume)),
            ],
        },
    ],
    default_screen: "PauseMenuRace",
    by_mode: &[
        ("tournament", "PauseMenuTournament"),
        ("speed_lap", "PauseMenuSpeed"),
    ],
    scrim_argb: 0x0000_0000,
    layout: Some(PauseLayout {
        definition: r"Data\Plugins\Frontend\NEWGUI\InGame_Definition.xml",
        chrome: None,
        grid: [960, 544],
        menu_rows: None,
        touch: Some(PauseTouch {
            tile: "Blue2048",
            cursor: "Orange2048",
        }),
    }),
    hud_under_scrim: false,
    song_line_under_scrim: true,
    offers_photo: true,
    music_list: None,
    music_list_ps2: None,
    origin: Origin::Measured,
};

#[cfg(test)]
mod tests {
    use super::PAUSE_MENU;

    #[test]
    fn a_mode_opens_the_screen_the_executable_picks_for_it() {
        let screen = |mode| PAUSE_MENU.screen_for(mode).name;
        assert_eq!(screen("tournament"), "PauseMenuTournament");
        assert_eq!(screen("speed_lap"), "PauseMenuSpeed");
        for mode in ["single_race", "time_trial", "zone", "head_to_head"] {
            assert_eq!(screen(mode), "PauseMenuRace", "{mode}");
        }
    }

    /// Every confirm screen a row names is a screen of the table, and the screen's own two tiles
    /// are the action and the way back.
    #[test]
    fn every_confirm_screen_a_row_names_is_in_the_table() {
        use oag_title::PauseAction::Resume;
        for screen in PAUSE_MENU.screens {
            for row in screen.rows {
                let Some(name) = row.confirm else { continue };
                let confirm = PAUSE_MENU.screen(name).expect(name);
                assert_eq!(confirm.rows.len(), 2, "{name}");
                assert_eq!(confirm.rows[0].builds, row.builds, "{name}");
                assert_eq!(confirm.rows[1].builds, Some(Resume), "{name}");
            }
        }
    }

    #[test]
    fn every_screen_a_mode_names_is_in_the_table() {
        for (mode, screen) in PAUSE_MENU.by_mode {
            assert!(PAUSE_MENU.screen(screen).is_some(), "{mode}: {screen}");
        }
        assert!(PAUSE_MENU.screen(PAUSE_MENU.default_screen).is_some());
    }

    /// Every screen a mode opens has the tick to resume with and a way out, so a table that
    /// dropped either would trap the player in the pause.
    #[test]
    fn every_screen_can_be_left() {
        use oag_title::PauseAction::{Quit, Resume};
        for screen in PAUSE_MENU
            .by_mode
            .iter()
            .map(|(_, name)| *name)
            .chain([PAUSE_MENU.default_screen])
            .filter_map(|name| PAUSE_MENU.screen(name))
        {
            let builds: Vec<_> = screen.rows.iter().filter_map(|row| row.builds).collect();
            assert!(builds.contains(&Resume), "{}", screen.name);
            assert!(builds.contains(&Quit), "{}", screen.name);
        }
    }
}
