# The Plasma, per tick: `PlasmaManager_Update` and what it calls

2026-09-16. Functions in `EBOOT.elf` (WipEout HD / Fury, PS3, `BCES-00664`,
EU). Picks up where [`weapons.md`](weapons.md) left `PlasmaManager_Construct`
(`0x001457b8`) and the `Plasma.cpp` constructor pair: the manager's vtable
(`PTR_DAT_008aadd4` -> `0x00864698`, slot 3 -> OPD `0x00877360` ->
`0x00146e68`) is the per-tick walker, and the walker holds the state
machine. This page was read **second**, against
[`ps4-omega-eu/plasma.md`](../ps4-omega-eu/plasma.md), whose x86 decompile
is the cleaner of the two; every number below is the PPC's own, resolved
through this function family's TOC (`0x008ad4d8`, `exact` per
`scripts/ps3-toc.py`, so the `DAT_` addresses Ghidra shows are the right
ones here - see [memory.md](memory.md) for why that has to be said).

**The names here are applied**, from [names.tsv](names.tsv).

| Address | Name | Confidence | What it is |
| --- | --- | ---: | --- |
| `0x00146e68` | `PlasmaManager_Update` | 88 | two passes over the 16-slot pool at `+0x84`, live count `+0xc4` |
| `0x0011fc88` | `Plasma_Update` | 85 | `(dt, plasma)`: the floor follower, a separate function here |
| `0x00120fe8` | `Plasma_Init` | 88 | arms the held bolt, 1.0 s charge, `WO_PLASMA_HEAD`/`_CHARGING` |
| `0x0011f810` | `Plasma_Launch` | 82 | `(plasma, node_matrix, craft_velocity)`; called at charge expiry |
| `0x00146838` | `Plasma_CheckShipHit` | 85 | `(manager, slot)`: the +-6.0 corridor test, damage, blast |
| `0x00121878` | `Plasma_PostUpdate` | 80 | the 10.0 s reap, the charge-glow ease, the visual |
| `0x0011f6c0` | `Plasma_Detonate` | 85 | `(plasma, &position)`: releases the psys, starts the explosion per viewport |
| `0x00127cd0` | `WeaponExplosions_Start` | 88 | `WO_PLASMA_LIGHTNING_EXPAND`, track fit, shows the models |
| `0x00127948` | `WeaponExplosions_Update` | 88 | vtable slot 3 of `0x00863ef8`: `age += dt`, done at 3.5 s |
| `0x00127770` | `WeaponExplosions_Collapse` | 85 | once at 1.3 s: `WO_PLASMA_LIGHTNING_COLLAPSE`, hides the models |
| `0x00126dc0` | `WeaponExplosions_Reset` | 88 | zeroes age, seeds the three ramps |
| `0x001270b8` | `WeaponExplosions_Draw` | 85 | advances the ramps, scales the three models, `UV_offset` |
| `0x0002ca00` | `WeaponStats_ParsePlasma` | 90 | the `Plasma` `<Stats>` reader - the same name `psp-pulse-usa`'s `0x0880cc2c` carries |

Unnamed: `0x00126d80` (a static initialiser that only writes the second and
third model lifetimes, `1.3`, into `0x008c18ac`/`0x008c18b0` - confidence
below 70 for a name), `0x00146470` (the blast-radius impulse loop, called
from the ship hit, not decompiled), `0x00145ef8` (the mine/bomb clearance
pass, not decompiled), `0x0011eba0`/`0x0011ebb0` (the pass-two "may I
recycle" query and the teardown, not decompiled).

## `PlasmaManager_Update` (`0x00146e68`)

Confidence **88**. `dt` arrives in `f1`. Slot layout differs from the PS4
but the shape is the same: flags `+0x40` (`8` live, `1` armed, `2` net-held,
`4` destroyed), charging `+0x50`, charge `+0x54`, age `+0x58`, destroy
pending `+0x12c`.

```c
for each live slot p:
    if (p->charging == 0) {
        Plasma_Update(dt, p);                              // 0x0011fc88
        if (p->flags & 1) {
            Plasma_CheckShipHit(mgr, i);                   // 0x00146838
            FUN_00145ef8(mgr, i);                          // mines/bombs in the path
        }
    } else {
        p->charge -= dt;
        if (p->charge <= 0.0f && !(p->flags & 2)) {        // 0.0 at TOC-0x26b8 = 0x008aae20
            node = p->craft->+0x5f34; orthonormalise node->matrix (+0x38);
            Plasma_Launch(p, matrix, ship_entry + 0xe0);   // 0x0011f810
            if (g_GameState.mode >= 0x10) net message 0x40 (PlasmaNetFire), 0x50 bytes
        }
    }
    Plasma_PostUpdate(p);                                  // 0x00121878
    if (p->destroy) Plasma_Detonate(p, &p->position);      // 0x0011f6c0, + 0xa0
pass two: for each slot with flags & 4, if FUN_0011eba0(p): swap-remove, FUN_0011ebb0(p), flags = 0
```

`Plasma_PostUpdate` (`0x00121878`) holds the reap:

```c
if (10.0f <= p->age || (p->flags & 4)) p->destroy = 1;    // DAT_008aa010 = 0x41200000
else {
    if (p->charging) p->glow += (p->glow_target - p->glow) * p->glow_rate;   // + 0x190 <- (+0x194, +0x198)
    ...visual placement (0x00121418) unless the craft is hidden
}
```

**Hardcoded 10.0 s, no authored `timetodie`** - and the Plasma `<Stats>`
block on this binary has no such attribute to read (below). Same as Pulse,
same as Omega.

## `Plasma_Init` (`0x00120fe8`): the wind-up is 1.0 s, hardcoded

Confidence **88**. `*(param_1 + 0x54) = DAT_008a9f44._4_4_` - that is the
word at `0x008a9f48` = `0x3f800000` = **1.0** - with `+0x50 = 1`
(charging), `+0x58 = 0` (age), `+0x40 |= 8`, `+0x110 = -(craft + 0x7850)`
(the carried normal starts as minus the craft's up), and `+0x124 =
*(0x008c1868 + 4)` = `0xbf99999a` = **-1.2**, the same seed Omega's
`Plasma_Init` writes as a literal. Two particle systems are attached at
`p + 0xb0`: `'PLHE'` (`WO_PLASMA_HEAD`) and `'PLCG'` (`WO_PLASMA_CHARGING`),
both owned by the race-wide `RaceManager` instance as
[weapons.md](weapons.md) already noted for the explosions. Rumble
`FUN_000a3528(0.07, 1.0, ...)` (`0x008aa000 = 0x3d8f5c29`, `0x008a9f48`)
if the craft is local, and two light descriptors are registered from
`p + 0x1a0` (`FUN_00677c78`, descriptors at `*0x008a9f40 + 4` and `+ 0x74`).

**`charge_time` is authored and unread, third binary.**
`WeaponStats_ParsePlasma` (`0x0002ca00`) stores it at stats `+0xbc`. A
program-wide `search_instructions` for `lfs` with operand `0xbc(r` returns
68 hits and **none in any function attributed to `Plasma.cpp` or
`PlasmaManager.cpp`** (`0x0011eba0`..`0x00121878`, `0x00145ef8`..
`0x00147020`); the same search scoped to `Plasma_CheckShipHit` for
`0xc0(r` finds the `damage` read at `0x00146d6c`, which is the
calibration. Pulse's authored `3` and this binary's own value are both
three times a wind-up that is compiled in.

## `Plasma_Update` (`0x0011fc88`): the floor follower

Confidence **85**. The PPC decompile is a wall of Altivec permutes, so the
constants were read out of the TOC rather than the pseudocode:

| Role | Where | Value |
| --- | --- | ---: |
| probe length along the carried normal | `0x008a9fc0` | **6.0** (Omega 6.0, Pulse 12.0) |
| fall, world Y, per second | `0x008a9fe4` | **-50.0** |
| ride height above the hit point | `0x008a9fd8` | **4.0** (Omega 4.0) |
| km/h -> m/s | `0x008a9fdc` | **`0x3e8e2eb2` = 0.2777**, the same odd literal as Omega's walker |
| launch-ramp window | `0x008a9f48` | 1.0 s |
| `+0x124` ramp: `min(0, x + dt * rate)` | `0x008a9fe8`, `0x008a9f20` | rate 3.0, ceiling 0.0 |

Control flow, read off the branch structure:

```c
p->age += dt;  p->prev = p->position;  next = p->position + p->velocity * dt;
kind = Collide(track, next, next - p->surface * 6.0f, &out);      // 0x0007be58
if (kind < 6 && kind in {0, 4, 5})  p->destroy = 1;                // a wall
else if (kind == 0x7f)               fall: velocity.y -= dt * 50.0f     // open air (Pulse's code, Pulse's role)
else                                 ride: surface = out.normal; next = hit + normal * 4.0f; renormalise
speed = class speed (g_GameState + 0xd4 selects stats +0xcc..+0xd8 - FOUR classes, no SuperPhantom)
if (p->age < 1.0f) speed = (1.0f - p->age) * p->launch_kmh + p->age * speed;   // + 0x4c
p->velocity = dir * (speed * 0.2777f);
kind = Collide(track, p->prev, next, &out);                        // the travel segment
if (kind < 6 && ((1 << kind) & 0x31))  p->destroy = 1;             // kinds 0, 4, 5 again
if (p->destroy) Event_Push(0x33, next);                            // FUN_00677048
p->ramp = min(0.0f, p->ramp + dt * 3.0f);                          // + 0x124
```

The wall/open-air enumeration is **Pulse's** (`0`/`4` wall, `0x7f` none -
[psp-pulse-usa/plasma.md](../psp-pulse-usa/plasma.md)) with `5` added;
Omega renumbered it (`4, 5, 6, 8` wall, `0xe` none). The gravity axis is
world-vertical on both HD ports and along the carried normal on the PSP.

`Plasma_Launch` (`0x0011f810`, confidence 82 - read from its constant loads
and its call site rather than a full decompile): `lfs` from the TOC pulls
`3.6` (`0x008a9fb8`), `1.0`, `0.0`, the launch light's `24.0` radius and
`(2.2, 1.6)` colour (`0x008a9fbc`, `0x008a9fac`, `0x008a9fb0`) and the
`0.06`/`0.3` rumble pair (`0x008a9fa4`, `0x008a9f80`) - the same five
things Omega's `Plasma_Launch` does with the same numbers.

## `Plasma_CheckShipHit` (`0x00146838`): the corridor and the damage

Confidence **85**. For every craft except the firer (`p + 0x44`), the craft
position is projected onto the segment `prev -> position`; a hit needs the
projection inside the segment and the perpendicular distance strictly
between `0x008aae24 = -6.0` and `0x008aae28 = 6.0`. Then:

```c
p->destroy = 1;
stop the travel emitter; Sound_Play(1.0f, emitter, ..., "PLASMAHITSHIP");   // 0x007852e8 - Pulse's cue name, not Omega's NGP_
if (victim is remote) Event_Push(0x33, hit);
online-stat bump for modes 0x10/0x11/0x14 (a counter at +0x8c of 0x0004d7e0's object)
if (g_GameState.mode < 0x10) {                                     // local play
    if (firer == local player) victim->hit_by_local = 1;           // + 0x124
    victim->last_attacker = firer;  victim->last_weapon = 2;       // + 0x13c, + 0x138
    victim->hits         += 1.0f;                                  // + 0x12c, 0x008aad8c
    victim->damage_taken += stats->damage;                         // + 0x120 += stats + 0xc0
    victim->slowdown     += stats->slowdown_time;                  // + 0x130 += stats + 0xe4
    FUN_00146470(mgr, hit, firer);                                 // the blast-radius impulse, unread
}
else net message 0x18 to the victim's owner
```

**No `weapon_damage_multiplier` here.** Omega multiplies `stats->damage` by
the firer's handling `+0x4c4`; this binary adds `stats->damage` raw. Whether
HD's handling XML has the attribute at all was not checked - the PS4 code
is the only place it was seen read.

`WeaponStats_ParsePlasma`'s block, in `strcasecmp` order, each name
resolved through the function's TOC:

| Attribute | stats offset |
| --- | ---: |
| `charge_time` | `+0xbc` (unread) |
| `damage` | `+0xc0` |
| `blastradius` | `+0xc4` |
| `blastforce` | `+0xc8` |
| `slowdown_time` | `+0xe4` |
| `venomspeed` / `flashspeed` / `rapierspeed` / `phantomspeed` | `+0xcc` / `+0xd0` / `+0xd4` / `+0xd8` |
| `launchspeed` | `+0xdc` |
| `absorb` | `+0xe0` |

Eleven attributes against Omega's fourteen: no `superphantomspeed`, no
`screen_shake_amount`/`_time`. The four-way class switch in `Plasma_Update`
matches.

## `Plasma_Detonate` (`0x0011f6c0`) and the explosion

Confidence **85**. Releases the head and charging psys (`+0x5c`, `+0x60`),
sets flags `(& ~8) | 4`, calls the teardown `FUN_0011ebb0`, then for each
viewport (`g_GameState + 0xe4` of them) the first that passes
`FUN_002d64d0(-1, pos, pos, v)` gets `WeaponExplosions_Start(p->explosion,
pos)` (`+0x140`) and the loop stops. A bolt no viewport can see never
starts its explosion, and pass two recycles the slot as soon as
`FUN_0011eba0` says so.

`WeaponExplosions_Start` (`0x00127cd0`): `+0x191 = 1` (active), spawns
`'PLED'` (`WO_PLASMA_LIGHTNING_EXPAND`) at the position, fits a basis to the
track under it (`FUN_000a97f0`) into `+0x90..+0xc0` and per viewport into
`+0xd0 + v * 0x40`, shows the three models (`+0x54`, `+0x58`, `+0x5c`:
`HD_plasma_ring`, `HD_plasma_sphere`, `HD_plasma_halo` in that order - the
`.vex` names resolved from `WeaponExplosions_Construct`'s TOC slots
`-0x31ec`, `-0x31e8`, `-0x31e4`), calls `WeaponExplosions_Collapse` once
(a no-op at age 0), registers **two** light descriptors (`+0x30` and
`+0xa0` of the block `WeaponExplosions_Construct` fills at `*0x008aa260`),
and sets `+0x34 |= 6`.

`HD_plasma_ball` is not among them: on this binary as on the PS4 it is the
bolt's own head, not part of the explosion.

What animates the three models:

| | ring | sphere | halo |
| --- | ---: | ---: | ---: |
| ramp `(cur, target, rate)` at | `+0x60` | `+0x6c` | `+0x78` |
| seeded from | `*0x00994070 + 0`, `0x008c18b4`, `0x008aa264` | `+4`, `0x008c18b8`, `0x008aa268` | `+8`, `0x008c18bc`, `0x008aa26c` |
| target | **100.0** | **7.1** | **7.0** |
| rate | 0.01 | 0.3 | 0.2 |
| advanced while `age <=` | 1.7 s (`0x008c18a8`) | 1.3 s (`0x008c18ac`, written by `0x00126d80`) | 1.3 s (`0x008c18b0`, same) |
| basis | per-viewport | shared (`+0x90`) | per-viewport |

`cur += (target - cur) * rate` per `WeaponExplosions_Draw` call, the model's
basis scaled by `cur`, then `FUN_002c1b30(age, model)` - the `UV_offset`
constant `WeaponExplosions_Construct` looks up by CRC on each model.
`WeaponExplosions_Update` adds `dt` to `+0x50` and retires the object when
`age >= 0x008aa2b4 = 0x40600000 = 3.5 s`; `WeaponExplosions_Collapse` fires
once at `0x008aa25c = 1.3 s`, spawning `'PLCE'` and hiding all three.

**One hedge on the second and third windows.** On disk `0x008c18ac` and
`0x008c18b0` hold `0.0`; the `1.3` is written at runtime by `0x00126d80`,
and that function is gated (`if (param_1 != 1) return; if (param_2 !=
0xffff) return;`) on two arguments whose meaning was not read. If the gate
never fires, the sphere and halo never scale at all. The reading stands on
corroboration rather than on the gate: Omega's `0x01372600` is a plain
`void (void)` static initialiser that writes the identical `1.7, 1.3, 1.3`
triple unconditionally, and two independently-compiled binaries agreeing on
the triple is what makes `1.3` the right number here.

The rates and lifetimes are Omega's exactly; the targets are not (Omega
50 / 2.0 / 3.0). Given that both titles ship the same three `.vex` names,
the likeliest reading is that the PS4's `rcsmodel` re-exports carry a
different base scale - unverified, and the reason this row is in the
comparison table rather than a claim about the models.

## What is not verified

- `Plasma_Launch`'s body beyond its constants and call site.
- The blast-radius loop `FUN_00146470` (Omega's, read in full, applies a
  `blastradius`/`blastforce` falloff impulse to every craft; this one was
  not opened).
- What `FUN_0011eba0` checks before a slot is recycled (Omega: the
  explosion's `active` byte).
- Whether the HD handling XML carries `weapon_damage_multiplier` at all.
- What `0x00126d80`'s `(1, 0xffff)` gate means, i.e. when the two `1.3 s`
  windows actually get written (see the hedge above).
