---
categories: [rendering, frontend]
---

# The pre-race flyby plays; its panel, its settle and its tail do not

2026-10-01. The maintainer's play report (the original flies the circuit before a race) was
measured and ported: the camera is `start_grid.vex`'s `grid_camera1` animation, not a `Camera`
node, played for `AnimEnd` seconds or until a held Cross, with the world held at its first tick
(so the countdown and every hash are unchanged). Everything measured is in
[race-intro.md](../../docs/gameplay/race-intro.md) and
[the code page](../../docs/ghidra/functions/psp-pulse-usa/race-intro.md); the camera is
`oag_vex::grid_camera` and `oag_game::race::intro_camera`; `--no-intro` skips it and
`--intro-ticks N --screenshot` photographs a tick of it.

## Open

- **The track-description panel** over the flyby (circuit name, a paragraph, the 30-tick
  fade-out as the chase view returns) is not drawn. The text is a per-circuit string nobody
  has located; the screen is a front-end one and carries pointer rules.
- **The world does not settle** under the flyby, by choice (ticking it would move every hash):
  the craft sits at its placement pose, about two units low on `16_Track`. A faithful fix is a
  view-side settle of the draw pose, or moving the placement pose, both a decision.
- **The scenery does not animate under the flyby**: `Scene::render` takes its animation clock from
  `World::tick`, held at 0 for the 25 s. One line in `scene/frame.rs` (`race.sim.world.tick` ->
  `Race::motion_tick`, at the two places it is read), but the file is 1,000 lines of one function
  and has to be split first.
- **The tail**: the music the intro starts and stops (`g_music_player` calls at the counter's
  `0x28` and `0`), and the `ScreenFlash` kind 10 wash at its end, are not played.
- **Demo mode** skips it (`g_game_mode == 2`); a byte at mode object `+0x40` also skips it
  without a held button and is unread. Only Single Race and Time Trial were captured.
- **A fresh menu-walk load on `03_Track` and Metropia reached the intro already in its fade-out
  substate** while RESTART RACE on both played it. Probably the walk's own button presses;
  not chased. `01_Track` (18 s) and `14_Track` (17.97 s) were never watched to their end.
- The field of view (`54.309` vertical degrees) is a measured constant: the leaf's payload is
  identical on all twelve circuits, and no derivation was found.

## Next Steps

1. Locate the track-description string (the string table, per circuit) and draw the panel with
   its fade, with the pointer support a new screen needs.
2. Split `scene/frame.rs`, then give the scenery and the gantry's clock `Race::motion_tick`.
3. Decide the settle: a view-side offset on the drawn craft for the flyby, or leave it.
4. Re-measure one of the short circuits (`01_Track`) through to its end, and a fresh-load
   Zone or Eliminator race.
