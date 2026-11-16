# HD: the firing craft's own box in a projectile's probe

Question carried over from Pulse
([psp-pulse-usa/rocket-visuals.md](../psp-pulse-usa/rocket-visuals.md), "The own-craft box is a
rotated frame"): does the firing craft's own box hide the track from a projectile's first probes?

**Answer: probably yes, with Pulse's consequence (a box hit is `0x7f`, the fall), but two links are
open, so confidence 65 and the title stays `chosen`.** Static, read 2026-11-13 on
`/hdfury/EBOOT-ps3-hdfury-eu.elf`. The Ghidra GUI decompiles `Collision_MarchSegment` to
`halt_baddata` (the `lvlx` trap), so the collision functions were read with

```sh
llvm-objdump -d --triple=powerpc64 --mattr=+altivec \
  --start-address=0x364c0 --stop-address=0x36d98 \
  data/extracted/ps3/hdfury-eu/PS3_GAME/USRDIR/EBOOT.elf
```

and the same command over `0x336b0..0x33af0` (the box test), `0x3a320..0x3a698` (the box collider's
setters), `0x7be58..0x7bf40` (the weapon query) and `0xddd58..0xdf200` (`Ship_Construct`). TOC
values come from `scripts/ps3-toc.py` (all four functions use TOC `0x008ad4d8`).

## The weapon query includes boxes and skips nothing

The Rocket's and Plasma's probe query, `0x0007be58` (named on
[projectile-launch-normal.md](projectile-launch-normal.md) and [plasma.md](plasma.md) as "the track
query"), is Pulse's `Collision_SweepSegment` without the cage:

```text
0007be90  li r9, 0x0          ; flags: mesh and box tests both on
0007be98  li r7, 0x0          ; collider to skip: none
0007be88  lbz r8, 0x167(r1)   ; the caller's flag byte
0007bea4  bl Collision_MarchSegment
0007beb8  lwz r0, 0x98(r1)    ; hit.kind
0007bebc  cmpwi cr7, r0, 1    ; a mesh: return the collider's +0x74 surface
0007bec8  li r3, 0x7f         ; anything else, a box included: 0x7f
```

`Collision_MarchSegment` (`0x000364c0`) is Pulse's `Collision_RaycastWorld`: step count from
`length * 0.01` (TOC `0x8a5ef0`), the per-collider filter at `0x36768` (`collider != r7`, then
`+0x74 != 2 || flag byte`), the shape kind at `+0x0c`, `1` to the mesh test at `0x367e0` (gated on
`flags & 1`), `3` to the box test at `0x36c80`, gated on `flags & 2` (`0x36668`, stored at
`sp+0x5e8`) and calling `0x000336b0`.

## The box test is Pulse's shape

`0x000336b0` (named here `Collision_RaycastBox`): an AABB clip (`bl 0x7ac00`), then the segment's
two ends, minus the centre `+0x60`, transformed as `p.x * row0 + p.y * row1 + p.z * row2 + w * row3`
with rows `+0x30/+0x40/+0x50` (`0x338ac..0x338d8`, `vmaddfp` chains on `vspltw` splats of the
point), the half dimensions `+0x190/+0x194/+0x198` times `0.5` (TOC `0x8a5e8c`), three axis tests
and three cross-product tests (the six `fabs` compares `0x338f0..0x339ac`), the corners
(`0x3a3d0`), the twelve triangles (`0x3a1c0`), the triangle test (`0x7a900`), and kind `3` written
at `0x33ac8`. A sum of rows weighted by the point's components is the transposed product Pulse was
measured to use.

`0x0003a4c8` copies a 4x4 matrix's rows unchanged into `+0x30..+0x60` and builds the corner axes
`+0xb0/+0x90/+0xa0` as `row0/row1/row2 * dimension * 0.5` (TOC `0x8a5f40`), so the corners are
built from the same rows the gate reads: the rows are the box's axes, as on Pulse.

## The box is the craft's, Misc times 0.75

`Ship_Construct` (`0x000ddd58`) stores `0.75` to the scale global (`0x008c15e0 + 0x44`, `0xde3f4`;
[camera.md](camera.md)), then at `0xdec5c..0xdece4` builds a `0x1a0`-byte collider with
`0x0003a320(obj, 5)` (shape `3`), stores `(stats+0x84, stats+0x8c, stats+0x88) * 0.75` through
`0x0003a1a8` into `+0x190/+0x194/+0x198`, and registers it with `Collision_AddObject` (`0x000345e0`,
which writes it into the same `world + i*8 + 0xf764` table `Collision_MarchSegment` walks). The
values are full dimensions (the box test halves them), so `0x0003a1a8`'s name on
[mp-time-trial-collision.md](mp-time-trial-collision.md) (`Collision_SetObbHalfExtents`) describes
the field's use, not its contents. Which of `+0x84/+0x88/+0x8c` is width, length and height was not
read from HD's own parser. In mode `14` with a byte clear the scale is `0.7`, not `0.75`.

## What keeps this at 65

1. **The `+0x74 == 2` filter.** `Collision_MarchSegment` skips a collider whose `+0x74` is `2`
   unless the caller's flag byte is set. Nothing read writes `+0x74` on the ship's box: the base
   constructor (`0x3a698`) writes `+0x00/+0x04/+0x08/+0x70/+0x78/+0x7c/+0x80`, `0x3a320` writes
   `+0x0c/+0x78/+0x19c`, `Ship_Construct` writes `+0x84`. Whether the allocator (`0x676748` to
   `0x392790`) zeroes the block, and what the weapon caller passes as the flag byte, are unread.
2. **The sign of row 0.** The gate region depends on the body's row 0 sign; Pulse's `-right` was
   pinned on Pulse disc data. HD's body rows were not tied to the craft's right.

One RPCS3 capture settles both: a breakpoint on the box call at `0x00036cac` while a parked craft
fires, and the return of `0x0007be58` on the first updates.

## Names

| Address | Name | Confidence | Evidence |
| --- | --- | ---: | --- |
| `0x0007be58` | `Collision_SweepSegment` | 80 | the listing above; Pulse `0x0883198c` without the cage |
| `0x000336b0` | `Collision_RaycastBox` | 72 | the shape-`3` arm of `Collision_MarchSegment`, the steps above |
| `0x0003a4c8` | `Collision_SetObbTransform` | 72 | copies four matrix rows to `+0x30..+0x60`, rebuilds the scaled axes |

## Cross-title

Pulse: applies, measured. Pure: applies, differs in consequence
([psp-pure-usa/own-craft-box.md](../psp-pure-usa/own-craft-box.md)). 2048 and Omega: **differs**,
the box is absent from their query ([vita-2048-eu-v104/own-craft-box.md](../vita-2048-eu-v104/own-craft-box.md),
[ps4-omega-eu/own-craft-box.md](../ps4-omega-eu/own-craft-box.md)), although HD is 2048's engine
lineage. HD's Plasma probe is `6.0` long ([plasma.md](plasma.md)) where `oag_weapons` uses Pulse's
`12.0` for every title: a mismatch written down, not wired here.
