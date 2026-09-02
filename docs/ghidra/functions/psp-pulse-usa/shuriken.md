# The Shuriken: one blade, thrown at a coin-flipped 20 degrees, that bounces perfectly

**Binary:** `pulse-psp` `BOOT.BIN`, image base `0x08804000`.

**Status:** **read, not built.** Four functions and the whole `<Stats>` block are
recovered; nothing in `oag_gameplay` fires one yet. This page exists so the
session that builds it starts from evidence rather than from the schema - the
reading was done on 2026-09-02 alongside the Plasma's, once
[plasma.md](plasma.md)'s dispatch table handed over the handler address.

**Why it was read but not built.** The Plasma cost almost nothing because it
shares the Rocket's whole flight model. The Shuriken does not: it needs a
perfect-reflection bounce that is *not* the Missile's, two separate damage
numbers where `oag_gameplay::projectile::blast_stats` returns one, a fuse, and a
seeded coin flip at launch. That is a session of its own, and half-doing it
would have been worse than handing over a complete read.

| Address | Name | Confidence |
| --- | --- | --- |
| `0x0880d790` | `WeaponStats_ParseShuriken` | 92 |
| `0x08870240` | `Weapon_FireShuriken` | 88 |
| `0x08877280` | `Shuriken_Init` | 88 |
| `0x088778ac` | `Shuriken_Bounce` | 85 |

Read [weapon-fire.md](weapon-fire.md) and [plasma.md](plasma.md) first: the
`jal`/`func_0x000NNNNN` operands here are image-base-relative, and it is
`plasma.md`'s full sixteen-bit dispatch read that says bit `0x20000` on
`world+0x68` is this weapon at all.

## `Weapon_FireShuriken` (`0x08870240`) throws one, and flips a coin

```c
void Weapon_FireShuriken(ShurikenPool *pool, Craft *craft, int craft_index) {
    pool->flags |= 2;
    craft->held  = -1;             // craft + 0x1bc
    craft->fire &= ~0x20000;       // craft + 0x1b8 - one throw, cleared at once

    if (pool->live < 0x10) {                    // pool + 0xac, cap 16
        float roll  = Rand01();                 // 0x088f8b60
        float speed = craft->speed;             // craft->0xf0->0x794 + 0x398
        Shuriken *s = pool->slot[pool->live];   // pool + 0x6c + live*4
        s->flags = 1;
        s->owner = craft_index;                 // + 0x40
        s->id    = ++g_next_projectile_id;      // + 0x44
        Shuriken_Init(speed * 3.6f,             // km/h - see below
                      s,
                      craft->matrix,            // craft + 0x4, the jump table's anchor
                      &craft->pose,             // craft + 0x10
                      roll > 0.5f,              // <- the coin flip
                      craft->emitter);
        pool->live += 1;
    }
}
```

**One throw**, on the same two arguments the Plasma's is one shot on: a single
spawn call with its own bit cleared in the same breath, and a `<Stats>` block
with no `spread`.

**`speed * 3.6f` is a corroboration worth having.** The craft's speed is held in
units per second and the constructor is handed it in **km/h**, which is the unit
[rocket-visuals.md](rocket-visuals.md) measured the authored weapon speeds in
from the other direction. Two subsystems, one convention, and this one is a
multiplication rather than an inference.

## `Shuriken_Init` (`0x08877280`): 20 degrees, left or right

The constructor builds an orthonormal basis out of the craft's matrix - forward
normalised, up re-orthogonalised against it (Gram-Schmidt), and their cross
product - and then rotates it about the up axis by a **literal angle chosen by
the caller's coin flip**:

```c
if (!left)  RotateAboutAxis( 0x3eb2b8c3, &fwd, &up);   //  0.349066 rad
else        RotateAboutAxis(0xbeb2b8c3, &fwd, &up);   // -0.349066 rad
```

**`0.349066` radians is 20.000 degrees**, to five significant figures, and the
two constants differ only in the sign bit. So a shuriken never leaves straight
ahead: it goes 20 degrees left or 20 degrees right, half the time each.
Confidence **88** - the constant is exact, its negation is literal, and the
caller's argument is a single `rand() > 0.5`.

**That is the weapon's whole character and it is nothing like the Rocket's
fan.** A Rocket throws three at `-spread`, `0`, `+spread` together; a Shuriken
throws *one*, at one of two fixed angles, and you do not get to pick which.

### Its speed is the craft's own plus the class's

```c
float authored = Shuriken_SpeedForClass(s);   // 0x08877830
s->speed = (craft_speed_kmh + authored) / 3.6f;
s->velocity = basis_right * s->speed;
```

**The firing craft's speed is inherited**, measured rather than assumed - which
settles for this weapon the question `oag_gameplay::projectile::rocket::launch`
records as still open for the Rocket ("the speed being the class's plus
`launchSpeed` ... is this engine's choice"). Here it is the craft's speed plus
the lookup's, and whether the lookup itself folds in `launchspeed` was not
chased.

### What it carries and what it plays

- `Data\Weapons\pulse_shuriken.vex` (`0x08a7cd74`) - it has a **model**, unlike
  the Plasma.
- `WO_SHURIKEN_TRAIL` (`0x08a7cd94`), fourcc `SHUT`, attached at `+0xb0` with
  its basis rotated a further `0xbfc90fdb` = **-pi/2** about the blade.
- `WO_SHURIKEN_HEAD` (`0x08a7cda8`), fourcc `SHUH`, attached at `+0x60`.
- A one-shot launch cue at `0x08a7cd38`, and `~SHURIKENTRAVEL` (`0x08a7cd48`) -
  **looping**, held on a voice handle at `+0x54` for the blade's whole flight,
  the same shape `~SHIELD` has in [shield-pickup.md](shield-pickup.md).

All four `WO_SHURIKEN_*` files are in `docs/formats/pob.md`'s 35-name list, so
nothing here needs an asset this project cannot reach.

## `Shuriken_Bounce` (`0x088778ac`) is a mirror, with no energy lost

```c
float d = dot(s->velocity, normal);
s->velocity = s->velocity + (-normal) * (2.0f * d);   // v - 2(v.n)n
position    = hit + normal * 0.1f;                    // 0x3dcccccd
Psys_Spawn_q(..., "WO_SHURIKEN_BOUNCE", 'SHBO', ...); // 0x08a7cdbc
Sound_Play(1.0f, s->travel_voice, ..., "SHURIKENHIT");// 0x08a7cd5c
```

**A perfect specular reflection with no damping term anywhere in the
function**, and the blade is nudged `0.1` units back out along the normal so the
next tick's sweep does not start inside the wall. Confidence **85**: the
arithmetic is unambiguous, the `0.1` is a code literal, and both the effect and
the cue are direct `.rodata` reads.

**This is not the Missile's bounce and must not reuse it.**
`oag_gameplay::projectile::missile::MAX_BOUNCES` glances off up to five walls
under the Missile's own rule; nothing here counts bounces at all, and what ends
a shuriken is its `fuse` (see below), not a bounce budget. Reusing the Missile's
path would be the same class of mistake as the Rocket's flare riding a mine.

## The `<Stats>` block: thirteen attributes at `+0x144`..`+0x174`

`WeaponStats_ParseShuriken` (`0x0880d790`), read the same way the Plasma's was.
Confidence **92**.

| Offset | Attribute | String |
| --- | --- | --- |
| `+0x144` | `blastdamage` | `0x08a78bc0` |
| `+0x148` | `rhicochetdamage` | `0x08a78bb0` |
| `+0x14c` | `venomspeed` | `0x08a78a4c` |
| `+0x150` | `flashspeed` | `0x08a78a58` |
| `+0x154` | `rapierspeed` | `0x08a78a64` |
| `+0x158` | `phantomspeed` | `0x08a78a70` |
| `+0x15c` | `launchspeed` | `0x08a78a80` |
| `+0x160` | `blastradius` | `0x08a78a8c` |
| `+0x164` | `blastForce` | `0x08a78b8c` |
| `+0x168` | `rhicochetForce` | `0x08a78bcc` |
| `+0x16c` | `absorb` | `0x08a78aac` |
| `+0x170` | `slowdown_time` | `0x08a78a3c` |
| `+0x174` | `fuse` | `0x08a78bdc` |

**Thirteen slots starting at `+0x144`, which is exactly where
[mine.md](mine.md)'s attribute-count arithmetic puts them** - it is now the
fourth measured anchor in that run, after Turbo's `+0x84`, Shield's `+0x8c` and
the Plasma's `+0x9c`. The run is fully closed at the Shuriken's end; only the
Quake's four (`+0x60`) and the Cannon's five (`+0x70`) are still unmeasured, and
they are the only two weapons left whose parsers nobody has needed.

**`blastForce` and `rhicochetForce` are the file's own capitalisation**, kept
verbatim as `launchSpeed` is - the parser's comparison is case-insensitive (see
[weapon-fire.md](weapon-fire.md)), so a reader matching names exactly should not
be surprised by them.

### The two damage numbers are the shape of the work

The Shuriken is the only weapon of the thirteen authoring **two** damages and
**two** forces: `rhicochetdamage`/`rhicochetForce` for a glancing hit off a
craft, `blastdamage`/`blastForce` for whatever ends it.
`oag_gameplay::projectile::blast_stats` returns a single
`(blastradius, damage, blastforce)` triple, so it does not fit as it stands -
that, rather than the flight, is the real cost of this weapon.

## What is not verified

- **What ends a shuriken.** `fuse` is authored at `2` on both shipped tables and
  nothing here reads its consumer; the pool teardown was not followed, so where
  `WO_SHURIKEN_EXPIRE` is played and whether the end spends `blastdamage` are
  both open. This is the first thing to read next.
- **Which of the two damages a craft hit spends**, and whether a hit ends the
  blade or lets it carry on. Nothing read says.
- **`0x08877830`**, the class-speed lookup, is deliberately unnamed: it was not
  decompiled, and a name off a structural analogy with `Rocket_SpeedForClass`
  alone is what the rubric's 50-70 band is for.
- **The axis the 20-degree rotation is about.** The basis is built forward /
  re-orthogonalised up / cross, and the velocity is taken from the **cross**
  member - so which of the three the rotation is applied about was not read off
  the `vpfx` prefixes. The same gap `weapon-fire.md` records for the Rocket's
  fan.
- **The launch cue's own name.** The pointer at `0x08a7cd44` resolves to
  `0x08a7cd38`, which is before the region read here.
- **`0x08a7cdbc`** is named `WO_SHURIKEN_BOUNCE` on the strength of sitting
  immediately after `WO_SHURIKEN_HEAD` in the same run and being spawned from
  the bounce handler with fourcc `SHBO`. The string itself was not read
  directly, unlike the other three - **read it before relying on it**, per the
  standard `Mine_SpawnExplosion` set.

## A neighbour read on the way: the Repulser

`Weapon_FireRepulser` (`0x0886ce8c`) was decompiled in the same pass and is
recorded here rather than on a page of its own, because one function is not
enough for one. It is **not** an instantaneous blast:

```c
craft->fire &= ~0x10000;
if (pool->live < 0x10) {                   // pool + 0xa8, array + 0x68
    craft->0x170 = stats[+0x128];
    craft->0x17c = stats[+0x13c];
    craft->0x17c = stats[+0x140];          // written twice, as decompiled
    craft->0x178 = stats[+0x134];
    ... allocate a pool entity, then Repulser_Init(entity, craft_index, ...)
}
```

Two things worth carrying: it **copies four of its own `<Stats>` onto the
firing craft** before spawning, which is a shape no other weapon here has and
reads as "the field is a state the craft is in"; and `stats[+0x128..+0x140]` is
exactly the seven-slot Repulser block the arithmetic predicts between the
LeachBeam's (`+0x104`) and the Shuriken's (`+0x144`), which is a fifth
confirmation of that run. `blast_time` (`0x08a78b98`) and `wave_time`
(`0x08a78ba4`) are located; which offsets they land on is not read.
