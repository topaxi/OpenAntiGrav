# Pure: how a Rocket and a Plasma bolt follow the floor

Question (maintainer, 2026-11-25): do Pure, Pulse and HD differ in how Rockets and Plasma bolts
follow the floor? Pure carried Pulse's law by inheritance (`floor_arms_steer_to_ride_point`,
"Pure's own `Rocket_Update` and `Plasma_Update` unread").

**Answer: Pure's law is its own, a third one.** Its Rocket and Plasma run the travel sweep before
the surface probe, and probe and ride along the normal they were launched with, which they never
change. Pulse adopts the floor's normal on every ride
([`psp-pulse-usa/rocket-visuals.md`](../psp-pulse-usa/rocket-visuals.md), "Pulse on the steepest
bank"); HD turns the velocity parallel to the floor
([`ps3-hdfury-eu/plasma.md`](../ps3-hdfury-eu/plasma.md)). Confidence **90**: decompiled in both
functions, the writer scan below, and ten flights watched live on PPSSPP, five of them chained in
ours to 0.0001 unit.

Program: `/pure/BOOT-psp-pure-usa.BIN` (UCUS98612 2.00). Its `jal` targets are pre-relocation
(`0x00020a9c` is `Collision_RaycastWorld`, `0x08824a9c`).

## The two updates, side by side with Pulse's

`Rocket_Update` (`0x0885f28c`; entity `+0xa0` position, `+0xb0` velocity, `+0xc0` previous
position, `+0xd0` carried normal, `+0x50` age) and `Plasma_Update` (`0x0885e348`; `+0xa0`, `+0xf0`,
`+0x100`, `+0x110`, `+0x58`), read in the disassembly:

| Step | Pure (both weapons unless named) | Pulse (`Rocket_Update` `0x0885d2a8`, `Plasma_Update` `0x0885c6cc`) |
| --- | --- | --- |
| order | travel sweep `prev -> next` first (`0x0885f340`, `0x0885e3f0`), then the probe (`0x0885f554`, `0x0885e4d0`) | probe first, then the travel sweep |
| query | `Collision_RaycastWorld(world, from, to, &hit, -1, 0, 0)`, branching on the hit kind themselves (`1` mesh; a box runs no arm, [own-craft-box.md](own-craft-box.md)) | `Collision_SweepSegment`, codes `0`, `1`, `3`, `4`, `0x7f` |
| sweep, floor | `next = hit + hit_normal * 3.0` (`0x0885f4b8`, `0x0885e44c`); no velocity, no normal written | Rocket: adopts the normal, `velocity = (hit + 3n - prev) / dt` raw; Plasma: moves only |
| sweep, wall (`collider+0x6c == 0`) | destroy (`flags \|= 0x14`, Rocket; `\|= 4`, Plasma) | destroy |
| probe | `next - carried * 12.0` (`0x0885f51c`, `0x0885e498`) | Rocket `6.0`, Plasma `12.0`, along the carried normal |
| probe, floor | `ride = hit + carried * 3.0` (Rocket, `0x0885f6dc`) or `* 6.0` (Plasma, `0x0885e534`): **the carried normal, not the hit's**; `velocity = normalize(ride - prev) * speed / 3.6`, `next = prev + velocity * dt` | adopts the hit normal; `ride = hit + n * 3.0` (Plasma `g_ride_height`); the same steer |
| probe, nothing | `velocity.y -= dt * 50.0`, world down, both (`0x0885f814`, `0x0885e668`) | Rocket world down; Plasma `velocity -= carried * dt * 50` |
| carried normal | **never written** in either update | the hit normal on every ride |

## Nothing else writes the carried normal

Every store to `+0xd0..+0xdc` and `+0x110..+0x11c` in `0x08850000..0x08870000`, scalar
(`scripts/psp-relocate.py field`) and VFPU (`sv.q`/`sv.s`, Ghidra's instruction search), excluding
the stack:

- Rocket `+0xd0`: `Rocket_Init` (`0x0885eed0`) alone. `0x0885e950` also stores to `+0xd0`, but on
  the Plasma entity (it copies `+0x70..+0xa0` to `+0xb0..+0xe0`); `Missile_Init`, `Missile_Update`
  and `BombBlast_Construct` are other classes.
- Plasma `+0x110`: `Plasma_Launch` (`0x0885e2cc`) alone.

`Rocket_Init` stores `+0xd0 = matrix[1]` and `+0xb0 = matrix[2] * FUN_0885f258() / 3.6` (the class speed lookup, unnamed); its
caller `Rocket_FireVolley` (`0x088567c8`) hands it the craft's matrix for the centre rocket and the
same matrix turned by `+-` the fan angle for the other two. `Plasma_Launch` stores
`+0x110 = craft[1]` and `+0xf0 = craft[2] * speed / 3.6`. `Missile_Init` (`0x0885c884`) also stores
`+0xd0 = matrix[1]`. So the normal is **the craft's up row**, not a located track record as on Pulse
(`-craft+0xb10`); and the rows carry the craft's display scale, so the normal is 0.75 long and every
length taken along it is three quarters of its literal: probe 9, rides 2.25 and 4.5. The same scale
puts the launch speed at 0.75 of the class speed until the first ride.

## Watched live

PPSSPP v1.20.4, `pure-psp-usa.chd` (state serial `UCUS98612_2.00`), the Time Trial restart states in
`data/saves/pure-psp/` (Vector, Feisar), cross held, the weapon given by writing its id into the
holder's `+0x1d8` and the fire byte (`scripts/psp-pure-weapon-fire.py`'s method), then a break at each
`Rocket_Update` or `Plasma_Update` entry reading the entity, and at the probe's return reading `v0`,
the hit kind and the hit normal. Pure updates 30 times a second here (`dt` 0.0334).

| Shot | Flights | Carried normal | What it shows |
| --- | --- | --- | --- |
| Vineta K, Rocket, start straight | 3, of 20, 17 and 22 updates | `(0.002, 0.750, 0.000)` in every update | probe hits kind `1` with normal `(0, 1, 0)` at y 33.22; the rocket rides at 35.47, **2.25** above. Launch 208.3 units/s, 277.8 from the first ride (1000 km/h) |
| Vineta K, Plasma, start straight | 1, of 17 updates | `(0.002, 0.750, 0.000)` throughout | rides **4.50** above; 187.5 units/s at release, 250 from the first ride |
| Sebenco Climb, Rocket | 3, of 14, 13 and 25 updates | `(0.000, 0.750, 0.000)` throughout | the probe meets floors `(-0.259, 0.966, -0.012)` and `(-0.012, 0.966, -0.260)`, **15 degrees** off the carried normal, and the normal does not move |
| Sebenco Climb, Plasma | 1, of 11 updates | `(-0.002, 0.750, -0.002)` throughout | as the Rocket |
| Vineta K, Rocket, at the first corner (fire 205 updates after the state) | 3 | fixed | every probe returned nothing (`v0 = 0`): the rockets fell at `50 * dt` for 150 updates to y -532. **Cause not separated**: the craft stopped against the corner's wall, so the volley may have left the floor ahead; not used as evidence of the law |

The craft's own hover raycast (`0x0892de48` in `0x0892da8c`) calls the same function on the same
world (`*(craft+0x2b8)`, `0x09022030`, 86 colliders: 44 of `+0x6c` type `0`, 33 type `1`, 9 type
`2`) with the box test off (seventh argument `2`); it hits the floor every update.

## In ours

`FloorArms::LaunchNormal` (`oag_weapons::projectile::launch_normal`), chosen by
`Weapons::floor_arms_ride_launch_normal` (Pure measured `true`, Pulse measured `false`, HD, 2048
and Omega chosen `false`). Ours keeps the carried normal a unit vector and scales the lengths:
`PROBE_LENGTH` 9, the rides 2.25 and 4.5, the sweep's push-out 3 along the hit normal unscaled.
Pure's `launch_normal_is_track_up` is now measured `false` (the craft's up) and its
`plasma_release_at_display_scale` measured `true` (187.5 against 250 live). The Missile keeps
Pulse's arms (`FloorArms::for_other_weapons`): Pure's `Missile_Update` is unread.

Acceptance, `crates/raceplay/tests/projectile_bank_ground_truth.rs` (chained from the original's
state at update 1, on its own `dt`):

| Flight | `LaunchNormal` | Pulse's arm |
| --- | --- | --- |
| Vineta K, three Rockets and the Plasma | 0.0001 unit worst; each ends on the original's last update | 0.94 (Rockets), 0.67 (Plasma) units off by update 2 |
| Sebenco Climb, the 25-update Rocket | 0.0001 unit; ends on update 25 | 1.89 units off; ends on update 15 |

Unit tests on a flat floor with a 15 degree carried normal:
`crates/weapons/src/projectile/tests/floor_arms.rs` (`under_the_launch_normal_*`).

## Not checked

- Pure's `Missile_Update` (`0x0885cf30`) floor arm: it writes `+0xd0` at `0x0885d410`, so its
  normal may move; the Missile keeps Pulse's arms (inherited).
- Pure's steepest bank: the census tool (`oag-trace track`) refuses a Pure image; the 15 degree
  floor is the steepest these shots met.
- Whether `Rocket_FireVolley`'s matrix is the body's or a hull node's: decompile only. Either way
  it is the craft's, not the located track's (which ours on Pulse takes from the spline).

## Cross-title

- **Pulse:** differs, measured on both (the section linked above).
- **HD:** differs (parallel to the floor each frame, `ps3-hdfury-eu/plasma.md`).
- **2048, Omega:** not checked against this page; their arms follow HD's lineage.

## Names

| Address | Name | Confidence | Evidence |
| --- | --- | ---: | --- |
| `0x088567c8` | `Rocket_FireVolley` | 78 | three `Rocket_Init` calls: the craft's matrix, then the matrix turned by `FUN_08a2e3fc(+angle)` and `(-angle)`; live, three rockets from one press, the two outer ones fanned symmetrically about the first (z velocity +10.4 and -10.4 from it, 2026-11-25) |
