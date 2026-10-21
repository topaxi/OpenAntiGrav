# HD's pit-box floor and Talon's start wall drew upside down: the flip came through a temporary

2026-10-21 (`hd-track-materials`), **closed for the claim, open for three leads.**
The finding, the disassembly and the numbers are in
[rcsmodel.md](../../docs/formats/rcsmodel.md), "The flip also arrives through a
temporary". The claim (one shared cause for Amphiseum's and Tech de Ra's start
grid and the left wall at Talon's start) is **true**: `Program::flips` only
counted a negated `v[Uv1]` written straight into an interpolator, and 20 of the
disc's 41 flipping vertex blocks write `1 - v` into a temporary that a later `MOV`
copies out.

## What is closed

- Amphiseum / Tech de Ra pit-box floor, Talon's `track_wall` block `0x19630`,
  Modesto Heights' `track_wall`: read as flipped, drawn like the RPCS3 original.
- Before and after on the same tree: grid floor Amphiseum 46.0 -> 30.3, Tech de
  Ra 55.1 -> 32.4 (grey RMSE), Talon's left wall 39.4 -> 9.3; no HD pair's
  whole-frame mean grey difference rises.

## Open

1. **`mageffect08`, `mageffect08_lh` and `reflectplane_dc_seawater` now flip**
   by the same rule and were not compared with an original (no pair frame holds
   the magstrip floor or the 03_track sea). A capture of a magstrip stretch on
   Amphiseum or Talon's, before and after, settles it.
2. **Omega and 2048 start grids** ship the same pit-box chunks under recompiled
   shaders (Vita `.gxp`, PS4); their orientation is not read at all
   (`mesh::rcs::psp2` has no flip reading). Look at one grid frame each and, if
   upside down, read the shader's coordinate setup.
3. **What is still different at the Amphiseum grid** after the fix: grid-floor
   RMSE is 30 against Talon's 25 on an already-right floor, so about 5 of it is
   Amphiseum's own light (the original's floor is smoother, ours keeps lightmap
   noise from `ile_mesh_combine_tracksurface_01-lmap`). Not investigated; the
   lightmap atlas for the pit boxes decodes to streaked noise on all three
   circuits and the chart UVs were not rasterised onto it.

## Next Steps

- Item 1 is a 15 minute capture if a magstrip stretch is already in a pair set;
  otherwise an afternoon with the RPCS3 drive script.
