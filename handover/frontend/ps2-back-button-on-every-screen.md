---
status: measure
titles: [pulse]
updated: 2026-11-20
summary: "Pulse PS2 leaves Cell and Grid Selection on the triangle (wired); other front-end screens still leave on the circle"
difficulty: medium (60)
---

# The PS2's Back is the triangle on every screen, wired on two

Found 2026-11-20 fixing GitHub issue 27 (lane `issue-27-quit-box`).

## Open

- The PS2 executable binds `backward` and `cancel` to the triangle (`docs/ghidra/functions/ps2-pulse-eu/input.md`,
  "The divergence", confidence 90) and the PS2 footers draw the triangle beside `Back`. `oag_title::FrontEnd::back_button(platform)`
  now answers it, and `Cell Selection` and `Grid Selection` read it (`CellSelection::with_buttons`, `GridSelection::with_back_button`,
  `docs/ui/campaign-screens.md`, "Back and Help on the PS2"). Every other screen still takes the circle as Back: `oag_ui::menu` (`menu.rs`, the `Button::Circle` arm), the pickers
  (`crate::picker`, `selection.rs`), `Team Selection`, the pause menu, the end-of-race pages.
- The player's own choice (CONTROLS > MENU BACK, `original`/`circle`/`triangle`) reaches every screen through one input exchange and one glyph
  substitution (`docs/ui/campaign-screens.md`, "Menu back"); it does not change `original`, so the PS2's other screens still leave on the circle by default.
- A screen that authors a `triangle` or `circle` redirect of its own can differ from the PSP's (Cell Selection's Help is `circle` on the PS2,
  `triangle` on the PSP), so a blanket circle/triangle swap at the input layer is not safe until each PS2 screen's redirects are read.

## Next Steps

1. Census the PS2 `GUI\*.xml` files in `WADS2.WAD` for `forward=`/`backward=` buttons against the PSP's, per screen.
2. Pass `FrontEnd::back_button` to each screen model that takes Back, the way `cell_buttons` (`oag_game::app::campaign_stage`) does.
3. A ground-truth drive per screen on `pulse-ps2-eu.chd`, like `campaign_cursor_ground_truth::cell_selection_backs_out_on_the_pressings_own_button`.
