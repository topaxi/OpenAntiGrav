---
status: measure
titles: [pulse]
updated: 2026-11-12
summary: "A ceiling light in a Gemini Dam Black (12_Track) tunnel flickers rapidly on Pulse PSP; unknown whether the original flickers there too"
size: S (30)
difficulty: medium (40)
---

# Gemini Dam Black: a tunnel ceiling light flickers rapidly

The maintainer saw this in play on 2026-11-12 (Pulse, PSP, Gemini Dam Black = `12_Track` forward). Ours shows a ceiling light
in a tunnel that flickers rapidly. Which tunnel and which light have not been pinned down yet.

## Open

- **Does the original flicker there?** That question splits this into two cases:
  - The original also flickers: the light is an authored effect, such as a stepped `TEXOFFSET` track like `16_Track`'s flicker
    pairs (`oag_vex::TexTransform`, `docs/formats/vex.md`). Then the question is rate and phase: our step-direction choice
    (`sample_with`, confidence 60) or the loop period could make it run faster than the original.
  - The original is steady: ours is a defect. Candidates are z-fighting between coplanar light and ceiling meshes, a PVS cell
    boundary toggling the mesh, a `TEXOFFSET` track ours plays that the original does not, or a mesh stacked at the origin
    (`docs/rendering/scenery-animation.md`).
- Which mesh or material is flickering on `12_Track`.

## Next Steps

1. In ours (`just play`, Pulse PSP, Gemini Dam Black), find the tunnel. Record the camera pose, and the mesh/material under the
   light if a debug readout exists.
2. Capture the original at the same pose in PPSSPP (software renderer) for at least 2 s, then compare frame to frame.
3. If both flicker, measure each one's period (frames per on/off cycle). If only ours does, list the 12_Track meshes at that
   point that carry a texture-transform block or a step flag, and look for coplanar overlap.
