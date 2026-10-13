//! Wipeout Pure's own main menu, as its disc authors it.

use oag_title::{
    Attached, Borrowed, MainMenu, Origin,
    main_menu::{SPLIT_HORIZONTAL, SPLIT_VERTICAL, original},
};

/// `Data\Plugins\PI001\GUI\Skin.xml`'s `Main Menu` `Mode` menu: five rows on the EU disc;
/// the USA disc authors the same without `Teasers`, which nothing is built behind
/// (`crates/game/tests/main_menu_ground_truth.rs`). Its first row is the entry
/// `Campaign::main_row` words as SINGLE PLAYER.
///
/// Pure ships no split screen, so it mirrors its closest relative (maintainer, 2026-10-13):
/// Pulse's two split rows, **borrowed**, right after the first row as Pulse's PS2 pressing
/// puts them after `FE_RACE_CAM`.
pub const MAIN_MENU: &MainMenu = &MainMenu {
    rows: &[
        original("Single Player", Some("campaign")),
        original("Multiplayer", None),
        original("Profile", None),
        original("On Line", None),
        original("Teasers", None),
    ],
    borrowed: &[
        Borrowed {
            after: "Single Player",
            disc_id: "FE_RACE_CAM_SS",
            builds: SPLIT_VERTICAL,
        },
        Borrowed {
            after: "Single Player",
            disc_id: "FE_RACE_CAM_SSH",
            builds: SPLIT_HORIZONTAL,
        },
    ],
    updated_rows: None,
    attached: Attached::Inline,
    origin: Origin::Measured,
};
