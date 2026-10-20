# Race shots: Pulse PSP and HD give mid-pack pairs with the whole field; trails, a closer pack and more circuits are open

The maintainer asked whether the original can be put "in the middle of the pack via automation" and shot there, with weapons off.
**Pulse PSP can** (2026-10-20, `compare-race-shots`): a Single Race on the Racebox page with `WEAPONS` Off (`psp-drive.py menu
--single-race --weapons-off`, `g_weapons_enabled` reads 0), the player flown by the original's Autopilot, a RAM hunt
(`scripts/psp-race-field.py hunt`) for a tick with the player in a wanted place and other craft near and in view, a savestate
saved right after, and `psp-pulse-pair.py --field` (HD: `rpcs3-hd-pair.py --field`) making the pair with ours posed from **all eight craft**, their teams, the
camera and the HUD's place, lap, clocks, speed, shield and best lap, every one read out of the state's RAM at the same stop as
the screenshot. Recipe, checks and caveats: [`docs/tools/compare-page.md`](../../docs/tools/compare-page.md), "Race shots".
`just compare-rerender` re-makes these pairs (run once each; drift between two loads of one state 15 to 25 units on PSP, 16 on HD).

## Open

- **A closer pack.** With weapons off the player starts eighth and the Autopilot reaches 6th to 5th late in lap 1 or in lap 2;
  the field is spread out, so a match with another craft within 45 units in view is rare (one race in two or three).
  Ideas: the Racebox `AI DIFFICULTY` row (the walk leaves it at EASY, a lower field speed is not an option), a longer race (no
  Laps row on the Racebox page), or swapping the player's grid pose with a mid-field slot's in RAM before GO (a manipulation, would
  need its own caveat).
- **Stage 3: the exhaust plume and trails.** A craft at rest in ours has no plume and no trail; the original's player has both.
  A frozen warm-up (ticks run with the field pinned to the captured poses, so each craft's trail and exhaust history exist) is not
  built. `--field-from` keeps the grid's opponents alive and posed, with no tick run.
- **The pickup row** (the Autopilot icon and its `1000000000.00` counter) is not drawn in ours: nothing in the pair carries the
  player's held pickup. `entity+0x1bc` holds it.
- **HD: one circuit.** Talon's Junction, 6th of 8 in lap 2, weapons off, the Autopilot; the walk, the hunt, the paused restore and the
  pair are in the docs page. Open: more circuits and teams; the HUD record's word order flips between boots (see the docs, item 4:
  decoded both ways, refused when ambiguous, not understood); the best-lap line (not located in the record); the Fury `Absorb`/`DAMAGE`
  strips; `Pilot Assist` was left on (the profile's default, `0101`), the Autopilot flew with it.
- **PS2, Pure**: not attempted.
- **Layout is asserted, not probed** (`--layout forward`): the grid probe is a start-line test. A mid-race check of the layout would
  compare the player's pose with our spline.
- **Two captures of one state differ by about 4 ticks** (the CPU stops wall-clock after the load): the pose and camera of each
  capture are its own, so the pair is exact, but a state's picture is not byte-stable.

## Next Steps

1. Run the hunt on more circuits (`psp-drive.py menu --single-race --weapons-off --track-down N --any-track`, sidecar with the layout) for a set.
2. Try the pack-closing options above, in the order given, only after a plain hunt has failed twice on a circuit.
3. HD: more circuits with `emu-hd-race-walk.sh <lane> <index>` and `rpcs3-hd-race-hunt.py`; locate the best-lap word of the HUD record.
