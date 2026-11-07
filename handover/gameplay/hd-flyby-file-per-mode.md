---
status: re-first
titles: [hd, omega]
updated: 2026-11-07
summary: "HD ships start_grid_ta and _de beside the plain fly-by and we load only the plain one; a lone craft's fly-by may show empty grid slots"
---

# HD's fly-by file per mode

The maintainer's question, 2026-11-07: on HD, a fly-by in Time Trial, Speed Lap, Zone or Head to
Head pans over grid slots no craft sits in. They suggest skipping those. They always skip the
fly-by on the original, so what it shows there is unknown: this is a question for the disc, not a
memory to match.

## Open

- **We load one file.** `oag_hd::pre_race` names plain `start_grid.vex` and the executable also
  builds `start_grid_de` and `start_grid_ta` (`docs/gameplay/race-intro.md`). Which mode takes
  which was not watched. Hypothesis, unmeasured: `_ta` is the lone craft's fly-by.
- **`_de` is unidentified.**
- **Zone.** `zone_N/start_grid.vex` has no `grid_camera1`, so what ours plays there needs its own
  check.
- **Head to Head.** It may take the plain file with six empty slots. If the original shows them,
  skipping is a departure of ours and needs the maintainer's call.
- **Omega ships HD's circuits**, so it gets the 2048 and Omega check; 2048's files differ.

## Next Steps

1. Find the caller that formats `%s\%sstart_grid%s.vex` in `/hdfury/EBOOT-ps3-hdfury-eu.elf` and
   read which mode picks which suffix.
2. Diff the camera tracks of plain, `_ta` and `_de` on one circuit.
3. Watch a Time Trial fly-by on RPCS3 against ours.
4. Wire the choice as `oag_title` data, one test per mode.
