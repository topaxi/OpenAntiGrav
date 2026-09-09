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

- **Where `charge_time` is spent.** Above. The biggest open item on the weapon,
  and one of the two named leads is now closed rather than merely unfollowed.
- **What `WO_PLASMA_FLASH` is for.** Authored, located, no call site found.
- **The Plasma's detonation effect.** `Plasma_Update`'s destroy bit is raised in
  two branches and the teardown that consumes it was not followed, so whether
  the pool's expiry path spawns an explosion - the way `FUN_08867370` does for
  the Mine - is unread. `Race::blast_for` returns `None` for the Plasma
  accordingly.
- **`0x0885c5a4`**, the speed lookup, deliberately unnamed - see above.
- **The bodies of the four unbuilt fire handlers** named at 82 in the table
  above. Only their dispatch is read.
- **`g_ride_height`** (`_DAT_002acf08`), the constant a redirected bolt is
  lifted off the surface by. Read as a global, its value not sampled.

## History

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
