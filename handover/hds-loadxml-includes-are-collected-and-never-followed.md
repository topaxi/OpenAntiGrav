# HD's `LoadXML` includes are collected and never followed

2026-08-18. `skin.xml` names 21 of them and `oag-game` reads only the root, so the 15 screens it parses are the boot dialogs and `Top FE Screen` - HD's actual `Main Menu` is in `MainMenu_Definition.xml` and is not among them. **The paths need rebasing**, which is the finding: the file spells them `Data\Plugins\PI001\GUI\MainMenu_Definition.xml`, the PSP-style numbered plugin that is *not on this disc at all*, while `Data\Plugins\Frontend\Gui\MainMenu_Definition.xml` resolves (11,035 bytes). The executable composes the skin path from the plugin's own location, so the runtime is presumably rebasing the same way. Following them is not by itself a menu: HD's main menu is a `<HorizMenu>`, and **this build draws one now** without following a single include - the widget's three numbers are in `oag_hd::frontend::MENU_SKIN.strip` per ADR-0022 and `crates/game/tests/hd_menu_ground_truth.rs` reads `MainMenu_Definition.xml` off all five archives that carry it. So **this is still open and is now smaller**: what following the includes would buy is HD's own *screens* and its own menu *tree*, not its layout. Worth doing when the disc's tree is wanted rather than this build's, which `docs/architecture/menus.md` argues it is not.

## Open

- Whether the runtime rebases `LoadXML` include paths the same way this reading presumes (`Data\Plugins\PI001\...` -> `Data\Plugins\Frontend\...`) is not confirmed
- Following the includes would recover HD's own screens and menu tree, not just its layout - not yet done

## Next Steps

- Follow the `LoadXML` includes to recover HD's own screen tree and menu structure, when the work needs the disc's authored tree rather than this build's menu
