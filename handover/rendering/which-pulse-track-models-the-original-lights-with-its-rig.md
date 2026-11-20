---
status: measure
titles: [pulse, pure]
updated: 2026-11-20
summary: "Pulse PSP track rig now batch-ruled; open: the flash sprites' state, the 97 RET models' raw texel, Pure's twin"
categories: [rendering]
---

# Which Pulse track models the original lights with its rig

Landed: the selector (`model + 0xac` plus `SceneNode_SubmitTree`'s visit) and the batch rule (normals, no colour) are read and
dumped; the loader marks every such track vertex (`docs/ghidra/functions/psp-pulse-usa/scene-light.md`, "Which models take the rig").

## Open

- The animated flash sprites (`cameraflash_glow_add`, `07_camflash_Glow_Add`) are held back on the stand-in: with the rig they
  moved `de-konstruct-black` and `outpost-7-black` the wrong way. No dump has one in frame; catch one (breakpoint or a dump
  on the frame a flash blinks) and read its lighting state, then drop `RIG_HELD_BACK` in `crates/raceplay/src/load/pulse_psp.rs`.
- The 97 models whose list stayed `RET` (hidden or pooled; weapons, pickups) draw the raw texel in the original; ours lights them
  with the stand-in where it draws them. Needs one frame with such a model visible.
- Outpost 7 white's frame has a 317-vertex lit-capable model with no light enabled that is not the hull's extra pass; unidentified.
- Pure: `Vex_UpdateLightLists`' twin was not located, so its track keeps the stand-in.

## Next Steps

1. Dump a flash frame on De Konstruct (`psp-ge-dump.py dump` while a flash is up) and read `lighting` and `light0..3`.
2. Find Pure's `SceneNode_SubmitTree` caller and `+0xac` walk by the instruction shapes in `scene-light.md`; census one Pure circuit.
