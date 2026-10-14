//! Wipeout Pure's own main menu, as its disc authors it.

use oag_title::{
    Attached, MainMenu, Origin,
    main_menu::{MULTIPLAYER, original},
};

/// `Data\Plugins\PI001\GUI\Skin.xml`'s `Main Menu` `Mode` menu: five rows on the EU disc;
/// the USA disc authors the same without `Teasers`, which nothing is built behind
/// (`crates/raceplay/tests/main_menu_ground_truth.rs`). Its first row is the entry
/// `Campaign::main_row` words as SINGLE PLAYER.
///
/// Its `Multiplayer` row opens the MULTIPLAYER page, which has the LOCAL MULTIPLAYER (split
/// screen) row Pure itself never had; the row's words are the disc's own, which are also the
/// project's (`MULTIPLAYER`).
pub const MAIN_MENU: &MainMenu = &MainMenu {
    rows: &[
        original("Single Player", Some("campaign")),
        original("Multiplayer", Some(MULTIPLAYER)),
        original("Profile", None),
        original("On Line", None),
        original("Teasers", None),
    ],
    multiplayer: None,
    updated_rows: None,
    attached: Attached::Inline,
    origin: Origin::Measured,
};
