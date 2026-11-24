---
status: measure
titles: [all]
updated: 2026-11-24
summary: "A member test-data-affected hold is the gpu-vram group over its cap of 4 (447 of 552 s); queue order buys 8 s; faster game launches are the lever"
size: M (45)
difficulty: medium (40)
---

# The gate lock hold is the `gpu-vram` group's floor

`docs/architecture/gate-wall-time.md` moved the compile out of the gate lock
(`gate-affected` hold 152 s to 89 s) and left the other half: a member's
`test-data-affected` holds the lock for 559 to 598 s when it selects the game and
raceplay packages, and the CPU is idle for most of it (load 4 to 14 on 24
threads).

## Open

- **The group is the floor.** In a 2,068-test selection 321 tests are in the
  `gpu-vram` group, 1,788 s summed; at the cap of 4 that is 447 s of the 552 s
  wall clock. A replay of the run's durations through a 24 slot scheduler gives
  514 s as queued, 506 s with the group first and 719 s last, so nextest
  priority is worth at most 8 s.
- **Where the group's seconds are.** `campaign_cursor_ground_truth` 210 s (33
  tests, mean 6.4 s), `drive_ground_truth` 193 s (24, 8.0 s),
  `pause_menu_ground_truth` 186 s (26, 7.2 s), `net_rejoin_ground_truth` 113 s
  (3), `photo_mode_ground_truth` 103 s (14), `footer_hint_ground_truth` 83 s
  (14). A typical test is one `oag-game --drive` launch. Where those 6 to 8 s go
  inside one launch is not measured (boot to first frame is 0.3 s on Pulse PSP,
  `docs/architecture/load-time.md`, so it is the script's ticks or the
  launch's own load).
- **The lead's slices are close to it as well**: 181 to 210 s of wall clock per
  slice against 118 to 133 s of group floor.

## Next Steps

1. Profile one `campaign_cursor_ground_truth` launch (`perf` with frame
   pointers, `docs/architecture/load-time.md`, "Method") and find what the
   6 s is. A launch that starts faster shortens every module above.
2. Merge tests that launch the game to answer one question each into one
   launch with several asserted states (`campaign_cursor_ground_truth` already
   reads two launches' states per test), keeping every assertion.
3. A larger cap is the maintainer's call (8 GiB card, 0.5 to 1.05 GiB per
   game); measure the peak with the cap raised before proposing it.

Measure with `just test-data-affected` on a branch that touches `oag-render`
through `scripts/gate-run.sh` (`gate-times.tsv` holds the hold) and the
per-module sums from its log (`docs/architecture/gate-wall-time.md`).
