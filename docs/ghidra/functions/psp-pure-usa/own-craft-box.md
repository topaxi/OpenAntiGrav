# Pure: the firing craft's own box in a projectile's probe

Question carried over from Pulse
([psp-pulse-usa/rocket-visuals.md](../psp-pulse-usa/rocket-visuals.md), "The own-craft box is a
rotated frame"): does the firing craft's own box collider hide the track from a projectile's first
surface probes on Pure, with which law and which box?

**Answer: the box is there and the probe sees it, with Pulse's raycast, Pulse's transposed-frame
gate and Pulse's box. What a box hit does to the projectile differs: on Pure it neither falls nor
rides that update.** Confidence **84**, the rubric's ceiling for decompilation only (instruction-level
identity with Pulse functions that were measured live, read 2026-11-13 on `/pure/BOOT-psp-pure-usa.BIN`). No live Pure capture.

## The probe goes through `Collision_RaycastWorld` with boxes in

`Rocket_Update` (`0x0885f28c`), `Plasma_Update` (`0x0885e348`, was `FUN_0885e348`) and
`Missile_Update` (`0x0885cf30`) each make two queries a tick, the travel segment and the surface
probe, both as

```c
FUN_08824a9c(world, from, &to, &hit, 0xffffffff, 0, 0)
```

`0x08824a9c` is Pulse's `Collision_RaycastWorld` (`0x08816ac0`) line for line in the decompile:
the step count `|to - from| * 0.01 + 1`, the broad-phase query on `world+0x444` (Pulse `+0x440`),
the per-collider filter `index != skip && (collider+0x6c != 2 || flag)`, the shape-kind dispatch
`1..2` mesh (`0x08823cc4`, Pulse's `Collision_RaycastMesh` by position; its body not read, so not named), `3` box (`0x08825aa4`), and the copy of the nearest hit to the caller's
record. The fifth argument `0xffffffff` skips no collider and the seventh (`0`) leaves both mesh and
box tests on: the same call Pulse's `Collision_SweepSegment` makes.

## The box test is Pulse's, in the same transposed frame

`Collision_RaycastBox` (`0x08825aa4`) against Pulse's (`0x08817b1c`), disassembled end to end:

| Step | Pulse | Pure |
| --- | --- | --- |
| AABB clip of the segment against `collider+0x00/+0x10` | `jal 0x08815038` | `jal` to the same-shaped clip; reject returns |
| rows `+0x80/+0x90/+0xa0`, centre `+0xb0` copied to the stack, the centre row replaced by a constant | `0x08817bb4..0x08817cb0` | `0x08825b44..0x08825c7c` |
| an inverse of that matrix computed into the stack and never read | `jal 0x08814e34` into `sp+0x1d0` | `jal` into `sp+0x200` (`0x08825ba8`) |
| `(start - centre)` and `(end - centre)` transformed by the **un-inverted** matrix: memory rows loaded into column registers, `vtfm4.q` on the transposed register | `lv.q C100..C130`, `vtfm4.q C000,E100,C200` | `0x088af00c`: `lv.q C600..C630`, `vtfm4.q C410,E600,C400` |
| half dimensions `+0x1d0/+0x1d4/+0x1d8` times `0.5` | `0x08817e18..0x08817e2c` | `0x08825d58..0x08825d74` |
| three axis tests, then the three cross-product tests | `0x08817e34..0x08818030` | `0x08825d78..0x08825fc4` |
| eight corners, twelve triangles, nearest hit written with kind `3` | `0x08818040..0x08818144` | `0x08825fcc..0x088260e8` |

Pulse's gate was read live as `R^T * v` (39 of 39 results, `rocket-visuals.md`); Pure builds it with
the same instruction pattern from the same layout, so the gate region is the same transposed box.

## The box is the craft's own, Misc times 0.75

`Craft_Construct` (`0x089261ac`) stores `0.75` to the global scale (`_DAT_002afd08`) and then:

```c
dims = (stats+0x78, stats+0x80, stats+0x7c) * 0.75;      // Misc width, height, length
FUN_08934b74(world, class, 1, 1, &craft+0x8b4, &dims);   // Pulse FUN_0884e694
```

`FUN_08934b74` is Pulse's `FUN_0884e694` (shape `1` builds a box body and, flag `1`, registers it):
it calls `Collision_AddBoxCollider` (`0x08822e28`), which calls `Collider_SetBoxDimensions`
(`0x08827038`: `swc1 f12,0x1d0(a0)`, `swc1 f13,0x1d4(a0)`, `swc1 f14,0x1d8(a0)`, found by its bytes)
with the body's collider at `body+0x3fc` (Pulse `+0x3bc`). So the box is Pulse's: Misc times 0.75,
centred on the body, a Venom `4.125 x 2.625 x 9.75` if Pure's Misc matches Pulse's.

## The consequence differs: a box hit is neither a fall nor a ride

Pulse's weapons call `Collision_SweepSegment`, which turns any hit that is not a mesh (`kind != 1`)
into `0x7f`, the fall arm. Pure's call the raycast directly and branch on the kind themselves:

```c
if (!hit)                         velocity.y -= dt * 50.0;      // fall
else if (hit.kind == 1) {
    if (surface == 0)             detonate / destroy;           // wall
    else                          ride (hit + normal * h);      // floor
}
// hit.kind == 3 (a box): no arm runs; the velocity is unchanged this update
```

Read in all three: `Rocket_Update` (`if (local_4b8 == 1) ...` with no `else`), `Plasma_Update`
(the same), `Missile_Update` (`if (local_a38 != 1) goto ...` past both arms). So on Pure the firing
craft's box hides the track exactly where it does on Pulse, but the projectile coasts through those
updates at its launch velocity instead of falling. The travel segment's box hit is ignored the same
way. There is no cage query in Pure's probe.

Two Pure numbers that differ from what `oag_weapons` uses for every title (not wired here, written
down): `Rocket_Update`'s probe is `12.0` long (`0x41400000`), not Pulse's `6.0`; `Plasma_Update`'s
ride height is `6.0` (`0x40c00000`).

## Names

| Address | Name | Confidence | Evidence |
| --- | --- | ---: | --- |
| `0x08824a9c` | `Collision_RaycastWorld` | 84 | Pulse `0x08816ac0` line for line |
| `0x08825aa4` | `Collision_RaycastBox` | 84 | the table above |
| `0x0885e348` | `Plasma_Update` | 84 | `+0xf0` velocity, `+0x110` carried normal, probe `12.0`, ride `6.0`, fall `50.0`, the `+0x124` ramp `min(0, x + 3 dt)` |
| `0x08822e28` | `Collision_AddBoxCollider` | 82 | sole caller of `0x08827038`; called from `0x08934b74`'s box arm as Pulse's |
| `0x08827038` | `Collider_SetBoxDimensions` | 84 | three stores to `+0x1d0..+0x1d8`, Pulse `0x08818954` byte for byte |

`Rocket_Update` (`0x0885f28c`) and `Missile_Update` (`0x0885cf30`) are named on their own pages.

## Cross-title

Pulse: **applies**, measured live (`rocket-visuals.md`). Pure: applies with Pulse's law and box,
**differs** in what the box hit does (this page). HD, 2048, Omega: their own pages,
[ps3-hdfury-eu/own-craft-box.md](../ps3-hdfury-eu/own-craft-box.md),
[vita-2048-eu-v104/own-craft-box.md](../vita-2048-eu-v104/own-craft-box.md),
[ps4-omega-eu/own-craft-box.md](../ps4-omega-eu/own-craft-box.md).
