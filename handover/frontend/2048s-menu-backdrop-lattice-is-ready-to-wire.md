---
status: ready-to-wire
titles: [2048]
updated: 2026-10-27
summary: "The 2048 pale triangle lattice is a mesh textured from canvasTexture.gxt; law and uvs are read, camera default is the open input"
---

# 2048's menu backdrop lattice: the law is read, the draw is not wired

[`canvas3d-lattice.md`](../../docs/ghidra/functions/vita-2048-eu-v104/canvas3d-lattice.md) has the
whole reading (2026-10-27, lane `gxp-usse-decoder`): the backdrop behind every 2048 page is a 90 x 32
mesh of equilateral triangle cells, each textured with one of two outlined-triangle regions in the top
left of `data/FE/NewImages/canvasTexture.gxt`, vertex colour white, alpha 1. The line width and grey
are the atlas's own pixels (core grey 222, about 2 px, falling off over 4 to 5 px), and at the 960 px
scale they land on the captures' 233 to 245 and 47.8 / 55 px without any fitting. Nothing is computed
by the shaders at default state; they only add an occlusion shade and a uv shift for painted cells.

## Open

- Measured 2026-10-27 on `2048/menu-title` after the title settled: the frame diff is 6.2 grey, of which the lattice is most (the logo band is 1.4, the prompt band's rest is pulse phase and lattice). The legal footer is a separate 22 grey in its band: 906 px wide in ours, 833 in the original at the same glyph height (tracking about 8 percent tighter there); not chased.

- The camera's runtime writers: defaults are read (pan 27.1976 / 0.39549, half-width 8.0, offsets 0)
  and give 60 px per unit at 960 px against the captures' 55.2. Find what writes the half-width
  (`+0xc0`, limits probably 5.0 and 17.0 at `+0x5ed18/1c`) and the pan before drawing; do not fit.
- Who sets bit `0x80` at `canvas+0x30` (which pages show the lattice).
- Omega's `Canvas3D` programs are GCN and unread; its mesh and atlas are the same item's.

## Next Steps

1. Read the writers of the camera fields (xrefs on the item's `+0xb8..+0xc0` in the `TouchScroll` and
   page-enter code): 30 to 60 minutes.
2. Draw the 2D equivalent of the mesh in `crates/ui` behind the pages: per cell, the triangle's
   corners at `(col * 0.5, row * 0.866) + corner offsets`, textured from the two atlas triangles
   (`canvasTexture.gxt`; `canvasTextureHD.gxt` carries the same pair). About 15 minutes of geometry
   on the existing atlas decode.
3. Compare a page against `2048/menu-*` captures (same page, 960 x 544) for line grey, scale and phase.
