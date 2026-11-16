---
status: measure
titles: [hd, omega]
updated: 2026-11-07
summary: "File per mode wired: Time Trial, Speed Lap and Zone fly _ta, Head to Head too by choice. Open: Zone circuits, a watched fly-by"
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

- **Zone's own circuits.** `zone_1` to `zone_4` ship no `_ta`; the fall-back `start_grid.vex` has
  no `grid_camera1`, so ours plays no fly-by there. What the original does is unwatched.
- **Watch the grid close-ups.** A Time Trial fly-by was watched on RPCS3 (mode 5, first ten
  seconds match ours on `_ta`), but those shots frame no grid slot. Ours shows the craft on `_ta`
  at ticks 1,200 and 10,200 where the plain file shows empty slots; the original's clock needs
  pinning (a memory read of the fly-by time) to compare those.
- **Unread branches.** The byte `0x009384e1` skips the grid file choice altogether (online?);
  mode 7 belongs to no mode of ours; Omega adds a track-name test and a game-mode-object field test to
  its `_ta` gate, and 2048's gate (`FUN_8121bce2`) was not read.
- **Omega** ships HD's circuits and no `_de`; its `PRE_RACE` is not on.

## Next Steps

1. Pin the original's fly-by clock on RPCS3 and compare its grid close-ups (Talon's Junction,
   Time Trial) with `--race --mode time_trial --intro-ticks 1200 --screenshot`.
