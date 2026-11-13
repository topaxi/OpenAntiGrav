# A batch's authored depth bias

Pulse's `.vex` batches carry a **depth bias** byte, `Batch::depth_bias` (`0..=9`, 5,605 of 67,638 PSP batches: glows,
additive cones, `newtree2` cutouts, `greenfade`). The original draws a biased batch with
`sceGuDepthRange(1001 + 6 * bias, 64946 + 6 * bias)`, which moves the batch's whole depth by `6 * bias` levels of its
16-bit buffer toward the camera (evidence, census and the live GE read:
[`mesh-draw.md`](../ghidra/functions/psp-pulse-usa/mesh-draw.md), "The per-batch depth bias is file data"). It is how
the artists put a light strip on the plate it lies on: without it the two tie and one of them wins per pixel.

## What ours does

`GpuVertex::depth_bias` carries the byte on every vertex of the batch (`0.0` on every other vertex of every other
title), and `mesh.wesl`'s `vs_main` passes `clip` through `depth_bias.wesl`'s `biased_clip`. Every pass that draws
a mesh (opaque, cutout, transparent, prepass, foliage, stamp) shares that one `vs_main`, so a batch drawn twice (a list A
and a list B copy, or an opaque and a cutout pass) gets the same offset each time and the prepass agrees with the shading
pass to the bit.

**The shift is in the original's depth units, not in world units.** Its window depth is `C + 76,779 / w` (`w` the view
distance; 63,945 levels over its near 1.2 / far 2000 projection, `proj[14] = -2.4014`), so `6 * bias` levels is
`6 * bias / 76,779` added to `1 / w`. That is a different distance at each range, and a bias of 3 pulls a surface about
14 world units toward the eye at 250 and 0.1 at 20. Ours has its own hyperbola `d = alpha + beta / w` (the race's near is
1.0, the asset viewer's 0.01, and the far varies), so the same shift in `1 / w` is `beta * shift` in depth, and in clip
space `clip.z += beta * shift * clip.w`. `alpha` and `beta` come out of the bound `view_projection`'s depth and `w` rows,
so it follows whichever projection is bound, standard or reversed; an orthographic matrix has no `w` to shift and is left
alone. The shift is not clamped: a vertex within about 0.1 % of the near plane is clipped by it, which no authored batch
reaches.

**Why a vertex lane.** A per-draw bias would be smeared across the texture run a `DrawCall` is; a vertex carries
its batch's value through every merge and copy. One pipeline per bias value (`DepthBiasState`) would need nine variants
of every pipeline, and on a `Depth32Float` buffer its `constant` scales with each primitive's depth exponent, so it is
not a constant shift. `texcoord2.y` was free on a `.vex` vertex, but HD authors a real second coordinate set there, so the
lane would have misread it.

**Not covered.** The shadow maps and the craft's own shadow passes use `oag_render`'s own vertex shaders and are not
biased. A portal view's oblique near plane (`oag_render::camera::portal::oblique_near`) rewrites the projection's depth
row, so `alpha` and `beta` read from that matrix are not the hyperbola's; the bias there is of the right sign and
the wrong size. Wipeout HD, 2048 and Omega draw `.rcsmodel` chunks and have no such byte (**checked, differs**); Pure's
`Mesh_InitBatch` is unread (**not checked**), and a Pure `.vex` reaches the same code with the byte its header holds.

## Evidence

- **Gemini Dam, `12_Track` (DLC), the tunnel's ceiling light strip** (nodes 121, 122, 123 bias 3, node 133 bias 6),
  spline row 1205, `--pose=X,-153.7,214.494 --anim-seconds 0.5`, four camera offsets 0.05 units apart in `x`
  (`-653.037`, `-652.987`, `-652.937`, `-652.887`), 1280x720. Before: the strip is cut into dashes that change with every
  offset, one with speckle. After: one continuous strip at all four.
- **Other circuits.** `--race --hold cross` frames of `12_Track` and `13_Track` every 150 ticks from 600 to 4,200
  (50 pairs), and `01 02 04 05 06 09 10 11` at ticks 1,800 and 3,200 (16 pairs), 960x544 (522,240 pixels), before against
  after: 44 of 66 pairs differ in 30 pixels or fewer, and the rest in 31 to 1,166 (at most 0.22 % of a frame). The largest
  are `05` at tick 3,200 (1,166 pixels) and 1,800 (430), where a light shaft lies over a sign and its frame differently, and
  `12_Track` at ticks 4,200 and 1,200 (264 and 252, pillar and stand edges). Read at 2x, no pair shows a new hole, dash or
  speckle; whether each changed edge matches the original was not compared against a capture.
- **Tests.** `depth_bias_coplanar` (oag-mesh, any GPU): two coplanar quads, the biased one wins the tie drawn first or
  second, an unbiased tie goes to the first drawn; drop the `biased_clip` call and it fails.
  `depth_bias_ground_truth` (disc): every vertex of `13_Track`'s model carries its batch's authored byte (155 of 1,823
  batches, 10,712 vertices).

## Setting

None. The behaviour is the original's, the circuits it moves are moved by at most 0.2 % of a frame, and the one it was
written for was visibly broken without it, so it is not behind a render-profile switch.
