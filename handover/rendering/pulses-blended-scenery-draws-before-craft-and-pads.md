---
status: measure
titles: [pulse]
updated: 2026-11-07
summary: "Trees and bushes in front of a craft or a pad draw behind it: the circuit's blended list draws before both in our pass, hypothesis unmeasured"
---

# Pulse's blended scenery draws before craft and pads

The maintainer's report from play, 2026-11-07: in Pulse, trees render behind ships, speed pads and
weapon pads, even when the tree or bush is in front. Observed on Platinum Rush White
(`13_Track`), so take the pair there first, then check Black, which is unobserved.

## Open

- **Our order, read in code.** `crates/raceplay/src/scene/frame.rs` pushes `Draw::Track` first, and
  the track draws its solid and blended lists inside it (`Recorder::draw_track`). Pads, the gantry
  and craft come after it.
- **Hypothesis, unmeasured.** Trees are list-B meshes, in `transparent_draws`
  (`docs/rendering/draw-order.md`). If that list writes no depth, nothing stops a later pad or hull
  drawing over a tree nearer the eye. Confirm the state of the tree materials first.
- **The original.** One queue sorted by key: list-B meshes sit on layer `0x4d` with a view depth.
  The layers of a hull and a pad are not recorded on that page. If they sort below `0x4d`, a tree
  draws over them there.
- **Knock-on.** Moving the blended list changes its order against engine flares, plumes, the Rocket
  ribbon and shadows. `draw-order.md` names part of this: the flare interleaving is open, and the
  list-B meshes draw after the ribbon on the original.
- **Other titles.** All share this pass. HD has the same queue shape (`renderer.md`). Pure and Omega
  are unchecked, and the report is Pulse's alone.

## Next Steps

1. A frame of ours with a tree between the camera and a pad or craft, and the original at the same
   spot on PPSSPP, to show the bug and the target.
2. Read the hull's and pads' `Gfx_Enqueue` keys and the blended materials' depth state.
3. Reorder on measured keys, as data, with a pair per case and the other titles checked.
