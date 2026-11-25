//! Wipeout 2048's own pause screens, as its disc authors them
//! (`docs/ui/pause-menu.md`, "2048").

use oag_title::{
    Origin,
    PauseAction::{Ghost, GhostList, Inert, Options, Photo, Quit, Restart, Resume},
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
/// and the pad's focus; the tile draws its icon alone. `CHOOSE_GHOST` opens `PauseMenuGhost`,
/// a sibling screen that repeats the tiles with the ghost tile filled in the cursor's colour and
/// adds a four-entry touch list (NONE, MY BEST, a friend's, the world's) and a tick back.
pub const PAUSE_MENU: &PauseMenu = &PauseMenu {
    heading: "IG_PAUSE_GPAUSED",
    screens: &[
        PauseScreen {
            name: "PauseMenuRace",
            rows: &[
                row("IG_PAUSE_OPT", Some(Options)),
                row("MSC_PHOTO_PHOTOMODE", Some(Photo)),
                named("quitButton", "IG_PAUSE_QUIT", Some(Quit)).opening("PauseQuitConfirm"),
                named("restartButton", "IG_PAUSE_REST", Some(Restart)).opening("PauseRetryConfirm"),
                named("continueButton", "IG_PAUSE_CONT", Some(Resume)),
            ],
        },
        PauseScreen {
            name: "PauseMenuTournament",
            rows: &[
                row("IG_PAUSE_OPT", Some(Options)),
                row("MSC_PHOTO_PHOTOMODE", Some(Photo)),
                named("quitButton", "IG_PAUSE_QUIT", Some(Quit))
                    .opening("TournamentPauseQuitConfirm"),
                named("continueButton", "IG_PAUSE_CONT", Some(Resume)),
            ],
        },
        PauseScreen {
            name: "PauseMenuSpeed",
            rows: &[
                row("IG_PAUSE_OPT", Some(Options)),
                row("MSC_PHOTO_PHOTOMODE", Some(Photo)),
                row("CHOOSE_GHOST", Some(Ghost)).opening("PauseMenuGhost"),
                named("quitButton", "IG_PAUSE_ENDSES", Some(Quit)).opening("SpeedPauseQuitConfirm"),
                named("restartButton", "IG_PAUSE_REST", Some(Restart))
                    .opening("SpeedPauseRetryConfirm"),
                named("continueButton", "IG_PAUSE_CONT", Some(Resume)),
            ],
        },
        PauseScreen {
            name: "PauseMenuGhost",
            rows: &[
                row("IG_PAUSE_OPT", Some(Options)),
                row("MSC_PHOTO_PHOTOMODE", Some(Photo)),
                row("CHOOSE_GHOST", Some(Inert)),
                named("GhostOption2048", "CHOOSE_GHOST", Some(GhostList)),
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
            inactive: "Grey2048",
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

    /// Every screen a row opens is a screen of the table. A confirm screen's own two tiles are
    /// the action and the way back; the ghost screen's list is the one a mode that has the ghost
    /// tile offers.
    #[test]
    fn every_screen_a_row_opens_is_in_the_table() {
        use oag_title::PauseAction::{Ghost, GhostList, Resume};
        for screen in PAUSE_MENU.screens {
            for row in screen.rows {
                let Some(name) = row.opens else { continue };
                let opened = PAUSE_MENU.screen(name).expect(name);
                if row.builds == Some(Ghost) {
                    assert!(
                        opened.rows.iter().any(|row| row.builds == Some(GhostList)),
                        "{name}"
                    );
                    continue;
                }
                assert_eq!(opened.rows.len(), 2, "{name}");
                assert_eq!(opened.rows[0].builds, row.builds, "{name}");
                assert_eq!(opened.rows[1].builds, Some(Resume), "{name}");
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
