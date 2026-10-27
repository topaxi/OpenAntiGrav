---
status: re-first
titles: [2048]
updated: 2026-10-27
summary: "2048 lattice wired on the title and game mode pages from the tilted matrix at full ink (lane lattice-final); the capture shows about half ink, open"
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

2026-10-27, lane `lattice-final`, part 1: **wired** (not held back any more). A 960 x 544 picture baked from the tilted
matrix and the live translation, shift (0, 0), edge NCC 0.94, at full ink; see `canvas3d-lattice.md`, "Wired". The
bullets below that talk about holding the draw back or the branch `2048-lattice` are history.

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

2026-10-27, lane `lattice-wire`: every static source of the phase and the grey was read and ruled out
(mesh window, parity, reset, stick tilt, parent `FeShellTouchScroll` scroll with `strcasecmp`
attributes, taps, cell height, viewports, the draw's opaque fragment variant, page overlays): table in
`canvas3d-lattice.md`, "Phase and grey: every static path reads zero". A draw from the read state scores
6.1 against white's 1.7 to 2.4, so nothing was wired.

2026-10-27, lane `lattice-live-read` (live read at the title in Vita3K, `canvas3d-lattice.md`, "The live read at the
title"): **the phase is the game's state.** The parent `FeShellTouchScroll` sits at (-3000, -1500) (it is not
pinned at (0, 0): that row is struck), the canvas pan follows it to (78.76, -26.824), and the live matrix
reduces to a lattice offset of (0.4027, 0.4933), the fitted (0.40, 0.50). Two boots and an empty-save boot read
the same. **The half ink is not game state**: the atlas in guest memory is byte-identical to the disc file,
`canvasTex` and `canvasTex2` are default. The live matrix with authored ink still loses to white (2.4 to 2.6
against 1.7 to 2.4), so the draw stays held back: it needs the ink.

## Next Steps

1. **The ink (half).** Not in memory and not in the texture state: the descriptor decodes to the file's
   `PVRTII4BPP` (the `0x82721600` is control word 0, sampler state: trilinear, mip count 10, LOD bias -1.5, so mip 0), see
   `canvas3d-lattice.md` "The atlas descriptor decoded". Left: a layer drawn after the canvas, or the pass's output
   blend (neither read), or the capture itself. Ask the maintainer how to compare.
2. **Who writes (-3000, -1500).** Candidates `FUN_810efc4a` (`0x810effcc`) and `FUN_810efff6` (`0x810f032a`), the campaign
   map's centring of the first event. Pin the call (a breadcrumb patch in `0x811042ae`, storing LR where
   `TouchScroll_Update` does not decay it), then derive the phase from the page's own node positions, not a
   fitted offset.
3. Then build the draw from the matrix with the live pan (sheared bands or a baked inverse projection) and
   re-measure in the five boxes.
