---
status: measure
titles: [hd, omega]
updated: 2026-11-07
summary: "File per mode wired (Time Trial, Speed Lap, Zone fly _ta). Open: Head to Head's file (maintainer), Zone circuits, a watched fly-by"
---

# HD's fly-by file per mode

The maintainer's report, 2026-11-07: on HD, a fly-by in Time Trial, Speed Lap, Zone or Head to
Head pans over grid slots no craft sits in. Cause: ours flew the plain `start_grid.vex` for every
mode, where the executable flies `start_grid_ta.vex` for the single-craft modes.

**Landed** (`docs/ghidra/functions/ps3-hdfury-eu/start-grid-selection.md`,
`docs/gameplay/race-intro.md`, "HD's file per mode"): the selection law read off
`Track_Construct_q` (`0x000b2af0`), `_de` is mode 2's `Demo` fly-by, `oag_title::pre_race::PreRace::by_mode`
and the loader's try-then-fall-back, one `hd_flyby_ground_truth` test per mode.

## Open

- **Head to Head: the maintainer's call.** The executable gives mode 9 (Head to Head) the plain
  file, where two craft sit in slots 1 and 2 and the fly-by mostly frames empty ones. The original
  never offered the mode, so this is ours either way. The data would be `_ta` (it frames slots 1
  and 2 on Talon's Junction) as a chosen row, not skipping slots and not cutting the camera.
- **Zone's own circuits.** `zone_1` to `zone_4` ship no `_ta`; the fall-back `start_grid.vex` has
  no `grid_camera1`, so ours plays no fly-by there. What the original does is unwatched.
- **Watch one.** No mode's fly-by was watched on the original except Single Race on Talon's
  Junction. A Time Trial fly-by on RPCS3 against ours would turn the static read (84) into a
  measurement.
- **Unread branches.** The byte `0x009384e1` skips the grid file choice altogether (online?);
  mode 7 belongs to no mode of ours; Omega adds a track-name test and a game-mode-object field test to
  its `_ta` gate, and 2048's gate (`FUN_8121bce2`) was not read.
- **Omega** ships HD's circuits and no `_de`; its `PRE_RACE` is not on.

## Next Steps

1. Ask the maintainer which file Head to Head flies.
2. Watch a Time Trial fly-by on RPCS3 (Talon's Junction, stop before `START RACE`) and compare it
   with `--race --mode time_trial --intro-ticks N --screenshot`.
