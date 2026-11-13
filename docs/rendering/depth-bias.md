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

**Cost.** The lane is four bytes on every vertex of every title (`GpuVertex` 100 to 104 bytes, 4 %); Omega's largest
circuit grows from 295.9 to 307.7 MiB and `park` from 239.1 to 248.7 MiB, under the 256 MiB buffer ceiling
([`vertex-buffers.md`](vertex-buffers.md)). Packing it into a field another title reads (`texcoord2`, `slots`, `xform`)
would save that and was not done.

**Which titles.** Pulse PSP is measured. **Pulse PS2 inherits PSP** (279 of its 5,753 non-VIF batches carry a byte, 1 to 9;
a VIF batch reads `0`; the PS2 executable is unread) and **Pure inherits Pulse by lineage**: 4,134 of its 24,298 batches carry a
byte of the same range and the same authoring (glows, cones), and its `Mesh_InitBatch` is unread, so nothing here shows
the original honours it there. Not gated off, unlike the back-face cull (`batch-cull.md` keeps Pure off because it
measurably worsened one circuit): six poses on Vineta's biased batches moved at most 81 pixels of 522,240 and five PS2
`16_Track` race frames moved none, and nothing looked worse. A title that turns out to ignore the byte is one rule away:
zero the lane in `build_class` for it. Wipeout HD, 2048 and Omega draw `.rcsmodel` chunks and have no such byte (**checked,
differs**).

**Not covered.** The shadow maps and the craft's own shadow passes use `oag_render`'s own vertex shaders and are not
biased.

## Under a portal's oblique near plane

A portal view's near plane is swung onto the exit (`oag_render::camera::portal::oblique_near`), which rewrites the
projection's depth row alone: the `x`, `y` and `w` rows are untouched. Depth is then no function of `w`, but along one
pixel's ray it is still `c + beta / w` (`c` varies per pixel), and `beta` is the same on every ray: the depth row
evaluated at the eye, where `w` is zero. The pull the original applies is a shift of `6 * bias / 76,779` in `1 / w`, so
the clip-space shift is `beta * shift * w` with that `beta`. Reading `beta` from the depth and `w` rows as the standard
projection allows (`depth.w - alpha * w.w`) gave the right sign and the wrong size, by `dot(depth.xyz - alpha * w.xyz,
eye)`.

`biased_clip` adds exactly that term, with the eye solved as the meeting point of the three untouched rows (a cross
product, so it holds under TAA jitter, which only adds multiples of the `w` row to `x` and `y`). It does so only where
the depth row's share off the `w` row is above `1e-8` of its squared length: a plain projection measures 0 to 2e-14
(near 1.0 and 0.01, eyes at hundreds to thousands of units, three headings), the test's oblique plane 0.41. A plain
projection therefore takes today's arithmetic unchanged and its clip z is bit-identical. Whether a fragment is clipped by
the oblique plane is not touched: only the added bias term changes. The eye is read from the matrix because the uniform
block has no free slot and the portal pass already binds the oblique `view_projection` there.

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
  speckle; whether each changed edge matches the original was not compared against a capture. Those race frames mostly
  show the craft parked against a wall near the start (`12_Track` at tick 1,200 is at `(-470, 14, -280)`, the tunnel is
  at `(-653, -154, 214)`), so the biased surfaces were also posed: nine `13_Track` poses on one batch of each bias 1 to 9
  (`--pose=X,Y,Z`, 960x544): seven moved more than 30 pixels (up to 1,305), one none; the changed ones are lamp halos, signs and
  bunting, which now sit over what they lie on, with no new hole, dash or speckle.
- **Tests.** `depth_bias_coplanar` (oag-mesh, any GPU): two coplanar quads, the biased one wins the tie drawn first or
  second, an unbiased tie goes to the first drawn; a bias of 9 reaches 0.7 of the predicted pull behind a plane and not
  1.3 of it (red with `LEVELS_PER_BIAS` 3 or 12, and with the `biased_clip` call dropped).
  `depth_bias_ground_truth` (disc): every vertex of `13_Track`'s model carries its batch's authored byte (155 of 1,823
  batches, 10,712 vertices).
  `depth_bias_portal` (oag-render, any GPU): the same pair under the plain projection and under `oblique_near` with a
  plane tilted 40 degrees off the view axis and the eye 50 units off the origin, bracketed at 0.7 and 1.3 of the pull;
  the oblique case is red with the pre-change shader (the eye-offset tilt is what the old coefficient misses) and
  `the_oblique_fixture_rewrites_the_depth_row` guards the fixture against becoming a plain projection.

## Setting

None. The behaviour is the original's, the circuits it moves are moved by at most 0.2 % of a frame, and the one it was
written for was visibly broken without it, so it is not behind a render-profile switch.
