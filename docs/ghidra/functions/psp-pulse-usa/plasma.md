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
- `get_xrefs_to 0x0886794c` returns a single hit: `FUN_08867b50`, the Mine's
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
shield or `entity+0x110`. `oag_gameplay::projectile::flight`'s expiry arm is
ported to this reading, `blast: false`, on 2026-09-16 - see
[`Impact::blast`](../../../../crates/gameplay/src/projectile.rs)'s doc comment
and `crates/gameplay/src/projectile/plasma/tests.rs`. ~~Read at the time as
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
[`PLASMA_FLARE_EFFECT`](../../../../crates/game/src/race/effect_names.rs)'s
doc comment for which half landed and why, and for the one piece not
ported (the `* 0.5` flag, unread).

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
first second of flight. **Ported 2026-09-16.** `oag_gameplay::projectile::flight`'s
`pinned_kmh` reads the craft's own velocity at the tick the charge ends -
`crates/gameplay/src/projectile/flight.rs`'s charging branch - and blends it
to the class speed with `missile::speed_kmh`, the identical linear form
`Missile_SpeedNow` (`0x0885a038`) independently tests over the same second;
see that function's own `SPEED_RAMP_SECONDS` doc comment. This is no longer
`oag_gameplay::projectile::launch`'s shared choice with the Rocket - that
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
despite the look of the guard. `Rocket_HitCraft_q`'s own caller,
`FUN_0886e7ac`, is called from `RocketPool_Update` behind the identical
`flags & 1` guard, which is the structural match that pointed at this
function in the first place, alongside the wrong turn a previous pass took:
the comment shape `if (p->flags & 1) Plasma_NetSend_q(pool, i)` in this page's
2026-09-16 morning read of `Plasmas_Update` was a **prose label that was never
a database name** - `search_functions("Plasma")` never lists a
`Plasma_NetSend_q`, and the address it was hung on is `Plasma_SweepCraftHit`,
not netcode. This is the same trap `apply-ghidra-names.py`'s own history
records for `Rocket_HitCraft_q`'s sibling functions: an informal name in a
decompile comment reads exactly like a real one and is not searchable as one.

### `Plasma_SweepCraftHit` (`0x0886afb8`, confidence 82) is the Rocket's own craft-sweep shape

Read in full. Per tick, per flying bolt, it walks every live craft
(`DAT_08b30f90` count, the same global the Rocket's and the Mine's own sweeps
use) except the bolt's **own firer** (`uVar17 != *(plasma+0x40)`, the `owner`
field `Weapon_FirePlasma` writes), and tests the bolt's swept segment against
each craft's hull with a cylinder test - a perpendicular-distance dot product
inside `-6.0 < d < 6.0`, the same bound `Rocket_HitCraft_q`'s own caller
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
strength of evidence that carried `Rocket_HitCraft_q`'s reading of the
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
`Rocket_HitCraft_q` writes `0` at the same offset, so this is some kind of
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

`crates/gameplay/src/projectile/flight.rs`, both `Weapon::Plasma` arms that
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

`crates/gameplay/src/projectile/blast.rs` gains
`blast_direct_hit(ships, point, stats, struck, owner, rules, absorbed)`,
called from `apply_impacts` only for `impact.kind == Weapon::Plasma &&
impact.struck.is_some()`. It credits `damage` and `slowdown_time` to `struck`
unconditionally (matching `Plasma_HitCraft`'s own lack of a distance test),
then sweeps every **active** craft except `owner` for a
`(1.0 - d/radius) * force` impulse within `blastradius` - matching
`Plasma_ApplyBlastForce` exactly, struck craft included (it is not excluded
in the original either). `blast()` itself is untouched; every other weapon
still goes through it exactly as before.

**Not ported: the Rocket's own equivalent split.** `Rocket_HitCraft_q`
(`0x0886ebdc`, already named, confidence not raised or lowered here) shows
the identical shape - direct `damage`/`slowdown_time` to the struck craft via
its own function, no radius sweep alongside it in the read this page did -
which means `blast()`'s current "full damage to everyone in radius" rule is
likely just as wrong for the Rocket as it was for the Plasma. Recorded as a
lead, not fixed: the brief this page answers is the Plasma's craft-hit
question specifically, and the Rocket's own wall-hit/craft-hit split (does
`RocketPool_Update`'s teardown ever damage anyone at all, or only
`Rocket_HitCraft_q`'s own direct credit?) is its own unread question with its
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
  independently re-derived past confirming it mirrors `Rocket_HitCraft_q`'s own
  caller shape - hence 82, not higher, and no `_q` since the *routing* is solid
  even where the geometry detail is not fully walked.
- **`Plasma_SpawnDetonation`'s double call on a craft hit.** `Plasma_SweepCraftHit`
  calls it inline (gated on `FUN_0883e37c`) and `Plasmas_Update`'s own pass-two
  teardown calls it again, unconditionally, for any entity left with the destroy
  bit set - which a craft hit always leaves set. Whether this is a genuine
  double-spawned flash or `FUN_0883e37c` degates it is not chased; owned by the
  `blast_models` lane, not renamed or touched here.
- **`DAT_08b36c08`**, compared against a Plasma bolt's owner inside
  `Plasma_HitCraft` to decide whether to set the struck craft's own
  `+0x124` byte (the flag `Ship_Damage` reads as its fifth argument). Reads as
  "is the firer a specific craft slot" - the local player's, most likely - but
  not measured; under 50, not renamed.
- **The blast object's own animation.** `PlasmaBlast_Construct` builds three
  ramps over its three models and nothing here reads what advances them.
- **`FUN_0885c650`**, the second speed lookup, deliberately unnamed - see
  above. `0x0885c5a4` left this list on 2026-09-09 by being decompiled.
- **The bodies of the four unbuilt fire handlers** named at 82 in the table
  above. Only their dispatch is read.
- **`g_ride_height`** (`_DAT_002acf08`), the constant a redirected bolt is
  lifted off the surface by. Read as a global, its value not sampled.

## History

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
  `oag_gameplay::projectile::flight`'s two Plasma/Rocket-shared branches,
  split by kind so the Rocket is untouched, and to a new
  `blast::blast_direct_hit`. The Rocket's own `Rocket_HitCraft_q` shows the
  identical split-shape and is left as a lead, not fixed - see
  [a craft hit is the third ending](#a-craft-hit-is-the-third-ending-and-it-does-spend-a-blast).
- **2026-09-16.** `FUN_0886b898` read in full and `Plasmas_Update`'s teardown
  re-read with no elision, closing the last item on the 2026-09-09 pass's own
  "not verified" list: a Plasma bolt spends no blast on either ending, wall or
  timeout. `Weapon_PostBlastImpulse`'s only caller in the binary is confirmed
  to be the Mine's own chain (`get_xrefs_to`), and no `Plasma_ApplyBlast`-shaped
  function exists. Ported to `oag_gameplay::projectile::flight`'s expiry arm;
  the wall-hit branch's own conflicting `blast: true` is flagged, not touched.
  `FUN_0886b898` itself is read as a mechanical sweep of the Mine's and the
  Bomb's pool shapes but not renamed - the radius it reads is unauthored for a
  Plasma, which holds its positive identity under 50.
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
