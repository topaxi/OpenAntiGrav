---
status: ready-to-wire
titles: [pulse, hd, omega, 2048, pure]
updated: 2026-11-24
summary: "Pulse, HD, Fury, Omega and Pure pause into the disc's own screen per mode; 2048 opens the main page"
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
- **Pure: wired** (`oag_pure::pause_menu`, original frames of Single Race, Time Trial, Tournament and Free
  Play on PPSSPP in `docs/ui/pause-menu.md`, "Pure"). Open: the original's cursor does not wrap and ours
  does; Free Play's END SESSION leaves like QUIT where the original shows a `RETURN TO MAIN MENU` screen first
  (chosen). The now-playing line under the pause is left out since 2026-11-23 (`song_line_under_scrim`); the pause has
  a compare pair (`docs/tools/compare-page.md`, "Pure Main Menu and pause menu pairs"): the band matches to the pixel row,
  ours' words are the original's width and start on the original's column (the centred start is the executable's own, `docs/ghidra/functions/psp-pure-usa/menu-text-centring.md`: the band-word box 610 to 240 px over 40 grey; `docs/tools/compare-page.md`, "The rows' rounding, recovered from the executable").
- **2048: not wired** (`pause_menu: None`). 2048 has `InGame_Definition.xml` (36 screens) and
  `LegacyInGame_Definition.xml` on Vita, with a binder of its own; Omega's reads the same mode ids as HD's, so
  compare 2048's against it first. No Vita frame yet. A 2048 finding is checked against Omega and the reverse.
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
- **Pulse `MusicList`**: built (`oag_ui::menu::music_list`, `docs/ui/pause-menu.md`), PSP and PS2 code read. Open,
  PS2: ours names the songs `TRACK N` (no PSP pressing paired on that boot; the now-playing line shares it), and the
  `--original-menu` still draws the PSP's six rows on PS2, so its QUIT RACE sits on the music row.
- **Pulse GAME MANUAL, END SESSION (Speed Lap), SKIP (Tournament)**: authored, nothing behind them here.
- **Pulse Tournament**: the screen is the static reading of the executable (conf 85); not walked live.
- **ESCAPE** (and the phone Back) resumes: chosen, the original has no such button; CIRCLE on the root does nothing, as on the disc.
- A Pilot Assist switch from the pause screen stops that run's recording (no replay or ghost lap): if a
  mid-race switch should be reproducible it has to enter the replay's input stream.

## Next Steps

1. 2048 (Vita3K) against Omega: a Vita frame of its pause screen, then its binder and table.
2. The front end's FE Style (HD palette: `DATA06`'s `HD_*` globals, opaque blocks, white page) so the HD-style
   pause frames can be matched too.
