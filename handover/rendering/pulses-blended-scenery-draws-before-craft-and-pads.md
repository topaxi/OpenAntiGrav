---
status: ready-to-wire
titles: [pulse]
updated: 2026-11-07
summary: "Ours matches the original on hull, pads, list B; flares are the measured gap (they sort into list B by depth). Hull and pad report not reproduced"
---

# Pulse's blended scenery draws before craft and pads

The maintainer's report from play, 2026-11-07: in Pulse, trees render behind ships, speed pads and
weapon pads, even when the tree or bush is in front. Observed on Platinum Rush White (`13_Track`).
Measured on PPSSPP; the numbers, the method and the confidence are on
`docs/rendering/draw-order.md`, "Hull, pads and trees on Platinum Rush White".

## Open

- **Hull and pads: ours already matches the original at the default.** The original draws hull, then
  speed and weapon pads (layer `0x4a000000`), then list B (`0x4d000000` plus depth, no depth write).
  Ours does the same whenever `Race::ribbons_first()` holds (PSP ribbon, `trail_order = original`, the
  maintainer's own setting). The symptom as reported is the other order: `over-floors`, or the HD tube
  path (HD, Omega, 2048). Not reproduced on White: three matched poses and one synthetic pose.
- **Flares are the measured gap, ready to wire.** The original keys a flare `0x4d` plus depth, so list-B
  meshes nearer than a flare draw after it: in the logged frame all twelve list-B meshes outrank the
  three AI flares and none outranks the player's. Ours draws every flare after the whole blended list,
  so a nearer tree is under a farther craft's flare. Rule and knock-on table on the page; plumes,
  shield shells, weapon bodies, wrecks and the Rocket ribbon have no key read and stay where they are.
- **Open on the original:** the maintainer's spot (their settings hold `camera_view = "far"`), which of
  White and Black each Track Select press landed on (`--track-down 18` and `19` gave the same start
  frame), the hull body's second layer (the `0x4a` bump moved it too), the tree's own key under a
  list-B bump (the prim vanished from the census), the weapon pads' submit (`0x0890cdc8`), PS2, Pure's,
  HD's, 2048's and Omega's queue keys.

## Next Steps

1. Wire the flares into the blended list by depth, in `Model::blended_order`'s units made the
   original's first (`display+0x4310 / 65`); pair at a frame with an AI flare behind a nearer tree.
2. Ask the maintainer for the spot; if the hull and pad report reproduces at `original`, read the
   frame's order with the key bump of the page.
3. For HD, Omega and 2048, read each title's queue keys before moving a blended list: their path draws
   it before the craft today.
