# The drawn hull's shake (Wipeout HD / Fury)

Read from the disassembly 2026-10-22 (`hd-hull-shake`). After `Ship_UpdateEntity` (`0x000eadb8`) builds
the hull's display matrix from the body and the drawn lean ([`ship-lean.md`](ship-lean.md), "The display
matrix"), two calls on an object at **`ship+0x6b00`** turn and move it a little every frame: a cubic
B-spline through random points, one new point a second, faded out by engine thrust. The live reads that
found the offset (1.1 to 1.8 degrees, up to 0.09 units, a different axis per read) are in
[`hd-ship-materials.md`](../../../rendering/hd-ship-materials.md), "The drawn hull is not the body".

The decompiler gives up on both functions (VMX), so everything here comes from the instructions. All
constants are TOC-relative off `0x008ad4d8`, the TOC both OPD entries carry (`0x00874768`, `0x00874908`).

| Address | Name | Confidence | Evidence |
| --- | --- | --- | --- |
| `0x000d0c70` | `HullShake_Update` | 75 | `(obj = ship+0x6b00, f1 = dt, f2 = (*(ship+0x6944))->f[0x4c4])`, called at `0x000ebc74`: a uniform cubic B-spline over four points at `obj+0x40..+0x70`, parameter at `obj+0x80` advanced by `dt`, refilled from three `rand()` calls when it passes 1; writes the angles to `obj+0x10` and the offset to `obj+0x00` ("The update") |
| `0x000d39d0` | `HullShake_Apply` | 72 | `(obj, node, 0)`, called at `0x000ebc84`: `node = Ry(obj.y) * Rz(obj.z) * Rx(obj.x) * node`, then the node's translation moves by `obj+0x00` along its own rows ("The apply"). The `r5 != 0` path (`0x000d3dd0`) is not read |
| `0x006ca948` | `Matrix_RotationX` | 85 | rows `(1,0,0)`, `(0,c,s)`, `(0,-s,c)`; reached through the stub `0x00677708` |
| `0x006ca868` | `Matrix_RotationY` | 85 | rows `(c,0,-s)`, `(0,1,0)`, `(s,0,c)`; stub `0x006776f8` |
| `0x006ca788` | `Matrix_RotationZ` | 85 | rows `(c,s,0)`, `(-s,c,0)`, `(0,0,1)`; stub `0x006776e8`, the same builder the drawn lean uses and whose sign `ship-lean.md` measured |
| `0x00390c68` | `Math_CosPoly` | 80 | an even polynomial in `x^2` up to `x^8`; its result lands on the rotation builders' diagonal |
| `0x00390ca0` | `Math_SinPoly` | 80 | an odd polynomial `x * p(x^2)`; its result lands off the diagonal, negated once |

All three builders wrap the angle the same way first: `trunc(a * 5340353.5)` (`0x4aa2f983`, `2^24 / pi`)
sign-extended from 25 bits, then times `1.8725e-7` (`0x34490fdb`, `pi / 2^24`). The angle is radians,
wrapped to `[-pi, pi)`.

## The update, `0x000d0c70`

```text
t      = obj[0x80]
b0..b3 = (1-t)^3/6, (3t^3 - 6t^2 + 4)/6, (-3t^3 + 3t^2 + 3t + 1)/6, t^3/6      ; TOC 1, 4, -6, 3, -3, 1/6
p      = b0*P0 + b1*P1 + b2*P2 + b3*P3                     ; P0..P3 at obj+0x40, +0x50, +0x60, +0x70
obj[0x80] = t + dt
while obj[0x80] >= 1:                                       ; 0x000d0de0 loop
    obj[0x80] -= 1
    P0, P1, P2 = P1, P2, P3
    P3 = (4.0 * (rand()*2^-30 - 0.5),                       ; TOC -0x4884 = 4, -0x4874 = 2^-30, -0x49d4 = 0.5
          1.8 * (rand()*2^-30 - 0.5),                       ; TOC -0x4a08 = 1.8
          1.8 * (rand()*2^-30 - 0.5), 0)

idx = (obj[0x450] + 1) % 60;  obj[0x450] = idx              ; magic 0x88888889, a 60-slot ring
H[idx] = p                                                  ; H at obj+0x90, 16 bytes a slot, ends at +0x450
q      = H[(idx + 59) % 60]                                 ; the previous frame's p

obj[0x10].xyz = (0.025*q.y - 0.025*q.z, -0.01*q.x, 0.025*q.x)   ; TOC -0x489c, -0x4898, -0x4894; .w kept
obj[0x00]     = 5.0 * (p - q)                               ; TOC -0x4890

thrust_s  = obj[0x20] = 0.99*obj[0x20] + 0.01*f2            ; TOC -0x488c, -0x49dc
s         = clamp((80 - thrust_s) * 0.02, 0, 1)             ; TOC -0x4870, -0x4888; two fsel
obj[0x10] *= s
obj[0x00] *= 0.8 * s                                        ; TOC -0x4a14
```

The spline is evaluated at the parameter *before* the step. `rand` is `Libc_Rand` (`0x006762f8`,
[`loading-screen.md`](loading-screen.md)), a stub into a PRX, so its range is not in this binary.
The three lane writes into `obj+0x10` go through `vsel` with masks loaded from `0x00c47730` (`+0x00`,
`+0x10`, `+0x20`). That address is in BSS and filled at run time, so the x, y, z order of the three
masks is assumed, not read. The constant `1.0` is TOC `-0x49c4`, `0.0` is `-0x49d8`.

`f2` is `(*(ship+0x6944))->f[0x4c4]`, the player's physics body's `+0x4c4`.
[`physics.md`](physics.md) measured that field following the throttle (r = 0.94; about 77 held, about 1
released). So the shake is at full size below a smoothed 30 and gone at 80: full while coasting or
at rest, nearly gone under full throttle. The `0.99` filter and the one-frame difference `p - q` are
per call, not scaled by `dt`, so their size depends on the frame rate. HD's 60 Hz is the rate they
were tuned at.

## The apply, `0x000d39d0`

With `r5 = 0` (the hull's call) it loads the node's four rows (`node+0x00..+0x30`, the translation last)
and builds three rotations on the stack:

```text
R1 = Matrix_RotationX(obj[0x10].x)     ; bl 0x00677708, f1 = obj+0x10
R2 = Matrix_RotationZ(obj[0x10].z)     ; bl 0x006776e8, f1 = obj+0x18
R3 = Matrix_RotationY(obj[0x10].y)     ; bl 0x006776f8, f1 = obj+0x14
node.rows = R3 * (R2 * (R1 * node.rows))
node.row3 += obj.x * row0' + obj.y * row1' + obj.z * row2'     ; 0x000d3d2c-0x000d3d38
```

Each product is `new_row_i = sum_j R[i][j] * row_j`. The rotations therefore act in the hull's own
frame, the same composition as the drawn lean. The node's rows are the body's basis scaled by 0.75
([`ship-lean.md`](ship-lean.md), "The sign"), so the offset is scaled by 0.75 in world units. That basis
is row 0 the hull's left, row 1 up, row 2 the nose. In it, `x` is pitch, `y` yaw and `z` roll:

| Term | Law | Bound, if `rand()*2^-30` is in `[0, 1)` |
| --- | --- | --- |
| roll | `0.025 * q.x` | `0.05` rad, 2.9 degrees |
| yaw | `-0.01 * q.x` | `0.02` rad, 1.1 degrees |
| pitch | `0.025 * (q.y - q.z)` | `0.045` rad, 2.6 degrees |
| offset | `4.0 * s * (p - q)` along left, up, nose, times 0.75 | follows the spline's slope, about `0.13` at 60 Hz |

Those are the bounds of the control points. The spline's weighted sum stays well inside them most of the
time, which is how the live reads come out at 1.1 to 1.8 degrees. Roll and yaw share `q.x`, so roll is
always 2.5 times yaw with the opposite sign.

**What is not read (confidence stays at 72-75):**
- **`rand`'s range.** If `RAND_MAX` is `2^31 - 1`, `rand()*2^-30 - 0.5` lies in `[-0.5, 1.5)`. The
  points would then average `(2, 0.9, 0.9)`, a standing roll of about 2.9 degrees and yaw of -1.1.
  The live reads (1.1 to 1.8 degrees, about an axis that changes) fit a centred range better.
  Reading `P0..P3` live settles it.
- The mask order at `0x00c47730`.
- How the constructor seeds `P0..P3` and `obj+0x80`.
- Whether any other code reads the 60-slot history.
- Whether opponents run the same block. It is inside the per-ship update, but no branch above
  `0x000ebb28` was read for a player test.

## Cross-title

- **Omega:** not checked. A search for `2^-30` (`0x30800000`) in Omega's `eboot.bin` returns 30-odd
  hits with no constant pool beside them. Finding it means locating Omega's ship update call site.
- **2048:** not checked. Two hits in v1.04, neither beside the shake's other constants.
- **Pulse, Pure:** not read.
