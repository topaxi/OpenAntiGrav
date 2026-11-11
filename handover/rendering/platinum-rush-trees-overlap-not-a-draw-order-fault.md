---
status: measure
titles: [pulse, pure]
updated: 2026-11-11
summary: "Platinum Rush's tree flip is the original's own order (maintainer-confirmed in PPSSPP); SCENERY SORT depth fixes it as an opt-in; decide the default"
categories: [rendering]
---

# Platinum Rush trees flip over each other while the camera pans: the original does the same

Measured and filed in `docs/rendering/draw-order.md`, "Trees overlapping trees on Platinum Rush White". Our order matches the
original's across meshes and inside a strip, and the maintainer saw the same flip in PPSSPP. Field of view, reconstruction and
motion blur are ruled out. `--scenery-sort depth` (`ScenerySort`, a row beside TRAIL ORDER, `original` the default) draws the
list's foliage with its own depth and leaves nothing for the order to decide at the fly-by (0 pixels change when the list
is reversed, against 42,723 at `original`).

## Open

- **Should `depth` be the default on Pulse?** The original has no depth here; `depth` keeps soft edges blending, so a nearer
  tree's dark rim lies over a farther solid one (visible inside a big tree at 3360x1440). The maintainer decides.
- **Pure** takes the same path and was not photographed over a tree (Vineta K has no list-B foliage in its fly-by); a Pure
  circuit with trees would confirm. **Pulse PS2** draws its trees as depth-writing cutouts already: the row changes little there.
- **Glow stamps over foliage.** `draw_stamps` tests `Less` against depth the foliage now writes, so a stamped glow byte on a
  foliage draw would fail its own depth test under `depth`. No Pulse foliage draw carries glow bits; unchecked on Pure.

## Next Steps

1. Ask the maintainer to play with `--scenery-sort depth` on Platinum Rush and say whether the dark rim bothers them.
2. If the rim does: a soft-edge pass that keeps the colour of texels below half alpha out of nearer solid texels
   (draw the foliage's colour pass in two alpha bands), or raise the depth cutoff.
