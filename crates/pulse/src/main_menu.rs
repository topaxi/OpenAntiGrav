//! Wipeout Pulse's own main menu, as its disc authors it.

use oag_title::{
    Attached, MainMenu, MainRow, Origin, Wording,
    main_menu::{MULTIPLAYER, original},
};

/// `Data\Plugins\PI001\GUI\MainMenu_Definition.xml`'s `Main Menu`, seven entries,
/// identical on the EU and USA PSP discs (`crates/raceplay/tests/main_menu_ground_truth.rs`).
/// The PS2 disc authors a different list: `FE_RACE_CAM_SS` and `FE_RACE_CAM_SSH` (the split
/// screen rows) after the first, no multiplayer or website row. This build shows the PSP's
/// `FE_MP` row, `MULTIPLAYER & SHARING`, on both pressings, and it replaces the PS2's two split
/// rows (the maintainer's flow, 2026-10-14: split screen is the LOCAL MULTIPLAYER row on the
/// MULTIPLAYER page); `crates/raceplay/tests/main_menu_ground_truth.rs` asserts the PS2's list
/// against the disc.
pub const MAIN_MENU: &MainMenu = &MainMenu {
    rows: &[
        original("FE_RACE_CAM", Some("campaign")),
        original("FE_RACEBOX", Some("racebox")),
        original("FE_MP", Some(MULTIPLAYER)),
        original("FE_WIPEOUT_DOT_COM", None),
        original("FE_PROFILE", None),
        original("FE_OPT_PLUS", Some("options")),
        original("FE_EXTRAS", None),
    ],
    multiplayer: Some(MainRow {
        label: Wording {
            id: "OAG_MENU_MULTIPLAYER_SHARING",
            text: "MULTIPLAYER & SHARING",
        },
        subtitle: Wording {
            id: "OAG_MENU_MULTIPLAYER_SHARING_HELP",
            text: "Race online or over Ad Hoc, and share data",
        },
    }),
    updated_rows: None,
    attached: Attached::Inline,
    origin: Origin::Measured,
};
