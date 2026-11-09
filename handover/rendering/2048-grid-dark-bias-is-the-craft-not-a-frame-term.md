---
status: re-first
titles: [2048, omega]
updated: 2026-11-09
summary: "2048 grid pairs: no frame term (fog, bloom, exposure, lighting) owns the dark bias; the bottom third is the craft box (pose, ring, light)"
difficulty: medium (50)
---

# The 2048 grid pairs' dark bias is the craft's box and a spatial error, not a frame term

2026-11-09, lane `tone-2048-grid` (measure, read-only; ours rendered headless on SwiftShader). Claim: one frame-wide term
of ours accounts for most of the dark bias on `2048-empire-climb-grid`, `2048-mall-grid` and `2048-dlc-vineta-k-grid`.
**False** (confidence 75; each original is one capture). Method, every toggle's numbers and the falsifier are in
[exposure.md](../../docs/ghidra/functions/vita-2048-eu-v104/exposure.md), "Where ours is darker than the original".

## Open

- **The bottom third is the craft's box.** The hull box is 37 % of the band and carries -15.9 of Empire Climb's -16.5 luma
  bias (Queens Mall -9.2 of -8.3, Vineta K -5.0 of -6.9); the band's scenery is -2.0 / +2.4 / -2.0. Three parts, not separated:
  the original's craft is 1.31 x wider and 1.35 x taller (pose, `pair.json`); a white ring under it that ours does not draw
  (not the HUD's Pilot Assist bracket, which both sides draw; unidentified); and the light on the paint. Yellow-paint median
  luma at baseline: Queens Mall ours 130.1 against 156.1 (same pixel count, ours 17 % dark: a light term), Empire Climb ours
  152.6 against 143.1 (not darker: the gap is the road glow, ring and pose). Ambient x3 to x4 on every no-lightmap material
  closes 62 / 69 / 46 % of the bottom third and lowers impact 10 / 14 / 9 % (x4: 9 / 18 / 3 %), but overshoots Empire Climb's
  paint (192.9) and white panels and lifts Vineta K's top third by 9.7: a **probe, not a law**.
- **The top and middle thirds' scenery is 10 to 15 luma dark** on Empire Climb and Vineta K, and no frame term closes it:
  lightmap scale x1.5 closes the middle and overshoots the bottom scenery by 15; the sun's lens flare and the haze round
  it are in the original's top third and absent in ours ([lens-flare.md](../../docs/rendering/lens-flare.md), 2048 not wired).
- **Not the cause, measured:** fog (density 0.001, off or x2 moves impact by 1 %), bloom factor and gate (x0, x3, gate 0
  move impact by -10 to +41 % in no consistent direction), exposure (E = 1.15 is the original's value; E 1.4 changes impact
  by -5 / +23 / +11 %), sun diffuse (x2 is +75 %), a display gain (best -8 % on Empire Climb, worse on the other two).
- **Impact does not fall on a tone term.** A per-band per-channel affine fit leaves 9.06 / 8.53 / 6.59 M of 9.83 / 7.40 /
  6.34 M, so rows 3, 4 and 6 of `compare-pairs-ranked-by-area-times-error.md` are spatial error: the craft's pose, ring and
  light, and the flare.
- The Vineta K baseline is 6.33 M by this method against the scout's 5.22 M; the other two agree within 0.1 M.
  Unreconciled (the scout's script is not in the repo).
- **Omega lineage:** fog, bloom, exposure: checked, differs. The craft's light: not checkable here (neither title's craft
  program read; Omega's `constantAmbientColour` is on 4,337 pixel shaders, not shown to be the craft's); recipe `gcn_dump` on
  an Omega craft material. No trusted Omega race frame to fit a factor.

## Next Steps

1. Match the craft's pose on the three pairs (the recorded 1.31 x 1.35 size difference is the first suspect) and identify the
   white ring under the original's craft; re-measure the hull box's bias and the pair's impact (method in exposure.md).
2. Then read the 2048 craft's pixel program from Vita3K's SPIR-V by the container's SHA-256 (`scripts/vita-gxp.py`,
   `2048-composite-leftovers.md` has the recipe) and name the term that lights the paint past `ambient + sun * N.L`
   (Queens Mall, 17 % dark).
3. The sun lens flare on 2048: whether any 2048 `.EnvSettings` key gates it is unchecked.
