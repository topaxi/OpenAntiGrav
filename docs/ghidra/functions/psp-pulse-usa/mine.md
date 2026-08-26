# The Mine: the weapon-id map corrected, and a cluster of seven

**Binary:** `pulse-psp` `BOOT.BIN`, image base `0x08804000`.

**Status:** the Mine is read end to end - its `<Stats>` block, its fire handler,
its entity constructor and its launch. It is also the page that **corrects the
weapon-id map**, and the correction moves two weapons: the staggered-burst
handler [weapon-fire.md](weapon-fire.md) attributed to the Cannon at confidence
72 is the **Mine**, and the backwards-firing handler
[contact-response.md](contact-response.md) attributed to the Mine is the
**Bomb**. Both attributions were informed guesses; this page replaces them with
four independent readings that agree.

Ported in `crates/gameplay/src/projectile/mine.rs`.

| Address | Name | Confidence |
| --- | --- | --- |
| `0x0880d124` | `WeaponStats_ParseMine` | 92 |
| `0x0880cef0` | `WeaponStats_ParseBomb` | 90 |
| `0x0880c774` | `WeaponStats_ParseCannon` | 90 |
| `0x088675cc` | `Weapon_DropMines` | 90 |
| `0x08859930` | `Mine_Construct` | 92 |
| `0x08859ac8` | `Mine_Init` | 90 |
| `0x08863a20` | `Weapon_FireBomb` | 88 |
| `0x08871ddc` | `Weapon_AnnounceIncoming` | 88 |

**The addresses on this page are real, not image-relative.** Ghidra renders this
program's `jal` targets and `lui`/`addiu` operands in the `0x0886xxxx` region as
`real - 0x08804000`; the trap and its regional limits are documented at length on
[weapon-fire.md](weapon-fire.md#read-this-first-the-disassemblys-addresses-are-image-base-relative).
It bit once during this read - `jal 0x0006dddc` inside `Weapon_RequestFire` is
`0x08871ddc`, and decompiling the literal `0x0886dddc` lands in an unrelated
destructor that decompiles cleanly and says nothing about weapons.

## The evidence in one line: the bit-`0x2` spawn plays `MINELAUNCH`

Before any of the ordering argument below, there is a reading that needs none of
it. `Mine_Init` (`0x08859ac8`) - the spawn the bit-`0x2` handler calls - ends
with two cue plays through `_DAT_00277ffc` and `_DAT_0027800c`. Those two
globals are the cue handles at `0x08a7bffc` and `0x08a7c00c`, and the strings
they point at, sitting immediately before them in `.rodata`, are **`MINELAUNCH`**
and **`MINERADAR`**.

The handler `weapon-fire.md` read as the Cannon plays the mine launch cue. Every
ordering argument below only explains *why* the wrong answer was so easy to
reach.

**And a maintainer who has played Pulse says the same thing**, asked without
being shown any of this: the Mine drops *several* small ones, the Bomb drops *a
single bigger one with a bigger explosion*. That is the burst-versus-single
split exactly, and it is the second time on this subsystem that from-play
knowledge has settled a question the decompiler left open - the first being the
rocket's three-abreast fan, recorded on
[weapon-fire.md](weapon-fire.md#history). The shipped `<Stats>` agree with the
description too: the Bomb's blast figures are the larger pair.

## Three enumerations, not one

The thing that made this map hard is that Pulse carries **three** weapon
orderings and they are not the same list. All three are read here:

1. **The pool order**, the class-name strings at `0x08a78c00`: `Rocket`,
   `Missile`, `Quake`, `Cannon`, `Turbo`, `Shield`, `Autopilot`, `Plasma`,
   `Bomb`, `Mine`, `LeachBeam`, `Repulser`, `Shuriken`. This is what
   `oag_formats::weapons::Weapon::ALL` already held; it is the order
   `WeaponStats_Parse` (`0x0880db7c`) tries the fourteen `type` strings in, and
   it is also **the `<Stats>` struct's own layout order** - the Mine's block
   below lands exactly where pool order predicts, and
   [shield-pickup.md](shield-pickup.md)'s Turbo (`+0x84`) and Shield (`+0x8c`)
   offsets put the Cannon's five attributes at `+0x70`..`+0x80`, ahead of both.
2. **The shipped file's element order**, which differs from the pool at one
   place: `WeaponStats_Race.xml` authors `Turbo` and `Shield` *before* `Cannon`.
   Two tables index by it, and neither is the weapon id - see below.
3. **The weapon id**, the value at `craft+0x1bc` that `Weapon_RequestFire`
   (`0x08862d9c`) switches on. It matches the pool order at **eleven** of
   thirteen positions and differs at exactly two: **id 8 reaches the Mine and id
   9 reaches the Bomb**.

Conflating (1) and (3) is what produced both wrong attributions, and conflating
(2) with (3) is what left [ai-stats.md](ai-stats.md) recording an unresolved
conflict. It is worth saying plainly: *the pool order is right about which
weapons exist, in what order they parse and how their `<Stats>` are laid out,
and wrong about two ids.*

## `Weapon_RequestFire`'s jump table, read as a table

`Weapon_RequestFire` is not a chain of comparisons - it is `sltiu t1,a2,0xd`
followed by an indirect jump through a thirteen-entry table at `0x08a7c710`.
That matters, because **entries 8 and 9 are out of address order** in the table
(`0x0005eef8` then `0x0005eed4`), so reading the case bodies in address order
transposes those two weapons. This is the mechanical cause of the old error.

Each case does three things: `ori` its bit into `craft+0x1b8`, store an emitter
anchor into `craft+0x4`, and call `Weapon_AnnounceIncoming` (`0x08871ddc`) with
a literal announcement index.

| id | reaches | bit | anchor | announce |
| --- | --- | --- | --- | --- |
| 0 | Rocket | `0x80` | `craft+0x20` | 0 |
| 1 | Missile | `0x40` | `craft+0x20` | 1 |
| 2 | Quake | `0x8` | `craft+0x20` | 2 |
| 3 | Cannon | `0x2000` | *none* | 5 |
| 4 | Turbo | `0x400` | `craft+0x60` | - |
| 5 | Shield | `0x20` | `craft+0x60` | - |
| 6 | Autopilot | `0x1000` | `craft+0x60` | - |
| 7 | Plasma | `0x4` | `craft+0x20` | 7 |
| **8** | **Mine** | **`0x2`** | **`craft+0xa0`** | **9** |
| **9** | **Bomb** | **`0x100`** | **`craft+0xa0`** | **8** |
| 10 | LeachBeam | `0x8000` | `craft+0x20` | 10 |
| 11 | Repulser | `0x10000` | `craft+0x60` | 11 |
| 12 | Shuriken | `0x20000` | `craft+0x20` | 12 |

Three columns of this table are evidence in their own right:

- **The anchor.** Three values, and they partition the weapons the way the
  weapons themselves do: `+0x20` for everything that leaves the nose, `+0x60`
  for the four that affect the craft itself, and `+0xa0` for **exactly two** -
  ids 8 and 9, the two that put something out of the back. Nothing else uses
  `+0xa0`.
- **The announcement index is the file order, and that is measurable.**
  `Weapon_AnnounceIncoming` dispatches through a second thirteen-entry table at
  `0x08a7cb60`, each case loading one cue-name pointer. The names read out in
  order as `rockets`, `missile`, `quake`, -, -, `CANNON_VO`, -, `plasma`,
  `bomb`, `mines`, `LEECH_VO`, `REPULSOR_VO`, `SHURIKEN_VO`. The three gaps sit
  at 3, 4 and 6 - which under **file** order are Turbo, Shield and Autopilot,
  the three weapons that hit nobody and so announce nothing - and the Cannon
  sits at 5, exactly where the file puts it. Under *pool* order the gaps would
  have to be 4, 5, 6 and the Cannon at 3, which is not what the table holds.
  `WeaponAiStats_Load` (`0x08851d88`) settles the same question from a second
  direction: its thirteen literals are explicit, not sequential, and they read
  `Turbo`→3, `Shield`→4, `Cannon`→5, `Bomb`→8, `Mines`→9. Two independent
  tables, one enumeration, and it is the file's.
- **The three ids with no announcement** are 4, 5 and 6, and
  [shield-pickup.md](shield-pickup.md) measured bit `0x400` as the Turbo's and
  `0x20` as the Shield's from the `<Stats>` offsets their handlers load. So the
  *id* space has Turbo at 4 and Shield at 5 where the *file* space has them at 3
  and 4 - which is what proves the announcement index is a remap rather than the
  id itself, and makes `3 -> 5` the Cannon's remap rather than a coincidence.

`Weapon_AnnounceIncoming` also gives the *reason* it is a per-weapon voice line:
it returns early when the firing craft is the player's own
(`*(int *)(_DAT_0005801c + 0x2c0) != param_2`) and when the longitudinal gap is
outside ±800. It is the "incoming!" call-out, not a fire sound.

### The one thing the remap cannot distinguish

Pool order and file order **agree** that the Bomb comes before the Mine, so
`8 -> 9` and `9 -> 8` are not explained by the pool-versus-file difference the
way `3 -> 5` is. Two readings fit the remap alone:

**A.** The id space reaches the Mine at 8 and the Bomb at 9, and the remap is
correct.
**B.** The id space is pool order throughout, and the remap at 8/9 is a shipped
bug - the game would announce "mines" for an incoming bomb.

**A wins, and not on the remap.** Under B, bit `0x2` would be the Bomb's, and
bit `0x2`'s spawn plays `MINELAUNCH` and loads `Pulse_Mine.vex`. B would need
the *handlers* to be transposed as well, at which point it has stopped being a
simpler explanation. Recorded because the distinction matters to anyone reading
`craft+0x1bc` in a save or a trace, and because the engine does not depend on
it: `oag_gameplay` keys on the weapon, and **bit `0x2` drops mines** under either
reading.

## The Mine's `<Stats>` block

`WeaponStats_ParseMine` (`0x0880d124`) is the same shape as the Rocket's and the
Missile's - match an attribute name, store the parsed float at a fixed offset.
Seven attributes, read straight off the stores. Confidence **92**: the offsets
are unambiguous and each attribute string is at a known address in the one
contiguous run at `0x08a78a2c`.

| Offset | Attribute | String |
| --- | --- | --- |
| `+0xe8` | `damage` | `0x08a78a34` |
| `+0xec` | `blastradius` | `0x08a78a8c` |
| `+0xf0` | `blastforce` | `0x08a78a98` |
| `+0xf4` | `timetodie` | `0x08a78b34` |
| `+0xf8` | `absorb` | `0x08a78aac` |
| `+0xfc` | `slowdown_time` | `0x08a78a3c` |
| `+0x100` | `trigger_radius` | `0x08a78b24` |

The block ends at `+0x100` and the Missile's ends at `+0x5c`, and what fills the
`0xa4` bytes between them is **arithmetic, not a read**: Quake's four attributes,
the Cannon's five, Turbo's two, Shield's two, Autopilot's two, Plasma's eleven
and the Bomb's eight, at four bytes each, come to exactly that. Two of the seven
are independently measured - [shield-pickup.md](shield-pickup.md) read Turbo's
pair at `+0x84` and Shield's at `+0x8c` off the handlers that spend them - and
those two land where the arithmetic puts them, which is what makes "the struct is
laid out in pool order" a supported claim rather than a bare one.

**The other five parsers were not read.** `WeaponStats_ParseBomb` (`0x0880cef0`)
in particular resisted one: it is not a defined function in the Ghidra database,
`create_function` refuses it, and the bridge cannot read bytes anywhere in
`.text` (it reads `.rodata` normally). So the Bomb's own offsets are **not
established here** - only its parser's address and name, which come from
`WeaponStats_Parse`'s dispatch chain and its unambiguous `type`-string pointers.
Nothing in this project needs those offsets: `oag_formats::weapons` matches
attributes by name.

`WeaponStats_Parse`'s dispatch chain names all fourteen parsers in one read; the
three this page needs are in the table at the top, and the Cannon's
(`0x0880c774`) is here because it is the weapon the burst handler was wrongly
attributed to.

## The blast: confirmed from the other end, and it falls off

The `<Stats>` offsets above are read off the parser's own stores. They are also
**independently confirmed by a consumer**, which is worth more than either
reading alone: `Weapon_PostBlastImpulse_q` (`0x0886794c`) - read at instruction
level on 2026-08-19, before any of this - spends exactly four offsets, and all
four are in this block:

```c
T->0x120 += stats->0xe8;                       // damage, accumulated flat
falloff   = 1.0f - d / stats->0xec;            // blastradius, a LINEAR falloff
T->0x110 += normalize(T->0x50 - S->0x90)
          * (falloff * stats->0xf0);           // blastforce
T->0x130 += stats->0xfc;                       // slowdown_time
```

Four offsets, four roles, and the roles are the ones the attribute names
predict. **The fourth is the cross-check**: `+0x130` was recorded as the
consumer of the unspent `<Global slowdown_limit>` mechanic long before anyone
knew `+0xfc` held `slowdown_time`, so the two identifications were made from
opposite ends and met in the middle.

**Two things fall out.**

1. **The blast impulse falls off linearly with distance and the damage does
   not.** `1.0 - d/blastradius` scales the impulse; the damage accumulator takes
   the authored figure flat. `oag_gameplay::projectile::blast` applied *both*
   flat and said so - "full damage everywhere inside `blastradius`, with no
   falloff" was recorded as this project's own reading, and half of it is now
   replaced by the original's. Not clamped in the original, incidentally: a hit
   outside the radius drives the term negative and nothing in that function
   stops it.
2. **The whole `0x08867xxx` chain is the Mine's.** `0x0886794c`, its caller
   `FUN_08867b50` and *its* caller `FUN_08867370` sit in `Weapon_DropMines`'
   own subsystem range, and the timer `FUN_08867370` counts down at `+0x48` is
   the one `Mine_Init` loads `timetodie` into. That is the third independent
   line pointing at `FUN_08867370` being this weapon's per-tick update - see
   [What is still ours](#what-is-still-ours-and-one-clean-negative-result) for
   the one measurement that still resists naming it.

See [contact-response.md](contact-response.md#weapon_postblastimpulse-0x0886794c-confidence-82)
for the instruction-level read this rests on.

## The fire handler drops a cluster, one every 0.1 s

`Weapon_DropMines` (`0x088675cc`) is `Weapons_DispatchFire`'s bit-`0x2` handler,
taking `(dt, world+0x48, craft, craftIndex)`:

```c
subsystem->flags |= 2;
if (subsystem->live < 0x40 && (craft->reload -= dt) <= 0.0f) {
    craft->reload = 0.1f;                    // 0x3dcccccd, a code literal
    craft->rounds -= 1;                      // craft + 0x1ac
    if (subsystem->live < 0x40) {
        entity = subsystem->pool[subsystem->live];
        entity->alive  = 1;                  // entity + 0x3c
        entity->owner  = craftIndex;         // entity + 0x40
        entity->spawn_id = ++global_counter; // entity + 0x44
        Mine_Init(entity, craft->anchor,     // craft + 0x4, set to craft+0xa0
                  &craft->velocity,          // craft + 0x10
                  craftIndex, ..., 0);
        subsystem->live += 1;
    }
    if (craft->rounds == 0) {
        craft->held       = -1;              // craft + 0x1bc
        craft->fire_flags &= ~0x2;           // craft + 0x1b8
    }
}
```

**One mine per `0.1 s` until a per-craft counter runs out**, after which the
handler clears its own bit - which is why `Weapons_DispatchFire` re-reads
`craft+0x1b8` after every handler. The pool cap here is `0x40`, not the `0x2e`
the Rocket and Missile test against; it is a different subsystem with a
different pool.

The interval is a **code literal**, not the `rate` attribute - and the Mine
authors no `rate` at all, which is the second reason `rounds`/`rate` on the
Cannon was never good evidence for this handler.

## `Mine_Init` scatters, and takes its fuse from `timetodie`

`Mine_Init` (`0x08859ac8`) is the launch, and two of the things it does are the
tightest evidence on this page:

```c
entity->matrix = *anchor_matrix;             // entity + 0x60 .. + 0x9c
entity->velocity = *direction;               // entity + 0xa0, the craft's own
entity->owner = craftIndex;                  // entity + 0x40
entity->flags |= 8;                          // entity + 0x3c

entity->fuse = stats->timetodie;             // entity + 0x48  <- stats + 0xf4
// (+ 10.0 when the caller's last argument is non-zero; the drop passes 0)

entity->drift = normalise(vec3(rand(-1,1), rand(-1,1), rand(-1,1)));
```

- **`entity+0x48` is loaded from `stats+0xf4`**, and `+0xf4` is the Mine's
  `timetodie` - the offset the parser above writes it to. A fuse taken from an
  attribute literally spelled "time to die" is not open to much interpretation,
  and the Cannon's block authors no such attribute at any offset.
- **Three `rand(-1, 1)` calls normalised into a unit vector.** A cluster that
  scatters. A cannon bolt does not need a random direction, and no other
  projectile constructor read in this tree has one.

The `+10.0` branch has no caller among the drop path (the handler passes `0`),
so what a ten-second-longer fuse is for is **unread**.

## `Mine_Construct` names the model outright

`Mine_Construct` (`0x08859930`) allocates the entity (`0x1d0` bytes) and loads
its model by name: the string argument is `0x08a7c018`, which is
`Data\Weapons\Pulse_Mine.vex`. The type-name field `entity+0x28` points at
`0x08a7c010`, and the two cue names immediately before it in `.rodata` are
`MINELAUNCH` and `MINERADAR` - the two `Mine_Init` plays.

The Bomb's group mirrors it exactly and is what puts the other half of the
correction beyond doubt: `BOMBLAUNCH`, `~BOMBRADAR` and
`Data\Weapons\Pulse_Bomb.vex` sit together at `0x08a7c748`, the vex string is
materialised by `FUN_08862fc0` (`0x088630a8`), and `Weapon_FireBomb`
(`0x08863a20`) - `Weapons_DispatchFire`'s bit-`0x100` handler - is in that same
function group.

`Mine_Init` is the very next function after `Mine_Construct` in the same group,
which is what ties the bit-`0x2` handler's spawn call to the mine model rather
than to a bolt.

## What is still ours, and one clean negative result

- **How many mines a drop releases.** `craft+0x1ac` is decremented by the
  handler and **written by nothing else in the binary** - a `search_instructions`
  sweep for every `sw` to a `+0x1ac` offset returns exactly one store on a craft
  base, the decrement itself. The shipped `<Stats>` authors no count for the
  Mine either. This is the same negative result
  [weapon-fire.md](weapon-fire.md#what-is-not-verified) recorded and it survives
  a second search. The engine picks a number and says so; see
  `crates/gameplay/src/projectile/mine.rs`.
- **What a mine does when a craft reaches it.** `trigger_radius` and
  `blastradius` are authored and their consumer was not chased here.
- **The `+10.0` fuse branch**, above.
- **`slowdown_time`**, which shares the unspent `<Global slowdown_limit>`
  mechanic's fate on every other weapon.

## What this changes for the docs

- [weapon-fire.md](weapon-fire.md): `Weapon_UpdateBurstFire_q` (`0x088675cc`) is
  **not** the Cannon. It is `Weapon_DropMines`, and the row moves to this page.
- [contact-response.md](contact-response.md): `FUN_08863a20` (`0x08863a20`) is
  **not** the Mine. It is `Weapon_FireBomb` - the negated-forward spawn, the
  rear `craft+0xa0` anchor and announcement index 8 all agree, and its own
  `.rodata` group carries `BOMBLAUNCH`, `~BOMBRADAR` and `Pulse_Bomb.vex`.
- [missile.md](missile.md)'s weapon-id table stays correct as printed - it never
  claimed which weapon ids 8 and 9 were, only which bits they set.
