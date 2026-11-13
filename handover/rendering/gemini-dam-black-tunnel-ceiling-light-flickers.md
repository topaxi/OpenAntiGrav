---
status: measure
titles: [pulse]
updated: 2026-11-13
summary: "Gemini Dam tunnel strip: depth bias applied, ours continuous; owed: a side-by-side with the original, and the arch lights"
size: S (25)
difficulty: easy (20)
---

# Gemini Dam Black: a tunnel ceiling light flickers rapidly

The maintainer saw this in play on 2026-11-12 (Pulse, PSP, Gemini Dam Black = `12_Track` forward). The maintainer confirmed it is the long
tunnel's ceiling strips; the measure lane (2026-11-12) found the cause at the long tunnel at spline rows about 1,180 to 1,430 and ruled out the time path.

## Settled (measured against the original)

- **Cause, confidence 75** (the bias was not applied in a test render; the signature and the data agree): the ceiling light strips of that tunnel (`Anotherblue_GLOW`, nodes 121, 122, 123, 133 of the pack's
  `track.vex`) are authored with a **depth bias** (`Batch::depth_bias` 3 on 121/122/123, 6 on 133) and lie on the ceiling
  plates. The original shifts the batch's depth range by `6 * bias` (`mesh-draw.md`, "The per-batch depth bias is file data"), so
  the glow wins; ours reads the byte and does not apply it, so the glow z-fights the ceiling. At a pose matched to about 1 unit
  (spline row 1205, craft held at the lifted sample) the original's band is one continuous cyan strip; ours is cut into dashes
  that **change pattern with a 0.05 unit camera step** (four steps, four different patterns, one with speckle), which is
  z-fighting, and shimmers in motion. A second site (row 1427, nodes 122 and 123) shows the original's strip whole and ours'
  edges ragged under the same steps, milder; the original's band was seen at two sites, once each. `--pvs false` and `true` give the same picture, so it is not visibility. Each of those
  meshes' batches is also listed twice in list A (`pass 0x1891` alpha-tested and `0x1011` opaque, same vertices) beside a list-B
  copy (`0x1092`); that is an authored triple and may be intended, but it doubles the tie.
- **The `TEXOFFSET` path is not the cause.** The original's per-frame offsets match `oag_vex::TexTransform::sample` (lerp, same
  keys and loops; `texture-animation.md`, "The updater's output measured per frame on Gemini Dam"), and the band's brightness
  follows the same 60-frame sawtooth in both. `12_Track` has no block with the step bit, so the 60-confidence snap direction of
  `sample_with` is untouched by this.
- `12_Track` is PSP DLC (Harimau `PACK3.edat`), run in the original with upper-case `.EDAT` names under
  `msroot:\PSP\GAME\UCES00465\`; Gemini Dam Black is Track Select entry 30 of 32 (`docs/formats/dlc-pack.md`).
  The grid state is `data/saves/pulse-psp/single-race-gemini-dam-black-grid.ppst` (USA serial).

## Landed

- **The authored `Batch::depth_bias` is applied** (2026-11-13, `gemini-depth-bias`): ours at spline row 1205 draws one
  continuous strip at four camera offsets 0.05 units apart, where it was cut into changing dashes
  ([`docs/rendering/depth-bias.md`](../../docs/rendering/depth-bias.md)). The other biased surfaces on 66 race frames
  moved by at most 0.22 % of a frame.

## Open

- **Ours after the fix against the original's crop** at row 1205 and row 1427 (nodes 122, 123) was not laid side by
  side: the fix was checked against ours before it, not against a new capture of the original.
- **The arch ceiling lights** (`screen1_GLOW` 4 cells at 12.5 frames, `screen3` 60-frame loop; nodes 89, 181, 275, 393, 405)
  were measured in the original's texture updater only; their frames were not compared (they hang above the chase camera).
  They carry no depth bias in the header read here, so they are not this defect; if the maintainer's light is one of them,
  the next step is a nose-up pose in the original. The maintainer confirmed the long tunnel's strips, not these.

## Next Steps

1. Ask the maintainer to fly Gemini Dam Black through the long tunnel and say whether the flicker is gone.
2. If it is, delete this thread; if a light still flickers, it is the arch lights: capture a nose-up pose in the original.
