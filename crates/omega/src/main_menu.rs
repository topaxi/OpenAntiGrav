//! Wipeout Omega Collection's own main menu, as its disc authors it.

use oag_title::{Attached, MainMenu, Origin, main_menu::original};

/// Omega's `MainMenu_Definition.xml` (`data09.psarc`): HD's five entries, in HD's order
/// (`crates/raceplay/tests/main_menu_ground_truth.rs`). Its front end is HD's carried forward.
pub const MAIN_MENU: &MainMenu = &MainMenu {
    rows: &[
        original("FE_RC", Some("campaign")),
        original("FE_RACEBOX", Some("racebox")),
        original("FE_ONLINE", None),
        original("FE_OPT_PLUS", Some("options")),
        original("FE_RECORDS", Some("records")),
    ],
    borrowed: &[],
    updated_rows: None,
    attached: Attached::Revealed,
    origin: Origin::Measured,
};
