//! Wipeout Pulse's own pause screens, as its disc authors them.

use oag_title::{
    Origin,
    PauseAction::{Options, Photo, Quit, Restart, Resume},
    PauseMenu, PauseMusicList, PauseScreen,
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
    // The paused frame of a Time Trial shows the HUD under the tint: the record time, the lap hexagon and the `current`
    // clock, dimmed with the scene (`docs/ui/pause-menu.md`, "Pulse").
    hud_under_scrim: true,
    song_line_under_scrim: true,
    offers_photo: true,
    music_list: Some(MUSIC_LIST),
    music_list_ps2: Some(MUSIC_LIST_PS2),
    origin: Origin::Measured,
};

/// The four screens' `MusicList` (`y="220"`, no `Wrap`), drawn by the executable's `List` code
/// (`docs/ghidra/functions/psp-pulse-usa/music-list.md`, "The draw"). The arrows' cuts and x offsets are
/// read off `ListWidget_Enter`, the half alpha at an end off `ListWidget_ShadeArrows`; their y offset is measured
/// off the PPSSPP frames.
pub const MUSIC_LIST: PauseMusicList = PauseMusicList {
    y: 220.0,
    font: "small",
    wraps: false,
    sheet: r"Data\FE\Images\pulse_assets.mip",
    arrows: [[282.0, 96.0, 25.0, 28.0], [306.0, 96.0, 25.0, 28.0]],
    arrow_offset: [(-32.0, -4.0), (-20.0, -4.0)],
    arrow_dim: 0.5,
};

/// The PS2 pressing's: the same four screens author `y="270"` and `font="menu"` in the PS2's grid.
/// The PS2 executable's `List` and `MusicList` code is the PSP's, read on both
/// (`docs/ghidra/functions/ps2-pulse-eu/music-list.md`): the same cuts, offsets, shade and law.
pub const MUSIC_LIST_PS2: PauseMusicList = PauseMusicList {
    y: 270.0,
    font: "menu",
    ..MUSIC_LIST
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

    /// The paused frame of a Time Trial keeps the HUD under the tint.
    #[test]
    fn the_hud_stays_under_the_tint() {
        assert!(PAUSE_MENU.hud_under_scrim);
    }

    /// Each pressing draws the music list its own screens author: the PSP's at `y="220"` in the small
    /// face, the PS2's at `y="270"` in the rows' face; neither wraps.
    #[test]
    fn each_pressing_draws_its_own_music_list() {
        use oag_title::Platform;
        let psp = PAUSE_MENU.music_list_for(Platform::Psp).expect("PSP");
        let ps2 = PAUSE_MENU.music_list_for(Platform::Ps2).expect("PS2");
        assert_eq!((psp.y, psp.font, psp.wraps), (220.0, "small", false));
        assert_eq!((ps2.y, ps2.font, ps2.wraps), (270.0, "menu", false));
        assert_eq!(ps2.arrows, psp.arrows, "the PS2's List code is the PSP's");
    }

    #[test]
    fn every_screen_a_mode_names_is_in_the_table() {
        for (mode, screen) in PAUSE_MENU.by_mode {
            assert!(PAUSE_MENU.screen(screen).is_some(), "{mode}: {screen}");
        }
        assert!(PAUSE_MENU.screen(PAUSE_MENU.default_screen).is_some());
    }
}
