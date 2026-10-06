# The Shuriken: one blade, thrown at a coin-flipped 20 degrees, that bounces perfectly

**Binary:** `pulse-psp` `BOOT.BIN`, image base `0x08804000`.

**Status:** **read and built.** Five functions and the whole `<Stats>` block are
recovered; `oag_weapons::projectile::shuriken` and
`oag_tables::weapons::ShurikenStats` are the port. The reading was done on 2026-09-02
alongside the Plasma's, once [plasma.md](plasma.md)'s dispatch table handed over
the handler address.

**A correction this page carries rather than hides.** Its first version said
"read end to end" while `Shuriken_Update` was the one function *not* read, and
concluded from the other four that "the Shuriken shares nothing" with the
weapons already built. **Reading the update reversed that**: it is the Rocket's
own floor follower - the same 12-unit probe along a carried surface normal, the
same speed-preserving redirect - and the single substantive difference is that a
**wall calls [`Shuriken_Bounce`](#shuriken_bounce-0x088778ac-is-a-mirror-with-no-energy-lost)
where a rocket detonates**. The estimate that followed from the unread version
("a session of its own") was wrong by a lot, and the lesson is the one
`plasma.md` already records: the expensive part of a weapon here is its
trajectory, and *the trajectory is the thing you have to actually read* rather
than infer from a constructor's entity layout.

| Address | Name | Confidence |
| --- | --- | --- |
| `0x0880d790` | `WeaponStats_ParseShuriken` | 92 |
| `0x08870240` | `Weapon_FireShuriken` | 88 |
| `0x08877280` | `Shuriken_Init` | 88 |
| `0x088778ac` | `Shuriken_Bounce` | 88 |
| `0x08877bdc` | `Shuriken_Update` | 85 |

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
settles for this weapon the question `oag_weapons::projectile::rocket::launch`
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

**2026-09-23: wired.** `Cue::ShurikenTravel` and `Cue::ShurikenHit` are in
`crates/sound/src/sfx/cue.rs`, both placed on the **firing craft's** own
emitter rather than the blade's. `craft->emitter` being handed to
`Shuriken_Init` directly, above, is read at this page's own confidence 88 -
what is not read is either play call's own emitter argument, so the
placement itself is **chosen, not measured, no confidence score**: the
absence of a bolt-owned `SoundEmitter_Init` the way `Rocket_Init`/
`Missile_Init` each get one is what rules a blade-owned emitter out, not a
read of `~SHURIKENTRAVEL`'s or `SHURIKENHIT`'s own call site. The one-shot
launch cue at `0x08a7cd38` stays unwired: its own name is still not
verified, and the bank's plain `SHURIKEN` cue is a name-matched guess this
port does not trust.

## `Shuriken_Bounce` (`0x088778ac`) is a mirror, with no energy lost

Called from `Shuriken_Update`'s travel-segment test, `case 0` - the branch that
ends a rocket.

```c
float d = dot(s->velocity, normal);
s->velocity = s->velocity + (-normal) * (2.0f * d);   // v - 2(v.n)n
position    = hit + normal * 0.1f;                    // 0x3dcccccd
Psys_Spawn_q(..., "WO_SHURIKEN_BOUNCE", 'SHBO', ...); // 0x08a7cdbc
Sound_Play(1.0f, s->emitter /* round+0x50, 300.0 radius */, ..., "SHURIKENHIT");// 0x08a7cd5c
```

**A perfect specular reflection with no damping term anywhere in the
function**, and the blade is nudged `0.1` units back out along the normal so the
next tick's sweep does not start inside the wall. Confidence **88**: the
arithmetic is unambiguous, the `0.1` is a code literal, both the effect and the
cue are direct `.rodata` reads, and the caller is now identified.

**This is not the Missile's bounce and must not reuse it.**
`oag_weapons::projectile::missile::MAX_BOUNCES` glances off up to five walls
under the Missile's own rule; nothing here counts bounces at all, and what ends
a shuriken is its `fuse` (see below), not a bounce budget. Reusing the Missile's
path would be the same class of mistake as the Rocket's flare riding a mine.

## `Shuriken_Update` (`0x08877bdc`) is the Rocket's floor follower, until a wall

Stripped of the VFPU shuffling and the orientation bookkeeping:

```c
s->age += dt;                                  // + 0x48, the fuse's clock
s->prev = s->position;                         // + 0x140 <- + 0x120
next    = s->prev + s->velocity * dt;          // + 0x130
probe   = next - s->surface * 12.0f;           // + 0x150, the carried normal

switch (Collide(world, next, probe, &hit, &normal, s->bounds, 0)) {
  case 0x7f: s->velocity.y -= dt * 50.0f;  break;   // no floor: fall
  case 4: case 0: break;
  default:                                          // a floor: ride it
      s->surface  = normal;
      next        = hit + s->surface * g_ride_height;
      s->velocity = (next - s->prev) / dt;
}

switch (Collide(world, s->prev, next, &hit, &normal, s->bounds, 0)) {
  case 4:            s->flags |= 0x14;                          break;  // destroy
  case 3: case 1:    ... ride the surface, as above ...          break;
  case 0:            next = Shuriken_Bounce(s, &hit, &normal);   break;  // <- a wall
}
s->position = next;
// then: rebuild the blade's basis from velocity and surface, copy it to the
// head (+0x60) and trail (+0xb0) anchors, rotate the trail's by -pi/2, and
// push the model transform.
```

**Compare `Plasma_Update` (`0x0885c6cc`) on [plasma.md](plasma.md).** The two
are the same function but for three things, and only the third matters to a
port:

1. The Plasma's no-floor case pushes velocity along the **carried surface
   normal**; the Shuriken's decrements **world y** alone (`+0x134`). Both use
   the same `50.0` literal.
2. The Shuriken rebuilds an orientation basis every tick, because it has a model
   and two anchored effects; the Plasma has neither.
3. **A wall bounces it instead of ending it.** That is the weapon.

Confidence **85** - the structure is unambiguous, the constants are literals, and
the `case 0` target resolves to `Shuriken_Bounce` by the image-base arithmetic
(`0x000738ac + 0x08804000`), which is also the only caller found for it.

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

### The two damage numbers, and which half is built

The Shuriken is the only weapon of the thirteen authoring **two** damages and
**two** forces. `blastdamage`/`blastForce` are what `blast_stats` spends when a
blade reaches a craft, which is the weapon's effect and is built.
`rhicochetdamage`/`rhicochetForce` are **not decoded**: a glancing hit off a
craft is the obvious reading and it is a reading, so
`oag_tables::weapons::ShurikenStats` leaves both out rather than picking one.
The engine's own bounce path only ever fires off *geometry* - a hull hit ends
the blade - so there is currently no moment at which a ricochet number would be
spent even if it were decoded.

## What is not verified

- **What ends a shuriken: read 2026-09-30.** `ShurikenPool_Update`
  (`0x0886ff38`) destroys a blade when its `fuse` is below its clock, or on
  `Shuriken_Update`'s craft-hit flag, and both reach the teardown
  `Shuriken_SpawnExpiry` (`0x08870c78`): `WO_SHURIKEN_EXPIRE`, a kind-0 screen
  flash, `SHURIKENEXPL`, and nothing that spends damage. See
  [screen-flash-callers.md](screen-flash-callers.md). Built, less the sound.
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
- ~~**`0x08a7cdbc`** is named from its position and fourcc rather than read.~~
  **Read directly, 2026-09-02**: `0x08a7cdbc` is `WO_SHURIKEN_BOUNCE`, 18 bytes
  and NUL-terminated. All four of the Shuriken's effect names now meet the
  direct-read standard `Mine_SpawnExplosion` set.
- **Which surface `case 0` actually means.** `Shuriken_Update` treats the
  collide result `0` as a wall and `4` as fatal, where `Plasma_Update` treats
  **both** `0` and `4` as fatal. The result codes themselves are not decoded
  anywhere in this tree, so "0 is a wall a blade can bounce off and 4 is one it
  cannot" is this page's reading of two switch statements, not a recovered
  enumeration.

## A neighbour read on the way: the Repulser

**Superseded 2026-10-04 by [repulser.md](repulser.md).** The four copies below
are written and never read, `+0x128` is `damage` rather than `blastforce`, and the
weapon is a pool entity whose two track-following waves do the work. The original
note is kept as it was written.

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

## 2026-09-30: the launch cue is `SHURIKEN`, and the blade owns an emitter (pulse-weapon-audio lane)

`Shuriken_Init` (`0x08877280`) decompiled whole, confidence 88:

- `Sound_Play(1.0, param_6, ..., "SHURIKEN", 0)` (pointer cell `0x08a7cd44`,
  string `0x08a7cd38`) on the emitter its caller passes last - the firing
  craft's own, before the blade's exists. The launch cue's name was never
  unverified: it is the plain string `SHURIKEN`, and `weapons.bnk` has a cue by
  that name (cue 29). Wired as `Cue::ShurikenLaunch`.
- It **then allocates a bolt-owned emitter**: `FUN_08946ce4(0x70)` +
  `SoundEmitter_Init`, stored at `round+0x50`, its `+0x50` pointed at the round's
  own matrix (`round+0xf0`) and its radius `+0x38` set to `0x43960000` =
  **300.0**, then plays `~SHURIKENTRAVEL` through it, keeping the handle at
  `round+0x54`. `Shuriken_Bounce` (`0x088778ac`; the `Sound_Play` for
  `SHURIKENHIT` is at `0x08877b60`) loads the same `round+0x50` and writes the
  same 300.0 before it plays.

**This corrects the 2026-09-23 note above**, which put both cues on the firing
craft's emitter on the strength of "no bolt-owned `SoundEmitter_Init` exists".
One does; both ride the blade, radius 300.0, measured.

## 2026-10-06: the model and the trail are drawn (weapon-visuals lane)

Re-read `Shuriken_Update` (`0x08877bdc`) and `Shuriken_Init` (`0x08877280`) at
instruction level, confidence 88:

- **The basis is the Rocket's.** Row 0 `n x f` (`vcrsp.t` at `0x08877fa8`),
  row 1 `n` (`+0x100`, the carried surface normal), row 2 `f` (velocity
  normalised, re-orthogonalised against `n`), row 3 position, built at
  `blade+0xf0` and handed to the model's scene node (`0x08945284`, `a0 =
  blade+0x160`) **unrotated**. No spin is written anywhere in the update, so a
  tumble can only be the model's own node animation. `Race::shuriken_model_matrices`
  is `projectile_model_matrices(Shuriken)`, the Rocket's velocity branch.
- **Two frames, both passed to `Psys_Spawn_q` with flag 1 (a pointer, read
  live).** `WO_SHURIKEN_HEAD` rides `blade+0x60`, a copy of the basis, unrotated:
  the emitter's `+Y` is the surface normal. `WO_SHURIKEN_TRAIL` rides
  `blade+0xb0`, a copy turned `-pi/2` about its own row 0 (`0xbfc90fdb`, `Math_RotateByAxisAngle`
  at `0x08a6b6b4`, the call `Shuriken_Init` and `Shuriken_Update` both make): the
  Rocket's flare frame, so `+Y` is the velocity. Both are spawned in `Shuriken_Init`
  and ride the whole flight.
- **Wired:** `Trigger::ShurikenTrail` rides the second (orbit) flare slot; the head
  is oriented to `Projectile::surface`, the trail to the velocity. Disc test:
  `shuriken_ground_truth::a_thrown_blade_rides_its_head_and_trail_and_has_a_model_pose`.
- **Not read:** whether `pulse_shuriken.vex` carries a node animation and which clock
  drives it. The drawable is written with the scene clock like the Mine's, **chosen,
  not measured**; if the model authors none this is a no-op.
- **Seen:** `data/scratch/weapon-visuals/strip_crop2.png` (frames 342-352 of a
  solo Eliminator throw): a flat bladed disc under the head and trail rings.

Cross-title: **not checkable here** - no HD, Pure or 2048/Omega blade model name
was recovered, so `WeaponModels::shuriken` is `None` on all four (HD's render
paths belong to logwarn-hd). The head/trail effects are shared by name only on
titles whose effect library carries them.
