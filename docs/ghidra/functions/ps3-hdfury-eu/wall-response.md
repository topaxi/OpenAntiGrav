# The wall response: HD's resolver, restitution table and steer-away torque

2026-10-20, lane `hd-wall-resolver`. The question
[physics.md](physics.md#the-wall-response-is-not-in-the-pair-narrowphase-2026-10-19)
left open: where HD resolves a craft against a wall, and whether its law is
Pulse's ([contact-response.md](../psp-pulse-usa/contact-response.md)). It is
Pulse's resolver with four differences, three of them HD's own and live-measured:
the restitution is `0.8` instead of `0.4`, the normal impulse is one-sided, a
steer-away yaw torque follows every contact, and the point velocity mixes frames.
Friction, the denominator, the `0.1` angular scale, the push-out and the shield drain
are Pulse's.

Program `/hdfury/EBOOT-ps3-hdfury-eu.elf`, TOC `0x008ad4d8` for every function here.

## How it was found: a string, not a breakpoint

The brief's route was a write watchpoint on the velocity quad. That needs the patched
RPCS3 (`scripts/build-rpcs3-watchpoints.sh`), which is not built on this host. A
string search found the resolver first:

1. `shipWallCofR` (`0x007813d8`) sits in a run of attribute names (`grip_air`,
   `weaponHitMassFactor`, `shipShipCofR`, `shipWallCofR`, `windowStart`, ...). HD's
   global `Data/XML/HandlingStats.xml` (`DATA02.PSARC`, the only copy on the disc)
   authors `<Collision weaponHitMassFactor="0.9" shipShipCofR="1.0f" shipWallCofR="0.80f"/>`.
2. `Handling_ParseCollision` (`0x000c67a8`) compares each attribute name and stores
   the parsed float through TOC slots `-0x4dbc`, `-0x4db4`, `-0x4dac`, which hold
   `0x008c1738` (`g_WeaponHitMassFactor`), `0x008c1564` (`g_ShipShipCofR`) and
   `0x008c156c` (`g_ShipWallCofR`).
3. `0x008c156c`'s only reader is `Collision_PairRestitution` (`0x000bdec0`), whose only
   caller is `Collision_AddContact` (`0x00033af0`). The contact array it fills is read
   back through `Collision_GetContact` (`0x00033618`), whose only caller is
   `Physics_StepWorld`'s contact loop, which calls `Body_ResolveContact`
   (`0x000f7770`).

## The coefficient tables

`Collision_PairRestitution` (`0x000bdec0`) and `Collision_PairFriction` (`0x000be068`)
take the two colliders' material words (`collider+0x84`), order them, and return a
float from one table at `0x008c1564` (TOC `-0x50ec` of either function). Defaults as
compiled, then live (2026-10-20, Time Trial and Single Race, Talon's Junction):

| Offset | Pair | Read by | Compiled | Live | Meaning |
| ---: | --- | --- | ---: | ---: | --- |
| `+0x00` | `(0, 0)` | restitution | `0.1` | `1.0` | `shipShipCofR` |
| `+0x04` | `(0, 2)`, `(0, 4)` | restitution | `1.0` | `1.0` | |
| `+0x08` | `(0, 3)` | restitution | **`0.4`** | **`0.8`** | `shipWallCofR` |
| `+0x0c` | `(1, 1)` | restitution | `0.5` | `0.5` | |
| `+0x10` | `(1, 2)`, `(1, 4)` | restitution | `0.3` | `0.3` | |
| `+0x14` | `(1, 3)` | restitution | `0.01` | `0.01` | |
| `+0x18`..`+0x20` | `(5, 2)`, `(5, 4)` | restitution | `0.05`, `2.5`, `0.5` | same | a random pick, `Libc_Rand` |
| `+0x24` | `(5, 3)` | restitution | `0.1` | `0.1` | |
| `+0x28` | `(0, 0)`, `(0, 1)` | friction | `0.035` | `0.035` | |
| `+0x2c` | `(0, 3)` | friction | **`0.035`** | **`0.035`** | ship against wall |
| `+0x38` | `(0, 3)` on four mode ids | friction | `0.025` | | mode mask `0x206040`, not seen |
| `+0x30`, `+0x34` | type `5` pairs | friction | `0.025`, `0.05` | | |

A pair outside the table returns `0.0` (TOC `-0x50e8`). The compiled ship-wall
restitution is Pulse's `0.4` and the friction Pulse's combined `(0.05 + 0.02) / 2`: HD
collapsed Pulse's per-collider friction into a pair table and kept the numbers. The
XML is what makes HD's restitution differ. `RPCS3 GDB` reads of the table on two boots
gave `1.0` and `0.8` at `+0x00` and `+0x08`, so the `f` suffix in `"0.80f"` is ignored
by the parser, as `atof` would. The craft's collider (`body+0x4cc`) has material `0`;
every one of 26 player contacts measured below was against material `3`.

**Confidence 92** for `g_ShipWallCofR = 0.8` reaching every craft-wall contact: the
parser, the table read and 26 of 26 live contact records (`+0x30 = 0.800`) agree.
**85** for the pair layout, the `Libc_Rand` branch and the mode-masked friction being
read only. Single Race and Time Trial both gave `0.035`; Zone or the other masked modes
were not run.

## `Collision_AddContact` (`0x00033af0`)

Called through a function pointer (OPD `0x00871110`) by the narrowphase. It appends a
`0x40`-byte record at `world+0xd760 + 0x40 * n` (`world+0xf760` is the count):

| Offset | Field | Live |
| --- | --- | --- |
| `+0x00` | contact point | world |
| `+0x10` | unit normal | out of the wall |
| `+0x20`, `+0x24` | the two colliders | craft `0`, wall `3` |
| `+0x28` | depth | `0.015`..`0.606` |
| `+0x2c` | `Collision_PairFriction(matA, matB)` | `0.035` |
| `+0x30` | `Collision_PairRestitution(matA, matB)` | `0.8` |
| `+0x34` | the world's tick (`world+0x40`) | |

## `Physics_StepWorld`'s contact loop (`0x000f88d0`-`0x000f8964`)

After `Collision_ProcessPairs`, for each record from `Collision_GetContact`: a contact
whose first collider has no owning body (`collider+0x80`) resolves the second body
alone, the reverse likewise (`Body_ResolveContact`, `0x000f8a50`); two owners where
neither body has bit `0x2` of `+0x4c8` set go to `Body_ResolveContactPair_q`
(`0x000f7bc0`, `0x000f8a7c`). Each side's `+0x210` is set to the other's material
first. The pair resolver was not read.

This runs **after** the third body loop, where each body is integrated
(`Body_Integrate`, `0x000f6870`) and its accumulators cleared
(`Body_ClearAccumulators`, `0x000f5810`, four quads `+0x150`..`+0x180`). So a torque a
contact adds is integrated on the **next** step.

## `Body_ResolveContact` (`0x000f7770`)

`r4` the contact, `r5` the body.

```text
r      = contact.point - body+0x200                     # 0x000f77a8, the position
v_p    = body+0x190 + cross(r, body+0x1a0)              # 0x000f77e0-0x000f7850, no rotation of omega
vn     = dot(v_p, contact.normal)                       # stored at sp+0x130
if !(0 >= vn) return                                    # 0x000f7880 vcmpgefp., blt cr6
invMass = body+0x4b4 ? body+0x4b0 : body+0x4a8          # 1.0 live
D      = invMass + dot(cross(M90 * cross(r, n), r), n)  # M90 = body+0x90
if D == 0 return                                        # 0x000f79ac
j      = vn * (-1.0 - contact+0x30) / D                 # -1.0 at TOC -0x418c
p      = j * n
if contact+0x2c > 0 and v_t != 0: p -= contact+0x2c * v_t   # v_t = v_p - n * vn
Body_ApplyImpulseAtPoint(body, contact.point, p, 1)     # 0x000f6348
Body_Translate(body, contact+0x28 * n)                  # 0x000f5ca0
Body_RecordContact_q(body, point, n, p, friction, ...)  # 0x000f56d0
s      = -clamp(dot(body+0x1d0, p), -20.0, 20.0)        # right row; TOC -0x4180/-0x417c
Body_AddTorque(body, body+0x1e0 * s * 5.0)              # up row; 5.0 at TOC -0x4188; 0x000f5848
```

`Body_ApplyImpulseAtPoint` (`0x000f6348`): `v += p * invMass`; `L(+0x1c0) -= M110 *
cross(r, p) * body+0x4c0`; `omega(+0x1a0) = Md0 * L`. `M110` is the basis transpose,
`body+0x4c0` reads `0.1` live (Pulse's `body+0x394`), and `Md0` is the inverse inertia
in the body frame.

### Live, 2026-10-20: sixteen wall hits

Time Trial, Talon's Junction, Venom, Feisar, interpreter decoder. `Z0` alternated
between `0x000f78b8` (past the separating gate) and `0x000f7ba8` (after the torque),
player body only, reading the contact record and the body's `+0x180..+0x210` either
side. `p` is the impulse on the stack (`sp+0x90`). The world-axis denominator uses the
diagonal `1/(17.333, 24, 17.333)`, which `body+0x90` holds live.

| # | speed | `vn` | `j` | denominator, implied | denominator, world-axis | `mu`, fitted | `Δv - p` | `ΔL / (0.1 r x p)` | kick `s` | `Δτ·up` |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | 132.3 | -15.13 | 13.75 | 1.982 | 1.982 | 0.0350 | 0.0000 | 1.000 | 14.197 | 70.986 |
| 2 | 98.7 | -13.44 | 12.40 | 1.951 | 1.951 | 0.0350 | 0.0000 | 1.000 | 12.804 | 64.019 |
| 3 | 118.1 | -53.29 | 57.12 | 1.679 | 1.679 | 0.0350 | 0.0000 | 1.000 | 20.000 | 100.000 |
| 4 | 98.1 | -1.58 | 1.62 | 1.748 | 1.748 | 0.0350 | 0.0000 | 1.000 | 0.452 | 2.260 |
| 5 | 94.7 | -0.74 | 0.75 | 1.785 | 1.785 | 0.0350 | 0.0000 | 1.000 | -0.236 | -1.180 |
| 6 | 96.6 | -4.67 | 4.30 | 1.954 | 1.954 | 0.0350 | 0.0000 | 1.000 | -4.783 | -23.914 |
| 7 | 91.1 | -46.92 | 40.59 | 2.081 | 2.081 | 0.0350 | 0.0000 | 1.000 | -20.000 | -100.000 |
| 8 | 93.8 | -9.04 | 7.54 | 2.159 | 2.159 | 0.0350 | 0.0000 | 1.000 | 7.403 | 37.013 |
| 9 | 89.0 | -8.51 | 14.38 | 1.066 | 1.066 | 0.0350 | 0.0000 | 1.000 | 14.335 | 71.677 |
| 10 | 84.9 | -12.08 | 10.38 | 2.095 | 2.095 | 0.0350 | 0.0000 | 1.000 | 10.423 | 52.113 |
| 11 | 93.9 | -39.30 | 38.91 | 1.818 | 1.818 | 0.0350 | 0.0000 | 1.000 | -20.000 | -100.000 |
| 12 | 88.8 | -3.99 | 3.47 | 2.070 | 2.070 | 0.0350 | 0.0000 | 1.000 | 3.273 | 16.367 |
| 13 | 84.9 | -10.66 | 18.24 | 1.052 | 1.052 | 0.0350 | 0.0000 | 1.000 | 18.022 | 90.111 |
| 14 | 80.8 | -3.13 | 2.73 | 2.064 | 2.064 | 0.0350 | 0.0000 | 1.000 | 2.804 | 14.020 |
| 15 | 89.8 | -5.04 | 8.49 | 1.069 | 1.069 | 0.0350 | 0.0000 | 1.000 | -8.607 | -43.036 |
| 16 | 86.1 | -12.02 | 10.66 | 2.030 | 2.030 | 0.0350 | 0.0000 | 1.000 | -10.856 | -54.279 |

"Implied" is `-(1 + e) vn / j` with `vn` from the recorded state; "world-axis" is the
denominator predicted from geometry alone. They agree to the third decimal on all
sixteen, with `e = 0.8` from the record. Ten more contacts in a Single Race (weapons
on) gave the same `e` and `mu`.

What each column settles:

- **Restitution `0.8`, friction `0.035`, the denominator.** Implied and predicted
  denominators match, `mu` fits to four decimals. Pulse's law with HD's `e`.
- **Inverse mass `1.0`, push-out by `depth`.** `Δv = p` exactly; the position moved by
  `depth * n` to the printed digit on all sixteen.
- **The angular scale is `0.1`.** `|ΔL| = 0.1 |r x p|` exactly, and `ΔL = -0.1 R^T (r x
  p)` component for component.
- **The kick.** `Δτ(+0x180) = up * 5 * s` with `s = -clamp(dot(right, p), -20, 20)`
  exactly, including the three clamped hits (3, 7, 11). Nothing else touches `+0x180`
  in the resolver.

### The point velocity mixes frames

`body+0x1a0` is the **negated body-local** angular velocity, measured over 39
consecutive `Body_Integrate` entries while steering: the rotation the basis rows make
from one entry to the next gives `omega_world`, and `body+0x1a0 = -R^T omega_world` on
every pair (`0.15` rad/s of yaw read as `+0.150` against `-0.150`, pitch and roll
components matching sign and size). The resolver uses it in `cross(r, body+0x1a0)`
with `r` in world space, so its point velocity is `v + (R^T omega) x r`. The textbook
`v + omega x r` was tested and fails: it implies denominators of `2.419`, `3.276` and
`1.019` on hits 4, 5 and 7 against the geometry's `1.748`, `1.785` and `2.081`. Pulse
rotates its stored rate back to world before the cross product
([contact-response.md](../psp-pulse-usa/contact-response.md#the-full-read-of-body_resolvecontact-0x0884e968));
HD does not. The two agree for a level craft (yaw is the up component in either frame)
and diverge with pitch or roll. **Confidence 85**: sixteen hits, three of them
discriminating, and the frame of `+0x1a0` measured independently.

### The normal impulse is one-sided

`0x000f7880`: `vcmpgefp. v9, v9, v0` with `v9 = 0` and `v0 = vn`, `blt cr6` to
`0x000f78b8` when `0 >= vn`; otherwise straight to the epilogue. Live, a `Z0` at the
shared exit `0x000f7888` read the stored `vn` (`sp+0x130`): four calls with `vn = 2.62,
3.07, 10.24, 10.36` left the player body's velocity untouched. PSP Pulse applies the
impulse unconditionally (a two-sided constraint); its PS2 build gates like HD. **Confidence
88.**

### The steer-away torque

`Body_AddTorque` (`0x000f5848`) adds to `+0x180`; the airbrake and pitch torques go
through `Body_AddBodyTorque` (`0x000f5dd8`) into `+0x170`. Fitted over twelve
consecutive integrator steps, `L(+0x1c0)` advances by `dt * (tau170 + R^T tau180)`
(`14.929 + 0.070` against `14.997`, `-1.321 + 0.070` against `-1.253`). With `L` and
`omega` both stored negated, the torque the craft actually feels is
`up * 5 * clamp(dot(right, p), -20, 20)`. A wall on the craft's right pushes along
`-right`, so the torque is about `-up` and the nose turns left, away from the wall.
At the clamp that is `100` units of torque for one step, `0.07` rad/s of yaw on the
`24` yaw inertia. Pulse's resolver ends at the impulse, the translate and the record
(`0x0884eea8`-`0x0884ef04`); it has no such term. **Confidence 85** for the law, 80
for the one-step delay (read from the loop order, not timed).

## The shield drain is Pulse's

`Body_RecordContact_q` (`0x000f56d0`) pushes the contact into the body's eight-entry
ring (`+0x220`, `0x50` a record, count `+0x4a0`): point at `+0x230`, impulse at
`+0x240`, normal at `+0x250`, the friction at `+0x260`, the other collider's `+0x74`
at `+0x264`. That float is the friction, not a length as
[xfade.md](xfade.md) read it from the caller. The craft update `FUN_000eadb8` walks the
ring from `0x000ec234` and scales `|p|` by `0.035` (TOC `-0x43f0`, `0x008a90e8`).

Live, Single Race, weapons on, ten contacts over eight frames: the shield
(`ship+0x5fa0`) fell by `0.035 * sum |p|` per frame on every frame (`|p| = 3.874`:
`140.000 -> 139.864`; two records `9.501 + 3.574`: `139.672 -> 139.214`; `16.077`:
`138.676 -> 138.113`). Pulse's `CONTACT_DAMAGE_SCALE`, per record. HD's impulses are
larger for the same approach (`e = 0.8`), so a hit costs more, through the impulse
alone. **Confidence 90.** The weapons-off halving was not measured on HD.

## 2048 and Omega

Checked, applies: both executables carry `weaponHitMassFactor`, `shipShipCofR` and
`shipWallCofR` (2048 `0x814901b0`.., Omega `0x018143e9`..), Pulse's does not, and their
global `handlingstats.xml` (2048 `data.psarc`, Omega `data00.psarc`) are byte-identical
(md5 `40719679c4e801d70ee5329067fa1689`) and author the same `<Collision>` element. The
resolver code itself was not read on either; both inherit HD's laws.

## Names

| Address | Name | Confidence |
| --- | --- | --- |
| `0x000f7770` | `Body_ResolveContact` | 90 |
| `0x000f7bc0` | `Body_ResolveContactPair` | 60 |
| `0x000f6348` | `Body_ApplyImpulseAtPoint` | 88 |
| `0x000f6870` | `Body_Integrate` | 82 |
| `0x000f5810` | `Body_ClearAccumulators` | 78 |
| `0x000f5848` | `Body_AddTorque` | 85 |
| `0x000f5dd8` | `Body_AddBodyTorque` | 78 |
| `0x000f5ca0` | `Body_Translate` | 88 |
| `0x00033af0` | `Collision_AddContact` | 88 |
| `0x00033618` | `Collision_GetContact` | 85 |
| `0x000bdec0` | `Collision_PairRestitution` | 90 |
| `0x000be068` | `Collision_PairFriction` | 85 |
| `0x000c67a8` | `Handling_ParseCollision` | 82 |
| `0x008c1564` | `g_ShipShipCofR` | 88 |
| `0x008c156c` | `g_ShipWallCofR` | 92 |
| `0x008c1738` | `g_WeaponHitMassFactor` | 80 |

`0x000f56d0` keeps its row from [xfade.md](xfade.md) (`Physics_QueueRayProbe`, 62);
the reading above says it is the body's contact record, `Body_RecordContact` by Pulse's
naming, and that lane's owner should take the rename.

## Reproducing it

`rpcs3-drive.py serve` under `--interpreter` from `hd-timetrial-talons-venom-start`
(`scripts/emu-restore-state.sh <lane> <state> --interpreter`), hold `cross` with the
stick hard over, and alternate `Z0` between `0x000f78b8` and `0x000f7ba8` with the
`hd-flare-owner-break.py` hop pattern (`resume`, `wait_for_stop`, `pause`, `drain`
between packets: a bare `resume`/`pause` pair leaves a queued stop reply that times out
the next command). Read `r30` (contact) and `r31` (body) at the first, the body's
`+0x180..+0x210` at both, and `sp+0x80..+0xb0` at the second. A killed run leaves a
breakpoint armed and the stub wedged; restore the state again.
