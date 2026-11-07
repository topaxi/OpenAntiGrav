---
status: ready-to-wire
titles: [pulse, hd, omega, 2048, pure]
updated: 2026-11-07
summary: "Pulse, HD, Fury and Omega pause into the disc's own screen per mode; 2048 and Pure open the main page"
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

- **Omega: wired** (`oag_omega::pause_menu`, original frames of Single Race and Time Trial on shadPS4 in
  `docs/ui/pause-menu.md`, "Omega"). Open: the START glyph draws HD's triangle where the original draws the
  OPTIONS pill (`FE_START_BUTTON` is `"I"`; is `data00`'s or `data08`'s `PS_BUTTONS.fnt` read?); Speed Lap,
  Eliminator, Tournament and Zone not all walked live (the doc says which); the original's own game options
  page was not read for `GHOST SHIPS VISIBLE` (Omega's `ship_visible` is on for the pause GHOST list).
- **2048, Pure: not wired** (`pause_menu: None`), in that order. 2048 has `InGame_Definition.xml` (36
  screens) and `LegacyInGame_Definition.xml` on Vita, with a binder of its own; Omega's reads the same
  mode ids as HD's, so compare 2048's against it first. Pure has `InGame Pause` and no photo screen,
  never seen live (PPSSPP, own frame needed). A 2048 finding is checked against Omega and the reverse.
- **HD, Fury: the FE Style** is the one thing a player in the HD style would see differ: the focused
  row, the opaque blocks and the grey follow `FEGlobals` (`HD_Blue`), and this build serves `DATA00`'s
  Fury palette on every page. The original's pause screen in the Fury style is red (RPCS3, Single Race and
  Time Trial) and matches ours; the HD style (white, teal) needs the front end's own style switch
  (`display.front_end_style` selects only the loading illustration here).
- **HD, small residuals**: the lists' step arrows stop at the ends where ours wrap; the greyed row's fill is
  chosen (`INERT_FILL_ALPHA`); END SESSION leaves like QUIT, the original finishes the race into the results.
- Tournament and Eliminator on HD are the static reading, not walked live (Single Race, Time Trial, Speed Lap and Zone were).
- **Split screen** keeps START's freeze and the ownership rule; HD's `InGame Pause Split Player 1`/`2`,
  `Tournament Split` and `Zone P1`/`P2` screens are the model (`PauseMenu_BindRedirectForMode` on HD
  picks them per pad); the same drawer (`oag_ui_screens::pause`) takes them.
- **Pulse `MusicList`** (the playing track at the foot, LEFT and RIGHT change it): not drawn.
- **Pulse GAME MANUAL, END SESSION (Speed Lap), SKIP (Tournament)**: authored, nothing behind them here.
- **Pulse Tournament**: the screen is the static reading of the executable (conf 85); not walked live.
- **ESCAPE** (and the phone Back) resumes: chosen, the original has no such button; CIRCLE on the root does nothing, as on the disc.
- A Pilot Assist switch from the pause screen stops that run's recording (no replay or ghost lap): if a
  mid-race switch should be reproducible it has to enter the replay's input stream.

## Next Steps

1. Pure: build `crates/pure/src/pause_menu.rs` beside `crates/pulse/src/pause_menu.rs` (Pulse's `Option`
   mechanism, the front end's list), a ground-truth test beside `pulses_*_pause_screens_*`, and a PPSSPP
   frame of `InGame Pause`; then 2048 (Vita3K) against Omega.
2. The front end's FE Style (HD palette: `DATA06`'s `HD_*` globals, opaque blocks, white page) so the HD-style
   pause frames can be matched too.
