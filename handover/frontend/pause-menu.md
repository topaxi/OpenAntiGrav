---
status: ready-to-wire
titles: [pulse, hd, omega, 2048, pure]
updated: 2026-11-25
summary: "Pulse, HD, Fury, Omega, Pure and 2048 pause into the disc's own screen per mode"
---

# The title's own pause screen

The read and what is built: `docs/ui/pause-menu.md`. The mechanism is `oag_title::Title::pause_menu`
(a table of `PauseScreen`s, the mode-to-screen map and `PauseAction`s), `oag_ui::menu::Definition::open_on_pause_screen`
(the page, at the root of the tree over a parked race, and the confirm pages a QUIT or RESTART row asks on) and
`session::pause_menu` in `oag-game`.

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
- **Pulse pause rows bloom** (`Renderer::set_glow_over_scene`, set for a frozen race; `docs/ui/pause-menu.md`). Open: the focused row's stamp phase is unpinned on the drive route, and the PS2's
  texel-alpha bloom over a pause page is unmeasured (`handover/frontend/the-worst-menu-pairs-after-the-pause-glow-and-the-title-faces.md`).
- **2048: wired** (`oag_2048::pause_menu`, 2026-11-25). Its screens are `<TouchButton>` icon pages
  (`PauseMenuRace`, `PauseMenuTournament`, `PauseMenuSpeed`), **not** HD's and Omega's widgets (checked, differs;
  Omega's copy of the file is a stub), drawn by `oag_ui_screens::pause::PauseScreen::read_touch`; QUIT and RESTART
  ask on the disc's confirm screens; pad and tap drive it. Open: the `SceneBlur` and the event card (`Callout
  CurGameMode`, native layout) the original draws are not drawn; the Speed Lap `CHOOSE_GHOST` tile opens `PauseMenuGhost`
  (NONE and MY BEST are `display.ghost_ship_visible`, the two online entries drawn grey and unselectable; layout
  chosen, not pictured); OPTIONS opens this build's page, the disc's is the touch `pause_options`
  tree; the Speed Lap and Tournament pages are tested by their tiles but were not walked live (the first campaign
  event is a Time Trial and `--race` has no `--drive`); the binder's default arm (an event object of ordinal 3 with
  no laps opens `PauseMenuSpeed`) is read, not walked.
- **HD, Fury: the FE Style** is the one thing a player in the HD style would see differ: the focused
  row, the opaque blocks and the grey follow `FEGlobals` (`HD_Blue`), and this build serves `DATA00`'s
  Fury palette on every page. The original's pause screen in the Fury style is red (RPCS3, Single Race and
  Time Trial) and matches ours; the HD style (white, teal) needs the front end's own style switch
  (`display.front_end_style` reaches the loading screen and the end screens, `oag_game::front_end_style`;
  the menus and the pause screen do not follow it yet).
- **HD, small residuals**: the lists' step arrows stop at the ends where ours wrap; the greyed row's fill is
  chosen (`INERT_FILL_ALPHA`); END SESSION leaves like QUIT, the original finishes the race into the results.
- Tournament and Eliminator on HD are the static reading, not walked live (Single Race, Time Trial, Speed Lap and Zone were).
- **Split screen** keeps START's freeze and the ownership rule; HD's `InGame Pause Split Player 1`/`2`,
  `Tournament Split` and `Zone P1`/`P2` screens are the model (`PauseMenu_BindRedirectForMode` on HD
  picks them per pad); the same drawer (`oag_ui_screens::pause`) takes them.
- **Pulse `MusicList`**: built (`oag_ui::menu::music_list`, `docs/ui/pause-menu.md`), PSP and PS2 code read. Open,
  PS2: the `--original-menu` still draws the PSP's six rows on PS2 (the PS2's own screen lists four), so the
  list stands under the sixth rather than at the original's `y`.
- **Pulse GAME MANUAL, END SESSION (Speed Lap), SKIP (Tournament)**: authored, nothing behind them here.
- **Pulse Tournament**: the screen is the static reading of the executable (conf 85); not walked live.
- **ESCAPE** (and the phone Back) resumes: chosen, the original has no such button; CIRCLE on the root does nothing, as on the disc.
- A Pilot Assist switch from the pause screen stops that run's recording (no replay or ghost lap): if a
  mid-race switch should be reproducible it has to enter the replay's input stream.

## Next Steps

1. 2048: a Vita3K frame of the Speed Lap and Tournament pages (and the confirm screen), and the event card the
   pause draws (`CampaignEventCard_Draw`, `docs/ghidra/functions/vita-2048-eu-v104/campaign-event-card.md`).
2. The front end's FE Style (HD palette: `DATA06`'s `HD_*` globals, opaque blocks, white page) so the HD-style
   pause frames can be matched too.
