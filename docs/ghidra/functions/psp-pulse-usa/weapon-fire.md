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
| `0x0886f038` | `Rocket_Spawn` - **corrected**, see [rocket-visuals.md](rocket-visuals.md) | 90 |
| `0x088675cc` | `Weapon_DropMines` - **corrected**, see [mine.md](mine.md) | 90 |
| `0x08869588` | **retired** - it is `MissilePool_Update`, see [missile.md](missile.md) | - |

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

**Corrected 2026-08-27: the wart is universal, not regional.** This page
previously claimed (dated 2026-08-17) that a sweep of `0x0882xxxx` and
`0x0885xxxx` found `jal` operands rendering absolute and in range there,
citing `jal 0x08851c94` as an example. Re-checked with `search_instructions`
across the whole program: that instruction does not exist anywhere in the
database (`jal` + operand `0x08851c94`, zero matches out of 524,728
instructions scanned), and no `jal` in the entire program - not a subset, all
of it - ever displays a target beginning `0x08` (same query, operand pattern
`"0x08"`, zero matches). A 25-address, evenly-spread sample across the whole
`.text` section (`0x08804000`-`0x08a76a3b`) turned up unrelocated targets at
every single site, including inside `0x0882xxxx` and `0x0885xxxx` themselves.
The 2026-08-17 finding was never reproducible from this database; treat it as
retracted. **Always apply the `image_base + ((word & 0x03FFFFFF) << 2)`
workaround to a static `jal` reading here - there is no region where it is
safe to skip it.** Cross-checked against four resolved targets: two land
exactly on a named function's entry point, two land on real, disassemblable
code immediately adjacent to a named function that Ghidra simply never gave
its own `Function` object - none landed on garbage.

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
if ((uVar5 & 0x1000) != 0) { FUN_088613bc(world, craft);            uVar5 = *(uint *)(craft + 0x1b8); }
if ((uVar5 & 0x4000) != 0) { FUN_088577ac(world->..., craft, i);    uVar5 = *(uint *)(craft + 0x1b8); }
if ((uVar5 & 0x0100) != 0) { FUN_08863a20(world+0x44, craft, i);    uVar5 = ...; }
if ((uVar5 & 0x0040) != 0) { FUN_088685cc(world+0x4c, craft, i);    uVar5 = ...; }
if ((uVar5 & 0x0002) != 0) { FUN_088675cc(dt, world+0x48, craft, i); uVar5 = ...; }
...
```

Sixteen bits are dispatched in all. **The word is re-read after every handler**,
which is what says a handler may clear its own bit - and one of them does.

Three of the handlers take `dt` and a `world`/`craft` pair with no subsystem
pointer (`0x08861404`, `0x08861534`, `0x08861630`, bits `0x800`, `0x200`,
`0x10`). Those are the shape a *timed* pickup wants rather than a projectile,
and `docs/gameplay/pickups.md` already records bit `0x200` as the Turbo's half
of the engine gate - the same bit number, which is corroboration rather than
proof. **All three are now read**: `0x200` and `0x10` are the Turbo's and the
Shield's countdowns, and their arming halves are bits `0x400` and `0x20`. See
[shield-pickup.md](shield-pickup.md).

**Correction, 2026-08-19: the five small-handler addresses above were wrong by
`0x4000` until this change**, and they are worth stating as a trap rather than
a typo. The decompiler prints an unresolved call as `func_0x000NNNNN`, and the
real address is that plus the **image base `0x08804000`** - not `0x08800000`,
which is what the first pass added. The *resolved* names on this page were
never affected (`func_0x0006a104` really is `Weapon_FireRocket` at
`0x0886e104`), so the error hid: only the five `FUN_...` addresses moved, and
each of the wrong ones lands inside `Rocket_Update` (`0x0885d2a8`-`0x0885db37`)
and decompiles as that whole function rather than failing.

**This is the word `HANDOVER.md` has been recording as "thirteen unread bits out
of fourteen".** It is the pickup word behind `craft+0x1c0`, and its *consumers*
are now read even though its **writer still is not**.

## The Rocket fires three at once, fanned by `spread`

`Weapon_FireRocket` (`0x0886e104`), the bit-`0x80` handler, is the answer to
"how many rockets". Its shape, with the VFPU noise stripped:

```c
world->flags |= 2;
craft->held      = -1;            // craft + 0x1bc - see the correction below
craft->fire_flags &= ~0x80;       // craft + 0x1b8 - one shot, cleared immediately

if (world->live < 0x2e) {
    // 1. straight ahead, through the craft's own matrix
    Rocket_Spawn(world, craft->matrix, &craft->pose, craft_index);

    // 2. rotated by +spread
    a  = rocket_stats[+0x24];                 // `spread`
    t  = vcst_s(5) * a;                       // 2/pi: radians -> VFPU turns
    m  = rotation_from(vcos_s(t), vsin_s(t));
    Rocket_Spawn(world, vmmul_q(m, craft->matrix), &craft->pose, craft_index);

    // 3. rotated by -spread
    a  = -rocket_stats[+0x24];
    t  = vcst_s(5) * a;
    m  = rotation_from(vcos_s(t), vsin_s(t));
    Rocket_Spawn(world, vmmul_q(m, craft->matrix), &craft->pose, craft_index);
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

## The other multi-shot weapon: a staggered burst, and it is the Mine

**Corrected 2026-08-26.** This section called the bit-`0x2` handler the Cannon
at confidence 72 and that was wrong; it is the **Mine**, at 90, and the
function is `Weapon_DropMines`. The whole read is on [mine.md](mine.md): the
weapon-id jump table's entries 8 and 9 are out of address order, so reading the
case bodies in address order transposes the Mine and the Bomb. The paragraphs
below are kept as written because the *mechanism* they describe is right - only
the attribution was wrong.

`Weapon_DropMines` (`0x088675cc`), the bit-`0x2` handler, is a different
mechanism and worth keeping straight from the Rocket's:

```c
if (world->live < 0x40 && (craft->reload -= dt) <= 0.0f) {
    craft->reload = 0.1f;                    // 0x3dcccccd, a code literal
    craft->rounds -= 1;                      // craft + 0x1ac
    ... spawn exactly one ...
    if (craft->rounds == 0) {
        craft->held = -1;                    // craft + 0x1bc - see below
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

## Correction: `craft+0x1bc` is the held weapon, not a target

**This page called `craft+0x1bc` a target and that was wrong** (corrected
2026-08-17). It is the **held-weapon slot**, and `-1` means "carrying nothing" -
which is why every fire handler writes it.

The evidence is on [missile.md](missile.md#a-correction-craft0x1bc-is-not-a-target):
two two-instruction arm functions write weapon *ids* to the same offset at the
same base (`0` for the Rocket, `1` for the Missile), `Weapon_RequestFire`
(`0x08862d9c`) switches on it to choose which bit of `+0x1b8` to set, and the
weapon-record reset writes `-1`.

That page also carries the **complete weapon-id to fire-bit map**, which is the
thing this page's sixteen-bit dispatch was missing, and the Missile's own half of
the subsystem.

## What is not verified

- ~~**Which weapon the burst handler belongs to**, at 72 for the Cannon.~~
  **Settled 2026-08-26: it is the Mine, at 90.** See
  [mine.md](mine.md#weapon_requestfires-jump-table-read-as-a-table). The subsystem-pointer table is **not** pool-ordered, so adjacency
  arguments are weak here: `world+0x44`'s handler (`0x08863a20`) fires
  *backwards* (`vneg_q` on the craft's forward row), which reads as a Bomb or a
  Mine and would be pool index 8 or 9 rather than -1. **Read in full,
  2026-08-19, chasing the collision-stun's pending-impulse producer chain**:
  see
  [contact-response.md](contact-response.md#fun_08863a20-0x08863a20-0x08863ba3-weapons_dispatchfires-world0x44-handler-and-it-looks-like-the-mines-own-fire-handler)
  for the read - pool alloc, the negated-forward spawn confirmed as the exact
  mechanism behind "fires backwards", and a correction worth carrying back
  here: the newly spawned entity's `+0x40` is the *owning craft's own index*,
  not a weapon-type enum, which is evidence (not proof) that the global
  per-weapon stats table `Weapon_PostBlastImpulse_q` reads through that same
  offset is keyed by craft rather than by weapon type.
- ~~**What writes `craft+0x1ac`**, the drop's mine count. No store to that offset
  on a craft base was found outside the decrement itself, and a second sweep on
  2026-08-26 reproduced that negative result exactly - see
  [mine.md](mine.md#what-is-still-ours-and-one-clean-negative-result). It is set by the code
  that arms a pickup - the same never-found grant/fire call site
  `docs/gameplay/pickups.md` has been recording since the pickups landed.~~
  **Found 2026-09-15 by a runtime write watch**: `WeaponPickup_ArmMine`
  (`0x0886759c`) stores a literal `5`, called from `WeaponPickup_Grant` - see
  [mine.md](mine.md#2026-09-15-the-cluster-is-five-measured-live-and-the-counters-writer-found).
- **Which axis the fan rotates about.** See above.
- **What writes `craft+0x1b8`.** The consumers are read; the producer is not.
- ~~**The initial value of `craft+0x1b0`** (the reload timer), which decides
  whether the first shot leaves on the arming tick or `0.1 s` after it.~~
  **`0.0`, written by `WeaponPickup_ArmMine`'s delay slot at grant time**, so
  the first mine leaves on the press frame - measured live 2026-09-15 on
  [mine.md](mine.md#2026-09-15-the-cluster-is-five-measured-live-and-the-counters-writer-found).

## What this changes for the engine

`oag_weapons::projectile` fired one rocket per press, with `spread` decoded
in name only and the count recorded as an open question. **Both are now
recovered**: it fires three, together, at `-spread`, `0` and `+spread` about the
craft's up axis, and `spread` is decoded and consumed. See
[pickups.md](../../../gameplay/pickups.md), whose recovered-versus-ours table moved
three rows from "ours" to "recovered" because of this page.

## History

- **2026-08-11, later.** **The spawn helper's address on this page was wrong**,
  and it was this page's own trap that did it: the `jal 0x0006b038` in
  `Weapon_FireRocket` is relative, so the target is `0x0886f038`, not
  `0x0886b038`. The row is corrected above and the function is read end to end in
  [rocket-visuals.md](rocket-visuals.md). `0x0886b038` is a real function - a
  segment-versus-craft sweep on the same pool - but it is not the spawn and
  should not have carried the name. Worth keeping because the trap is documented
  three paragraphs into this very page and still caught the next reader: knowing
  about it is not the same as applying it in the direction that bites.
- **2026-08-11.** Written while answering "should a rocket fire three?".
  **The first version of this page got it wrong** and the mistake is worth
  keeping: the bit-`0x2` handler was found first, reads as a plausible
  multi-shot weapon, and was written up at 68 confidence as the Rocket - which
  would have produced a staggered burst 0.1 s apart rather than a fan. A
  maintainer who had actually played the game said the three fly in parallel,
  which sent the search back out and turned up `Weapon_FireRocket` and its three
  literal spawn calls. **The lesson is the one the methodology already states**:
  a handler that merely *could* be the weapon is not the weapon, and "fires
  multiple" was not a specific enough fingerprint to identify one on.
