---
status: re-first
titles: [2048]
updated: 2026-10-27
summary: "2048 lattice draw built but not merged: scale 10 percent off the captures (projection unread), lines too dark; blurred pages and painted cells open"
---

# 2048's menu backdrop lattice: draw built, held back on a scale gap

Built 2026-10-27 (lane `2048-lattice`) and **not merged**: a tile baked off `canvasTexture.gxt` and
tiled on the title screen and game mode page with the item's reset camera. The lead held the draw back
because it measured further from the original than plain white: in the title pair's empty areas, mean
grey 2.4 -> 6.3 (above the logo), 1.7 -> 7.1 (below the copyright), 2.2 -> 6.2 (left of the logo). At the
wrong scale the lines miss the original's, and they are darker. The code is on branch `2048-lattice`
until the scale and the grey are read; then re-measure and merge.
Law, camera writers (pinch and drag only) and measurements:
[`canvas3d-lattice.md`](../../docs/ghidra/functions/vita-2048-eu-v104/canvas3d-lattice.md).

## Open

- **The period is 53.0 px against the captures' 47.75** (a 10 % scale gap at the reset camera).
  Read `FUN_812e4a04` and the matrix at `+0x5ebd0..+0x5ec10` that `Canvas3D_BuildWorldViewProj`
  multiplies in, or check whether the capture's Vita3K state pinched the canvas. Do not fit.
- Our lines are darker and wider than the capture's (ink about 2.7x): the original's sampling filter.
- The home, team, extras, options and profile pages show a blurred backdrop instead of the sharp
  lattice; what it is (a blur of this canvas, or a separate 3D scene) is unread.
- Who sets bit `0x80` at `canvas+0x30`; painted cells (the campaign map, `canvasTex` alpha and
  `canvasTex2` shifts); the apex-up/down parity convention (`col + row` even, read) was not checked
  against a capture because of the scale gap.
- Omega ships `canvasTexture.gnf`; its triangle region and GCN programs are unread.

## Next Steps

1. Read the projection matrix source (`FUN_812e4a04`) to find the 10 %: 30 to 60 minutes.
2. With the scale right, re-measure period, phase and parity on `2048/menu-title` and `menu-game-mode`.
