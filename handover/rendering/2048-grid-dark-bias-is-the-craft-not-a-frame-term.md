---
status: re-first
titles: [2048, omega]
updated: 2026-11-09
summary: "2048 grid pairs: no frame term (fog, bloom, exposure, lighting scale) owns the dark bias; the bottom third is the craft; ambient x3-4 is a probe"
difficulty: medium (50)
---

# The 2048 grid pairs' dark bias is the craft and a spatial error, not a frame term

2026-11-09, lane `tone-2048-grid` (measure, read-only; ours rendered headless on SwiftShader). Claim: one frame-wide term
of ours accounts for most of the dark bias on `2048-empire-climb-grid`, `2048-mall-grid` and `2048-dlc-vineta-k-grid`.
**False** (confidence 80). Method, every toggle's numbers and the falsifier are in
[exposure.md](../../docs/ghidra/functions/vita-2048-eu-v104/exposure.md), "Where ours is darker than the original".

## Open

- **The craft is the bottom third.** The hull box is 37 % of the band and carries -15.9 of Empire Climb's -16.5 luma bias
  (Queens Mall -9.2 of -8.3, Vineta K -5.0 of -6.9); the band's scenery is -2.0 / +2.4 / -2.0. The original's craft is bright
  saturated yellow with a white shield ring and a glow on the road under it. Read the 2048 craft's pixel program
  (`constantAmbientColour`, the texel decode, any reflection or rim term) first. Ambient x3 to x4 on every no-lightmap
  material is a **probe, not a law**: it closes 62 / 69 / 46 % of the bottom third and lowers impact 10 / 14 / 9 % (x4: 9 /
  18 / 3 %), the hull box reaches zero bias at about x4.5 to x5.5, and on Vineta K it also lifts the top third by 9.7.
- **The top and middle thirds' scenery is 10 to 15 luma dark** on Empire Climb and Vineta K, and no frame term closes it:
  lightmap scale x1.5 closes the middle and overshoots the bottom scenery by 15; the sun's lens flare and the haze round
  it are in the original's top third and absent in ours ([lens-flare.md](../../docs/rendering/lens-flare.md), 2048 not wired).
- **Not the cause, measured:** fog (density 0.001, off or x2 moves impact by 1 %), bloom factor and gate (x0, x3, gate 0
  move impact by -10 to +41 % in no consistent direction), exposure (E = 1.15 is the original's value; E 1.4 changes impact
  by -5 / +23 / +11 %), sun diffuse (x2 is +75 %), a display gain (best -8 % on Empire Climb, worse on the other two).
- **Impact does not fall on a tone term.** A per-band per-channel affine fit leaves 9.06 / 8.53 / 6.59 M of 9.83 / 7.40 /
  6.34 M, so rows 3, 4 and 6 of `compare-pairs-ranked-by-area-times-error.md` are spatial error, mostly the craft's pose
  and light and the flare.
- The Vineta K baseline is 6.33 M by this method against the scout's 5.22 M; the other two agree within 0.1 M.
  Unreconciled (the scout's script is not in the repo).
- **Omega lineage:** fog, bloom, exposure: checked, differs. The craft's ambient term: checked, applies, not wired (same
  `constantAmbientColour` on 4,337 Omega pixel shaders); its factor is not checkable on Omega (no trusted race frame).

## Next Steps

1. Read the 2048 craft's pixel program from Vita3K's SPIR-V by the container's SHA-256 (`scripts/vita-gxp.py`,
   `2048-composite-leftovers.md` lists the recipe) and name the term that lights the craft past `ambient + sun * N.L`.
2. Re-run the three pairs with the `Pair`'s same flags after any craft-light change and quote the hull box's bias and the
   pair's impact (method in exposure.md); the target is the hull box at about 0, not a frame mean.
3. Decide with the maintainer whether the sun lens flare (`Lighting.Use Lens Flare`-style key on 2048, if authored) is a lane.
