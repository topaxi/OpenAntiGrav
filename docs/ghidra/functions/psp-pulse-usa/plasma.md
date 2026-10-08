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

**Closed 2026-09-16: a wall hit and the `10.0 < age` timeout never spend a
blast; a craft hit does, and it is a third ending this page did not
enumerate until the same day's second pass.** Neither the wall nor the
timeout ending reaches `damage`/`blastradius`/`blastforce` or the
pending-impulse slot every real blast function writes -
`FUN_0886b898`, the pass-one call this page used to leave unread, is read in
full and is not that function. See
[the expiry settles a damage question](#the-expiry-settles-a-damage-question-nobody-had-read).
**A direct craft hit is different and was found later the same day**: it
credits full `damage` and `slowdown_time` to the struck craft and a
falling-off `blastforce` impulse to every other craft in `blastradius` except
the bolt's own firer - see
[a craft hit is the third ending](#a-craft-hit-is-the-third-ending-and-it-does-spend-a-blast).
The wall-hit conflict this page used to flag against `oag_gameplay`'s own
`Impact::blast` is resolved, not left open: the wall arm now matches (no
blast), and the craft-hit arm is ported the same day it was read.

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
| `0x0886afb8` | `Plasma_SweepCraftHit` | 82 |
| `0x0886ad60` | `Plasma_HitCraft` | 88 |
| `0x0886ae08` | `Plasma_ApplyBlastForce` | 88 |

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
`crates/raceplay/src/weapons.rs` cites by address. That is what makes the five
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
damage and impulse in `oag_weapons::projectile::blast` still land.

**What this engine plays, 2026-09-16.** `PLASMA` fires at the press -
`Cue::Plasma`, `Placement::Craft`, `crates/raceplay/src/weapons.rs` - through
the firing craft's own emitter, confirmed by decompiling `Plasma_Init` in full
this session: the `Sound_Play` call above runs on the argument the function
was handed directly, before it ever constructs the bolt's own emitter a few
lines later. See the next section's own note for the two cues that ride that
second, bolt-owned emitter instead.

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
([rocket-visuals.md](rocket-visuals.md)). `oag_weapons::projectile::advance`
already implements all of it, which is why porting the Plasma needed no flight
code at all.
**Correction 2026-10-08:** not quite all - the `0x7f` fall axis was world `-Y`
here until `flight.rs` gave the Plasma its own arm, `velocity -= surface * (dt * 50)`
(see [projectile-floor.md](../../../gameplay/projectile-floor.md), last section).

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

**Pass two, the teardown**, over every entity carrying the destroy bit -
**re-read 2026-09-16 at instruction level with no elision**, the `...` below
resolved rather than left standing:

```c
if (p->flags & 4) {                                // + 0x3c
    Psys_Release_q(g_psys, p->head_instance, 1);   // + 0x58, the WO_PLASMA_HEAD
    Plasma_SpawnDetonation(pool, &p->position);    // 0x0886ac88, + 0xa0
    flags = p->flags;
    if (p->emitter != 0) {                         // + 0x5c
        if (p->node != 0) { FUN_08939bcc(); p->node = 0; }   // + 0x60
        Sound_Play(1.0f, p->emitter, ..., "PLASMAHITWALL", 0);
        FUN_08939460(p->emitter, 0);                // stops the emitter's own loop
        p->emitter = 0;
        flags = p->flags;
    }
    p->flags = flags & ~8;                          // + 0x3c, clears bit 8 - not bit 4
    p->subsystem_flags &= ~4;                       // + 0x2c, a different field
    p->owner = 0xff;                                // + 0x40
    // swap-remove: pool->live -= 1, swap slot i with slot live
}
```

**Bit `4` itself - the destroy flag the `if` above tests - is not cleared
here.** Only bit `8` is (`&= ~8`, both branches). That reads as harmless: the
slot is about to be swap-removed and overwritten by whatever fills it next,
so nothing downstream reads a freed slot's stale destroy bit. Not chased
further; named because the snippet above would otherwise look like it clears
the very flag it is testing.

Three things fall out of that and each was an open item:

- **The bolt is held before it flies.** The charge branch.
- **The detonation exists and has a call site.** `Plasma_SpawnDetonation`.
- **A bolt's own lifetime is a hardcoded `10.0` seconds**, not an authored
  `timetodie` - the Plasma's `<Stats>` has no such attribute, and this is where
  the ceiling actually lives.
- **That is the whole teardown - two calls and a sound, nothing else, for
  either ending.** No elided fourth call, no craft touched. See
  [the expiry settles a damage question](#the-expiry-settles-a-damage-question-nobody-had-read)
  below.

Also read here: `PLASMAHITWALL` at `0x08a7c99b`, a direct `.rodata` read, and
`~PLASMATVL` at `0x08a7c09c`, the looping travel cue `Plasma_Launch` starts and
this teardown stops.

**Both ride the bolt's own emitter, not the firing craft's - decompiling
`Plasma_Init` in full 2026-09-16 settles which.** The constructor allocates
its own `0x70`-byte `SoundEmitter_Init` record, stores it at the entity's
`+0x5c` (`p->emitter`), and points that record's own `+0x50` scene-node field
at `&p->matrix` (`p+0x70`) - the bolt's own pose, seeded from the craft's node
at construction and rewritten by `Plasma_Launch` at release, never at the
craft's own node. So `p->emitter` tracks *the bolt*, and both `~PLASMATVL`
(started on it by `Plasma_Launch`, above) and `PLASMAHITWALL` (played on it by
this teardown) are heard from wherever the bolt actually is - unlike `PLASMA`
one section up, which plays on the argument `Plasma_Init` was handed directly,
before this second emitter is even constructed. Neither `Plasma_Init` nor
`Plasma_Launch` ever writes this emitter's own `+0x38` (radius), so it keeps
`SoundEmitter_Init`'s `200.0` default - the same radius the craft's own
emitter never overrides either (`positional-audio.md`).

**What this engine plays.** `Cue::PlasmaTravel` (`~PLASMATVL`) opens a held,
looping voice the tick a bolt's `charge` first reads `<= 0.0` and follows its
position every tick after, keyed by **projectile slot** rather than by grid
slot - the shape `mine.md`'s own `MINERADAR` note names as the gap nothing in
this engine could address before now (`SfxVoices::plasma_travel`,
`crates/sound/src/sfx.rs`). `Cue::PlasmaHitWall` (`PLASMAHITWALL`) fires
for a wall hit and the 10 s timeout, matching this teardown's own "identical
for either ending" reading; a craft hit (`Impact::struck.is_some()`) plays the
bank's own distinct `Cue::PlasmaHitShip` (`PLASMAHITSHIP`) instead - measured,
not chosen, once [`Plasma_SweepCraftHit`](#plasma_sweepcrafthit-0x0886afb8-confidence-82-is-the-rockets-own-craft-sweep-shape)
was read: it plays `PLASMAHITSHIP` and clears the bolt's own emitter before
`Plasmas_Update`'s pass-two teardown would otherwise play `PLASMAHITWALL`
unconditionally, so the original plays exactly one of the two per ending,
never both. Confirmed present via `oag-wad sounds`, alongside `PLASMA`,
`PLASMAHITWALL` and `~PLASMATVL`, all in `weapons.bnk` (bank `#866`, hash
`01bec824`).

**The age check itself also plays a cosmetic broadcast, separate from the
teardown.** Re-read in full alongside the teardown: `if (p->age > 10.0f) {
p->flags |= 4; if (13 < DAT_08b31048 && p->flags & 1) { ... build a message
from p->position (+0xa0..+0xa8, three floats - the entity's own position, not
a stats block) and the sentinel -1 ...; FUN_0894c784(msg, 0x18, DAT_08aca838,
0x50); } }`. `DAT_08b31048` is `*(0x08b30f90 + 0xb8)` by address arithmetic -
the same global `FUN_0886b898` reads through a different route (see below) -
gating a HUD/network announcement family that also fires on launch (category
`0x40`, `DAT_08aca830`, read off the charge-release branch in the same
function) and, if the read below holds, from `FUN_0886b898` itself (categories
`0xc`/`DAT_08aca350` and unlabelled/`DAT_08aca354`). None of the four sites is
a string - `DAT_08aca350` inspects as sixteen 32-bit values, not text - so
these read as a family of binary message-template calls rather than debug
strings, and none of them is on a path that reads or writes a craft's shield
or `entity+0x110`.

## The expiry settles a damage question nobody had read

**Closed 2026-09-16, as a negative, and it is symmetric.** The one thing this
page's own "What is not verified" list named - whether `FUN_0886b898` (called
per live entity in pass one) is the blast sweep that spends `damage`,
`blastradius` and `blastforce`, and therefore whether a timed-out bolt damages
craft near it - is settled by reading the function in full rather than by
guessing at its shape.

**`FUN_0886b898`'s full decompile references neither `+0xa0` (`damage`), `+0xa4`
(`blastradius`) nor `+0xa8` (`blastforce`) anywhere in its body.** The two
"radius"-shaped floats it does read come from the *active* weapon's own
per-mode stats pointer (`&DAT_08b32420 + DAT_08b32428 * 4)` -
[Plasma_SpeedForClass](#plasma_speedforclass-0x0885c5a4-and-the-launch-ramp)'s
own pointer, so it is genuinely the Plasma's block while this function runs -
at `+0x100` and `+0xe0`. Both are past `+0xc4`, the last offset
`WeaponStats_ParsePlasma` ever writes, so for a Plasma specifically neither is
an authored attribute; whatever sits there is not something the file's own
`<Stats>` element controls.

**Nor does it write `entity+0x110`, the pending-impulse slot every real blast
function spends.** Checked directly against `Missile_ApplyBlastForce`
(`0x08868ea4`, confidence 88, `missile.md`) in the same pass: that function
loops candidate craft, computes a falloff off its own weapon-type's `+0x48`
radius, and ends with `*(target+0x110) += falloff * force * direction` - the
exact shape [contact-response.md](contact-response.md#weapon_postblastimpulse-0x0886794c-confidence-82)
reads for `Weapon_PostBlastImpulse` too. `FUN_0886b898` has no write at that
offset anywhere in its body.

**`Weapon_PostBlastImpulse` has exactly one caller in the whole executable**
- `get_xrefs_to 0x0886794c` returns a single hit: `Mine_SweepCraftTrigger`, the Mine's
own chain (`mine.md`, `contact-response.md`). That one-caller fact is still
true and still means the Plasma never reaches *this specific address*.
~~And `Weapon_PostBlastImpulse` [is] the only function in the binary that
reads `damage`/`blastradius`/`blastforce` off any weapon's stats block and
writes an impulse~~ and ~~No `Plasma_ApplyBlast`-shaped function exists~~ are
both **wrong, corrected the same day below**: `search_functions("Blast")`
missed `Plasma_HitCraft` (`0x0886ad60`) and `Plasma_ApplyBlastForce`
(`0x0886ae08`) for the reason every miss on this page has had - neither was
named in the database yet, so a name search cannot find it. See
[a craft hit is the third ending](#a-craft-hit-is-the-third-ending-and-it-does-spend-a-blast).

**So a Plasma bolt spends its blast on neither the wall ending nor the
timeout ending** - that half of the original claim stands. `Plasmas_Update`'s
teardown (above, re-read the same pass with no elision) is identical for a
wall hit and a `10.0 < age` timeout - `Psys_Release_q`,
`Plasma_SpawnDetonation`, `PLASMAHITWALL`, nothing else - and neither route
into it, nor the teardown itself, nor `FUN_0886b898`, touches a craft's
shield or `entity+0x110`. `oag_weapons::projectile::flight`'s expiry arm is
ported to this reading, `blast: false`, on 2026-09-16 - see
[`Impact::blast`](../../../../crates/weapons/src/projectile.rs)'s doc comment
and `crates/weapons/src/projectile/plasma/tests.rs`. ~~Read at the time as
"neither ending"~~ - a **third** ending, a direct craft hit, was not yet
enumerated when that line was written; it is read and ported later the same
day, below.

~~A conflict this settles rather than closes: the wall-hit branch in
`Projectiles::advance` still credits `blast: true` for the Plasma, and this
reading says the original spends nothing there either.~~ **Resolved, not
just flagged, later the same day**: the wall-hit branch now emits
`blast: false` for the Plasma, matching this reading, and the craft-hit
branch it shared code with now emits `blast: true` routed to the new
direct-hit rule - see
[a craft hit is the third ending](#a-craft-hit-is-the-third-ending-and-it-does-spend-a-blast).

### What `FUN_0886b898` actually spends its two radii on - a hypothesis, not a name

Mechanically, the function sweeps two entity lists every tick a bolt is
flying (not gated on the destroy bit at all):

- **Pass one**: count at `+0x164`, pointer array at `+0x64` off
  `*(pool+0xac)` - exactly the Mine's own pool shape, per
  [the pool comparison](#weapon_fireplasma-0x0886a868-fires-exactly-one)
  above (`+0x164`/`+0x64`). Radius: `stats+0x100`.
- **Pass two**: count at `+0xc4`, pointer array at `+0x44` off `*(pool+0xb0)`
  - exactly the Bomb's own pool shape (`+0xc4`/`+0x44`,
  `contact-response.md`'s read of `FUN_08863a20`). Radius: `stats+0xe0`.

For each struck entity within its pass's radius, it sets `entity+0x3c |= 4` -
the same "destroy" bit `mine.md` documents as consumed by `Mine_SpawnExplosion`
- and, gated on the same `13 <` race-mode counter the age check above uses,
runs the same message-broadcast shape (`FUN_0885ebc4` + `FUN_0894c784`,
categories `0xc` and unlabelled/`DAT_08aca354`).

**Read as "a flying Plasma bolt can chain-detonate a nearby Mine or Bomb
early," this would be a third, previously undocumented way for either of
those weapons to die** - alongside their own fuse-timeout and, for the Mine,
its own `trigger_radius` sweep (`mine.md`). It is not named or claimed at that
strength here, for one specific reason: **the radius it reaches for is
unauthored for a Plasma.** `+0x100` and `+0xe0` sit past the eleven offsets
`WeaponStats_ParsePlasma` ever writes, so nothing in the file controls what a
Plasma's own block holds there - it could be a genuinely shared field written
by some common part of `WeaponStats_Parse` this page has not located, or it
could be whatever happened to be in memory. Distinguishing those needs reading
`WeaponStats_Parse`'s own common-attribute path (if it has one) or measuring
the value live, neither done here. Confidence on the mechanical shape (which
two pools, which bit, which flag family): decent, from three independent
matches to already-documented pool layouts. Confidence on what it is *for*:
**under 50**, so `FUN_0886b898` is not renamed - the rubric's own line for
this case. Left for whoever reads `WeaponStats_Parse`'s common block or sets a
breakpoint on `+0x100` next.

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
This engine rides the glow and, as of 2026-09-16, ramps it too - see
[`PLASMA_FLARE_EFFECT`](../../../../crates/title/src/engine_effects.rs)'s
doc comment for which half landed and why.

**`craft+0x6d` is the internal/cockpit camera flag, closed the same day by two
independent readings elsewhere on this disc, and the `* 0.5` is now ported.**
[`camera.md`](camera.md)'s SELECT-view cycle writes it `1` for the internal
tripod and `0` for both external ones, on the player craft alone; confidence
88 for the cycle, 70 for the flag's own meaning there. `Ship_ApplyShield`'s
own animation (see [`shield-pickup.md`](shield-pickup.md)) is a second,
independent consumer - it picks the cockpit shell over the hull shell on
exactly the same byte - which is what raises the flag's own confidence to
**82**: two unrelated call sites reading the same offset the same way is
corroboration, not the same finding counted twice. This engine already
modelled the byte as `oag_display::CameraView::draws_own_ship` before this
page ever asked what `+0x6d` was for, so the Plasma's own consumer is a third
site rather than a new one - `weapons::visuals::plasma_flare_scale` now takes
a `cockpit` parameter, `projectile.owner == 0 && !Race::draws_own_ship()`,
since the flag is only ever written for the player's own craft (an
opponent's charging bolt reads a byte the original never sets).

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
now measured rather than "not found yet". `oag_weapons::projectile::plasma::CHARGE_SECONDS`
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
first second of flight. **Ported 2026-09-16.** `oag_weapons::projectile::flight`'s
`pinned_kmh` reads the craft's own velocity at the tick the charge ends -
`crates/weapons/src/projectile/flight.rs`'s charging branch - and blends it
to the class speed with `missile::speed_kmh`, the identical linear form
`Missile_SpeedNow` (`0x0885a038`) independently tests over the same second;
see that function's own `SPEED_RAMP_SECONDS` doc comment. This is no longer
`oag_weapons::projectile::launch`'s shared choice with the Rocket - that
function's own doc comment now says so - the Rocket alone still flies at
`class + launchspeed` throughout, unread rather than chosen.

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

## A craft hit is the third ending, and it does spend a blast

**Closed 2026-09-16, later the same day as
[the expiry settles a damage question](#the-expiry-settles-a-damage-question-nobody-had-read).**
That section proved a wall hit and the `10.0 < age` timeout spend nothing.
It did not ask about a **direct craft hit**, because nothing on this page had
yet traced the call chain that reaches one - `search_functions("Blast")`
cannot find an unnamed function, and the two functions this section is about
were both still `FUN_xxxxxxxx` until this pass.

**The chain starts in `Plasmas_Update` (`0x0886b490`) itself**, one line this
page's own decompile of it already carried but had not followed:

```c
if (p->charging == 0) {
    Plasma_Update(dt, p, g_world);
    if (p->flags & 1) Plasma_SweepCraftHit(pool, i);   // 0x0886afb8
    FUN_0886b898(pool, i);                             // the Mine/Bomb sweep, unrelated
}
```

`p->flags & 1` is set the moment a bolt is fired (`Weapon_FirePlasma` writes
`p->flags = 1`) and nothing ever clears it before a hit, so
`Plasma_SweepCraftHit` runs every tick a bolt is flying - not conditionally,
despite the look of the guard. `Rocket_HitCraft`'s own caller,
`FUN_0886e7ac`, is called from `RocketPool_Update` behind the identical
`flags & 1` guard, which is the structural match that pointed at this
function in the first place, alongside the wrong turn a previous pass took:
the comment shape `if (p->flags & 1) Plasma_NetSend_q(pool, i)` in this page's
2026-09-16 morning read of `Plasmas_Update` was a **prose label that was never
a database name** - `search_functions("Plasma")` never lists a
`Plasma_NetSend_q`, and the address it was hung on is `Plasma_SweepCraftHit`,
not netcode. This is the same trap `apply-ghidra-names.py`'s own history
records for `Rocket_HitCraft`'s sibling functions: an informal name in a
decompile comment reads exactly like a real one and is not searchable as one.

### `Plasma_SweepCraftHit` (`0x0886afb8`, confidence 82) is the Rocket's own craft-sweep shape

Read in full. Per tick, per flying bolt, it walks every live craft
(`DAT_08b30f90` count, the same global the Rocket's and the Mine's own sweeps
use) except the bolt's **own firer** (`uVar17 != *(plasma+0x40)`, the `owner`
field `Weapon_FirePlasma` writes), and tests the bolt's swept segment against
each craft's hull with a cylinder test - a perpendicular-distance dot product
inside `-6.0 < d < 6.0`, the same bound `Rocket_HitCraft`'s own caller
(`FUN_0886e7ac`) uses. On a hit it:

1. Sets the bolt's own destroy bit, `plasma+0x3c |= 4` - the same bit a wall
   hit and the timeout set, so a craft-hit bolt reaches the identical
   `Plasmas_Update` pass-two teardown already read above.
2. Calls `Plasma_SpawnDetonation` (`0x0886ac88`) **inline**, gated on
   `FUN_0883e37c`, in addition to the unconditional call pass-two makes for
   any destroy-bit entity. Whether this double-fires the flash or
   `FUN_0883e37c` degates the second call is not chased here - `blast_models`
   owns that function and this page does not touch it.
3. Plays `PLASMAHITSHIP` (`0x08a7c998`) through the bolt's own emitter and
   immediately stops and clears that emitter (`+0x5c = 0`) - which is why
   pass-two's later, unconditional `if (p->emitter != 0) Sound_Play(...,
   "PLASMAHITWALL", ...)` does not also fire on this path: the emitter it
   tests is already cleared by the time pass-two runs. No double sound, by
   construction rather than by a branch on which ending this is.
4. Calls `Plasma_HitCraft` (`0x0886ad60`) with the struck craft's slot and the
   bolt's own pool slot.
5. Calls `Plasma_ApplyBlastForce` (`0x0886ae08`) with the hit point and the
   bolt's `owner`.
6. Runs the same race-mode-gated (`13 < DAT_08b31048`) HUD/network broadcast
   family the age-timeout branch and the launch branch already use.

Confidence 82, not higher: the routing (which two functions it calls, in what
order, on what condition) is solid, but the cylinder test's own VFPU geometry
was read enough to identify the bound and not walked past that. No `_q` since
the routing - the load-bearing half for this page's own question - is not in
doubt.

### `Plasma_HitCraft` (`0x0886ad60`, confidence 88) credits the struck craft, unconditionally

```c
void Plasma_HitCraft(Pool *pool, int craft_slot, int plasma_slot) {
    Body *body = *(Body **)(pool + craft_slot*4 + 0x44);   // the struck craft's own physics body
    if (*(int *)(pool + plasma_slot*4 + 100)->owner == DAT_08b36c08)   // one specific craft slot, unmeasured
        body->flag_0x124 = 1;                               // Ship_Damage's own 5th argument
    Stats *stats = active_weapon_stats();                   // &DAT_08b32420 + DAT_08b32428*4, the Plasma's own block
    body->pending_damage   += stats->damage;                 // +0x120 += +0xa0
    body->hits_taken       += 1.0f;                          // +0x12c (0x300 decimal), a counter
    body->pending_slowdown += stats->slowdown_time;          // +0x130 += +0xc4
    body->pending_weapon_id = 2;                              // +0x138, meaning not chased
    body->pending_attacker  = plasma->owner;                  // +0x13c, the firer's own slot
}
```

Every offset on the right is `WeaponStats_ParsePlasma`'s own table
(`damage` at `+0xa0`, `slowdown_time` at `+0xc4`) - an exact match, the same
strength of evidence that carried `Rocket_HitCraft`'s reading of the
Rocket's own `+4`/`+0x2c` pair. `body+0x120`/`+0x130` are the exact fields
`Ship_ApplyPendingWeaponDamage` (`0x0883f13c`) and
`Ship_ApplyPendingWeaponRepair`-adjacent code already drain each tick, per
this project's own read of that function. **Full damage and full
`slowdown_time`, credited unconditionally** - there is no distance test here
at all, because the struck craft is already known; the falloff term this
page's own `blast()` and the original's own `Weapon_PostBlastImpulse` both
apply is a *force* term, computed separately, next.

`body+0x138 = 2` is a smaller, separate enumeration from the `weapon_id = 7`
`Weapon_RequestFire`'s jump table uses for the Plasma (`weapon-fire.md`) -
`Rocket_HitCraft` writes `0` at the same offset, so this is some kind of
per-family damage-source tag `Ship_Damage` reads for its own purposes
(sound/visual selection, most likely), not the fire-dispatch weapon id. Not
chased further; recorded so nobody re-derives the "7" collision a second
time.

`DAT_08b36c08`, compared against the firer's own slot to decide whether to
set `body+0x124`, reads as "is the firer one specific craft" (the local
player's, most likely, given the byte feeds `Ship_Damage`'s hit-reaction
argument) - not measured, confidence under 50, not renamed.

### `Plasma_ApplyBlastForce` (`0x0886ae08`, confidence 88) pushes everyone but the firer

```c
void Plasma_ApplyBlastForce(Pool *pool, Vec3 *point, int owner_slot) {
    for (int i = 0; i < craft_count; i++) {
        if (i == owner_slot) continue;                  // the firer alone is excluded
        Body *body = craft_body(i);
        Vec3 offset = body->hit_anchor /* +0x50 */ - *point;
        float d = length(offset);
        Stats *stats = active_weapon_stats();            // the Plasma's own block, same pointer as above
        if (d < stats->blastradius /* +0xa4 */) {
            Vec3 dir = normalize(offset);
            body->pending_impulse /* +0x110 */ +=
                dir * (1.0f - d / stats->blastradius) * stats->blastforce /* +0xa8 */;
        }
    }
}
```

`+0xa4`/`+0xa8` are `blastradius`/`blastforce` on `WeaponStats_ParsePlasma`'s
own table, and `+0x110` is the exact pending-impulse slot
`Missile_ApplyBlastForce` (`0x08868ea4`, confidence 88, `missile.md`) and
`Weapon_PostBlastImpulse` (`0x0886794c`) both write, with the identical
`(1.0 - d/radius) * force` falloff this project's own `blast()` already
implements for every other weapon. `Weapon_PostBlastImpulse`'s "exactly one
caller in the whole executable" is still true of *that address* -
`Plasma_ApplyBlastForce` is a second, Plasma-owned copy of the same
arithmetic, the same relationship `Missile_ApplyBlastForce` already has to
it. This is why "no `Plasma_ApplyBlast`-shaped function exists" (written
before this pass, struck above) was wrong: the search that would have found
it needed the name, and the name did not exist yet.

**The loop excludes only `owner_slot` (the firer), never the struck craft
itself.** `point` is one endpoint of the bolt's swept segment for this tick
(`FUN_0885c154`'s output, passed straight through from `Plasma_SweepCraftHit`)
rather than the exact hull contact point - close enough that the struck craft
sits at or near `d = 0` and takes a falloff term close to the full
`blastforce`, *in addition to* the direct `damage`/`slowdown_time`
`Plasma_HitCraft` already credited it. **This means `Impact`'s own doc
comment claim that "the firing craft is not excluded" is backwards for the
Plasma's own craft-hit ending** - the firer is the one craft explicitly
excluded from this sweep, and the struck craft (which may be the firer's own
victim, never the firer, since a bolt cannot hit its own launcher - see
[`Plasma_Update`](#plasma_update-0x0885c6cc-is-rocket_updates-floor-follower)'s
sweep, which already excludes the owner's hull) is not excluded at all.

### What was ported

`crates/weapons/src/projectile/flight.rs`, both `Weapon::Plasma` arms that
used to share code with the Rocket:

- The floor-probe wall branch (`Some(hit) if matches!(kind, Rocket | Plasma)`,
  ~line 213): `blast` is now `kind != Weapon::Plasma` - the Rocket keeps
  `blast: true` unconditionally (untouched, out of scope), the Plasma gets
  `blast: false`, matching the wall reading above.
- The travel-sweep hit branch (the non-bounce `else` arm, ~line 319): `blast`
  is now `kind != Weapon::Plasma || struck.is_some()` - a Plasma wall hit
  here (`struck: None`, the sweep found a wall ahead rather than a floor
  below) also gets `blast: false`; a Plasma craft hit (`struck: Some(_)`) gets
  `blast: true`, routed to the new rule below. Every other weapon's `blast`
  is unchanged.

`crates/weapons/src/projectile/blast.rs` gains
`blast_direct_hit(ships, point, stats, struck, owner, rules, absorbed)`,
called from `apply_impacts` only for `impact.kind == Weapon::Plasma &&
impact.struck.is_some()`. It credits `damage` and `slowdown_time` to `struck`
unconditionally (matching `Plasma_HitCraft`'s own lack of a distance test),
then sweeps every **active** craft except `owner` for a
`(1.0 - d/radius) * force` impulse within `blastradius` - matching
`Plasma_ApplyBlastForce` exactly, struck craft included (it is not excluded
in the original either). `blast()` itself is untouched; every other weapon
still goes through it exactly as before.

**Not ported: the Rocket's own equivalent split.** `Rocket_HitCraft`
(`0x0886ebdc`, already named, confidence not raised or lowered here) shows
the identical shape - direct `damage`/`slowdown_time` to the struck craft via
its own function, no radius sweep alongside it in the read this page did -
which means `blast()`'s current "full damage to everyone in radius" rule is
likely just as wrong for the Rocket as it was for the Plasma. Recorded as a
lead, not fixed: the brief this page answers is the Plasma's craft-hit
question specifically, and the Rocket's own wall-hit/craft-hit split (does
`RocketPool_Update`'s teardown ever damage anyone at all, or only
`Rocket_HitCraft`'s own direct credit?) is its own unread question with its
own regression story.

## What is not verified

- ~~**Where `charge_time` is spent.**~~ **Closed 2026-09-09 as a negative**,
  and the negative is calibrated - see above. Nothing reads it in either PSP
  executable; the wind-up it looks like it describes is a separate, hardcoded
  1.0 s.
- ~~**What the teardown's other two calls do, and whether `FUN_0886b898` is the
  blast sweep.**~~ **Closed 2026-09-16 as a negative.** `Psys_Release_q`
  (`FUN_088f3298`) plainly stops the riding head instance;
  `FUN_0886b898` is read in full and touches neither
  `damage`/`blastradius`/`blastforce` nor `entity+0x110` - see
  [the expiry settles a damage question](#the-expiry-settles-a-damage-question-nobody-had-read).
  A timed-out bolt draws the explosion and damages nobody, exactly like a wall
  hit.
- **What `FUN_0886b898` actually spends its two radii on.** Mechanically a
  sweep of the Mine's and the Bomb's own pool shapes, setting the same
  "destroy" bit their own fuse-timeout does - see
  [the hypothesis section](#what-fun_0886b898-actually-spends-its-two-radii-on---a-hypothesis-not-a-name).
  Under 50 confidence on what it is *for*, because the radius it reads is
  unauthored for a Plasma; not renamed.
- ~~**The wall-hit branch in `Projectiles::advance` still credits `blast: true`
  for the Plasma**, which the reading above says the original does not do
  either. Flagged, not fixed.~~ **Closed 2026-09-16, same day, second pass.**
  The wall-hit and craft-hit arms shared one code path in the engine; they are
  split now, and the craft-hit arm carries the new direct-hit rule below - see
  [a craft hit is the third ending](#a-craft-hit-is-the-third-ending-and-it-does-spend-a-blast).
- **What `Plasma_SweepCraftHit` (`0x0886afb8`) actually tests, past the hull
  cylinder.** Read enough to find and route its two calls
  (`Plasma_HitCraft`/`Plasma_ApplyBlastForce`); its own VFPU-heavy geometry
  (the plane/cone tests before the `-6.0 < d < 6.0` cylinder check) was not
  independently re-derived past confirming it mirrors `Rocket_HitCraft`'s own
  caller shape - hence 82, not higher, and no `_q` since the *routing* is solid
  even where the geometry detail is not fully walked.
- ~~**`Plasma_SpawnDetonation`'s double call on a craft hit.** `Plasma_SweepCraftHit`
  calls it inline (gated on `FUN_0883e37c`) and `Plasmas_Update`'s own pass-two
  teardown calls it again, unconditionally, for any entity left with the destroy
  bit set - which a craft hit always leaves set. Whether this is a genuine
  double-spawned flash or `FUN_0883e37c` degates it is not chased; owned by the
  `blast_models` lane, not renamed or touched here.~~ **Settled 2026-09-16 - see
  [a craft hit spawns two detonation objects](#a-craft-hit-spawns-two-detonation-objects-and-fun_0883e37c-does-not-degate-it)
  below: `FUN_0883e37c` evaluates true in every mode this project plays, so a
  craft hit genuinely double-spawns.**
- **`DAT_08b36c08`**, compared against a Plasma bolt's owner inside
  `Plasma_HitCraft` to decide whether to set the struck craft's own
  `+0x124` byte (the flag `Ship_Damage` reads as its fifth argument). Reads as
  "is the firer a specific craft slot" - the local player's, most likely - but
  not measured; under 50, not renamed.
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
  constructor every object of this kind runs first, `Object_ConstructBase`, does not
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

- **2026-09-16, later still.** `PLASMAHITSHIP` wired for the craft-hit ending,
  closing the "chosen, not measured" gap the `plasma-cues` session left open
  the same day: `oag_sound::sfx::Cue::PlasmaHitShip` fires for
  `Impact { kind: Plasma, struck: Some(_), .. }` and `PlasmaHitWall` for
  everything else, matching `Plasma_SweepCraftHit`'s own emitter-clear-then-play
  order read above. Confirmed in `weapons.bnk` (bank `#866`, hash `01bec824`)
  by `oag-wad sounds`: `PLASMAHITSHIP` is cue 14, 4 waveforms, 0 looping,
  3.59s total - the same shape `PLASMAHITWALL` already carries. A real-disc
  ground truth run of the sibling wall-hit test
  (`crates/game/tests/plasma_cues_audio_ground_truth.rs`) turned up something
  this page had not accounted for: `Mode::SingleRace` fields a full eight-craft
  grid unconditionally whenever the track authors a `Start Position` (every
  shipped circuit does), so a bolt fired early in a real race can meet a real
  opponent instead of a wall - measured on Talons Junction, it does, at tick
  82. That test is now robust to either ending rather than asserting
  `PLASMAHITWALL` specifically, and a second ground-truth test pins the
  craft-hit ending down deterministically by repurposing an already-active
  opponent slot as a point-blank target.
- **2026-09-16, later still.** `craft+0x6d`, read above in
  ["the charge is real"](#the-charge-is-real-and-it-is-not-charge_time), is
  the internal/cockpit camera flag - confidence raised 70 -> 82 the same
  session, corroborated by a second independent consumer in
  [`shield-pickup.md`](shield-pickup.md) (`camera.md` itself is corrected to
  match, since that raise was recorded there before but never actually
  applied). `weapons::visuals::plasma_flare_scale` now takes the `cockpit`
  parameter this page's own "not ported" note used to flag as unread -
  `projectile.owner == 0 && !Race::draws_own_ship()`, since the byte is only
  ever written for the player's own craft.
- **2026-09-16, later pass.** A direct craft hit is a third ending the
  morning pass did not enumerate. `Plasmas_Update`'s own `if (p->flags & 1)`
  call, previously prose-labelled `Plasma_NetSend_q` with no database name
  behind it, is `Plasma_SweepCraftHit` (`0x0886afb8`, now named, 82): the
  Rocket's own craft-sweep shape, gated the same way `RocketPool_Update`
  gates `FUN_0886e7ac`. It calls `Plasma_HitCraft` (`0x0886ad60`, 88), which
  credits full `damage`/`slowdown_time` to the struck craft unconditionally
  off `WeaponStats_ParsePlasma`'s own `+0xa0`/`+0xc4`, and
  `Plasma_ApplyBlastForce` (`0x0886ae08`, 88), which pushes every craft but
  the bolt's own firer with a `(1 - d/blastradius) * blastforce` impulse -
  the exact shape `Weapon_PostBlastImpulse` and `Missile_ApplyBlastForce`
  already carry, corrected from this page's own "no `Plasma_ApplyBlast`-shaped
  function exists" (wrong; the function just was not named yet). Ported to
  `oag_weapons::projectile::flight`'s two Plasma/Rocket-shared branches,
  split by kind so the Rocket is untouched, and to a new
  `blast::blast_direct_hit`. The Rocket's own `Rocket_HitCraft` shows the
  identical split-shape and is left as a lead, not fixed - see
  [a craft hit is the third ending](#a-craft-hit-is-the-third-ending-and-it-does-spend-a-blast).
- **2026-09-16.** `FUN_0886b898` read in full and `Plasmas_Update`'s teardown
  re-read with no elision, closing the last item on the 2026-09-09 pass's own
  "not verified" list: a Plasma bolt spends no blast on either ending, wall or
  timeout. `Weapon_PostBlastImpulse`'s only caller in the binary is confirmed
  to be the Mine's own chain (`get_xrefs_to`), and no `Plasma_ApplyBlast`-shaped
  function exists. Ported to `oag_weapons::projectile::flight`'s expiry arm;
  the wall-hit branch's own conflicting `blast: true` is flagged, not touched.
  `FUN_0886b898` itself is read as a mechanical sweep of the Mine's and the
  Bomb's pool shapes but not renamed - the radius it reads is unauthored for a
  Plasma, which holds its positive identity under 50.
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
- **2026-09-16, `plasma-flash-ai` pass.** `FUN_0883e37c` read in full
  (disassembly, not just the decompile, which drops its one argument) and
  settles the double-call question the `blast_models` lane left open: see
  [a craft hit spawns two detonation objects](#a-craft-hit-spawns-two-detonation-objects-and-fun_0883e37c-does-not-degate-it).
  Not renamed - the mechanism is certain, the subsystem it belongs to is not.

## A craft hit spawns two detonation objects, and `FUN_0883e37c` does not degate it

**The decompiler's `bool FUN_0883e37c(void)` signature is wrong: it drops the
one argument the call site passes.** The disassembly shows no register setup
before `jal 0x0883e434` beyond what the caller already left in `a0`, so the
argument (`*(struck_craft.body + 0xf0)`, a back-pointer of some kind) passes
straight through. Read whole, stripped of the VFPU-free integer noise:

```c
bool FUN_0883e37c(void *entity) {
    int class_or_slot = (char)*(byte *)(*(int *)(entity + 0xae4) + 0x60);  // FUN_0883e434
    if (class_or_slot & 0xffffffc0) return true;      // out of range (e.g. a signed -1 sentinel)
    uint64_t mine = FUN_0891e908(g_display);          // this local viewer's own 64-bit mask
    uint64_t bit  = (uint64_t)1 << class_or_slot;      // FUN_0897bc08 is a generic 64-bit variable shift
    return (mine & bit) != 0;
}
```

`FUN_0891e908(param_1)` reads: if `DAT_08abff58` (a viewport/local-player
count) is `> 1` and the index is in range, index `param_1 + idx*8` for a
per-viewport 64-bit mask; otherwise return the fallback pair
`DAT_08a885c8`/`DAT_08a885cc` unconditionally, ignoring `param_1` entirely -
so `g_display` is only consulted at all in split-screen. `FUN_0897bc08` is
confirmed to be exactly `(hi:lo) << shift` on 64-bit operands split across two
32-bit registers - the classic idiom a MIPS compiler emits for a
variable-width 64-bit shift it cannot do natively.

**Both constants were read directly out of the image rather than assumed:**

| Address | Bytes | Reading |
| --- | --- | --- |
| `0x08a7b928`/`2c` (the `1ULL` fed to the shift) | `01 00 00 00 00 00 00 00` | confirms the "compute `1<<n`" idiom |
| `0x08a885c8`/`cc` (the non-split-screen fallback mask) | `ff ff ff ff ff ff ff ff` | **every bit set** |

So outside split-screen - which is every mode this project plays or has wired
- `FUN_0891e908` always returns all-ones, `class_or_slot` is masked against
0..63 either way (the AI/no-viewport sentinel case returns `true` even
earlier, without touching the mask at all), and **`(mine & bit) != 0` is true
for every value the byte can hold.** `FUN_0883e37c` cannot return `false` in
single-player. It does not degate the second call - it is a split-screen
"does the current local viewer's own class mask enable this" gate that is
compiled out to "always yes" everywhere this project's own play is concerned.

**Answering the brief's question directly: two, not one, and it is not a
guess-dressed-as-a-name situation, it is a measured fact.** A craft hit:

1. `Plasma_SweepCraftHit` sets the bolt's own destroy bit (`plasma+0x3c |= 4`)
   and, gated on the now-always-true `FUN_0883e37c`, calls
   `Plasma_SpawnDetonation` inline with `&local_380` - a copy of **the struck
   craft's own body position** (`craft.body+0x50`, the same field
   `Plasma_ApplyBlastForce` reads as its per-craft `hit_anchor`).
2. Because the destroy bit is now set, `Plasmas_Update`'s own pass-two
   teardown - unconditional for any destroy-bit entity, in the **same**
   `Plasmas_Update` invocation, later the same tick - calls
   `Plasma_SpawnDetonation` again, this time with `&p->position`, **the
   bolt's own last position**.

**Same models, different positions.** `Plasma_SpawnDetonation` takes no
identity/dedup argument and no static state that would make a second call a
no-op: it always allocates a fresh `0x170`-byte object
(`FUN_08946e40(0x170, &DAT_08a7c9b0, 0x21a)`) and always calls
`PlasmaBlast_Construct` on it, so the two calls are two independent
`PlasmaBlast` instances, each playing its own baked 1.5 s animation over the
identical three-model set `PlasmaBlast_Construct` always builds - see
[the blast's own per-tick animation](#the-blast-objects-own-per-tick-animation-plasmablast_update).
The two positions are close (the struck craft's body position versus the
bolt's own position on the tick that swept into it, within the `-6.0 < d <
6.0` cylinder `Plasma_SweepCraftHit` tests) but not identical, so this reads
as "the flash draws once where the ship got hit and once where the bolt was,"
not a literal duplicate of the same object.

**Confidence 88 on the behaviour** (every step is a decompile or a
disassembly plus two directly-read memory constants, not inference), **under
50 on `FUN_0883e37c`'s own identity** - "a per-local-viewer, per-craft-class
visibility mask" is the shape the code has, but neither `entity+0xae4`'s
struct nor `+0x60`'s exact meaning ("local player index", guessed) is
corroborated anywhere else, so it is left as `FUN_0883e37c` rather than
renamed on one reading.

**Not ported.** The fix belongs in `oag_weapons::projectile::blast` /
`Impact` and the visuals that consume it (`blast_direct_hit`, the render
side) - both outside this pass's owned files (`crates/weapons/src/projectile/**`
and `crates/raceplay/src/weapons/visuals.rs` are `plasma-speed-blend`'s and
`blast_models`'s lanes respectively). This section is the read those lanes
need to act on: a Plasma craft hit should spawn **two** `Impact`-triggered
flash/blast instances, one at the struck craft's position and one at the
bolt's, not one.
