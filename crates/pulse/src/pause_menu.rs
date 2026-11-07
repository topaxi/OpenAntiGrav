//! Wipeout Pulse's own pause screens, as its disc authors them.

use oag_title::{
    Origin, PauseMenu, PauseScreen,
    PauseAction::{Options, Photo, Quit, Restart, Resume},
    pause_menu::row,
};

/// `Data\Plugins\PI001\GUI\InGame_Definition.xml`'s four single-player pause screens, identical
/// on the EU and USA PSP discs (`crates/raceplay/tests/pause_menu_ground_truth.rs`). The
/// executable picks the screen from the game mode (`FUN_088e35d4` on the USA binary: mode 4
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
    offers_photo: true,
    origin: Origin::Measured,
};
