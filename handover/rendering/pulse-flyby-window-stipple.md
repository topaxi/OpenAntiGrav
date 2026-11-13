---
status: measure
titles: [pulse]
updated: 2026-11-13
summary: "Pulse flyby leftovers: the fade-in from white, weather under the flyby's section, depth precision of coplanar copies"
categories: [rendering, gameplay]
difficulty: medium (35)
---

# Pulse's flyby: what the stipple pass left open

The stipple itself is fixed: the flyby is masked by the section under its camera
([race-intro.md](../../docs/gameplay/race-intro.md#the-flyby-is-masked-by-the-section-under-its-camera-measured-2026-11-07-lane-pulse-flyby-fix)).

## Open

- ~~**The authored depth bias is read and not applied**~~ landed 2026-11-13 (`gemini-depth-bias`): applied per vertex in
  the original's depth units, Gemini Dam's tunnel light strip fixed, other circuits within 0.22 % of a frame
  ([depth-bias.md](../../docs/rendering/depth-bias.md)). Unmeasured: `Mesh_InitBatch` on Pure.
- **The opening second differs in a second way**: the original fades in from white over about 40
  frames (frames 6 to 45 of the capture are washed out; ours shows the full scene from tick 1).
  Not chased; it is visible on every circuit.
- **Depth precision stays different**: coplanar copies drawn inside one section row would stipple in
  ours and tie cleanly in the original's 16-bit strict test. No opening shot shows one now; the
  later shots were not swept with the blend probe. One such pair is found and handled: a board's
  front and back copy on Moa Therma, separated by the per-batch back-face cull
  ([batch-cull.md](../../docs/rendering/batch-cull.md)); two front copies a ULP apart (nodes 868 and
  929) still draw twice there, identical, so harmless.
- **The weather under the flyby**: `Weather_Update` reads the same published section, and
  `Race::advance_weather` does not take the flyby's. Unmeasured whether a player can tell.

## Next Steps

1. Run the blend probe over the later shots of each circuit, not only the opening.
