# The Plasma: one bolt, the Rocket's flight, and a charge nobody spends

**Binary:** `pulse-psp` `BOOT.BIN`, image base `0x08804000`.

**Status:** the weapon is read end to end - request bit, fire handler,
constructor, flight step and `<Stats>` block - at confidence **85 or better**
on every part. It is the cheapest weapon this project has added since the Bomb,
and for the mirror-image reason: the Bomb reused the Mine's whole module and the
Plasma reuses the Rocket's whole flight model.

**One thing is open and it is open uncomfortably.** The file authors
`charge_time="3"` on both shipped Pulse tables and on Pure's, and **three
functions on the press-to-flight path spend it nowhere** - see
[below](#charge_time-is-authored-and-nothing-read-spends-it).

| Address | Name | Confidence |
| --- | --- | --- |
| `0x0880cc2c` | `WeaponStats_ParsePlasma` | 92 |
| `0x0886a868` | `Weapon_FirePlasma` | 88 |
| `0x0885bd18` | `Plasma_Init` | 90 |
| `0x0885c6cc` | `Plasma_Update` | 85 |

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
| `0x4000` | `0x088537ac` | `world+0x58` | *not in the jump table* - see below |
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
  dispatched and is **not** in `Weapon_RequestFire`'s
  jump table at all, so it is set by something else - `0x088537ac`, on
  `world+0x58`, remains the obvious candidate for where the Cannon actually
  fires and is still unchased.
- **Every other unbuilt weapon does have a handler.** Quake, LeachBeam, Repulser
  and Shuriken are each dispatched to a real function whose body has **not**
  been read here. They are named in `names.tsv` at 82 on the strength of the bit
  map plus this dispatch and nothing else; do not assume anything about what
  they *do*.

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
`Psys_Spawn_q` call. The Missile makes two; nothing here needs the orbiting
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
exists and has not been found**, and the search should start at the second jump
table (`0x08a7bc90`), which this page did not follow, and at
`WO_PLASMA_FLASH`'s absent call site.

`oag_formats::weapons::PlasmaStats` therefore carries **no** `charge_time`
field, under the module's own rule that an attribute earns a field when
something reads it - the same treatment `BombStats` gives `damageradius`. The
engine fires instantly, which is what the read code does. This is the one place
the port is knowingly at odds with a from-play report, and it is recorded rather
than papered over with an invented three-second timer.

## What is not verified

- **Where `charge_time` is spent.** Above. The biggest open item on the weapon.
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
