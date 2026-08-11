# Firing a weapon: the request word, and the Rocket's three

**Binary:** `pulse-psp` `BOOT.BIN`, image base `0x08804000`.

**Status:** the fire *dispatch* is read end to end at confidence **80**, and with
it the word this project has been calling unread for three passes -
**`entity+0x1b8` is the weapon-fire request word**, one bit per weapon, each bit
calling its own handler.

**And the Rocket fires three, simultaneously, fanned by `spread`.** Confidence
**88**: `Weapon_FireRocket` (`0x0886e104`) spawns one projectile straight ahead,
then rotates the craft's matrix by `+spread` and spawns a second, then by
`-spread` and spawns a third - three calls to one spawn helper in one
invocation, with no timer between them. `spread` is read from the Rocket stats
block at `+0x24`, which is **the consumer this page set out to find**.

Unmeasured, so capped at **84** by the
[confidence rubric](../../../reverse-engineering/confidence-rubric.md).

| Address | Name | Confidence |
| --- | --- | --- |
| `0x0880c058` | `WeaponStats_ParseRocket` | 88 |
| `0x08861814` | `Weapons_DispatchFire` | 80 |
| `0x0886e104` | `Weapon_FireRocket` | 88 |
| `0x0886b038` | `Rocket_Spawn_q` | 78 |
| `0x088675cc` | `Weapon_UpdateBurstFire_q` | 72 |
| `0x08869588` | `Projectiles_Update_q` | 75 |

## Read this first: the disassembly's addresses are image-base-relative

**A trap that costs an hour if you meet it cold.** Ghidra renders this program's
operands and `jal` targets as `real_address - 0x08804000`:

```text
0880c0a8: lui a1,0x27
0880c0b4: addiu a1,a1,0x4a2c      ; 0x274a2c, and the string is at 0x08a78a2c
```

So `get_xrefs_to` and `get_function_callers` return **nothing** for almost
everything here, and a search for the real address finds nothing either. What
works is `search_instructions` on the *relative* value, zero-padded:

```text
jal 0x00126050      ; calls 0x0892a050
jal 0x000645cc      ; calls 0x088685cc
```

Every "no callers found" in this page's history was this, not an absent caller.

## The Rocket's `<Stats>` struct

`WeaponStats_ParseRocket` (`0x0880c058`) is the same shape as the three simple
parsers: match an attribute name, `swc1` the float at a fixed offset. Eleven
attributes, eleven offsets, read straight off the stores:

| Offset | Attribute |
| --- | --- |
| `+0x04` | `damage` |
| `+0x08` | `venomspeed` |
| `+0x0c` | `flashspeed` |
| `+0x10` | `rapierspeed` |
| `+0x14` | `phantomspeed` |
| `+0x18` | `launchspeed` |
| `+0x1c` | `blastradius` |
| `+0x20` | `blastforce` |
| **`+0x24`** | **`spread`** |
| `+0x28` | `absorb` |
| `+0x2c` | `slowdown_time` |

`0880c2b4: swc1 f0,0x24(s1)` is `spread`. Confidence **88** - the store offsets
are unambiguous and the attribute strings sit in one contiguous run at
`0x08a78a34`.

**The executable spells it `launchspeed`, all lower case**, while the shipped XML
authors `launchSpeed`. The comparison the parser uses is therefore
case-insensitive, which is worth knowing before matching an attribute name
exactly anywhere else.

**`+0x24` is read by `Weapon_FireRocket`** - see below. It is the fan angle.

## `entity+0x1b8` is the fire-request word

`Weapons_DispatchFire` (`0x08861814`) walks every craft once a frame and reads
`*(craft + 0x1b8)`, dispatching one handler per set bit:

```c
uVar5 = *(uint *)(craft + 0x1b8);
if ((uVar5 & 0x1000) != 0) { FUN_0885d3bc(world, craft);            uVar5 = *(uint *)(craft + 0x1b8); }
if ((uVar5 & 0x4000) != 0) { FUN_088577ac(world->..., craft, i);    uVar5 = *(uint *)(craft + 0x1b8); }
if ((uVar5 & 0x0100) != 0) { FUN_08863a20(world+0x44, craft, i);    uVar5 = ...; }
if ((uVar5 & 0x0040) != 0) { FUN_088685cc(world+0x4c, craft, i);    uVar5 = ...; }
if ((uVar5 & 0x0002) != 0) { FUN_088675cc(dt, world+0x48, craft, i); uVar5 = ...; }
...
```

Sixteen bits are dispatched in all. **The word is re-read after every handler**,
which is what says a handler may clear its own bit - and one of them does.

Three of the handlers take `dt` and a `world`/`craft` pair with no subsystem
pointer (`0x0885d404`, `0x0885d534`, `0x0885d630`, bits `0x800`, `0x200`,
`0x10`). Those are the shape a *timed* pickup wants rather than a projectile,
and `docs/gameplay/pickups.md` already records bit `0x200` as the Turbo's half
of the engine gate - the same bit number, which is corroboration rather than
proof.

**This is the word `HANDOVER.md` has been recording as "thirteen unread bits out
of fourteen".** It is the pickup word behind `craft+0x1c0`, and its *consumers*
are now read even though its **writer still is not**.

## The Rocket fires three at once, fanned by `spread`

`Weapon_FireRocket` (`0x0886e104`), the bit-`0x80` handler, is the answer to
"how many rockets". Its shape, with the VFPU noise stripped:

```c
world->flags |= 2;
craft->target    = -1;            // craft + 0x1bc
craft->fire_flags &= ~0x80;       // craft + 0x1b8 - one shot, cleared immediately

if (world->live < 0x2e) {
    // 1. straight ahead, through the craft's own matrix
    Rocket_Spawn_q(world, craft->matrix, &craft->pose, craft_index);

    // 2. rotated by +spread
    a  = rocket_stats[+0x24];                 // `spread`
    t  = vcst_s(5) * a;                       // 2/pi: radians -> VFPU turns
    m  = rotation_from(vcos_s(t), vsin_s(t));
    Rocket_Spawn_q(world, vmmul_q(m, craft->matrix), &craft->pose, craft_index);

    // 3. rotated by -spread
    a  = -rocket_stats[+0x24];
    t  = vcst_s(5) * a;
    m  = rotation_from(vcos_s(t), vsin_s(t));
    Rocket_Spawn_q(world, vmmul_q(m, craft->matrix), &craft->pose, craft_index);
}
```

**Three spawns, one invocation, no timer between them.** They leave together and
fly parallel-ish, diverging only by the fan. Confidence **88**: three literal
calls to one helper, the middle argument differing only by a rotation built from
`spread` and its negation, is not open to much interpretation.

### `spread` is an angle in radians

`vcst_s(5)` is the VFPU constant `2/pi`, and Allegrex's `vsin_s`/`vcos_s` take
their argument in **quarter-turns** rather than radians - `vsin_s(x)` computes
`sin(x * pi/2)`. Multiplying by `2/pi` is exactly the radians-to-quarter-turns
conversion, so **the authored `spread` is radians**, not degrees. Confidence
**82**: the constant index is unambiguous and the conversion only makes sense
one way, but it has no runtime leg.

Which axis the rotation is about is **not** settled here - the matrix is built
through four `vpfxs`-prefixed `vmov_q` lanes and reading the axis off the
prefixes was not attempted. A lateral fan about the craft's **up** axis is the
reading this engine implements, on the grounds that it is what a spread of
forward-firing rockets is for; it is a reading, not a finding.

### One bounds check for three spawns

`world->live < 0x2e` is tested **once**, before all three, against a pool whose
cap is `0x30` elsewhere. Three spawns from a single check at 45 live would take
it to 48. Recorded because it is the kind of detail worth *not* reproducing.

## The other multi-shot weapon: a staggered burst

`Weapon_UpdateBurstFire_q` (`0x088675cc`), the bit-`0x2` handler, is a different
mechanism and worth keeping straight from the Rocket's:

```c
if (world->live < 0x40 && (craft->reload -= dt) <= 0.0f) {
    craft->reload = 0.1f;                    // 0x3dcccccd, a code literal
    craft->rounds -= 1;                      // craft + 0x1ac
    ... spawn exactly one ...
    if (craft->rounds == 0) {
        craft->target = -1;                  // craft + 0x1bc
        craft->fire_flags &= ~0x2;           // craft + 0x1b8
    }
}
```

One projectile per `0.1 s` until a round counter on the *craft* runs out, after
which the handler clears its own bit - which is why `Weapons_DispatchFire`
re-reads the word after every handler. **Confidence 72 that this is the
Cannon**, whose `<Stats>` is the only one authoring `rounds` and `rate`; the
interval being a code literal rather than the authored `rate` is what keeps it
off 84. It is **not** the Rocket, which is settled above, and not the Missile,
whose constructor (`0x0885a160`, twin `Trail_InitPreset` - see
[exhaust.md](exhaust.md)) differs from this one's `0x08859ac8`.

## What is not verified

- **Which weapon the burst handler belongs to**, at 72 for the Cannon - see
  above. The subsystem-pointer table is **not** pool-ordered, so adjacency
  arguments are weak here: `world+0x44`'s handler (`0x08863a20`) fires
  *backwards* (`vneg_q` on the craft's forward row), which reads as a Bomb or a
  Mine and would be pool index 8 or 9 rather than -1.
- **What writes `craft+0x1ac`**, the burst's round count. No store to that offset
  on a craft base was found outside the decrement itself. It is set by the code
  that arms a pickup - the same never-found grant/fire call site
  `docs/gameplay/pickups.md` has been recording since the pickups landed.
- **Which axis the fan rotates about.** See above.
- **What writes `craft+0x1b8`.** The consumers are read; the producer is not.
- **The initial value of `craft+0x1b0`** (the reload timer), which decides
  whether the first shot leaves on the arming tick or `0.1 s` after it.

## What this changes for the engine

`oag_gameplay::projectile` fired one rocket per press, with `spread` decoded
in name only and the count recorded as an open question. **Both are now
recovered**: it fires three, together, at `-spread`, `0` and `+spread` about the
craft's up axis, and `spread` is decoded and consumed. See
[pickups.md](../../../gameplay/pickups.md), whose recovered-versus-ours table moved
three rows from "ours" to "recovered" because of this page.

## History

- **2026-08-11.** Written while answering "should a rocket fire three?".
  **The first version of this page got it wrong** and the mistake is worth
  keeping: `Weapon_UpdateBurstFire_q` was found first, reads as a plausible
  multi-shot weapon, and was written up at 68 confidence as the Rocket - which
  would have produced a staggered burst 0.1 s apart rather than a fan. A
  maintainer who had actually played the game said the three fly in parallel,
  which sent the search back out and turned up `Weapon_FireRocket` and its three
  literal spawn calls. **The lesson is the one the methodology already states**:
  a handler that merely *could* be the weapon is not the weapon, and "fires
  multiple" was not a specific enough fingerprint to identify one on.
