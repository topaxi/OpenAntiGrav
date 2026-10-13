//! Wipeout Pulse's own main menu, as its disc authors it.

use oag_title::{
    Attached, Borrowed, MainMenu, Origin,
    main_menu::{SPLIT_HORIZONTAL, SPLIT_VERTICAL, original},
};

/// `Data\Plugins\PI001\GUI\MainMenu_Definition.xml`'s `Main Menu`, seven entries,
/// identical on the EU and USA PSP discs (`crates/game/tests/main_menu_ground_truth.rs`).
/// The PS2 disc authors a different list: `FE_RACE_CAM_SS` and `FE_RACE_CAM_SSH` after the
/// first, no multiplayer or website row. [`MainMenu::borrowed`] holds those two so both
/// pressings show them where the PS2 puts them (the PSP **borrows** the PS2's design); the
/// rest keep their order on the PS2, which `crates/game/tests/main_menu_ground_truth.rs`
/// asserts against the PS2 disc.
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
    borrowed: &[
        Borrowed {
            after: "FE_RACE_CAM",
            disc_id: "FE_RACE_CAM_SS",
            builds: SPLIT_VERTICAL,
        },
        Borrowed {
            after: "FE_RACE_CAM",
            disc_id: "FE_RACE_CAM_SSH",
            builds: SPLIT_HORIZONTAL,
        },
    ],
    updated_rows: None,
    attached: Attached::Inline,
    origin: Origin::Measured,
};
