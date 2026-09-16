# The Plasma, per tick: `PlasmaManager_Update` and what it drives

2026-09-16. Functions in `eboot.bin` (WipEout: Omega Collection, PS4,
`CUSA05670`, EU), `x86:LE:64:default`, image base `0x01000000`. Picks up
where [`weapons.md`](weapons.md) left `PlasmaManager_Construct`
(`0x01361b00`): the vtable that constructor installs (`PTR_FUN_019168f8`,
slot 3) is the manager's per-tick walker, and - exactly as
[`psp-pulse-usa/plasma.md`](../psp-pulse-usa/plasma.md) found for Pulse -
**the walker is where the Plasma's whole state machine lives.** The item's
own vtable (`PTR_FUN_01916890`, slots `0x0135bcf0`/`0x0135c610`/
`0x0135c630`/`0x0135c780`) was not needed to answer any question below and
is unread.

The cross-check is [`ps3-hdfury-eu/plasma.md`](../ps3-hdfury-eu/plasma.md),
read in the same pass: the same structure, the same constants where they
matter, and one enum that got renumbered between the two ports.

**The names here are applied**, from [names.tsv](names.tsv).

| Address | Name | Confidence | What it is |
| --- | --- | ---: | --- |
| `0x0135e140` | `PlasmaManager_Update` | 90 | vtable slot 3 of `PTR_FUN_019168f8`; two passes over the 16-slot pool at manager `+0xe8`, live count at `+0x168` |
| `0x0135cad0` | `Plasma_Init` | 90 | `(plasma, craft)`: arms the bolt held on the nose, sets the 1.0 s charge |
| `0x0135d5d0` | `Plasma_Launch` | 90 | `(plasma, node_matrix, craft_velocity)`: releases it along the node's forward |
| `0x0135bd70` | `Plasma_UpdateVisual` | 82 | called directly by the walker each tick: places `HD_plasma_ball`, pushes the travel light |
| `0x0135dff0` | `Plasma_Teardown` | 85 | stops the travel emitter, plays `NGP_PlasmaHitWall`, resets the charge-glow ramp |
| `0x01360970` | `PlasmaManager_Destruct` | 85 | vtable slot 9; unhooks the per-system track objects the constructor resolved |
| `0x01371990` | `WeaponExplosions_Update` | 88 | vtable slot 3 of `PTR_FUN_01916e68`: `age += dt`, done at 3.5 s |
| `0x013719e0` | `WeaponExplosions_Collapse` | 85 | at 1.3 s, once: spawns `WO_PLASMA_LIGHTNING_COLLAPSE`, hides the three models |
| `0x01371e20` | `WeaponExplosions_Reset` | 88 | zeroes age, re-seeds the three scale ramps, hides the models |
| `0x01371f10` | `WeaponExplosions_Draw` | 85 | vtable slot 5: advances the ramps, scales the models, emits the blast light |
| `0x01656130` | `WeaponStats_ParseXml` | 90 | the whole `<WeaponStats>` parser; the `Plasma` block maps to `+0x114..+0x148` |

Left unnamed, deliberately (below 70, or not decompiled):

- `0x01372600`, a static initialiser that writes the explosion's three model
  lifetimes (`_DAT_01a15bf4 = 1.7, 1.3`, `_DAT_01a15bfc = 1.3`) among ~40
  CRC-looking words whose meaning was not chased.
- `0x01606280` / `0x01605f50`, two entry points into a 24-slot pool at
  `0x01f8fae0` that takes a descriptor pointer and a position (by pointer or
  by copy), stores `desc+0x2c` as a countdown and `1/max(0.01, desc+0x3c)`,
  `1/max(0.01, desc+0x40)` as rates. Reads as a timed light or post-effect
  registry; what consumes it was not found.
- `0x0174a4f0`, a 20-slot event ring on `DAT_020fd940` that the walker pushes
  id `0x33` into with the impact position on every wall hit and on a ship hit
  when the victim is a remote craft. Probably the impact rumble/camera event;
  not chased.
- `0x01257f00`, the collision query `(world, from, to, &hit)`; only its
  return codes were read, from how the walker branches on them.

## `PlasmaManager_Update` (`0x0135e140`), pass one

Confidence **90**. `dt` arrives in `xmm0`. For each live slot `p` (flags
`+0x74 & 8`):

```c
if (p->charging == 0) {                       // + 0x84, a byte
    Plasma_Update(p)                          // inlined - the whole floor follower, below
    if (p->flags & 1) {                       // + 0x74 - set by Plasma_Launch
        ship hit test over every craft, below
        mine/bomb clearance in the bolt's path, below
    }
} else {
    p->charge -= dt;                          // + 0x88
    if (p->charge <= 0.0f && !(p->flags & 2)) {
        node = p->craft->weapon_node;         // + 0xa8 -> + 0x6470, matrix at + 0x68
        Plasma_Launch(p, node->matrix, ship_entry + 0xe0);   // 0x0135d5d0
        net: message 0x400e (PlasmaNetFire) if in a networked mode
    }
}
if (10.0f <= p->age || (p->flags & 4)) p->destroy = 1;      // + 0x8c, + 0x16c - the hard reap
else {
    if (p->charging) p->glow += (p->glow_target - p->glow) * p->glow_rate;  // + 0x1e0 <- (0.8, 0.05)
    Plasma_UpdateVisual(p);                   // 0x0135bd70
}
if (p->destroy) { the destroy path, below }
```

**The wind-up is 1.0 s and hardcoded**, on the same evidence shape as Pulse:
`Plasma_Init` writes `*(param_1 + 0x88) = 0x3f800000` and `*(param_1 + 0x84)
= 1`, the walker counts `+0x88` down and launches at zero. The authored
`charge_time` exists in this binary's XML too (`WeaponStats_ParseXml` stores
it at `param_1[0x45]`, stats `+0x114`) and **nothing on the Plasma path
reads it**: `search_instructions` for an operand of `0x114]` across
`FUN_0135e140`, `FUN_0135cad0`, `FUN_0135d5d0` and `FUN_0135bd70` returns
zero matches, while the same search for `0x118]` (damage, `param_1[0x46]`)
finds the walker's read at `0x0135f44d` - the calibration Pulse's own sweep
used. Same authored-but-dead attribute, third binary.

**A bolt is reaped at a hardcoded 10.0 s** (`10.0 <= *(lVar20 + 0x8c)`),
there is no `timetodie` in the Plasma `<Stats>` block, and the reap takes
the identical destroy path as a wall hit - so a bolt that expires still
detonates visually and plays `NGP_PlasmaHitWall`, but **applies no damage
and no impulse**: both of those live inside the ship-hit block and nowhere
else.

## The flight step, inlined

Confidence **88** (decompiled cleanly; the one lane-level claim was
confirmed in the disassembly). Entity layout: position `+0xe0`, previous
`+0x140`, velocity `+0x130`, carried surface normal `+0x150`, basis rows
`+0xb0/+0xc0/+0xd0`, age `+0x8c`, launch speed `+0x80`.

```c
p->age += dt;
p->prev = p->position;
next    = p->position + p->velocity * dt;
probe   = next - p->surface * 6.0f;                       // the probe, 6.0 not Pulse's 12.0
hit     = Collide(g_world, next, probe, &out);            // 0x01257f00
if (!hit || out.kind == 0xe) {                            // open air
    p->velocity.y -= dt * 50.0f;                          // world Y: vmovshdup/vsubss/vinsertps 0x10 at 0x0135e9ca..d7
    rebuild the basis from the new velocity
} else if (out.kind in {4, 5, 6, 8}) {                    // a wall
    p->destroy = 1;                                       // + 0x16c
} else {                                                  // a floor: ride it
    p->surface  = out.normal;                             // + 0x150
    next        = out.point + out.normal * 4.0f;          // the ride height
    p->velocity = normalize(next - p->prev) * ...
    p->velocity = normalize(p->velocity) * (Plasma_GetSpeed(p) * 0.2777f);
}
hit = Collide(g_world, p->prev, next, &out);              // the travel segment
if (hit && out.kind in {4, 5, 6, 8}) p->destroy = 1;      // mask 0x170 over kinds 0..8
if (p->destroy) Event_Push(0x33, next);                   // 0x0174a4f0
p->ramp = min(0.0f, p->ramp + dt * 3.0f);                 // + 0x164, seeded -1.2 by Plasma_Init
```

Two things are worth the reader's attention:

- **The fall is world-vertical here, not along the carried normal.** Pulse's
  `Plasma_Update` subtracts `surface * (dt * 50.0)`; this binary extracts
  lane 1 of the velocity, subtracts `dt * 50.0` and inserts it back
  (`vmovshdup xmm1, xmm0` / `vsubss xmm3, xmm1, [rsp+0x158]` / `vinsertps
  xmm4, xmm0, xmm3, 0x10` at `0x0135e9ca`-`0x0135e9d7`, the `50.0` coming
  from `[0x01802570]` at `0x0135e180`). Same magnitude, different axis.
  `ps3-hdfury-eu` does the same (lane `0xd4` of a stack vector, `-50.0` at
  `0x008a9fe4`), so this is the HD engine's choice, not a port artefact.
- **The km/h factor is `0.2777`, not `1/3.6`.** The walker multiplies by
  `[0x0180257c] = 0x3e8e2eb2 = 0.27770001`; `Plasma_Launch` multiplies by
  `0.2777778`. HD carries the identical `0x3e8e2eb2` at `0x008a9fdc`, so it
  is a literal typed into `Plasma.cpp`'s update once and inherited, worth
  0.028 % of the class speed and nothing more.

`Plasma_GetSpeed` is inlined (its error string
`"Error(Plasma::GetSpeed) Invalid classSpeed"` names it): a `switch` on the
speed class `DAT_01f999d8` over **five** entries reading stats `+0x124`
(`venomspeed`) .. `+0x134` (`superphantomspeed`), and then the launch
ramp:

```c
if (p->age < 1.0f) speed = (1.0f - p->age) * p->launch_kmh + p->age * class_kmh;
```

- exactly Pulse's `Plasma_SpeedForClass`, blending the craft's own launch
speed into the class speed over the first second of flight.

## `Plasma_Init` (`0x0135cad0`) and `Plasma_Launch` (`0x0135d5d0`)

Confidence **90** each. `Plasma_Init(plasma, craft)`:

```c
p->flags     |= 8;  p->age = 0;  p->ramp_a = 0;           // + 0x74, + 0x8c, + 0x160
p->charging   = 1;  p->charge = 1.0f;                     // + 0x84, + 0x88 = 0x3f800000
p->emitter    = Sound_CreateEmitter("Plasma", ...);       // + 0x98, parented to the weapon node
Sound_Play(1.0f, p->emitter, ..., "NGP_Plasma", 0, 0xb);
p->surface    = -craft->up;                               // + 0x150 <- -(craft + 0x8040)
p->position   = weapon_node->matrix.row3;                 // + 0xe0
p->ramp       = -1.2f;                                    // + 0x164 = 0xbf99999a
Psys_Spawn("WO_PLASMA_HEAD"    -> 'PLHE', attached to p + 0xf0);   p->head_id  = + 0x90
Psys_Spawn("WO_PLASMA_CHARGING" -> 'PLCG', attached to p + 0xf0);   p->charge_id = + 0x94
p->ball_matrix = weapon_node->matrix, orthonormalised;    // + 0x1a0.. + 0x1d0
Rumble(0.07f, 1.0f, ...) if the craft is local;           // 0x0137eba0
Light_Register(p + 0x1f0, &DAT_01a148f0); Light_Register(p + 0x1f0, &DAT_01a14950);
```

`Plasma_Launch(plasma, node_matrix, craft_velocity)`:

```c
p->launch_kmh = length(craft_velocity) * 3.6f + stats->launchspeed;   // + 0x80, stats + 0x138
p->position   = node_matrix.row3;
p->charging   = 0;
p->velocity   = node_matrix.forward * (Plasma_GetSpeed(p) * 0.2777778f);   // age is 0, so launch_kmh
p->surface    = -craft->up;
p->flags     |= 8;
Sound_Play(1.0f, p->emitter, ..., "_NGP_Plasmatvl", p + 0xa0, -1);   // the loop the teardown stops
Rumble(0.06f, 0.3f, ...) if local;
Psys_Spawn("WO_PLASMA_LAUNCH" -> 'PLLH', at the weapon node);
Light_PushFrame(p->position, 1/24, 24.0f, colour (2.2, 1.6, 2.2));      // the 32-slot list at 0x01fc6e20
```

The node matrix is re-read at release, as in Pulse: a craft that turns
during the wind-up fires down its new heading. The `flags & 1` bit the
walker gates the ship-hit test on is not set by either function as
decompiled; it is set somewhere between `Init` and the first flying tick
(most likely by the fire handler that calls `Init`, unread), which is why
this page does not claim a held bolt cannot hit a craft.

## Ship hit, damage, and where every number comes from

Confidence **88**. Inside the flying branch, for every craft `c` that is
not the firer (`p + 0x78`) and is alive:

```c
d = c->position - p->prev;  t = dot(d, dir) in [-1, 1];  side = perpendicular distance
if (-6.0f < side && side < 6.0f) {
    p->destroy = 1;
    release the travel emitter; Sound_Play(..., "NGP_PlasmaHitShip", 0, 0xc);
    if (c is remote) Event_Push(0x33, hit);
    if (not a spectator/replay case) {
        if (firer == local player) c->hit_by_local = 1;                 // + 0x100
        c->damage_taken += stats->damage * firer_handling->weapon_damage_multiplier;   // + 0x104 += + 0x118 * + 0x4c4
        c->hits         += 1.0f;                                        // + 0x10c
        c->slowdown     += stats->slowdown_time;                        // + 0x130 += + 0x140
        c->last_weapon   = 2;  c->last_attacker = firer;                // + 0x138, + 0x13c
        for every craft q: if (|q - hit| < stats->blastradius)          // + 0x11c
            q->impulse += falloff * stats->blastforce * dir;            // + 0x120 into q + 0x120
    }
    net: message 0x140e to the victim's owner
}
```

Every stats offset above is where `WeaponStats_ParseXml` put it. The
`Plasma` block of that parser (its `strcasecmp` chain, in order):

| Attribute | `param_1[i]` | stats offset | Read by |
| --- | ---: | ---: | --- |
| `charge_time` | `0x45` | `+0x114` | **nothing** (the calibrated negative above) |
| `damage` | `0x46` | `+0x118` | the ship hit, `0x0135f44d` |
| `blastradius` | `0x47` | `+0x11c` | the blast loop |
| `blastforce` | `0x48` | `+0x120` | the blast loop |
| `venomspeed` .. `superphantomspeed` | `0x49`..`0x4d` | `+0x124`..`+0x134` | `Plasma_GetSpeed`, both copies |
| `launchspeed` | `0x4e` | `+0x138` | `Plasma_Launch` |
| `absorb` | `0x4f` | `+0x13c` | not on this page's path |
| `slowdown_time` | `0x50` | `+0x140` | the ship hit |
| `screen_shake_amount` / `_time` | `0x51`/`0x52` | `+0x144`/`+0x148` | not on this page's path |

`weapon_damage_multiplier` is the **firer's** handling value: the ship-table
parser (`FUN_012d8e30`, the `<Handling>` reader with the `Weapons` block)
stores it at `param_1[0x131]` = `+0x4c4`, and the hit code reaches it
through `ship_entry[firer] + 0xf0 -> + 0x6518 -> + 0xc8 -> + 0x4c4`. The same
`<WeaponSet>` block is where a ship's `plasma="true"` bit (`+0x4a8 | 0x80`)
comes from - a by-catch, recorded here because the parser was already open.

**Mines and bombs in the bolt's path** are cleared too: after the ship
loop, every live entry of the manager at `param_1 + 0x170` within
`stats+0x19c` (that block's `trigger_radius`) of the bolt is sent a `0x50e`
net event and destroyed, then the same over a second pool. Which of the
two is the Mine and which the Bomb was not pinned.

## The destroy path and the explosion

Confidence **88**. When `p->destroy` is set at the end of a tick:

```c
sprintf(&DAT_01a15b90, "(%i/%i)", firer, slot);           // a debug label, never read here
release the WO_PLASMA_HEAD and WO_PLASMA_CHARGING instances (+0x90, +0x94)
p->flags = (p->flags & ~0xc) | 4;   hide the ball (+0x60 &= ~4)
Plasma_Teardown(p);                                       // 0x0135dff0: NGP_PlasmaHitWall, ramp reset
for each viewport v: if (Visible(-1, p->position, p->position, v)) {   // 0x0171e0c0
    x = p->explosion;                                     // + 0x188, this slot's own WeaponExplosions
    x->active = 1;                                        // + 0x1e1
    x->position = p->position;                            // + 0x80
    Psys_Spawn("WO_PLASMA_LIGHTNING_EXPAND" -> 'PLED', at the position, HD asset root only);
    fit a basis to the track under the bolt (0x01380140 / 0x01381300), write it to x + 0xe0..0x100
        and to every viewport's copy at x + 0x120 + v * 0x40;
    show the three model nodes (+0x98, +0xa0, +0xa8 flag |= 4);
    WeaponExplosions_Collapse(x);                         // no-op at age 0
    Light_Register(x + 0x80, &DAT_01a15d60);              // 0x01605f50
    x->flags |= 6;
    break;
}
```

The craft-hit block nulls the emitter (`*(lVar13 + 0x98) = 0`) after
playing `NGP_PlasmaHitShip`, and `Plasma_Teardown` only plays
`NGP_PlasmaHitWall` when the emitter is still there - that is the mechanism
by which one destroy path serves both endings without the wall cue
double-firing on a craft kill.

Pass two of the walker then swap-removes any slot whose flag `4` is set
**and** whose explosion reports `+0x1e1 == 0` - so a slot stays out of the
pool until its explosion has run, and a bolt nobody could see (no viewport
passed the test) is recycled the same tick.

The explosion is the `WeaponExplosions.cpp`-tagged `0x1f0`-byte object
`PlasmaManager_Construct` builds inline per slot, and this corrects one
detail of [`weapons.md`](weapons.md): **`HD_plasma_ball` is the bolt's own
head model**, loaded onto the `Plasma.cpp` item (`+0x190`); the explosion
loads only `HD_plasma_ring` (`+0x98`), `HD_plasma_sphere` (`+0xa0`) and
`HD_plasma_halo` (`+0xa8`). Three models, three ramps - Pulse's
`PlasmaBlast_Construct` shape exactly.

What animates them, from `WeaponExplosions_Reset`, `_Update`, `_Collapse`
and `_Draw`:

| | ring | sphere | halo |
| --- | ---: | ---: | ---: |
| ramp `(cur, target, rate)` at | `+0xb0` | `+0xbc` | `+0xc8` |
| seeded | `(0, 50.0, 0.01)` | `(0, 2.0, 0.3)` | `(0, 3.0, 0.2)` |
| advanced while `age <=` | 1.7 s | 1.3 s | 1.3 s |
| basis | per-viewport (`+0x120`) | shared (`+0xe0`) | per-viewport |

Per draw call, not per second: `cur += (target - cur) * rate`, then the
model's matrix is its basis scaled by `cur`, and `0x0170e860(age, node)`
pushes the age into the model (HD's copy names the parameter: a `UV_offset`
shader constant). The rates are applied once per `_Draw`, so the ease is
frame-rate dependent. `_Update` adds `dt` to `+0x90` and retires the object
at **3.5 s**; `_Collapse` fires once at **1.3 s**, spawning
`WO_PLASMA_LIGHTNING_COLLAPSE` (`'PLCE'`) and hiding all three models -
after which only the ring's ramp is still nominally running, on a hidden
model. `_Draw` also pushes a light every frame while `age <= 1.3`:
intensity `(1 - (age / 1.3)^2) * 100`, colour `(14, 10, 14)` under the HD
asset root or `(2, 6, 20)` under 2048's, radius term `0.01 / f`.

The three lifetimes are written by the static initialiser `0x01372600`
(`_DAT_01a15bf4 = 0x3fa666663fd9999a`, `_DAT_01a15bfc = 0x3fa66666`), the
ramp seeds by `WeaponExplosions_Reset` (`0x42480000`, `0x3c23d70a`,
`0x40000000`, `0x3e99999a`, `0x40400000`, `0x3e4ccccd`).

## Lights, for the record

Three per-frame light pushes into the 32-slot list at `0x01fc6e20`
(`pos, 1/radius, radius, colour, -1`), all hardcoded:

| When | radius | colour |
| --- | ---: | --- |
| `Plasma_Launch` | 24.0 | (2.2, 1.6, 2.2) |
| every `Plasma_UpdateVisual` | 15.0 | (0.6, 0.3, 0.6) |
| `WeaponExplosions_Draw`, `age <= 1.3` | `100 * (1 - (age/1.3)^2)` | (14, 10, 14) HD / (2, 6, 20) 2048 |

Plus the three descriptor registrations (`0x01a148f0`, `0x01a14950` at
`Init`; `0x01a15d60` at detonation) into the 24-slot pool, whose consumer
is unread. Their raw words are in `PlasmaManager_Construct`, which writes
them fresh on every construction; `0x01a15d60` begins `100.0, ?, 1.0, 2.0,
3.0, 100.0, 63.33, 64.0, 65.33, 66.66, 0.05, 1.0, 0.0, 0.1, 1.0, 1.5, 0.9,
-1.0, -1.0, 25.0, 0.5, -0.05` and `+0x2c = 1.0` is the countdown the pool
reads.

## What is not verified

- The item vtable's four Plasma-specific slots (`0x0135bcf0`, `0x0135c610`,
  `0x0135c630`, `0x0135c780`) and the fire handler that calls `Plasma_Init`
  - so where `flags & 1` is set is inferred, not read.
- The meaning of collision kinds `4, 5, 6, 8` (wall) and `0xe` (open air)
  beyond how the walker branches on them; `ps3-hdfury-eu` uses `0, 4, 5`
  and `0x7f` for the same two roles, Pulse `0, 4` and `0x7f`.
- The 24-slot descriptor pool (`0x01605f50`/`0x01606280`) and the event
  ring (`0x0174a4f0`): shapes read, consumers not.
- Which of the two manager pointers at `param_1 + 0x170` / `+0x178` is the
  Mine manager and which the Bomb.
