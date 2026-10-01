# The view test the original makes before it draws anything: a section's own box

**Binary:** `pulse-psp` `BOOT.BIN`, image base `0x08804000`. Recovered 2026-10-01
(`pulse-cull`), headless Ghidra plus PPSSPP v1.20.4 on `pulse-psp-usa.chd`.

The question was whether the original frustum-culls moving meshes by a bound this
engine lacks, because 4 to 12 moving draws per pose passed our section mask and
never reached its GE list. **Answer: it culls by a bound, and the bound is the
section's, not the mesh's.** One shared node predicate tests the governing
section's mask bit and then the section's own authored world-space box against
the view; a moving mesh has no bound the original tests it by on this path, and
needs none, because its section does not move.

| Address | Name | Confidence |
| --- | --- | --- |
| `0x0892b638` | `Node_TestSectionVisible` | 80 |
| `0x08902a98` | `ViewCull_TestBox` | 88 |
| `0x08902594` | `ViewCull_TestBoxCorners` | 90 |
| `0x08902894` | `ViewCull_ComputeOutcode` | 90 |
| `0x08902a58` | `ViewCull_LoadViewProjection` | 85 |
| `0x08902a2c` | `ViewCull_LoadWorldMatrix` | 85 |
| `0x0890296c` | `ViewCull_BuildViewProjection` | 85 |
| `0x0890cc80` | `Mesh_SubmitNode` | 85 |

## The test

```c
// ViewCull_BuildViewProjection (0x0890296c): called by the camera submits
//   (Camera_SubmitScene, VexCamera_Submit, Mode3D_*).  VP = view * proj, from the
//   display's matrix stacks, into DAT_08af2500.
// ViewCull_LoadViewProjection (0x08902a58): DAT_08af24c0 = DAT_08af2500.
// ViewCull_LoadWorldMatrix (0x08902a2c): DAT_08af24c0 = world * DAT_08af2500.

// ViewCull_ComputeOutcode (0x08902894), one clip-space point, VFPU vcmp:
code = (x < -w) | (y < -w) << 1 | (z < 0) << 2 | (x > w) << 3 | (y > w) << 4;
// the z > 0 bit of the second compare is masked off (and -0x21)

// ViewCull_TestBoxCorners (0x08902594): the box is { min.xyz, flag@+0xc, max.xyz@+0x10 }
and_all = 0x1f;
for corner in 8 corners of the box:  and_all &= outcode(corner * DAT_08af24c0);
box->flag (+0xc) = (and_all == 0);        // visible iff no plane has every corner outside

// Node_TestSectionVisible (0x0892b638), a vtable method of twelve node classes:
if (target = node->+0x54) {
    if ((mask_bit(target->section) & display_visible_mask) == expected) return node->+0x5c = 0;
    if (box = target->+0x5c) {
        ViewCull_LoadViewProjection();                 // the box is in WORLD space: no world matrix
        if (!target->+0x68)  ViewCull_TestBox(box);    // else a shadow-volume variant (FUN_08902af4, 800.0)
        if (!(box->flag & 1)) return node->+0x5c = 0;
    }
}
return node->+0x5c = 1;
```

- **The box is the `.vex` `section` node's own**, payload `+0x10` (minimum) and
  `+0x20` (maximum), world space - `oag_vex::pvs::Aabb`. Read live: the box the
  predicate tests for section `1` of Talon's Junction was
  `(-485.99, -117.56, -380.73)..(-141.98, -26.37, -157.88)`, the file's own to the
  digit.
- **There is no far plane**, and **`z < 0` is not the near plane.** In the GE's
  clip space `z_c = m22 * z_v + m32` crosses zero at `d = -m32 / m22`, half way
  between the planes' harmonic positions: `2.42 ..= 2.52` units over 25 dumps
  (`2.482` at the rest fov) against a near plane of about `1.24`. A box wholly
  within about two and a half units of the camera plane, or behind it, is
  rejected; nothing else is, on depth.
- **The mask test is read as a section-bit test** (`FUN_0897bc08` against
  `FUN_0891e908(g_display)`, a 64-bit AND); that half of the score is lower
  (70) than the box half (88), because only the box half was exercised.

## Evidence

**1. Against the original's own frames (22 PPSSPP GE dumps).** Metropia
(`03_Track`, 8 poses) and Tech De Ra (`04_Track` reversed, 14). The view and
projection are the dump's own registers (`psp-ge-dump.py census` now carries
`view`, `proj` and `world` per PRIM), the sections are the file's own, a static
draw is matched on vertex count and world box to `0.5` units, a moving one on
vertex count and exact diameter, and **draws in a visible section claim their
twins first** so an instanced copy cannot hand its twin to a culled one.
`scripts/pvs-cull-check.py`.

| | section culled, no twin | section culled, **twin** | section visible, no twin | section visible, twin |
| --- | --- | --- | --- | --- |
| Metropia static | 1,107 | **0** | 465 | 2,856 |
| Metropia moving | 23 | **0** | 25 | 163 |
| Tech De Ra static | 829 | **0** | 606 | 4,590 |
| Tech De Ra moving | 45 | **0** | 52 | 141 |

The falsifier - a draw the original submits although its section's box is
rejected - is `0` of 2,004 culled draws. **It was not 0 on the first pass**: before
draws in a visible section claimed their twins first, 9 moving draws in 3 dumps
(Tech De Ra rows 1500, 2400, 2700) read as submitted-yet-culled, all of them the
batches of nodes `283` and `285` in section `1`. Checked directly rather than
assumed: each has a same-signature sibling (nodes `961` and `963`, same vertex
count and diameter) in a section the view reaches, and the dump's one object of
that signature (submitted twice, with two textures) sits at `(230, 16 .. 24, 45)`,
outside section `1`'s box (`x` from `-650` to `0`). It belongs to the sibling; the
object a rejected section holds is not in the list. That attribution rests on the
signature and the position, not on an exact match of the moving mesh. **Positive control**: the same test with
the view turned a quarter circle reports `73` and `135` such draws at two poses.
The test explains 64 % of the static draws and 47 % of the moving draws that pass
the section mask and have no twin; the rest are the small transparent quads whose
twin the signature cannot find and draws a batch split differently. Not a claim
that the original submits nothing else we cull.

**2. Live (PPSSPP, Talon's Junction).** A breakpoint on `Node_TestSectionVisible`
read, for 299 calls, the box it tests and the view-projection in force: the
result byte at `node+0x5c` was `1` only when the corner test, run on the box read
out of memory, passed (43 of 43), and `0` on every one of 116 where it failed;
the other `0`s are calls the mask test returned before the box. A second
breakpoint on `ViewCull_TestBox` for the `Mesh` nodes' own boxes (below)
read the game's own visible flag after each call: it agreed with the same corner
test on all 297 whose hold counter was zero (252 rejected, 45 kept).

**3. What it is not: the per-mesh box test.** `Mesh_SubmitNode` (`0x0890cc80`)
runs the same corner test on each `Mesh` node's own box (`node+0x80`/`+0x90`, the
payload header's `+0x10`/`+0x20` over the position scale) with the node's
current world matrix, and a hit sets a hold counter (`node+0x7b`, `8 + rand() % 7`
frames of "visible" without testing). **It does not predict what reaches the GE
list**: at the start grid, 19 of 46 tested nodes were rejected by it and still had
a PRIM in the dump inside their world box (node `54` exactly: its 97-vertex PRIM,
identical in vertex count and box, with the node rejected on every one of 30
frames). The reason is the second draw path
(`Mesh_BuildModelDrawData` / `Mesh_BuildLayerBatchSets`, whose submit
`FUN_0892ece0` enqueues with no box test: 581 of 2,000 `Gfx_Enqueue` calls against
432 from the mesh submit). So this crate does **not** port the per-mesh test;
applying it per draw would hide batches the original submits.

**4. The recurring unmatched moving nodes were an artifact.** The census's
double-sweep diameter read `8 %` short of the exact one the dump side computes
(`101.7` against `110.2`, `143.7` against `156.9`) and the join's `3 %` tolerance
turned every rotated moving mesh into a miss. With both sides exact, Metropia
node `225` and Tech De Ra `1076` and `1078` have twins at every pose and are not
culled by anything.

## Implemented

`oag_render::pvs::sections_in_view` and `VisibleSet::within_view`
(`crates/render/src/pvs/section_view.rs`), wired in
`oag_game::race::scene::frame` (the unjittered view-projection) through
`Visibility::set`. **Chosen, not measured**: the test is made against this
camera's view-projection, where the original hard-codes `480/272` into its own
planes (`Camera_SubmitScene`); on a wider picture the original's test would cut
the screen's edges and this one cuts exactly what the picture cannot show. At
`480x272` they are the same. **Left out on purpose**: the hold counter - it only
keeps drawing geometry found wholly off screen, from an unseeded `rand()`.

Seven native `480x272` poses (Metropia `03_Track` 400, 2300, 3200; Tech De Ra
reversed 600, 900, 1800, 2000, the Assegai) are **pixel-identical** before and
after (`0` differing pixels each), with `0 .. 10` fewer draws submitted
(`443 -> 441`, `740 -> 730`, `278 -> 276`): the cull is conservative, so a correct
port moves submissions and not pixels. **The narrowing is title-agnostic** (it runs wherever a track authors sections),
and only Pulse PSP's original was read, so it was checked where it was not: Wipeout
Pure PSP (USA and EU), Pulse PS2 EU and Pulse PSP EU, an autopilot race at ticks 1,
300, 700, 1100 and 1500 each, before and after - **0 differing pixels in all 20
frames**, and the check bites: draws submitted fell on Pure at four of five ticks
(`372 -> 361`, `367 -> 356`, `204 -> 199`, `247 -> 246`, identical on USA and EU), on
Pulse PS2 EU (`372 -> 363` at tick 700) and on Pulse PSP EU (`299 -> 286`). So on the
titles whose cull was not read the port removes draws and no pixel changes. That is
neutrality against our frames, not evidence the original culls Pure by section; Pure's
executable was not read. The data-backed test
`crates/render/tests/pvs_section_view_ground_truth.rs` runs it over 24 circuit
files: the craft's own section is never rejected from a chase camera, and `49 %`
of section tests reject.

## Open

- The mask half of `Node_TestSectionVisible` (the `FUN_0897bc08` AND) is read, not
  exercised; and why 173 spline control points lie outside their own section's
  box (the original has the same box and the same test).
- Whether the batch-set path carries a bound of its own, which would explain the
  `1,071` static draws that pass the section test and have no twin.
- No tunnel or loop circuit among the 22 dumps.
