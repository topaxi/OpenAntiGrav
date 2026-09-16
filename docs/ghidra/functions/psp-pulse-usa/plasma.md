# The Plasma: one bolt, the Rocket's flight, and a charge nobody spends

**Binary:** `pulse-psp` `BOOT.BIN`, image base `0x08804000`.

**Status:** the weapon is read end to end - request bit, fire handler,
constructor, flight step and `<Stats>` block - at confidence **85 or better**
on every part. It is the cheapest weapon this project has added since the Bomb,
and for the mirror-image reason: the Bomb reused the Mine's whole module and the
Plasma reuses the Rocket's whole flight model.

**Closed 2026-09-09, and both ways at once.** The Plasma **does** wind up
before it fires - one second, held on the firing craft's nose - so the
from-play report was right and this page's earlier "three functions on the
press-to-flight path spend it nowhere" was reading the wrong three functions.
And `charge_time` is **still** spent nowhere: the wind-up is a hardcoded
`1.0f` in `.rodata`, not the `charge_time="3"` the file authors, in Pulse and
in Pure alike. See [the charge](#the-charge-is-real-and-it-is-not-charge_time).
The detonation is closed in the same pass: `WO_PLASMA_FLASH` is it, and the
call site is the pool teardown - see [the detonation](#the-detonation-and-the-three-models-under-it).

| Address | Name | Confidence |
| --- | --- | --- |
| `0x0880cc2c` | `WeaponStats_ParsePlasma` | 92 |
| `0x0886a868` | `Weapon_FirePlasma` | 88 |
| `0x0885bd18` | `Plasma_Init` | 90 |
| `0x0885c6cc` | `Plasma_Update` | 85 |
| `0x0886b490` | `Plasmas_Update` | 90 |
| `0x0885bf84` | `Plasma_Launch` | 90 |
| `0x0885c170` | `Plasma_UpdateCharge` | 88 |
| `0x0885c5a4` | `Plasma_SpeedForClass` | 88 |
| `0x0886ac88` | `Plasma_SpawnDetonation` | 88 |
| `0x0885fd90` | `PlasmaBlast_Construct` | 85 |
| `0x0886a920` | `Plasma_SpawnRemote` | 75 |
| `0x08a7c098` | `g_plasma_charge_seconds` (data) | 90 |

Read [weapon-fire.md](weapon-fire.md) first for the two traps this page depends
on: `entity+0x1b8` is the fire-request word, and every `jal` operand and
`func_0x000NNNNN` the decompiler prints here is **image-base-relative**, so the
real address is that plus `0x08804000`. That second one bit this read - see
[History](#history).

## `Weapons_DispatchFire`'s full sixteen, which settles five weapons at once

`weapon-fire.md` printed five of the dispatched bits and truncated the rest with
a `...`. Here is the whole body of `Weapons_DispatchFire` (`0x08861814`), with
the image base added back to every target and each bit labelled from
[mine.md](mine.md#weapon_requestfires-jump-table-read-as-a-table)'s
thirteen-entry jump table:

| Bit | Handler | Subsystem | Name |
| --- | --- | --- | --- |
| `0x1000` | `0x088613bc` | `world`/`craft` | `Autopilot_Fire` |
| `0x4000` | `0x088577ac` | `world+0x58` | `Weapon_FireCannon` - *not in the jump table*, see below and [cannon-quake-leachbeam.md](cannon-quake-leachbeam.md) |
| `0x400` | `0x088614c4` | `world`/`craft` | Turbo, arming |
| `0x20` | `0x08861568` | `world`/`craft` | `Shield_Fire` |
| `0x100` | `0x08863a20` | `world+0x44` | `Weapon_FireBomb` |
| `0x80` | `0x0886e104` | `world+0x54` | `Weapon_FireRocket` |
| `0x40` | `0x088685cc` | `world+0x4c` | `Weapon_FireMissile` |
| **`0x8`** | **`0x0886c600`** | **`world+0x5c`** | **`Weapon_FireQuake`** |
| **`0x4`** | **`0x0886a868`** | **`world+0x50`** | **`Weapon_FirePlasma`** |
| `0x2` | `0x088675cc` | `world+0x48` | `Weapon_DropMines` |
| **`0x8000`** | **`0x08866658`** | **`world+0x60`** | **`Weapon_FireLeachBeam`** |
| **`0x10000`** | **`0x0886ce8c`** | **`world+0x64`** | **`Weapon_FireRepulser`** |
| **`0x20000`** | **`0x08870240`** | **`world+0x68`** | **`Weapon_FireShuriken`** |
| `0x800` | `0x08861404` | `world`/`craft` | `Autopilot_Update` |
| `0x200` | `0x08861534` | `world`/`craft` | Turbo countdown |
| `0x10` | `0x08861630` | `world`/`craft` | Shield countdown |

**Six of these rows are cross-checks and they all pass**: `0x1000`, `0x4000`,
`0x100`, `0x80`, `0x40` and `0x2` land exactly on the addresses
`weapon-fire.md`, `mine.md`, `shield-pickup.md` and `autopilot.md` already
carry, and `0x20`'s lands on `Shield_Fire` (`0x08861568`), which
`crates/game/src/race/weapons.rs` cites by address. That is what makes the five
bolded rows worth writing down: they come out of the same read, by the same
arithmetic, in the same pass.

**Two negative results, both of which matter to whoever picks this up next:**

- **The Cannon's bit `0x2000` is dispatched by nothing.** Sixteen bits are
  tested and `0x2000` is not among them, which turns
  [pickups.md](../../../gameplay/pickups.md)'s "dispatched by nothing" from a
  claim about five printed bits into one about all sixteen. `0x4000` **is**
  dispatched and is **not** in `Weapon_RequestFire`'s jump table at all, so it
  is set by something else. **Corrected 2026-09-07: the address is
  `0x088577ac`, not `0x088537ac`** - the latter was a `0x4000` arithmetic slip
  in this table (the same magnitude as the bit itself) and decompiles as
  `Ai_Construct`, not a weapon handler at all. `0x088577ac` is confirmed the
  Cannon's own burst spawn, and the whole mechanism - including what actually
  arms bit `0x4000`, since nothing in `Weapon_RequestFire` does - is read on
  [cannon-quake-leachbeam.md](cannon-quake-leachbeam.md).
- ~~**Every other unbuilt weapon does have a handler.**~~ **Read in full,
  2026-09-07**: Quake, LeachBeam and the Cannon are read end to end on
  [cannon-quake-leachbeam.md](cannon-quake-leachbeam.md) - raised to 88/85
  confidence there. The Repulser's handler alone is still unread past the
  reading `shuriken.md`'s own last section already carries (it copies four of
  its own `<Stats>` onto the firing craft before spawning anything), and stays
  deferred as Eliminator-only regardless.

## `Weapon_FirePlasma` (`0x0886a868`) fires exactly one

The whole handler, 44 instructions:

```c
void Weapon_FirePlasma(PlasmaPool *pool, Craft *craft, int craft_index) {
    pool->flags   |= 2;
    craft->held    = -1;         // craft + 0x1bc
    craft->fire   &= ~0x4;       // craft + 0x1b8 - one shot, cleared immediately

    if (pool->live < 0x10) {                       // pool + 0xa4, cap 16
        Plasma *p = pool->slot[pool->live];        // pool + 0x64 + live*4
        p->flags  = 0;
        p->flags  = 1;
        p->owner  = craft_index;                   // + 0x40
        p->id     = ++g_next_projectile_id;        // + 0x44
        Plasma_Init(p, pool->live, world->craft[craft_index]->emitter);
        pool->live += 1;
    }
}
```

**One spawn, no fan, no reload.** It has neither of the two shapes that make a
weapon fire more than once in this executable: not `Weapon_FireRocket`'s three
literal calls to one spawn helper, and not `Weapon_DropMines`' reload timer with
a round counter. Confidence **88** - the same figure `Weapon_FireRocket` carries,
for the same reason (a count read straight off the call structure), capped below
90 because it is unmeasured.

The `<Stats>` agree from the other end: the Plasma authors no `spread`, and
`spread` is what the Rocket's fan is built from. The same absence reads the same
way on the Missile, which also fires one.

**The pool is its own**, cursor at `+0xa4` and a 16-entry pointer array at
`+0x64` running exactly up to it - against the Mine's `+0x164`/`+0x64` and the
Bomb's `+0xc4`. Three weapons, three pools, three cursors, and the cap differs
too (16 here, 0x40 for mines).

## `Plasma_Init` (`0x0885bd18`) plays `PLASMA` and `WO_PLASMA_HEAD`

Confidence **90**, and both strings are **direct memory reads** rather than
inferences from a plausible name - the standard this subsystem adopted after
`Mine_SpawnExplosion`:

```text
0x08a7c0ac  "PLASMA"           <- the cue, played at volume 1.0 through the craft's emitter
0x08a7c0c0  "WO_PLASMA_HEAD"   <- the effect, spawned with fourcc 0x45484c50 = "PLHE"
```

The constructor copies the firing craft's `+0xad8` quad into the entity's
`+0x130`, seeds the entity's own matrix from the craft's `+0x60` node, zeroes the
age at `+0x54` and the two ramps at `+0x124`/`+0x128`, and makes **one**
`Psys_Spawn_q` call.

**And it marks the bolt as charging, which the 2026-09-02 read of this
function missed.** Two adjacent stores, between the age and the flags:

```c
*(undefined1 *)(entity + 0x4c) = 1;              // charging
*(undefined4 *)(entity + 0x50) = _DAT_08a7c098;  // 0x3f800000 == 1.0f
```

That is the whole wind-up, and [the section below](#the-charge-is-real-and-it-is-not-charge_time)
is what spends it. The `WO_PLASMA_HEAD` instance is spawned **parented to the
craft's own weapon node** - `Psys_Spawn_q(inst, "WO_PLASMA_HEAD", 'PLHE', 0,
0x10, node)`, the sixth argument being `*(craft_entity + 0x60)` - so the glow
sits on the nose for the charge and `Plasma_Launch` re-parents it to the bolt
at release. The Missile makes two; nothing here needs the orbiting
second anchor
[missile.md](missile.md#the-two-flare-anchors-orbit-the-missiles-own-flight-line)
derives.

**`PLHE` is a per-effect tag, and that is consistent with what `mine.md` found.**
`MIEX` turned out to be a shared "this is an explosion" tag rather than the
Missile's own label; `PLHE` reads as `PLasma HEad`, matching the file it spawns.
Nothing here says the tags are a namespace - only that this one is not evidence
of anything beyond the string beside it.

**`WO_PLASMA_FLASH` is authored on the same disc and is not wired.** It is in
`docs/formats/pob.md`'s 35-name list and in the 24-entry full-path table at
`0x08a7c3e4`, and **no read call site plays it**. Whether it is the muzzle
flash, the detonation, or the charge-up the section below is about, is open. It
is deliberately not guessed at: `Race::blast_for` returns `None` for the Plasma,
which is the honest "not implemented" state and not "it does not explode" - the
damage and impulse in `oag_gameplay::projectile::blast` still land.

## `Plasma_Update` (`0x0885c6cc`) is `Rocket_Update`'s floor follower

Confidence **85**. Stripped of the VFPU register shuffling, one tick is:

```c
p->age += dt;                                  // + 0x54
p->prev = p->position;                         // + 0x100 <- + 0xa0
next    = p->prev + p->velocity * dt;          // + 0xf0
probe   = next - p->surface * 12.0f;           // + 0x110, the carried normal

switch (Collide(world, next, probe, &hit, &normal, p->bounds, 0)) {
  case 0x7f:                                   // the probe found no floor
      p->velocity -= p->surface * (dt * 50.0f);      // fall along the carried axis
      break;
  case 0: case 4:                              // a wall
      p->flags |= 4;                           // destroy
      break;
  default:                                     // a floor: ride it
      p->surface = normal;
      next       = hit + p->surface * g_ride_height;
      p->velocity = (next - p->prev) / dt;
      p->velocity = normalize(p->velocity) * (Plasma_Speed(p) / 3.6f);
      next       = p->prev + p->velocity * dt;
}
// ... then the same collision test again over prev -> next, the travel segment
p->position = next;
```

**That is the Rocket's flight model instruction for instruction** - the same
`12.0` probe along the carried surface normal, the same redirect that preserves
speed by renormalising and rescaling, the same fall when the probe finds
nothing, the same detonate on a wall, and the same `/ 3.6` that says the
authored speeds are **km/h**
([rocket-visuals.md](rocket-visuals.md)). `oag_gameplay::projectile::advance`
already implements all of it, which is why porting the Plasma needed no flight
code at all.

`0x0885c5a4` is the per-entity speed lookup whose result is divided by `3.6`,
structurally `Rocket_SpeedForClass`'s twin. **It is deliberately unnamed**: it
was not decompiled, and a name off a structural analogy alone is exactly what
the rubric's 50-70 band is for.

`+0x124` and `+0x128` are two ramps advanced at `dt * 3.0` and `dt * 1.8`, the
second clamped to `1.0`. They are read by nothing this page followed and are
almost certainly the flare's own fade; not chased.

## `Plasmas_Update` (`0x0886b490`) is the pool, and it is where everything was

Confidence **90**. This is the function the 2026-09-02 read did not have, and
it holds both of that read's open items. It is the Plasma's pool walker - the
cursor at `+0xa4` and the 16-entry pointer array at `+0x64` that
`Weapon_FirePlasma` fills - and it runs two passes.

**Pass one, per live entity:**

```c
Plasma *p = pool->slot[i];
if (p->charging == 0) {                       // + 0x4c, a byte
    Plasma_Update(dt, p, g_world);            // fly
    if (p->flags & 1) Plasma_NetSend_q(pool, i);
    FUN_0886b898(pool, i);
} else {
    Plasma_UpdateCharge(p);                   // 0x0885c170 - ride the craft
    p->charge -= dt;                          // + 0x50
    if (p->charge <= 0.0f) {
        node = p->craft->node;                // + 0x64 -> + 0x60
        if (node->flags & 0x1000) Vex_UpdateNodeWorldMatrix(node);
        Plasma_Launch(p, node->matrix, p->craft + 0xe0);   // 0x0885bf84
    }
}
if (p->age > 10.0f) p->flags |= 4;            // + 0x54, the hard reap
```

**Pass two, the teardown**, over every entity carrying the destroy bit:

```c
if (p->flags & 4) {
    Psys_Release_q(g_psys, p->head_instance, 1);   // + 0x58, the WO_PLASMA_HEAD
    Plasma_SpawnDetonation(pool, &p->position);    // 0x0886ac88, + 0xa0
    if (p->emitter != 0) {                         // + 0x5c
        Sound_Play(1.0f, p->emitter, ..., "PLASMAHITWALL", 0);
        ...
    }
    // swap-remove: pool->live -= 1, swap slot i with slot live
}
```

Three things fall out of that and each was an open item:

- **The bolt is held before it flies.** The charge branch.
- **The detonation exists and has a call site.** `Plasma_SpawnDetonation`.
- **A bolt's own lifetime is a hardcoded `10.0` seconds**, not an authored
  `timetodie` - the Plasma's `<Stats>` has no such attribute, and this is where
  the ceiling actually lives.

Also read here: `PLASMAHITWALL` at `0x08a7c99b`, a direct `.rodata` read, and
`~PLASMATVL` at `0x08a7c09c`, the looping travel cue `Plasma_Launch` starts and
this teardown stops.

## The charge is real, and it is not `charge_time`

`Plasma_UpdateCharge` (`0x0885c170`, confidence **88**) is what a charging
entity gets instead of a flight step. It copies the firing craft's weapon-node
world matrix straight onto the entity (`+0x70`..`+0xac`, the same four rows
`Plasma_Init` seeded), pushes the same matrix onto the `WO_PLASMA_HEAD`
instance at `+0x12c`, and scales the effect by

```c
(_DAT_08a7c098 - p->charge) * DAT_08ab0f14      // (1.0 - remaining) * 0.75
```

with a second `* 0.5` when the craft's `+0x6d` flag is set. **So the bolt sits
on the nose and its glow grows for the length of the wind-up**, which is
exactly what a player describes as the Plasma winding up before it fires.
This engine rides the glow and does not ramp it - see
[`PLASMA_FLARE_EFFECT`](../../../../crates/game/src/race/effect_names.rs)'s
doc comment for which half landed and why.

`Plasma_Launch` (`0x0885bf84`, confidence **90**) ends it:

```c
p->launch_kmh = length(craft_velocity) * 3.6f + stats->launchspeed;  // + 0x48, stats + 0xbc
p->matrix     = craft_node_matrix;                                   // + 0x70..0xac
p->charging   = 0;                                                   // + 0x4c
p->velocity   = craft_node_forward * (Plasma_SpeedForClass(p) / 3.6f);
p->surface    = -craft->up;                                          // + 0x110 <- -(craft + 0xb10)
p->flags     |= 8;
Psys_Reparent_q(p->head_instance, &p->matrix);
Sound_Play(1.0f, p->emitter, ..., "~PLASMATVL", &p->pose);
```

Note the direction: the node matrix is re-read **at release**, so a player who
presses fire and then turns gets a bolt down the *new* heading.

**A cross-check from a path read for a different reason.** `Plasma_SpawnRemote`
(`0x0886a920`, confidence 75 - the netcode's own spawn) calls `Plasma_Init`,
then `Plasma_Launch` **immediately**, then runs `(now - packet_timestamp) /
0.01` catch-up `Plasma_Update` steps to bring a remote bolt up to date. That
sequence is only coherent if `Plasma_Init` leaves the bolt *held* and
`Plasma_Launch` is what releases it - which is what a function read on the
other side of the file says independently. This page's own
[History](#history) records that writing the cross-checkable rows down first
is what catches the arithmetic slips; this is the same discipline applied to a
reading rather than to an address.

### `charge_time` is authored, and a calibrated sweep says nothing reads it

The wind-up above is **one second, from a `.rodata` literal**. The file authors
`charge_time="3"` - on Pulse's `WeaponStats_Race.xml`, on its
`WeaponStats_Elimination.xml` and on Pure's `weaponstats.xml`, the only weapon
that authors it at all - and `_DAT_08a7c098` is not that number and is not
reached from the stats block.

`WeaponStats_ParsePlasma` stores the attribute at `stats+0x9c` (`0x0880cd44`,
`_swc1 f0,0x9c(s1)`). The weapon-stats block is reached through
`*(int *)(&DAT_08b32420 + DAT_08b32428 * 4)` - the per-mode table pointer
`Plasma_SpeedForClass` uses - and **69 functions in the executable reference
that table**. Sweeping every load and store at a stats offset across all 69,
with `sp`-relative operands excluded:

| Offset | Attribute | Accesses |
| --- | --- | --- |
| `+0x9c` | `charge_time` | **0** |
| `+0xac` | `venomspeed` (control) | 14, including `Plasma_SpeedForClass` at `0x0885c5f4` |

**The control is what makes the negative worth writing down.** A sweep that
found nothing at either offset would be a broken sweep; this one finds the
consumer it is supposed to find. Widened to the whole binary, every `lwc1` at
`+0x9c` off a non-`sp` base is 70 instructions and not one of them is on a
weapon-stats block - the four inside `PlasmaBlast_Construct` are its own ramp
array, constructed there.

**Pure does the same thing, hardcode included.** Pure's plasma parser is
`FUN_08809044` (`charge_time` -> `+0xac`, `damage` -> `+0xb0`, `blastradius`
-> `+0xb4`, `blastforce` -> `+0xb8`, `speed` -> `+0xbc`, `absorb` -> `+0xc0`,
`slowdown_time` -> `+0xc4`). Pure's `Plasma_Init` is `FUN_0885df98` and it
writes an **immediate** `0x3f800000` into the entity's own countdown:

```c
*(undefined1 *)(e + 0x50) = 1;
*(undefined4 *)(e + 0x54) = 0x3f800000;
```

Its allocator call carries the original source path -
`c:/Work/Wipeout/Code/Backend/Weapons/Plasma.cpp`, line 63 - which is as
direct a confirmation of what the file is as this project gets. Pure's pool
walker `FUN_088552a0` has the identical charge / launch / 10-second reap /
`PLASMAHITWALL` teardown shape with every offset shifted by four.

**So `charge_time` is dead data in both PSP titles**, and the wind-up is three
times shorter than the attribute suggests. `oag_tables::weapons::PlasmaStats`
still carries no field for it, under the module's own rule that an attribute
earns a field when something reads it; what changed is that the *reason* is
now measured rather than "not found yet". `oag_gameplay::projectile::plasma::CHARGE_SECONDS`
carries the 1.0 instead.

**The HUD lead is closed too, and it was never open.** `docs/ui/hud.md`
already records that `fexml`'s known-element table has no HUD, gauge, meter or
bar element at all; the charge is drawn by the `WO_PLASMA_HEAD` instance's own
scale ramp on the nose, not by a widget.

## `Plasma_SpeedForClass` (`0x0885c5a4`) and the launch ramp

Confidence **88**, and it is no longer unnamed - it was decompiled in this
pass, which is what the 50-70 band was holding it below:

```c
float Plasma_SpeedForClass(Plasma *p) {
    stats = *(int *)(&DAT_08b32420 + DAT_08b32428 * 4);
    class_kmh = (float[]){ stats->venomspeed,  // + 0xac
                           stats->flashspeed,  // + 0xb0
                           stats->rapierspeed, // + 0xb4
                           stats->phantomspeed // + 0xb8
                         }[g_class];           // DAT_08b31040
    age = p->age;                              // + 0x54
    if (age < 1.0f)
        return p->launch_kmh * (1.0f - age) + class_kmh * age;   // + 0x48
    return class_kmh;
}
```

Two things: it confirms `plasma.md`'s own `<Stats>` offsets from the reading
end, and **it is the `launchspeed` consumer** - the bolt leaves at the firing
craft's own speed plus `launchspeed` and blends to the class speed over its
first second of flight. This engine does not implement the ramp: it flies at
`class + launchspeed` throughout, which is `oag_gameplay::projectile::launch`'s
shared choice, taken so the Rocket and the Plasma cannot drift apart, and
flagged there rather than restated.

`FUN_0885c650` is a **second** lookup on the same four offsets, without the
ramp. It was not decompiled past its four `lwc1`s and stays unnamed for the
reason this page gave `0x0885c5a4` until today: a name off a structural
analogy alone is what the rubric's 50-70 band is for.

## The detonation, and the three models under it

`Plasma_SpawnDetonation` (`0x0886ac88`, confidence **88**) is the teardown's
own call. It allocates a `0x170`-byte object and constructs it with
`PlasmaBlast_Construct` (`0x0885fd90`, confidence **85**) at the bolt's
position. That constructor:

- **spawns `WO_PLASMA_FLASH`** with the fourcc `0x4c464c50` = `PLFL`, string at
  `0x08a7c22c`, read straight out of `.rodata`. That is the answer to "what is
  `WO_PLASMA_FLASH` for": it is the **detonation**, not the muzzle flash and
  not the charge-up.
- loads **three models**, all three strings read directly:
  `Data\Weapons\pulse_plasma_halo1.vex` (`0x08a7c1b0`),
  `Data\Weapons\pulse_plasma_hemisphere2.vex` (`0x08a7c1d4`) and
  `Data\Weapons\pulse_plasma_hemisphere1.vex` (`0x08a7c200`) - a halo and two
  hemispheres, the expanding shell of the blast.
- orients itself to the track through `AiTrack_LocatePosition`, building an
  orthonormal basis from the located surface normal and offsetting the whole
  thing along it, and
- builds three animation ramps (`+0x90`..`+0x114`) over the three models.

`Data\Weapons\Bomb_Shockwave.vex` sits immediately before the halo in the
same string run (`0x08a7c190`), which is a neighbour reading and nothing more -
it is named here so the next person to open the Bomb's teardown knows the
string exists.

**What this engine draws.** `Race::blast_for` now returns `WO_PLASMA_FLASH`
for the Plasma, at the impact point, for every ending - the teardown pass does
not branch on what was struck. The three models are **not** drawn: nothing is
substituted for them, which is the honest partial rather than an invention.

## The `<Stats>` block, and it closes an arithmetic the Mine's page opened

`WeaponStats_ParsePlasma` (`0x0880cc2c`) is the same shape as the Rocket's and
the Mine's - match an attribute name, `swc1` the parsed float at a fixed offset.
Eleven attributes, eleven offsets, confidence **92**:

| Offset | Attribute | String |
| --- | --- | --- |
| `+0x9c` | `charge_time` | `0x08a78b08` |
| `+0xa0` | `damage` | `0x08a78a34` |
| `+0xa4` | `blastradius` | `0x08a78a8c` |
| `+0xa8` | `blastforce` | `0x08a78a98` |
| `+0xac` | `venomspeed` | `0x08a78a4c` |
| `+0xb0` | `flashspeed` | `0x08a78a58` |
| `+0xb4` | `rapierspeed` | `0x08a78a64` |
| `+0xb8` | `phantomspeed` | `0x08a78a70` |
| `+0xbc` | `launchspeed` | `0x08a78a80` |
| `+0xc0` | `absorb` | `0x08a78aac` |
| `+0xc4` | `slowdown_time` | `0x08a78a3c` |

**`mine.md` predicted this block and it lands exactly there.** That page derived
the layout of the `0xa4` unread bytes between the Missile's block (ends `+0x5c`)
and the Mine's (starts `+0xe8`) from nothing but attribute counts - "Quake's
four, the Cannon's five, Turbo's two, Shield's two, Autopilot's two, Plasma's
eleven and the Bomb's eight, at four bytes each, come to exactly that" - and put
the Plasma at `+0x9c`..`+0xc4`. Measuring it there turns a supported claim into
a checked one, and it is now the **third** measured anchor in that run, after
Turbo's `+0x84` and Shield's `+0x8c`.

### All fourteen parsers, from one read of the dispatch chain

`WeaponStats_Parse` (`0x0880db7c`) is a fourteen-deep `if/else` over the `type`
attribute, and the type strings sit in one contiguous run at `0x08a78bf4`:

| String | Parser | Name |
| --- | --- | --- |
| `0x08a78c08` | `0x0880c058` | `WeaponStats_ParseRocket` |
| `0x08a78c10` | `0x0880c31c` | `WeaponStats_ParseMissile` |
| `0x08a78c18` | **`0x0880c60c`** | **`WeaponStats_ParseQuake`** |
| `0x08a78c20` | `0x0880c774` | `WeaponStats_ParseCannon` |
| `0x08a78c28` | **`0x0880c92c`** | **`WeaponStats_ParseTurbo`** |
| `0x08a78c30` | `0x0880ca2c` | `WeaponStats_ParseShield` |
| `0x08a78c38` | **`0x0880cb2c`** | **`WeaponStats_ParseAutopilot`** |
| `0x08a78c44` | **`0x0880cc2c`** | **`WeaponStats_ParsePlasma`** |
| `0x08a78c4c` | `0x0880cef0` | `WeaponStats_ParseBomb` |
| `0x08a78c54` | `0x0880d124` | `WeaponStats_ParseMine` |
| `0x08a78c5c` | **`0x0880d328`** | **`WeaponStats_ParseLeachBeam`** |
| `0x08a78c68` | **`0x0880d58c`** | **`WeaponStats_ParseRepulser`** |
| `0x08a78c74` | **`0x0880d790`** | **`WeaponStats_ParseShuriken`** |
| `0x08a78c80` | **`0x0880dab0`** | **`WeaponStats_ParseGlobal`** |

**Four rows are cross-checks and all four pass**: Rocket, Cannon, Shield, Bomb
and Mine land on the addresses `weapon-fire.md` and `mine.md` already record.
That is what carries the eight new rows to confidence **88** without reading
each body - the chain is one read, the string table is one read, and the two
agree at every checkable point.

**This also retires an open item.** `mine.md` recorded that "five of the fourteen
`<Stats>` parsers were never defined as functions" in the Ghidra database and
that `create_function` refused to make one, leaving their offsets unreachable.
That is no longer true for the Plasma's: `decompile_function` works on
`0x0880cc2c` directly, and `inspect_memory_content` reads `.rodata` at
`0x08a78a2c` and `0x08a78bf4` normally. Whatever the earlier blocker was, the
route through the decompiler plus a `.rodata` string read is open, and it is how
this whole page was read.

## `charge_time` is authored and nothing read spends it

The file authors `charge_time="3"` for the Plasma - on Pulse's
`WeaponStats_Race.xml`, on its `WeaponStats_Elimination.xml`, and on Pure's
`weaponstats.xml`. **It is the only weapon that authors it.**

Three functions sit between the button and the bolt, and **none of them holds a
shot back**:

1. `Ship_FireHeldWeapon` (`0x08844ae8`) calls `Weapon_RequestFire`
   (`0x08862d9c`) with no timer in front of it, then dispatches through a second
   fourteen-entry table at `0x08a7bc90` indexed by `weapon_id + 1`.
2. `Weapon_RequestFire`'s Plasma case (id 7) does the same three things every
   other case does - `ori` bit `0x4` into `craft+0x1b8`, store the `craft+0x20`
   emitter anchor, announce index 7 - per
   [mine.md](mine.md#weapon_requestfires-jump-table-read-as-a-table).
3. `Weapon_FirePlasma` spawns on the very next `Weapons_DispatchFire`, and
   `Plasma_Update` reads `+0x54` as a plain age and gates nothing on it.

**And a maintainer who plays Pulse, asked cold, says the Plasma does wind up
before it fires.** That is the same oracle that settled the Rocket's parallel
fan and the Mine-versus-Bomb attribution, and it has been right both times. So
the reading here is *not* "the attribute is vestigial" - it is **the consumer
exists and has not been found**.

### The second jump table is read, and it is not the charge

`0x08a7bc90` was the obvious next place to look and it is now **closed**. It is
not a call table at all: it is a **computed goto** inside `Ship_FireHeldWeapon`
itself, which is why the decompiler emitted `(**(code **)(...))()` under a
`WARNING: Treating indirect jump as call`. Every arm ends in
`b 0x08844eb4` - a branch to the function's shared exit - rather than in a
return.

Fourteen arms, index `weapon_id + 1`, and the shape is uniform. The Plasma's
(index 8, `0x08844d04`) is five instructions:

```c
if (FUN_08809b38()) {                       // jal 0x00005b38
    (*(int *)(*(int **)0x00057fdc + 0x1ac))++;
}
// b 0x08844eb4, the shared exit
```

The Bomb's (`0x08844c80`) is byte-identical but for `+0x1a0`, the Mine's
(`0x08844cac`) but for `+0x1a4`, and the next along but for `+0x1a8`.
**Consecutive word offsets on one global object, one per weapon, incremented by
one on each shot** - which reads as a per-weapon "times fired" tally rather than
anything in the fire path. Not every arm is that stanza: the Quake's
(`0x08844d30`) plays a sound through the same helper `Plasma_Init` uses, and the
Cannon's arm *is* the shared exit, so it does nothing at all.

**No arm holds a timer, and the Plasma's holds nothing but a counter.** The lead
is closed; `charge_time`'s consumer is somewhere else.

**A trap this leaves behind, and it is a bad one.** That counter is at `+0x1ac`
**on the object at `0x00057fdc`** - *not* on a craft. `weapon-fire.md` and
`mine.md` both record two independent failed sweeps for what writes
**`craft+0x1ac`**, the Mine's own round counter, and the offsets collide
exactly. Anyone who finds this increment while hunting that writer will think
they have it. They have not: different base, different object, and the value is
incremented here where `Weapon_DropMines` decrements a craft field.

What is left to try is `WO_PLASMA_FLASH`'s absent call site, and the HUD - a
charging weapon usually has a meter, and `Arcade_HUD.xml` is fully parsed.

`oag_tables::weapons::PlasmaStats` therefore carries **no** `charge_time`
field, under the module's own rule that an attribute earns a field when
something reads it - the same treatment `BombStats` gives `damageradius`. The
engine fires instantly, which is what the read code does. This is the one place
the port is knowingly at odds with a from-play report, and it is recorded rather
than papered over with an invented three-second timer.

## What is not verified

- ~~**Where `charge_time` is spent.**~~ **Closed 2026-09-09 as a negative**,
  and the negative is calibrated - see above. Nothing reads it in either PSP
  executable; the wind-up it looks like it describes is a separate, hardcoded
  1.0 s.
- **What the teardown's other two calls do.** `Psys_Release_q`
  (`FUN_088f3298`) plainly stops the riding head instance, and `FUN_0886b898`
  - called per live entity in pass one, and unread - is the obvious candidate
  for the blast sweep that spends `damage`, `blastradius` and `blastforce`.
  Reading it is what would settle whether a bolt that times out at 10 s does
  damage as well as drawing an explosion, which is the one thing this engine's
  own expiry path cannot decide without it.
- ~~**The blast object's own animation.** `PlasmaBlast_Construct` builds three
  ramps over its three models and nothing here reads what advances them.~~
  **Struck 2026-09-16**: `PlasmaBlast_Update` is that reader - see
  [the blast's own per-tick animation](#the-blast-objects-own-per-tick-animation-plasmablast_update).
- ~~**`FUN_0885c650`**, the second speed lookup, deliberately unnamed - see
  above. `0x0885c5a4` left this list on 2026-09-09 by being decompiled.~~
  **Struck 2026-09-16**: renamed `Plasma_ClassSpeed`, confidence 82 - see
  below.
- **The bodies of the four unbuilt fire handlers** named at 82 in the table
  above. Only their dispatch is read.
- **`g_ride_height`** (`_DAT_002acf08`), the constant a redirected bolt is
  lifted off the surface by. Read as a global, its value not sampled.

## `Plasma_ClassSpeed` (`0x0885c650`) is `Plasma_SpeedForClass` without the ramp

Confidence **82**, decompiled in full and unambiguous - eleven lines, no VFPU:

```c
float Plasma_ClassSpeed(void) {
    stats = *(int *)(&DAT_08b32420 + DAT_08b32428 * 4);   // the same per-mode block
    switch (g_class) {                                     // DAT_08b31040
        case 0: return stats->venomspeed;    // + 0xac
        case 1: return stats->flashspeed;    // + 0xb0
        case 2: return stats->rapierspeed;   // + 0xb4
        case 3: return stats->phantomspeed;  // + 0xb8
        default: return 0.0f;
    }
}
```

Exactly `Plasma_SpeedForClass`'s four-way switch with the `age < 1.0f` launch
blend cut out - the same stats block, the same four offsets, the same class
selector. Its one caller, `FUN_08850edc` (`0x08850f64`), is well outside the
Plasma's own address range and unread, so what asks for a class's steady-state
speed without the launch ramp is still open; the body itself is not.

## The blast object's own per-tick animation: `PlasmaBlast_Update`

2026-09-16. `PlasmaBlast_Update` (`0x0885f680`, confidence **82**) is vtable
slot 3 (offset `0xc` into the seven-slot table) of `DAT_08aca6e8`, the vtable
`PlasmaBlast_Construct` installs at the object's own `+0x38` - found by
reading that vtable's bytes directly (`inspect_memory_content` at
`0x08aca6e8`) rather than guessed from a slot count. Six of the table's seven
entries are `0x0894xxxx` addresses shared with other classes' generic
object-lifecycle vtables (destroy, name lookup, and the like - not chased,
since they carry nothing Plasma-specific); the seventh, at the Update slot, is
the only address inside the Plasma's own module range, and it is called once a
tick with `(dt, blast)`.

**No separate `Draw` slot exists for this class.** The three loaded `.vex`
models are children the object parents into the scene graph
(`PlasmaBlast_Construct`'s `local_1f0`/`+0x40` link), and they draw through the
engine's generic Vex-model render pass the same way the Rocket's, the Mine's
and the Bomb's own models do - `PlasmaBlast_Update` only ever *writes* their
transform, tint and animation-time state, never issues a draw call of its own.
That is a structural reading (no `Draw`-shaped vtable slot fits, and every
`0x0894xxxx` slot decompiles as generic-looking bookkeeping), not a decompiled
negative on all six, so it is offered at the same 82 rather than higher.

### The bolt's own detonation lasts a hardcoded 1.5 seconds

The whole of `PlasmaBlast_Update`'s tail:

```c
if (1.5f <= blast->age) {                 // + 0x50 - lui a0,0x3fc0, an immediate, not a DAT_ load
    blast->flags = (blast->flags & ~4) | 0xa;   // + 0x2c
    unregister from the 32-slot list at DAT_08b30f10 if present, then FUN_08944a38(blast);  // release
    return 0;   // "I am done, recycle me"
} else {
    blast->age += dt;
    return 1;   // "still alive"
}
```

Confidence **88** for the `1.5` figure specifically: `search_instructions` on
this function for the operand `0x3fc0` finds exactly one hit, `lui a0,0x3fc0`
building `0x3f800000`'s neighbour `0x3fc00000` = `1.5f` as an immediate two
instructions before the comparison - not a `DAT_` load, so there is no shared
constant to mis-attribute. **This is a real, Pulse-specific number and it
disagrees with both HD ports**: `ps4-omega-eu`'s and `ps3-hdfury-eu`'s
`WeaponExplosions_Update` retire the equivalent object at **3.5 s**, more than
twice as long, with an intermediate 1.3 s "collapse" step that hides the three
models early and lets the last ramp keep running on a hidden mesh. **Pulse has
no collapse step at all** - `FUN_08944a38` (the release call) is the only
teardown path found, called once, at 1.5 s, with nothing in between that
clears a visibility flag on the three models ahead of the object's own death.
So the map the HD ports gave this page going in - "a `(cur, target, rate)`
ease per model, a collapse partway through, then a longer full retire" -
turned out to describe the HD engine, not Pulse's: Pulse's blast is shorter
and single-stage.

### Each model gets a camera-facing basis, an anim-time scrub and a tint - not a `(cur, target, rate)` ease

Confidence **75** for the anim-time-scrub, dead-ramp and scale-nudge findings
below, the same figure for all three since they share the same evidence
pass; lower than the retire time because the color path is genuinely
irregular in the shipped binary and that irregularity is reported rather
than resolved. **The camera-basis bullet is confidence 55** - what it reads
and negates is pinned to a specific camera-struct field by cross-reference,
but which semantic axis (or combination) that field represents is not, so
the render side below implements the ordinary "face the camera" billboard as
a stated substitute rather than this specific, unresolved vector math - see
that section's own note.

For each of the three models (index 0 = the halo, 1 = `hemisphere2`, 2 =
`hemisphere1`, `PlasmaBlast_Construct`'s own load order), every tick:

- **A basis rebuilt every tick from the active camera - not, on a re-check of
  the actual decompile, a "look from the camera at the object" vector.** The
  function opens by reading three floats spaced `0x10` apart at the active
  camera's own `+0x48`/`+0x58`/`+0x68` - `DAT_08ab10b0` is the same global
  `zone-mode.md` already documents as *the active camera*, and `exhaust.md`
  independently reads that struct's `+0x40..0x64` as "columns 0 and 1 of the
  camera matrix" (`up`, then `right`) with `contact-response.md` separately
  placing a world *position* at `+0x70` - so `+0x40`/`+0x50`/`+0x60` read as
  three successive `vec4` **columns** of a camera-to-world matrix (`up`,
  `right`, a third column, then position), and `+0x48`/`+0x58`/`+0x68` are
  each column's own third (`z`) component. **This page does not have enough
  to say which single semantic axis that combination extracts** - whether
  it is one row of the rotation part re-assembled, the view/forward column
  specifically, or something this reading has not isolated - only that it is
  *some* function of the camera's current orientation, negated and normalised
  before use, and that no term anywhere in this function reads the blast's
  own position as part of building it (the earlier draft of this bullet said
  it did; it does not, and that was an assumption written down without the
  disassembly to back it, corrected in the same pass that found it). Gram-
  Schmidt-orthogonalising a second axis off the object's own stored reference
  vector follows the same shape `PlasmaBlast_Construct`'s own basis build
  uses. Whatever this resolves to, it is **not** the fixed track-fitted basis
  `PlasmaBlast_Construct` builds once at spawn - the two coexist, and this one
  is recomputed from the camera every tick, so the model re-orients as the
  camera moves even though the group's anchor point does not.
- **`Node_SetAnimTimeTree(age * rate[i], model)`** - already a named, shared
  engine function (not touched here), called with `0.0` once at spawn
  (`PlasmaBlast_Construct`'s own three-iteration loop, `FUN_08912890(0, ...)`
  before it carried this name) and with `age * rate[i]` every tick after. The
  three `rate[i]` values are read straight out of the same small shared
  constants table `PlasmaBlast_Construct`'s scale/offset fields come from:
  `DAT_08ab0f6c = 0.1`, `DAT_08ab0f70 = 0.07`, `DAT_08ab0f74 = 0.07` for
  models 0/1/2. Since the three `.vex` files are the ones the disc ships and
  `Node_SetAnimTimeTree` scrubs a *time*, the expansion this page's own intro
  calls "the expanding shell of the blast" is most likely baked into each
  model's own authored vertex animation and scrubbed by this call, **not**
  computed by a scale ramp this engine would have to reproduce by hand - a
  materially different shape than HD's `(cur, target, rate)` ease over a
  basis scale, and good news for a straight port: play the model's own
  animation at this time value rather than re-deriving an ease curve.
  Un-chased: what `Node_SetAnimTimeTree` does with a value past the model's
  own clip length, and whether the three per-model rates were themselves
  meant to be read from `+0x90..+0x114`'s keyframe table (next bullet) rather
  than hardcoded here - the two mechanisms sit side by side in the same
  function and this page did not find the second one wired to anything.
- **The `+0x90`..`+0x114` "three ramps" are a real keyframe-curve mechanism
  that ships dead.** `PlasmaBlast_Construct` spends real effort precomputing
  per-keyframe reciprocal rates (`1.0f / (time[k+1] - time[k])`, stored at
  `+0xac`/`+0xb0` per ramp) for exactly the shape a multi-key ease would need,
  gated on each ramp's own keyframe count at `+0xb4`/`+0xe4`/`+0x114`. That
  count is **never written** by `PlasmaBlast_Construct`, and the shared base
  constructor every object of this kind runs first, `FUN_08943f08`, does not
  touch it either (checked directly: it writes offsets `0x00`-`0x38` only,
  keyed by dword index, nowhere near `0xb4`). A freshly allocated object's
  `+0xb4` is therefore `0`, `1 < 0` is false, and
  `PlasmaBlast_Update`'s own keyframe-search loop is skipped for every
  constructed blast - confirmed in the disassembly (`0885fc48`..`0885fd90`
  region reads `+0xb4`/`+0xe4`/`+0x114` before any earlier instruction in
  either function writes them). This is the same shape as the `charge_time`
  finding above: real, working code with nothing to trigger it. **What runs
  instead** is the loop's own `count == 0` fallback, which reads one dword
  *before* each ramp's own value array - for model 0 that lands on
  `DAT_08ab0f74` (`0.07`, the same constant read as model 2's anim-time rate
  a few lines away - a coincidence of layout, not a second meaning for that
  float), and for models 1 and 2 it lands inside the *previous* ramp's own
  unused rate slots, which are `0.0` for the same reason `+0xb4` is. So as
  shipped, model 0's tint value is a constant `0.07` every tick and models 1
  and 2's are a constant `0.0` - not a fade, and this page does not know
  whether that is an authored intent (a flash confined to the halo, with the
  two hemispheres carrying their look in the model's own material rather than
  a tint) or a shipped bug in a feature nothing since exercised. Recorded as
  observed, not resolved.
- **The tint itself lands on the model's own submeshes, not a shader
  uniform this engine already exposes.** `FUN_089122b4(model, colour)`
  packs its float argument as a byte into `byte * 0x010101 - 0x01000000`
  (an opaque grayscale colour, full alpha) and writes it to offset `+0x6c` of
  every submesh group `Mesh_ClassTag()` matches on that model - a per-group
  field, not a whole-model uniform. Given the values above are effectively
  constant (0.07 or 0.0, not time-varying), this reads as closer to "tinted
  once at a fixed shade" than "animated," and is not itself the source of any
  fading look a player would see - if the halo visibly flashes and fades, that
  animation is most likely the baked vertex-colour or opacity track
  `Node_SetAnimTimeTree` scrubs, not this per-group tint.
- **Per-model scale/offset in the group `PlasmaBlast_Construct` builds
  alongside the anim-time rate is `0.0` for all three models** (`+0x6c`,
  `+0x78`, `+0x84`, the first float of each of the three triples the
  constructor reads from `DAT_08ab0f34`/`f44`/`f54`) - `PlasmaBlast_Update`
  reads exactly that field to scale a translation nudge added on top of the
  billboard basis, and a `0.0` nudge is no nudge at all. So the three models
  sit exactly at the construct-time track-fitted position with no per-model
  offset; whatever separation the halo and the two hemispheres show on
  screen comes from their own authored geometry, not from this engine's
  layout math.

**Read together, the three DAT tables at `0x08ab0f34`-`0x08ab0f78`
(scale-nudge, anim-time rate, and a middle `8.0`/`3.5`/`3.0` field this page
did not find a reader for) look like a small shared tuning block for a
generic "expanding shell" object class rather than Plasma-specific constants**
- the neighbouring `Data\Weapons\Bomb_Shockwave.vex` string this page already
flagged as worth a look for whoever reads the Bomb's own teardown is the
obvious next place that would confirm or rule this out; not chased further
here since the Bomb's teardown is outside this page's own function set.

## History

- **2026-09-16.** `PlasmaBlast_Update` found off the vtable
  `PlasmaBlast_Construct` installs and read in full: a hardcoded 1.5 s
  lifetime with no HD-style collapse stage, a per-model camera-facing basis,
  and an anim-time scrub that most likely plays each model's own baked
  expansion rather than this engine computing one. `Plasma_ClassSpeed`
  (`0x0885c650`) decompiled cleanly and closed the last unnamed function this
  page's own "What is not verified" list carried. Both closed by
  `plasma-blast-models`, alongside the render side wired the same day - see
  `docs/gameplay/pickups.md` and the weapons handover thread for what draws.
- **2026-09-02.** Written while implementing the Plasma, the first of the six
  remaining weapons. **The page's own trap caught this read too**, which is now
  the fourth time: `weapon-fire.md` warns that `func_0x000NNNNN` needs
  `+ 0x08804000`, and the first pass through `Weapons_DispatchFire` added the
  base without carrying, turning `0x0005d3bc` into `0x0885d3bc` instead of
  `0x088613bc`. Every handler address in the first draft of the table above was
  wrong by the same slip, and the thing that caught it was not care - it was
  that six of the rows are cross-checks against pages that already had the right
  answer. **Write the cross-checkable rows down first and the arithmetic errors
  announce themselves**; a table of only-new addresses would have shipped.
