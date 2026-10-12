//! Wipeout Pulse's own main menu, as its disc authors it.

use oag_title::{Attached, MainMenu, Origin, main_menu::original};

/// `Data\Plugins\PI001\GUI\MainMenu_Definition.xml`'s `Main Menu`, seven entries,
/// identical on the EU and USA PSP discs (`crates/game/tests/main_menu_ground_truth.rs`).
/// The PS2 disc authors a different list (`FE_RACE_CAM_SS` and `FE_RACE_CAM_SSH` after the
/// first, no multiplayer or website row); the rows this build has something behind are
/// the same five-or-fewer and keep their order there, which the same test asserts.
pub const MAIN_MENU: &MainMenu = &MainMenu {
    rows: &[
        original("FE_RACE_CAM", Some("campaign")),
        original("FE_RACEBOX", Some("racebox")),
        original("FE_MP", None),
        original("FE_WIPEOUT_DOT_COM", None),
        original("FE_PROFILE", None),
        original("FE_OPT_PLUS", Some("options")),
        original("FE_EXTRAS", None),
    ],
    updated_rows: None,
    attached: Attached::Inline,
    origin: Origin::Measured,
};
