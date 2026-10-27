---
status: re-first
titles: [2048]
updated: 2026-10-27
summary: "2048 lattice: scale read (camera tilt, executed matrix); phase offset and a half-ink factor measured not read; draw held back"
---

# 2048's menu backdrop lattice: draw held back on a phase offset and an ink factor

Built 2026-10-27 (lane `2048-lattice`) and **not merged**: a tile baked off `canvasTexture.gxt` and
tiled on the title screen and game mode page with the item's reset camera. The lead held the draw back
because it measured further from the original than plain white: in the title pair's empty areas, mean
grey 2.4 -> 6.3 (above the logo), 1.7 -> 7.1 (below the copyright), 2.2 -> 6.2 (left of the logo). At the
wrong scale the lines miss the original's, and they are darker. The code is on branch `2048-lattice`
until the scale and the grey are read; then re-measure and merge.
Law, camera writers (pinch and drag only) and measurements:
[`canvas3d-lattice.md`](../../docs/ghidra/functions/vita-2048-eu-v104/canvas3d-lattice.md).

## Open

Read 2026-10-27 (lane `lattice-scale`, [`canvas3d-lattice.md`](../../docs/ghidra/functions/vita-2048-eu-v104/canvas3d-lattice.md),
"The camera is tilted"): the scale gap is the camera's tilt (`-0.45` and `-0.1` rad Euler angles at the
reset, rotation built by `FUN_812e4a04`); the executed matrix gives 59.7 x 55.1 px per unit, a 47.72 px
row period (capture 47.75) and a 2.6 px/unit shear, and fits the capture at edge NCC 0.87. **The
branch `2048-lattice`'s axis-aligned tile draw is the wrong projection** (it has no shear and the
wrong scale); redo it from the matrix, do not patch it.

- **A phase offset of about (0.40, 0.49) world units** (screen 22.6 px right, 27 px up, equal on the
  title and game mode pairs) is needed to land the lattice on the capture: the cell origin or mesh
  placement, or a page-load pan writer. Unread.
- **An ink factor of about 0.5** (lattice = 0.5 white + 0.5 atlas) is what the capture needs; the
  shader read so far cannot make it and no mip filter does. A translucent white layer is the
  hypothesis (40): find the page background item's alpha, or the canvas draw's blend state.
- With both fed in, a numpy render of the lattice beats white in all five empty boxes (0.5 against
  1.7 to 2.4 mean grey difference); without the ink factor it loses to white. Merge only when both
  are read from code.
- The home, team, extras, options and profile pages show a blurred backdrop instead of the sharp
  lattice; what it is is unread.
- Who sets bit `0x80` at `canvas+0x30`; painted cells (the campaign map, `canvasTex` alpha and
  `canvasTex2` shifts).
- Omega ships `canvasTexture.gnf` and the same reset angles (`0x018086ec`); its GCN programs are unread.

## Next Steps

1. Find the offset: read `FUN_8107369a` (the window placement) and `Canvas3D_BuildLatticeMesh`'s cell
   origin against the executed matrix, 30 to 60 minutes.
2. Find the 0.5: the blend state in `Canvas3D_Draw` (`0x81074280`) and the front-end background item
   drawn over it, 30 minutes.
3. Then build the draw from the matrix (sheared bands or a baked inverse projection) and re-measure in
   the five boxes.
