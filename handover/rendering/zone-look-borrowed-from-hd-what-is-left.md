---
categories: [rendering, frontend]
---

# HD's Zone look on Pulse and Pure: what is left

2026-10-17. `graphics.zone_look` (off by default) works end to end: setting, menu
row, CIRCUIT list under Zone, loader, ground-truth test
`zone_ground_truth::a_borrowed_hd_zone_look_keeps_the_ordinary_circuit`. See
`docs/rendering/hd-zone-recolour.md`, "HD's look on Pulse and Pure". Screenshots
taken on Pulse PSP EU and Pure PSP EU, headless.

## Open

- HD's Zone sky (`RaceDefaults::zone_sky`) is not applied: `hd_sky_model` is only
  reached on a `.rcsmodel` circuit. Pulse and Pure keep their own sky under the grade.
- The HUD still reads Pulse's and Pure's own Zone widgets; HD's speed-class
  readout is not borrowed.
- Not played through a whole race: only the opening stage was screenshotted. A
  600-tick-plus capture should confirm the stage steps and the sphere sweeps on a
  Pulse mesh.
- Pulse PS2 and the `--zone-stage` pin were not exercised.
- `Session::sync_zone_look` and the prepare-time row drop each survey the disc
  search path; one shared survey would be cheaper.
- The loader report line (`zone look: borrowed from ...`) was not seen in a
  terminal run; confirm it lands in the report.

## Next Steps

1. Capture 1,500 ticks of `--race --mode zone --zone-look-source <hd.iso>` on Pulse and check stages 1 to 3.
2. Decide whether the borrowed look also swaps the sky.
