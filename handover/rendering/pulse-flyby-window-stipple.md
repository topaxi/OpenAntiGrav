---
status: measure
titles: [pulse]
updated: 2026-11-07
summary: "Platinum Rush flyby panes stipple: coplanar building copies tie in float depth; fix needs one GE dump of the held shot"
categories: [rendering, gameplay]
difficulty: medium (35)
---

# Pulse's flyby opening shot: the left dome's window panes stipple

2026-11-07 (`pulse-flyby-flicker`). The maintainer saw "a flickering texture at the beginning of
the flyby on the left half" on Platinum Rush. Reproduced in ours (the original is clean in the same
view); cause and evidence are in
[race-intro.md](../../docs/gameplay/race-intro.md#the-opening-shots-window-panes-stipple-on-platinum-rush-measured-2026-11-07-lane-pulse-flyby-flicker).
Short version: three copies of the start building (nodes 365, 342, 349 of `13_Track`) carry
exactly coplanar window panes (`rf_spanbuilding_windows_shinemap`); our 32-bit float depth breaks
the tie per pixel, the original's 16-bit strict-`GREATER` depth ties and the first drawn wins.

## Open

- **Does the original draw node 365 in the held shot?** One GE dump decides the fix. Capture it with
  `scripts/psp-ge-dump.py dump` while the track-description screen's first 28 ticks run (the
  `--restart` route presses Cross and skips the flyby; start a race from the menu walk,
  `psp-drive.py menu --single-race --unlock --track-down 19`, which gave Platinum Rush **Black** on
  this machine, and dump on the first `InGameTrackDescriptionScreen`), then
  `psp-ge-dump.py census` and match node 365's batches by vertex count and sorted extents
  (pane: 30 indices, extents 28.8 x 36.1 x 36.1; the 342/349 pane: 255 indices, 25.6 x 167 x 181).
  - **365 absent:** the fix is which copies the flyby admits (the PVS eye or section placement while the
    camera is off the racing line; `visible_set` in `crates/raceplay/src/camera.rs`, which the
    `photo-cull` lane owns - coordinate before touching it).
  - **365 present:** the fix is the tie rule: the first drawn wins within one 16-bit depth step
    (`1/63945` of the range, 1.2 levels a unit at 250). Mechanisms measured or ruled out: the
    viewport depth range (the original's own `sceGuDepthRange`; `Scene::draw_track` already sets
    one for the sky) shifted by one step per render layer (`0x45` before `0x4a`), which the shine
    stage must repeat per draw; a per-pipeline `DepthBiasState` (its unit is the primitive's own
    float exponent, so not a constant); the authored `batch+0x29` bias (`0` on every pane and
    wall, so no). The layer shift is a rule nobody measured and moves ties on every Pulse
    circuit: gate it on a measured frame, not on this lane's picture.
- **A red test for either fix**: the probe above (`--ticks 0 --intro-ticks 5` at `--draw-alpha 0.5`
  against `1.0`, mean absolute difference over the pane region `(250,290)-(365,370)` of a 960x544
  capture: `45.72` today, `0.00` with any two copies hidden). Put it in the lowest crate that reaches
  the race scene and its module in `.config/nextest.toml`'s `gpu-vram` group.
- **The authored depth bias is read and not applied**: `Batch::depth_bias` (`0..=9`) is on 5,605 of
  67,638 PSP Pulse batches (glows, additive cones, trees, `greenfade`); the original shifts the
  batch's depth range by `6 * bias` (live: `minz = 1001 + 6 * bias`). Applying it orders those
  batches against what they sit on; unmeasured on screen, and `Mesh_InitBatch` is unread on Pure.
- **The opening second differs in a second way**: the original fades in from white over about 40
  frames (frames 6 to 45 of the capture are washed out; ours shows the full scene from tick 1).
  Not chased; it is visible on every circuit.

## Next Steps

1. Capture the held-shot GE dump above and settle which branch applies.
2. Land the matching fix with the probe as its test; run the glow-mask and decal ground truths and
   a before/after pixel diff of `16_Track` and two other circuits.
3. Decide separately whether `Batch::depth_bias` is applied.
