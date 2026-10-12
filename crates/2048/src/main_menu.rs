//! Wipeout 2048's own main menu, as its disc authors it.

use oag_title::{Attached, MainMenu, Origin, main_menu::original};

/// `NEWGUI/Definition.xml`'s `GameModeChoice`: the base package authors four
/// `<TouchButton>`s, the v1.04 update's copy six (`FE_RC_HD` and `FE_RC_FURY` between the
/// first and the multiplayer tile). The grid is the disc's own touch front end
/// (`oag_ui::frontend`), not a `menu.toml` row, so no entry builds one; this build's own
/// tiles come from the extras (`docs/architecture/menus.md`). The update is a patch, not
/// DLC: the two DLC flags (`g_bDlc1Mounted`, `g_bDlc2Mounted`) gate hotspots on the
/// campaign map, not tiles of this grid.
pub const MAIN_MENU: &MainMenu = &MainMenu {
    rows: &[
        original("FE_SP_CAMPAIGN", None),
        original("FE_MP_CAMPAIGN", None),
        original("FE_ADHOC", None),
        original("FE_CROSSPLAY", None),
    ],
    updated_rows: Some(&[
        original("FE_SP_CAMPAIGN", None),
        original("FE_RC_HD", None),
        original("FE_RC_FURY", None),
        original("FE_MP_CAMPAIGN", None),
        original("FE_ADHOC", None),
        original("FE_CROSSPLAY", None),
    ]),
    attached: Attached::Inline,
    origin: Origin::Measured,
};
