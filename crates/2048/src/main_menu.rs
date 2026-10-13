//! Wipeout 2048's own main menu, as its disc authors it.

use oag_title::{Attached, MainMenu, Origin, main_menu::original};

/// `NEWGUI/Definition.xml`'s `GameModeChoice`: the base package authors four
/// `<TouchButton>`s, the v1.04 update's `data2.psarc` six (`FE_RC_HD` and `FE_RC_FURY`
/// between the first and the multiplayer tile). `data1.psarc`, the earlier patch, authors the
/// four plus an HD tile reusing `FE_SP_CAMPAIGN`; `data2` shadows it when both mount. The grid is the disc's own touch front end
/// (`oag_ui::frontend`), not a `menu.toml` row, so no entry builds one; this build's own
/// tiles come from the extras (`docs/architecture/menus.md`). The update is a patch, not
/// DLC. Whether the executable hides `FE_RC_HD` and `FE_RC_FURY` without the two DLC
/// packages is **not read** (`g_bDlc1Mounted` and `g_bDlc2Mounted` are read at dozens of
/// sites across the front end, `docs/ghidra/functions/vita-2048-eu-v104/frontend-campaign-map.md`;
/// the DLC archives themselves carry no front-end file), so the list follows the patch alone.
pub const MAIN_MENU: &MainMenu = &MainMenu {
    rows: &[
        original("FE_SP_CAMPAIGN", None),
        original("FE_MP_CAMPAIGN", None),
        original("FE_ADHOC", None),
        original("FE_CROSSPLAY", None),
    ],
    borrowed: &[],
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
