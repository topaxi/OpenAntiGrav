# HD: the firing craft's own box in a projectile's probe

Question carried over from Pulse
([psp-pulse-usa/rocket-visuals.md](../psp-pulse-usa/rocket-visuals.md), "The own-craft box is a
rotated frame"): does the firing craft's own box hide the track from a projectile's first probes?

**Answer: yes, the box answers, with Pulse's gate and `0x7f`. What follows is not Pulse's: HD's
Rocket does not fall on `0x7f`.** Static read 2026-11-13, measured live on RPCS3 2026-11-26 (three
boots, "Measured live" below), on `/hdfury/EBOOT-ps3-hdfury-eu.elf`. Confidence: the box answers a
probe of the firer's own projectile **92**, the transposed gate **92**, the probe lengths (6.0 for
Rocket, Plasma, Missile) **95**, the Rocket's `0x7f` arm **88** (two boots). Wired as HD title
data ([projectile-floor.md](../../../gameplay/projectile-floor.md), "HD's box and open-air Rocket").
The Ghidra GUI decompiles
`Collision_MarchSegment` to `halt_baddata` (the `lvlx` trap), so the collision functions were read
with

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

## Measured live (2026-11-26, RPCS3)

Falsifier written before the capture: the box arm is never reached with the firer's collider
(filtered), or reached and never writes kind 3 on probes that geometrically cross it, or no
prediction of the transposed-gate model crosses (then non-informative).

**Method.** `scripts/rpcs3-hd-own-box.py` on `hd-racebox-talons-venom-weapons-grid-rpcs3-20574`,
served with `--interpreter`, craft parked, a weapon written to the pickup slot and fired. Breakpoints:
`0x0007bea4` (segment, flag byte, caller), `0x00036cac`/`0x00036cb0` (collider and the shared hit
record around the box test), the two returns of `0x0007be58`. Reproduce:

```sh
scripts/emu-restore-state.sh <lane> data/saves/hd-fury/hd-racebox-talons-venom-weapons-grid-rpcs3-20574.SAVESTAT.zst --interpreter
OAG_RPCS3_ATTACH=1 uv run --with evdev python3 scripts/rpcs3-hd-own-box.py capture --log-dir <dir> --weapon rocket --fires 5
python3 scripts/rpcs3-hd-own-box.py replay <dir>/rocket.json          # 0 mispredicted
python3 scripts/rpcs3-hd-own-box.py replay --frame rows <dir>/rocket.json   # the control
```

The player's collider is index 537: shape 3, `+0x74 = 0` (not the skipped `2`), the march's flag byte
0, so **the filter passes**. Its rows equal the body's rows and its centre the body's position to the
last bit on every call (rows `+0x1d0..+0x1f0`, position `+0x200`), and its row 0 is `row1 x row2`,
**the craft's left** as on Pulse. Full dimensions `(3.75, 2.625, 9.75)`, Pulse's order (width,
height, length) at `0.75`.

| Boot | Weapons fired | Box calls on the own collider | Answered kind 3 | Mispredicted, transposed gate (Pulse's) | Mispredicted, true-box frame |
| --- | --- | ---: | ---: | ---: | ---: |
| 1 | 6 Plasma, 6 Rocket volleys, 3 Missile | 108 | 13 | **0** | 15 |
| 2 | 19 Rocket volleys | 142 | 38 | **0** | 10 |
| 3 | 5 Rocket volleys, 3 Plasma, 3 Missile | 97 | 13 | **0** | 12 |

So **HD reads the box through the transposed rotation, as Pulse does** (the true-box frame
mispredicts 37 of 347, the transposed one none).

**Which probes the box answers, parked.** Rocket: a side rocket's probe in 10 of 30 volleys (the
others miss by heading; the model says so). Its sweeps also meet the box. Plasma and Missile: never a
probe (they start 5.5 ahead of the centre, the box ends at 4.875); their first sweeps can. On the
Plasma and the Missile the first probe from the release rides, so no title-wide window is needed.

**Probe lengths, live:** Rocket (`0x001249dc`), Plasma (`0x0011fd80`) and Missile (`0x0011d3a4`) all
`6.0` on every call. The Shuriken's cannot be measured: HD builds no Shuriken
([open-air-arms.md](open-air-arms.md)); its dead update casts `12.0`.

Records (boot 1, 3 decimals, Talon's Junction grid; start and end are in world space, the rows are the
collider's, the centre its position):

| Call | Rows (0, 1, 2) | Centre | Start | End | Original |
| --- | --- | --- | --- | --- | --- |
| volley 1, call 3 (probe) | (-0.222, -0.008, -0.975) (0.002, 1.000, -0.008) (0.975, -0.003, -0.222) | (-131.858, -49.602, -175.515) | (-131.087, -50.115, -177.626) | (-131.106, -56.115, -177.577) | box clear, mesh floor (1) |
| volley 1, call 4 (sweep) | (-0.222, -0.008, -0.975) (0.002, 1.000, -0.008) (0.975, -0.003, -0.222) | (-131.858, -49.602, -175.515) | (-134.653, -50.150, -176.655) | (-131.088, -50.620, -177.622) | box kind 3 |
| volley 1, call 5 (probe) | (-0.222, -0.008, -0.975) (0.002, 1.000, -0.008) (0.975, -0.003, -0.222) | (-131.858, -49.602, -175.515) | (-130.280, -50.162, -173.954) | (-130.300, -56.161, -173.905) | box kind 3, returned 0x7f |
| Plasma sweep, yaw 76 | (0.952, 0.218, -0.215) (-0.188, 0.970, 0.151) (0.242, -0.103, 0.965) | (379.269, -33.228, -143.065) | (380.508, -33.826, -138.584) | (380.715, -33.769, -137.679) | clear (a true-box frame would hit) |

## The Rocket on `0x7f` turns parallel and does not fall

`Collision_SweepSegment` returns the same `0x7f` for the box and for open air, so `Rocket_Update`
cannot tell them apart. Its arm (`0x124d64`) clears the ride byte `sp+0x204` and rejoins the common
tail at `0x1240b8`, and it loads no `50.0` from its TOC (the Plasma's does). Live, the rocket's
velocity `+0xe0` was read at its probe call and again at its travel sweep call of the same update
(boots 2 and 3): on 8 updates whose probe returned `0x7f` (6 from the box, 2 from open air) it equals
the **parallel turn** `normalize(v - (v.n) n) * |v'|` on the carried normal `+0x100`, which the
update leaves unchanged:

| Hypothesis for the velocity after the `0x7f` update | Largest error over 8 | Smallest error over 6 box records |
| --- | ---: | ---: |
| parallel turn on the carried normal | **0.0006** | |
| fall `50 * dt` and then the turn | | 0.0069 |
| unchanged | | 0.1400 |

(The two open-air records already had `v.n = 0`, so "unchanged" fits them too; the fall misses them by
0.29 and 0.35.) The next probe of the same rocket reads the same velocity, so no fall is applied
later either. The rocket moves to `from + v * dt` (the sweep's end equals the probe's start).

## What is not read

- The Plasma wired here still falls along its carried normal (Pulse's arm), where its static read is
  world Y.
- The Plasma's `0x7f` arm is a fall on world Y, static only ([plasma.md](plasma.md)); no box answered
  a Plasma probe. The Missile's and Shuriken's arms are on [open-air-arms.md](open-air-arms.md).
- Not wired: HD re-pins the Rocket to class speed through `0.2777` (222.16 units/s on Venom) where
  `oag_weapons` divides by `3.6` (222.22).
- An opponent's box, and the travel sweep's own box meeting (the sweeps `0x124448` and `0x1202d8`
  return `0x7f` off the box and off nothing alike).

## Names

| Address | Name | Confidence | Evidence |
| --- | --- | ---: | --- |
| `0x0007be58` | `Collision_SweepSegment` | 90 | the listing above; Pulse `0x0883198c` without the cage; returns observed live |
| `0x000336b0` | `Collision_RaycastBox` | 90 | the shape-`3` arm of `Collision_MarchSegment`; its writes watched live |
| `0x0003a4c8` | `Collision_SetObbTransform` | 72 | copies four matrix rows to `+0x30..+0x60`, rebuilds the scaled axes |

## Cross-title

Pulse: applies, measured. Pure: applies, differs in consequence
([psp-pure-usa/own-craft-box.md](../psp-pure-usa/own-craft-box.md)). 2048 and Omega: **differs**,
the box is absent from their query ([vita-2048-eu-v104/own-craft-box.md](../vita-2048-eu-v104/own-craft-box.md),
[ps4-omega-eu/own-craft-box.md](../ps4-omega-eu/own-craft-box.md)), although HD is 2048's engine
lineage. HD's `6.0` Missile and Plasma probes and its Rocket arm are wired for HD alone; Omega's
`Plasma_Update` reads `6.0` too ([plasma.md](plasma.md)), **checked, applies, not wired**.
