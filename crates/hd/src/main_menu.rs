//! Wipeout HD / Fury's own main menu, as its disc authors it.

use oag_title::{Attached, MainMenu, Origin, main_menu::original};

/// `MainMenu_Definition.xml`'s `<HorizMenu name="Mode">`, five entries, identical in the
/// five archives that carry the file (`crates/raceplay/tests/main_menu_ground_truth.rs`). HD has no
/// DLC that changes it. ONLINE has nothing behind it here; RECORDS opens this project's
/// records page.
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
