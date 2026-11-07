//! Wipeout HD / Fury's own pause screens, as its disc authors them.

use oag_title::{
    Origin,
    PauseAction::{Audio, Options, Photo, Quit, Restart, Resume},
    PauseMenu, PauseScreen,
    pause_menu::row,
};

/// `Data\Plugins\Frontend\Gui\ingame_definition.xml`'s five single-player pause screens, identical
/// in the five archives that carry the file (`crates/raceplay/tests/main_menu_ground_truth.rs`).
/// The rows are the screen's live `Block` and `List` widgets in document order: the
/// `aVertMenu` and `aBlock` elements the file spells with an `a` prefix are switched off and are
/// not rows. The executable picks the screen from the game mode (`PauseMenu_BindRedirectForMode`
/// at `0x0023d660`, `docs/ghidra/functions/ps3-hdfury-eu/pause-menu.md`): mode 4 `Tournament`,
/// 5 `Time Trial`, 10 `Speed Lap`, 6 and 14 `Zone` take their own screen and every other
/// single-view mode the redirect's default `InGame Pause SP`. A split-screen race takes the
/// `Split Player` screens, a network race the `MP` ones; a race here opens neither.
///
/// Rows with nothing behind them in this build are not drawn: `FE_PILOT_ASSIST` and `FE_GHOST` are
/// on/off lists for settings that live on the CONTROLS and DISPLAY pages here, and
/// `ONL_VIEW_INV` is PSN. `IG_PAUSE_ENDSES` is the Speed Lap screen's only way out (it has no
/// RESTART and no QUIT) and leaves the race the way QUIT RACE does.
pub const PAUSE_MENU: &PauseMenu = &PauseMenu {
    heading: "IG_PAUSE_GPAUSED",
    screens: &[
        PauseScreen {
            name: "InGame Pause SP",
            rows: &[
                row("IG_PAUSE_CONT", Some(Resume)),
                row("FE_PILOT_ASSIST", None),
                row("OPT_GAME_OPT", Some(Options)),
                row("FE_AUDIO_OPT", Some(Audio)),
                row("MSC_PHOTO_PHOTOMODE", Some(Photo)),
                row("ONL_VIEW_INV", None),
                row("IG_PAUSE_REST", Some(Restart)),
                row("IG_PAUSE_QUIT", Some(Quit)),
            ],
        },
        PauseScreen {
            name: "InGame Pause SP Time Trial",
            rows: &[
                row("IG_PAUSE_CONT", Some(Resume)),
                row("FE_PILOT_ASSIST", None),
                row("FE_GHOST", None),
                row("OPT_GAME_OPT", Some(Options)),
                row("FE_AUDIO_OPT", Some(Audio)),
                row("MSC_PHOTO_PHOTOMODE", Some(Photo)),
                row("ONL_VIEW_INV", None),
                row("IG_PAUSE_REST", Some(Restart)),
                row("IG_PAUSE_QUIT", Some(Quit)),
            ],
        },
        PauseScreen {
            name: "InGame Pause SP Speed Lap",
            rows: &[
                row("IG_PAUSE_CONT", Some(Resume)),
                row("FE_PILOT_ASSIST", None),
                row("FE_GHOST", None),
                row("OPT_GAME_OPT", Some(Options)),
                row("FE_AUDIO_OPT", Some(Audio)),
                row("MSC_PHOTO_PHOTOMODE", Some(Photo)),
                row("ONL_VIEW_INV", None),
                row("IG_PAUSE_ENDSES", Some(Quit)),
            ],
        },
        PauseScreen {
            name: "InGame Pause SP Tournament",
            rows: &[
                row("IG_PAUSE_CONT", Some(Resume)),
                row("FE_PILOT_ASSIST", None),
                row("OPT_GAME_OPT", Some(Options)),
                row("FE_AUDIO_OPT", Some(Audio)),
                row("MSC_PHOTO_PHOTOMODE", Some(Photo)),
                row("ONL_VIEW_INV", None),
                row("IG_PAUSE_QUIT", Some(Quit)),
            ],
        },
        PauseScreen {
            name: "InGame Pause Zone",
            rows: &[
                row("IG_PAUSE_CONT", Some(Resume)),
                row("OPT_GAME_OPT", Some(Options)),
                row("FE_AUDIO_OPT", Some(Audio)),
                row("MSC_PHOTO_PHOTOMODE", Some(Photo)),
                row("ONL_VIEW_INV", None),
                row("IG_PAUSE_REST", Some(Restart)),
                row("IG_PAUSE_QUIT", Some(Quit)),
            ],
        },
    ],
    default_screen: "InGame Pause SP",
    by_mode: &[
        ("time_trial", "InGame Pause SP Time Trial"),
        ("speed_lap", "InGame Pause SP Speed Lap"),
        ("tournament", "InGame Pause SP Tournament"),
        ("zone", "InGame Pause Zone"),
    ],
    scrim_argb: 0xc000_0000,
    offers_photo: true,
    origin: Origin::Measured,
};

#[cfg(test)]
mod tests {
    use super::PAUSE_MENU;

    /// The executable's own switch on the game mode: Tournament, Time Trial, Speed Lap and Zone
    /// have screens, every other single-view mode the redirect's default.
    #[test]
    fn a_mode_opens_the_screen_the_executable_picks_for_it() {
        let screen = |mode| PAUSE_MENU.screen_for(mode).name;
        assert_eq!(screen("time_trial"), "InGame Pause SP Time Trial");
        assert_eq!(screen("speed_lap"), "InGame Pause SP Speed Lap");
        assert_eq!(screen("tournament"), "InGame Pause SP Tournament");
        assert_eq!(screen("zone"), "InGame Pause Zone");
        for mode in ["single_race", "head_to_head", "eliminator"] {
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

    /// Every screen has something to resume with and a way out: the Speed Lap screen's is END
    /// SESSION, so a table that dropped it would trap the player in the pause.
    #[test]
    fn every_screen_can_be_left() {
        use oag_title::PauseAction::{Quit, Resume};
        for screen in PAUSE_MENU.screens {
            let builds: Vec<_> = screen.rows.iter().filter_map(|row| row.builds).collect();
            assert!(builds.contains(&Resume), "{}", screen.name);
            assert!(builds.contains(&Quit), "{}", screen.name);
        }
    }
}
