---
status: ready-to-wire
titles: [pulse, hd, omega, 2048, pure]
updated: 2026-11-07
summary: "Pulse and HD/Fury pause into their own screen per mode; Omega read, not wired; 2048 and Pure open the main page"
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

- **Omega: read, tested, not wired** (`oag_omega::pause_menu`, `Title::pause_menu` is `None`): wire it
  (one line) once a frame of the original says what the title and footer look like over the tint; ours
  draws them in dark ink that is unreadable there. No shadPS4 or `sway` on the machine that did this.
- **2048, Pure: not wired** (`pause_menu: None`), in that order. 2048 has `InGame_Definition.xml` (36
  screens) and `LegacyInGame_Definition.xml` on Vita, with a binder of its own; Omega's reads the same
  mode ids as HD's, so compare 2048's against it first. Pure has `InGame Pause` and no photo screen,
  never seen live. A 2048 finding is checked against Omega and the reverse.
- **HD, Fury, Omega: drawn differences** (`docs/ui/pause-menu.md`, "HD and Fury", "Omega"): the race HUD
  is not drawn under the tint (the original draws it dimmed); the focused row is the front end's red, the
  original's `HD_Blue`; the list is at the page's left edge, the original's at `x = 480` of 1920; the
  footer has NAVIGATION where the original has `RESUME GAME`; the on/off lists `FE_PILOT_ASSIST` and
  `FE_GHOST` and the greyed `ONL_VIEW_INV` are not drawn; END SESSION leaves like QUIT, the original
  finishes the race into the results.
- Tournament and Eliminator on HD are the static reading, not walked live (Single Race, Time Trial, Speed Lap and Zone were).
- **Split screen** keeps START's freeze and the ownership rule; HD's `InGame Pause Split Player 1`/`2`,
  `Tournament Split` and `Zone P1`/`P2` screens are the model (`PauseMenu_BindRedirectForMode` on HD
  picks them per pad).
- **Pulse `MusicList`** (the playing track at the foot, LEFT and RIGHT change it): not drawn.
- **Pulse GAME MANUAL, END SESSION (Speed Lap), SKIP (Tournament)**: authored, nothing behind them here.
- **Pulse Tournament**: the screen is the static reading of the executable (conf 85); not walked live.
- **ESCAPE** (and the phone Back) resumes: chosen, the original has no such button; CIRCLE on the root does nothing, as on the disc.

## Next Steps

1. Omega: get a shadPS4 frame (`scripts/shadps4-omega-pair.py`'s rig, START after the grid), then wire
   `pause_menu: Some(...)` and fix the ink if the original's differs.
2. 2048: read `InGame_Definition.xml`'s pause screens and its binder in `/2048/eboot-vita-2048-eu-v104.elf`,
   add `crates/2048/src/pause_menu.rs` beside `crates/hd/src/pause_menu.rs`, a ground-truth test beside
   `hds_pause_screens_...`, then check Omega against it. Frames from Vita3K (the lane's recipe in
   `docs/reverse-engineering/emulator-recipes.md`).
3. Pure, then the HD-style differences above (the HUD under the tint is the one a player sees first).
