---
status: measure
titles: [hd, 2048, omega, pulse, pure]
updated: 2026-11-09
summary: "Wingtip contrails drawn on every title; open: a matched RPCS3 pair, the Turbo half, 2048 and Omega's own update"
---

# HD's wingtip contrails: drawn; the matched pair and the Turbo half are open

Evidence: [`wingtip-vortices.md`](../../docs/ghidra/functions/ps3-hdfury-eu/wingtip-vortices.md) (the update, the
`VortexLine` push, update, opacity, config, colour, blend; live reads) and
[`hd-wingtip-contrails.md`](../../docs/rendering/hd-wingtip-contrails.md) (frames, the live trigger runs, and "Wired in
ours").

## Landed (lane `hd-wingtip-wire`)

`oag_fx::fat_line::Vortex`, `Style::vortex_line`, `oag_raceplay::wingtip`, `oag_title::wingtip::Wingtip`, the livery's
`wingtip` pair (`locator_pair`), `crates/raceplay/tests/hd_wingtip_ground_truth.rs`. HD measured, 2048 and Omega by
lineage (labelled), Pulse and Pure behind `graphics.wingtip_contrails` (off by default).

## Open

- **No matched pair against the original** (the on-screen match is unknown; the original's natural-crossing frames show no
  readable streak either). Our frames were read against the original's description
  (two thin pale streaks from the wing tips running back toward the camera) and against ours with the contrails left out;
  no RPCS3 frame at our pose with a metric on a wing-tip box was taken (`scripts/rpcs3-hd-wingtip-poll.py` finds the nodes;
  the pad force time forced as in the doc's A/B).
- **Turbo half is code-only (80)**: no craft held a Turbo in the live runs; flag `0x40` is unnamed. Ours triggers on
  `turbo_timer > 0` (the flag `0x20` half).
- **The Zone picture** is not isolated: no frame separates the streaks from wall-scrape sparks.
- **2048 and Omega**: the update and push are not read (2048: importer string; Omega: `FUN_016fb470`); ours runs HD's by
  lineage and nobody has watched either title lay them.
- **The Pulse and Pure anchor is chosen**: the widest, rearmost lateral `Ship Collision Fx` locator per side. No
  original draws it, so no picture can settle it; the speed ramp is HD's units on a title that never ran it.
- **Craft states**: only `Destroyed` and `Eliminated` (the original's 4 and 5) gate the trigger here; state 6 and the
  others are unmodelled.

## Next Steps

1. RE lane: the Turbo half live (give a craft the Turbo, poll `+0xe4`), and 2048's and Omega's update (`FUN_016fb470`).
2. Capture lane: a matched RPCS3 pair on a wing-tip box (pad force time written as in the doc's A/B, pose from
   `scripts/ps3_pose.py`, ours through `--camera-pose`).
