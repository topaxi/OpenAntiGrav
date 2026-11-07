---
status: measure
titles: [pulse]
updated: 2026-11-07
summary: "Pulse flyby leftovers: authored Batch::depth_bias unapplied, the fade-in from white, weather under the flyby's section"
categories: [rendering, gameplay]
difficulty: medium (35)
---

# Pulse's flyby: what the stipple pass left open

The stipple itself is fixed: the flyby is masked by the section under its camera
([race-intro.md](../../docs/gameplay/race-intro.md#the-flyby-is-masked-by-the-section-under-its-camera-measured-2026-11-07-lane-pulse-flyby-fix)).

## Open

- **The authored depth bias is read and not applied**: `Batch::depth_bias` (`0..=9`) is on 5,605 of
  67,638 PSP Pulse batches (glows, additive cones, trees, `greenfade`); the original shifts the
  batch's depth range by `6 * bias` (live: `minz = 1001 + 6 * bias`). Applying it orders those
  batches against what they sit on; unmeasured on screen, and `Mesh_InitBatch` is unread on Pure.
- **The opening second differs in a second way**: the original fades in from white over about 40
  frames (frames 6 to 45 of the capture are washed out; ours shows the full scene from tick 1).
  Not chased; it is visible on every circuit.
- **Depth precision stays different**: coplanar copies drawn inside one section row would stipple in
  ours and tie cleanly in the original's 16-bit strict test. No opening shot shows one now; the
  later shots were not swept with the blend probe.
- **The weather under the flyby**: `Weather_Update` reads the same published section, and
  `Race::advance_weather` does not take the flyby's. Unmeasured whether a player can tell.

## Next Steps

1. Run the blend probe over the later shots of each circuit, not only the opening.
2. Decide separately whether `Batch::depth_bias` is applied.
