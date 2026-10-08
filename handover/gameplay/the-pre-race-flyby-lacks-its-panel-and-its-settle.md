---
categories: [rendering, frontend]
---

# The pre-race flyby plays with its panel; its settle and its tail do not

2026-10-01. The maintainer's play report (the original flies the circuit before a race) was
measured and ported: the camera is `start_grid.vex`'s `grid_camera1` animation, not a `Camera`
node, played for `AnimEnd` seconds or until a held Cross, with the world held at its first tick
(so the countdown and every hash are unchanged). Everything measured is in
[race-intro.md](../../docs/gameplay/race-intro.md) and
[the code page](../../docs/ghidra/functions/psp-pulse-usa/race-intro.md); the camera is
`oag_vex::grid_camera` and `oag_raceplay::intro_camera`; `--no-intro` skips it and
`--intro-ticks N --screenshot` photographs a tick of it.

## Open

- ~~The track-description panel~~ **landed 2026-10-02**: `InGameTrackDescriptionScreen` read off
  `InGame_Definition.xml`, name = the `PI_Track` id as a table key and paragraph =
  `MSC_TRACK_<nn>` (`RaceManager_Construct`), fade measured per frame (0.7 s linear, a 2 s width
  wipe in, 0.7 s out - not the 30 ticks this said), pointer press skips. See
  [race-intro.md](../../docs/gameplay/race-intro.md#the-track-description-panel). What is left of
  it: the name draws 2-3 % narrower than the original's and the paragraph's wrap width (450) is
  chosen, not measured (line breaks match on `16_Track` and `03_Track`); only Single Race and
  Time Trial were measured; Zone, Eliminator and the others take the same path unwatched.
- **The world does not settle** under the flyby, by choice (ticking it would move every hash):
  the craft sits at its placement pose, about two units low on `16_Track`. A faithful fix is a
  view-side settle of the draw pose, or moving the placement pose, both a decision.
- ~~HD's flyby craft sits 1.84 lower on the original~~ **landed 2026-10-08 (`hd-flyby-hover`)**: the
  original's hover target is `0.75 * min(5.5, entry+0x348)` with `+0x348` a `3.0x` clamp in the grid
  state and a seconds timer from GO, so the craft sits low through the flyby **and the countdown** and
  rises over 2.46 s after GO. Ported as HD `LaunchHover` title data: the spring reads the clamp, written
  per tick from the countdown clock, and the placement height is the lowered spring's rest height, so the
  held world at tick 0 is already low. **This changes the flyby's world-at-tick-0 design for HD only**: the
  placement pose is 1.85 lower than Pulse-style placement, and HD's countdown pose and first 2.5 s of
  racing physics differ from before (the craft rises under the released target). Evidence and the
  ours-against-original table: [hd-ride-height.md](../../docs/physics/hd-ride-height.md), the read:
  [hover-target.md](../../docs/ghidra/functions/ps3-hdfury-eu/hover-target.md). Open: which caller of
  `Craft_SetState(1)` runs at GO (timing measured, caller unnamed); modes that skip the grid branch
  (`0x000f2390`); a respawn's state 0 may re-lower a craft; rivals unmeasured; HD golden hashes that move
  (none found in the affected gate). Pulse's craft update was not read for the same branch.
- **The scenery does not animate under the flyby**: `Scene::render` takes its animation clock from
  `World::tick`, held at 0 for the 25 s. One line in `scene/frame.rs` (`race.sim.world.tick` ->
  `Race::motion_tick`, at the two places it is read), but the file is 1,000 lines of one function
  and has to be split first.
- **The tail**: the music the intro starts and stops (`g_music_player` calls at the counter's
  `0x28` and `0`), and the `ScreenFlash` kind 10 wash at its end, are not played.
  **The wash was photographed 2026-10-02** (`data/scratch/pulse-flyby-panel/fade-a/h0103.png` to
  `h0125.png`, one frame per call, scratch): the picture is solid white on the call after
  substate 2 first reads (h0103), already half-transparent over the chase view by h0105, and
  clear by about h0120, with the panel fading under it. Colour and curve are not measured; it is
  white at full strength at its start.
- **Demo mode** skips it (`g_game_mode == 2`); a byte at mode object `+0x40` also skips it
  without a held button and is unread. Only Single Race and Time Trial were captured.
- **A fresh menu-walk load on `03_Track` and Metropia reached the intro already in its fade-out
  substate** while RESTART RACE on both played it. Probably the walk's own button presses;
  not chased. `01_Track` (18 s) and `14_Track` (17.97 s) were never watched to their end.
- The field of view (`54.309` vertical degrees) is a measured constant: the leaf's payload is
  identical on all twelve circuits, and no derivation was found.

- **Other titles (2026-10-07)**: Wipeout HD's flyby is ported as `PreRace` Title data (camera loops, a press skips); see
  [race-intro.md](../../docs/gameplay/race-intro.md#other-titles-census-2026-10-07). Open there: HD's `START RACE`
  prompt overlay, HD's own fov/hold/lock/mode file (chosen), Pulse PS2 (same files, watch it on PCSX2 then widen
  `PRE_RACE.on`), 2048 (needs `AnimEnd`-less reading, watch on Vita3K), Pure's `StartSequenceCamera` (code camera,
  unread), HD Zone (`zone_N/start_grid.vex` has only `camera1_group`).

## Next Steps

1. Split `scene/frame.rs`, then give the scenery and the gantry's clock `Race::motion_tick`.
2. Decide Pulse's settle (HD's is landed as a lowered spring): read Pulse's `Ship_UpdateCraft` for the
   grid-state hover branch, then measure whether its flyby craft sits low.
3. Re-measure one of the short circuits (`01_Track`) through to its end, and a fresh-load
   Zone or Eliminator race.
