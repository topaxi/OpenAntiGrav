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

- **Does the original draw node 365 in the held shot?** The one fact that picks the fix. Two GE-dump
  routes were tried and failed: `psp-flyby.py --restart` presses Cross and skips the flyby; a
  menu-walk dump (`psp-drive.py menu --single-race --unlock --track-down 19`, Platinum Rush **Black**)
  taken at the description screen's onset is 721 two-vertex UI prims, and one taken 1 s later is a
  830-prim frame whose geometry sits near x -700 (the grid), nothing near the building. A dump 5 s
  into the flyby (camera at the right dome) has 349's pane and not 365's. Routes that avoid the
  timing problem (not run):
  - **Save state, then dump**: stop at about hit 10 of `psp-flyby.py --frames`, save a state, dump
    from the state (`docs/reverse-engineering/ppsspp-debugger.md`, save states).
  - **Match the pane's look, no emulator timing**: `frames.jsonl` of a `psp-flyby.py --frames` run
    carries each frame's render view. Render ours with `--camera-pose`/`--camera-fov` at a held frame,
    once with only node 365 drawn and once with only 342/349, and see which pane look matches the
    original's frame (365 draws first and would win ties, so a 342/349 look means the original culls
    365; a 365 look is ambiguous).
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

1. Settle which branch applies by one of the two routes above.
2. Land the matching fix with the probe as its test; run the glow-mask and decal ground truths and
   a before/after pixel diff of `16_Track` and two other circuits.
3. Decide separately whether `Batch::depth_bias` is applied.
