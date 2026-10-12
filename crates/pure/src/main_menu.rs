//! Wipeout Pure's own main menu, as its disc authors it.

use oag_title::{Attached, MainMenu, Origin, main_menu::original};

/// `Data\Plugins\PI001\GUI\Skin.xml`'s `Main Menu` `Mode` menu, identical on the EU and
/// USA discs (`crates/game/tests/main_menu_ground_truth.rs`). Its first row is the entry
/// `Campaign::main_row` words as SINGLE PLAYER.
pub const MAIN_MENU: &MainMenu = &MainMenu {
    rows: &[
        original("Single Player", Some("campaign")),
        original("Multiplayer", None),
        original("Profile", None),
        original("On Line", None),
        original("Teasers", None),
    ],
    updated_rows: None,
    attached: Attached::Inline,
    origin: Origin::Measured,
};
