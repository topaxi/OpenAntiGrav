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
| `0x000d0c70` | `HullShake_Update` | 78 | `(obj = ship+0x6b00, f1 = dt, f2 = (*(ship+0x6944))->f[0x4c4])`, called at `0x000ebc74`: a uniform cubic B-spline over four points at `obj+0x40..+0x70`, parameter at `obj+0x80` advanced by `dt`, refilled from three `rand()` calls when it passes 1; writes the angles to `obj+0x10` and the offset to `obj+0x00` ("The update") |
| `0x000d39d0` | `HullShake_Apply` | 72 | `(obj, node, 0)`, called at `0x000ebc84`: `node = Ry(obj.y) * Rz(obj.z) * Rx(obj.x) * node`, then the node's translation moves by `obj+0x00` along its own rows ("The apply"). The `r5 != 0` path (`0x000d3dd0`) is not read |
| `0x006ca948` | `Matrix_RotationX` | 85 | rows `(1,0,0)`, `(0,c,s)`, `(0,-s,c)`; reached through the stub `0x00677708` |
| `0x006ca868` | `Matrix_RotationY` | 85 | rows `(c,0,-s)`, `(0,1,0)`, `(s,0,c)`; stub `0x006776f8` |
| `0x006ca788` | `Matrix_RotationZ` | 85 | rows `(c,s,0)`, `(-s,c,0)`, `(0,0,1)`; stub `0x006776e8`, the same builder the drawn lean uses and whose sign `ship-lean.md` measured |
| `0x00390c68` | `Math_CosPoly` | 80 | an even polynomial in `x^2` up to `x^8`; its result lands on the rotation builders' diagonal |
| `0x00390ca0` | `Math_SinPoly` | 80 | an odd polynomial `x * p(x^2)`; its result lands off the diagonal, negated once |
| `0x0043c280` | `Libc_RandImpl` | 85 | the body `Libc_Rand` (`0x006762f8`) branches to after its TOC switch: a Bays-Durham shuffle over the LCG `x = 1664525 * x + 1013904223`, returning the low 30 bits ("`rand` is centred", below) |

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

The spline is evaluated at the parameter *before* the step. `dt` is `Ship_UpdateEntity`'s own `f1`
(`fmr f26,f1` at `0x000eadd4`, added into `ship+0x6a30` at `0x000eaea4`), passed as `fmr f1,f26` at
`0x000ebb74` with no write to `f1` before the call. That makes "one point a second" hold if `dt` is
in seconds, which this code does not show.
The three lane writes into `obj+0x10` go through `vsel` with masks loaded from `0x00c47730` (`+0x00`,
`+0x10`, `+0x20`). That address is in BSS and filled at run time, so the x, y, z order of the three
masks is assumed, not read. The constant `1.0` is TOC `-0x49c4`, `0.0` is `-0x49d8`.

`f2` is `(*(ship+0x6944))->f[0x4c4]`, the player's physics body's `+0x4c4`.
[`physics.md`](physics.md) measured that field following the throttle (r = 0.94; about 77 held, about 1
released). So the shake is at full size below a smoothed 30 and gone at 80: full while coasting or
at rest, nearly gone under full throttle. The `0.99` filter runs once per call and ignores `dt`,
so its time constant depends on the frame rate. The offset `4s(p - q)` is the spline's change over one
frame, so it is proportional to `dt`. A port runs both on a fixed 60 Hz tick, HD's own rate, not per
rendered frame.

**The two live frames already fit this.** The offsets read on two consecutive frames,
`(-0.013, +0.087, -0.086)` then `(-0.008, +0.054, -0.052)`
([`hd-ship-materials.md`](../../../rendering/hd-ship-materials.md)), shrink by the same 0.61 in all three
components while the angle barely moves (1.12 to 1.13 degrees). That is the slow spline under a frame time
that dropped by about 0.6 between the frames. The interpreter's frame time was not steady, so those
offsets are not a 60 Hz size.

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

| Term | Law | Bound |
| --- | --- | --- |
| roll | `0.025 * q.x` | `0.05` rad, 2.9 degrees |
| yaw | `-0.01 * q.x` | `0.02` rad, 1.1 degrees |
| pitch | `0.025 * (q.y - q.z)` | `0.045` rad, 2.6 degrees |
| offset | `4.0 * s * (p - q)` along left, up, nose, times 0.75 | follows the spline's slope, about `0.13` at 60 Hz |

Those are the bounds of the control points. The spline's weighted sum stays well inside them most of the
time, which is how the live reads come out at 1.1 to 1.8 degrees. Roll and yaw share `q.x`, so roll is
always 2.5 times yaw with the opposite sign.

**`rand` is centred.** `0x006762f8` is only a TOC switch into `0x0043c280`. That code warms the LCG
`x = 1664525 * x + 1013904223` (`0x19660d`, `0x3c6ef35f`) eight steps on first use and fills a 32-slot
table. Each call then steps `x`, returns the slot picked by the previous output's top 5 bits and puts
`x` there (a Bays-Durham shuffle). The return is `rldicl r3, r0, 0, 34`, the low 30 bits, so `RAND_MAX`
is `2^30 - 1`. `rand() * 2^-30` is in `[0, 1)` and every point is centred on zero. The bounds in the
table above hold. A port draws from its own seeded `Rng`: the shuffle state is global and shared with
every other `rand` caller, so a sequence match is not on offer.

**What is not read:**
- The mask order at `0x00c47730`.
- How the constructor seeds `P0..P3` and `obj+0x80`.
- Whether any other code reads the 60-slot history.
- What body `+0x4c4` is. [`physics.md`](physics.md) scored it against a throttle square wave as thrust.
  [`xfade.md`](xfade.md) multiplies it by 3.6 (a m/s to km/h shape) and `magstrip-wake.md` calls it
  speed. That decides whether a craft coasting fast shakes. A live read after a throttle release at
  speed settles it: thrust drops at once, speed does not.
- Whether opponents run the same block. It is inside the per-ship update, but no branch above
  `0x000ebb28` was read for a player test.

## Cross-title

- **Omega:** not checked. A search for `2^-30` (`0x30800000`) in Omega's `eboot.bin` returns 30-odd
  hits with no constant pool beside them. Finding it means locating Omega's ship update call site.
- **2048:** not checked. Two hits in v1.04, neither beside the shake's other constants.
- **Pulse, Pure:** not read.
