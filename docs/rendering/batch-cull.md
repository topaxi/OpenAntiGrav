# A Pulse circuit's batches cull their back faces, and a trackside board needs it

**Question and falsifier (written before any capture).** Does a Moa Therma board near `[335.3, 3.7, 309.3]`
(a player's report, "flickering ad board", `tick 1800 speed 148.43 spline 5.03 height 4.17`) flicker in ours
from one cause, and does the original draw it steady? Falsified, for ours, if over 40 or more consecutive
frames every draw of the board is present in each frame and the board's pixels stay within a steady control
box's frame-to-frame step; falsified, for the claim, if the original's frames swing the same way.

**Result: true.** Ours drew every batch two-sided. The board is a front copy and a back copy in one plane,
and the original culls each batch by its own bit, so it draws one of them per side. Two-sided, the copies
z-fight and the board stipples differently on every frame. Confidence 85 for the cause (4 original captures
over 2 emulator loads, one frame sequence of ours that matches it exactly, an instruction-level read of the
cull state); 40 for the PS2 carry.

## The draw

Pulse PSP `Data\Environments\03_Track\track.vex` (the layout the report's pose reads in: spline `5.03`
forward, `5.04` reversed), the four arch boards at `(250,62,366)`, `(120,52,405)`, `(-29,49,444)` and
`(-201,49,486)`. The player's flicker is the second arch's ornament pill (`concentration shuuchuu2.tga`,
128x64, 54 triangles, centre `(119.9, 52.2, 405.5)`).

| Scene node | Section group | Triangles | Winding / picture | Draws in ours |
| --- | --- | --- | --- | --- |
| 868 | 57 | 54 | front, picture upright | yes |
| 929 | 63 | 54 | front, positions within 6.1e-5 (one or two ULP) of 868, same UV and colour | yes |
| 928 | 63 | 55 | **back copy**: winding reversed, `u` mirrored, a different texture on the arch's other face | yes |

All three batches carry `pass_mask & 0x20 == 0` (`oag_vex::vex::Batch::is_culled`, `GU_CULL_FACE` on:
[mesh-draw.md](../ghidra/functions/psp-pulse-usa/mesh-draw.md), "The plume is two-sided", confidence 90) and
`depth_bias 0`. The craft's own PVS mask in sections 47 and 48 names both 57 and 63, so **both copies are in
the visible set in the original too**: this is not a visibility toggle, and `oag_render::pvs::SwapConflicts`
does not (and should not) treat 57 and 63 as alternatives, because it needs bit-identical vertices.

## Why it flickered

`DrawCall::culled` carried the bit all along, but the circuit's `Drawable` was built with `cull_back = false`
and the per-draw culling twin existed for PS3 models only (`!model.material_state.is_empty()`). Every Pulse
`.vex` batch was therefore drawn two-sided: from the approach side the front copy (868, 929) lies *behind*
the back copy (928) by zero, and from there the two ornaments (upright and mirrored) alternate pixel by pixel
as the vertex transform rounds. The arch-top board shows the other symptom: ours drew the cyan copy mirrored
(its back, seen through), where the original draws the yellow and pink board readable.

## Measurement

Reproduction (ours): `oag-game pulse-psp --race --mode single_race --track 03_Track\track.vex --autopilot
--ticks N --screenshot` for N = 1880 to 1960, one run per tick. The autopilot is only a way to be there:
it passes within 3 units of the reported pose at tick 1917, speed 131. Metric: the share of pixels in the
pill's box (960x544 frame, x 400-560, y 100-190) that differ by more than 24 luma from the reference frame of
the same tick, the reference being the board drawn from node 928 alone.

| Ticks 1908 to 1923 | Share of pixels off the reference, per tick |
| --- | --- |
| Before (two-sided) | 1.9 2.4 9.3 11.3 9.0 8.9 5.2 8.6 13.0 9.3 11.7 12.4 13.4 13.7 12.5 12.9, mean 9.7 |
| After (`per_batch_cull`) | 0.0 on all 16 |

**Discrimination** (the same box, hide by scene node, scratch hook not kept): hide 929: no change; hide 928:
none of the stipple (868 and 929 never visibly fight, the same picture); hide 868: steady 12 % (928
alone, the mirrored ornament); a blanket back-face cull: identical to hiding 868 and 929. **What it is not:** a
PVS toggle (all three nodes are drawn every tick of 1880 to 1953; the set changes once, at tick 1954), a UV
animation (a fixed pose with the tick running does not stipple), or a sort order (opaque list, one pipeline).

**The original** (PPSSPP v1.20.4, software renderer, Moa Therma White time trial state, `psp-drive.py place`
then `psp-trace.py --camera --shot-every 1`): a craft placed at `(333.8, 3.5, 306.7)` facing the track
(trace pose `333.76 3.44 306.9`, camera `344.3 6.91 303.4`) draws one clean ornament where ours drew two,
and the arch-top board reads yellow and pink with its text the right way round. Moving at 104 to 121 u/s
across the reported pose, 12 consecutive frames in each of two placements (a reload between) show the pill
steady, with no speckle. A third run after a reload came up with the camera tumbling and was discarded.

## The fix

`oag_title::Looks::batch_back_face_cull` (Pulse PSP, measured) and `batch_back_face_cull_carried` (Pulse
PS2, inherited from the PSP): where either applies, the loader sets `Model::per_batch_cull` on the circuit's
model, and `mesh_render::build_with` builds the culling twin of the opaque, depth-prepass and cutout
pipelines for it, which the draws select by their own `culled` (the way HD's per-material cull has worked
since `hd-backface-cull`). Only the circuit: the hulls, pads, plume, gantry and weapon models are untouched,
and blended draws already selected a culled or two-sided pipeline by `culled`.

**Before and after, 29 race pairs** (26 Pulse PSP time-trial and Zone, 2 PSP single race, the PS2 pair;
`compare-rerender-ours` on copies, same binary with the rule off and on, rmse against `theirs.png`): a pixel
moved on 26 of 29; rmse better on 13, unchanged on 15, worse on 1 (metropia-white, 0.0355 to 0.0356). Largest: de-konstruct-white 0.0869 to 0.0575 (a distant chevron board
the original draws dark red, ours drew pink and white), moa-therma-black 0.0417 to 0.0372, vertica-black
0.0361 to 0.0343, de-konstruct-black 0.0458 to 0.0451, fort-gale-black 0.1024 to 0.1013. The PS2 pair
moved 2 pixels (`0.1777` both ways): the PS2 circuit's batches author the same bit but the pair's frame
holds no board. No geometry vanished in any pair read (outpost-7-black and talons-black have more "worse"
than "better" pixels in a 100 by 70 region, and the frames show a stray speckle gone and nothing missing).

## Lineage

- **Pulse PS2**: carried from the PSP (the same `.vex` batch word), unmeasured there; labelled inherited.
- **Pure**: off (`Rule::UNREAD`). The same batch word is there but nothing of Pure's draw state was read
  (`Mesh_InitBatch` is unread on Pure); the rule is chosen to stay off, not measured. Open: a Pure circuit
  with a back-to-back board, and `Mesh_InitBatch` on Pure.
- **HD, 2048, Omega**: not applicable. They cull per material (`STATE_CULL`, `Model::honour_cull`), which is
  already wired; the circuit's boards there are not batch copies.

## Open

- The two coplanar front copies (868, 929) are drawn twice in the original too (the mask names both
  sections); harmless, the same picture to one or two ULP. The original's 16-bit depth would tie them exactly.
- `Batch::depth_bias` is still read and not applied (an open item of the Pulse flyby thread).
