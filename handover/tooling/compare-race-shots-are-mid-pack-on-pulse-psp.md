# Race shots: Pulse PSP and HD give mid-pack pairs with the whole field; a closer pack and more circuits are open

The maintainer asked whether the original can be put "in the middle of the pack via automation" and shot there, with weapons off.
**Pulse PSP can** (2026-10-20, `compare-race-shots`): a Single Race on the Racebox page with `WEAPONS` Off (`psp-drive.py menu
--single-race --weapons-off`, `g_weapons_enabled` reads 0), the player flown by the original's Autopilot, a RAM hunt
(`scripts/psp-race-field.py hunt`) for a tick with the player in a wanted place and other craft near and in view, a savestate
saved right after, and `psp-pulse-pair.py --field` (HD: `rpcs3-hd-pair.py --field`) making the pair with ours posed from **all eight craft**, their teams, the
camera and the HUD's place, lap, clocks, speed, shield and best lap, every one read out of the state's RAM at the same stop as
the screenshot. Recipe, checks and caveats: [`docs/tools/compare-page.md`](../../docs/tools/compare-page.md), "Race shots".
`just compare-rerender` re-makes these pairs (drift between two loads of one state is up to 90 units on PSP, so a rerender can be refused by the pack check and kept as it was; HD restores landed on 14.0 to 14.2 s five times in six).

## Open

- **A closer pack.** With weapons off the player starts eighth and the Autopilot reaches 6th to 5th late in lap 1 or in lap 2;
  the field is spread out, so a match with another craft within 45 units in view is rare (one race in two or three).
  Ideas: the Racebox `AI DIFFICULTY` row (the walk leaves it at EASY, a lower field speed is not an option), a longer race (no
  Laps row on the Racebox page), or swapping the player's grid pose with a mid-field slot's in RAM before GO (a manipulation, would
  need its own caveat).
- **Trails and plume landed (2026-10-20, `--field-warmup`)**, an exhaust-only replay of every craft on a straight path at its captured velocity.
  Still open: the original's plume is wider and brighter than ours (a bloom, not a ribbon); the ribbon is straight where the original's curved;
  an opponent's throttle and every craft's flare block (`boost_timer`, `plume_timer`) are not read from RAM (full thrust and the engine's own state are chosen).
- **2026-10-26 (`talons-alongside`): a posed race shot was lit, culled and animated as if no tick had run** (hull at full light,
  every section visible, the neon clock 43 s short): fixed, whole frame 14.14 to 3.29 mean grey on `pulse-psp-race/talons-alongside`;
  `docs/tools/compare-page.md` has the table. **To adopt:** `just compare-rerender-ours --only pulse-psp-race` from the main checkout
  (it recomputes the world clock for a stored race shot). The Pure race shots, if any are made, need the same two fallbacks checked
  (they sit in generic `oag-raceplay` code, so they apply). **Still open here:** the Autopilot pickup row (icon and `5.00` counter) is
  not drawn (`--give` exists; the pair script would read the held pickup at `entity+0x1bc` and its timer and pass both), the speed
  readout (the HUD record's own value), and the ribbon's curve.
- **PSP hull skew.** On `pulse-psp-race/talons-alongside` the original hull is about 1.5x larger than ours at the camera read from RAM; holding
  the body back one tick of its velocity (about 1.8 units) matches its size (docs: "The player's hull in a race shot"). Find which of the body
  read and the camera node is a tick off the frame the PSP presents (the screenshot is the previous frame in a PPSSPP stop), then feed ours the
  right one. HD has no such skew.
- **The pickup row** (the Autopilot icon and its `1000000000.00` counter) is not drawn in ours: nothing in the pair carries the
  player's held pickup. `entity+0x1bc` holds it.
- **HD: one circuit.** Talon's Junction, 6th of 8 in lap 2, weapons off, the Autopilot; the walk, the hunt, the paused restore and the
  pair are in the docs page. Open: more circuits and teams; the HUD record's word order flips between boots (see the docs, item 4:
  decoded both ways, refused when ambiguous, not understood); the best-lap line (not located in the record); the Fury `Absorb`/`DAMAGE`
  strips; `Pilot Assist` was left on (the profile's default, `0101`), the Autopilot flew with it.
- **PS2, Pure**: not attempted.
- **An opponent's driver lane index is its grid row's, not its field pose's** (`start.rs`, `opponent.driver.index`): harmless at `--ticks 0` and for the
  exhaust-only warm-up (no AI runs); a capture that runs real ticks from a field would steer the AI at the wrong part of the circuit.
- **A second Pulse pair** (`talons-p6`, a loose pack) was dropped: its state never passed the pack check again; only `race-talons-venom-alongside` is kept.
- **Layout is asserted, not probed** (`--layout forward`): the grid probe is a start-line test. A mid-race check of the layout would
  compare the player's pose with our spline.
- **Two captures of one state differ by about 4 ticks** (the CPU stops wall-clock after the load): the pose and camera of each
  capture are its own, so the pair is exact, but a state's picture is not byte-stable.

## Next Steps

1. Run the hunt on more circuits (`psp-drive.py menu --single-race --weapons-off --track-down N --any-track`, sidecar with the layout) for a set.
2. Try the pack-closing options above, in the order given, only after a plain hunt has failed twice on a circuit.
3. HD: more circuits with `emu-hd-race-walk.sh <lane> <index>` and `rpcs3-hd-race-hunt.py`; locate the best-lap word of the HUD record.
