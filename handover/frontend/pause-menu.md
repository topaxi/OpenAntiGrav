---
status: ready-to-wire
titles: [pulse, hd, omega, 2048, pure]
updated: 2026-11-07
summary: "Pulse pauses into its own authored screen per mode; HD/Fury, Omega, 2048, Pure still open the main page over the parked race"
---

# The title's own pause screen

The read and what is built: `docs/ui/pause-menu.md`. The mechanism is `oag_title::Title::pause_menu`
(a table of `PauseScreen`s, the mode-to-screen map and `PauseAction`s), `oag_ui::menu::Definition::open_on_pause_screen`
(the page, at the root of the tree over a parked race) and `session::pause_menu` in `oag-game`.

Claim and oracle (maintainer): pausing a race opens the title's own authored pause screen for its mode,
drawn off the disc, with rows that work; a title that fails falls through to the main page, never a
stand-in screen. Photo mode is one implementation on all five (the row moves from the main page to the
pause screen where one applies). Online pause stays the maintainer's decision
(`handover/gameplay/pause-in-multiplayer.md`).

## Open

- **HD, Fury, Omega, 2048, Pure: not wired** (`pause_menu: None`), in that order. HD's
  `ingame_definition.xml` has 58 screens, a `Photo` block on the split-player, tournament and Zone pairs; the
  mode-to-screen choice is the executable's (`PauseMenu_BindRedirectForMode` is the Pulse one; HD's is unread).
  Omega's front end is HD's: check, do not assume. 2048 has `InGame_Definition.xml` and
  `LegacyInGame_Definition.xml`; Pure has `InGame Pause` and no photo row, never seen live. A 2048 finding is
  checked against Omega and the reverse.
- **Pulse `MusicList`** (the playing track at the foot, LEFT and RIGHT change it): not drawn.
- **Pulse GAME MANUAL, END SESSION (Speed Lap), SKIP (Tournament)**: authored, nothing behind them here.
- **Pulse Tournament**: the screen is the static reading of the executable (conf 85); not walked live, the
  Racebox flow lands on the tournament setup.
- **CIRCLE** on the pause root resumes (the original does nothing); the abstract back button is shared with ESCAPE.
- **Split screen** keeps START's freeze and the ownership rule; the original's `InGame Pause Split Player 1`
  screens (HD) are the model for it.

## Next Steps

1. HD and Fury: read `ingame_definition.xml`'s pause screens and where the executable picks one, add a
   `pause_menu` table beside `crates/hd/src/main_menu.rs`, a ground-truth test beside
   `main_menu_ground_truth`, frames from RPCS3 (ask the lead; one shared instance).
2. Omega (checked, applies or differs against HD), then 2048, then Pure.
