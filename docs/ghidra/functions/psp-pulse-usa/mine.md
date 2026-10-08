# The Mine and the Bomb: the weapon-id map corrected, and one weapon in two sizes

**Binary:** `pulse-psp` `BOOT.BIN`, image base `0x08804000`.

**Status:** the Mine is read end to end - its `<Stats>` block, its fire handler,
its entity constructor and its launch. It is also the page that **corrects the
weapon-id map**, and the correction moves two weapons: the staggered-burst
handler [weapon-fire.md](weapon-fire.md) attributed to the Cannon at confidence
72 is the **Mine**, and the backwards-firing handler
[contact-response.md](contact-response.md) attributed to the Mine is the
**Bomb**. Both attributions were informed guesses; this page replaces them with
four independent readings that agree.

Both are ported in `crates/weapons/src/projectile/mine.rs` - one module,
because they are one weapon in two sizes. See
[The Bomb is the same weapon](#the-bomb-is-the-same-weapon-one-size-up).

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
| `0x08867f1c` | `Mine_SpawnExplosion` | 90 |
| `0x0886759c` | `WeaponPickup_ArmMine` | 92 (new 2026-09-15) |
| `0x088638b8` | `BombPool_Update` | 85 (new 2026-09-15) |
| `0x08863d7c` | `Bomb_UpdateTrigger` | 85 (new 2026-09-15) |
| `0x088633d0` | `Bomb_AdvanceFuse` | 85 (new 2026-09-15) |
| `0x08863440` | `Bomb_InArmingDelay` | 75 (new 2026-09-15) |
| `0x08859f04` | `Mine_InArmingDelay` | 75 (new 2026-10-07) |
| `0x088643d0` | `Bomb_ApplyBlast` | 85 (new 2026-09-15) |
| `0x088640c8` | `Bomb_Detonate` | 88 (new 2026-09-15) |
| `0x08872078` | `BombBlast_Construct` | 90 (new 2026-09-15) |
| `0x0887250c` | `BombBlast_Update` | 85 (new 2026-09-23) |
| `0x08859ce4` | `Mine_PoseNode` | 92 (new 2026-09-24) |
| `0x08859ecc` | `Mine_Update` | 75 (new 2026-09-24) |
| `0x08862fc0` | `Bomb_Construct` | 85 (new 2026-09-24) |
| `0x08863188` | `Bomb_Init` | 88 (new 2026-09-24) |

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
   `oag_tables::weapons::Weapon::ALL` already held; it is the order
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
Nothing in this project needs those offsets: `oag_tables::weapons` matches
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
   the authored figure flat. `oag_weapons::projectile::blast` applied *both*
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

## `Mine_SpawnExplosion` plays `WO_MINE_EXPLO`

**Recovered 2026-08-26, confidence 90 - direct instruction-level read, closed
the same day this page's own doc comment in `crates/weapons/src/projectile/mine.rs`
started calling this "unread".** `FUN_08867370`'s teardown pass - the second
loop, over every entity whose `+0x3c` bit `4` ("destroy") is set - calls
`func_0x00063f1c(param_2, iVar4, auStack_40)` for each. That is the same
image-base-relative rendering this page's introduction warns about: the real
target is `0x08867f1c`, now named **`Mine_SpawnExplosion`**. Ghidra's own
`get_function_callers` finds **no** caller for it - the call is exactly this
kind of unresolved-at-analysis-time reference, which is why the address only
turned up by reading `FUN_08867370`'s decompilation by hand rather than by
searching xrefs.

`Mine_SpawnExplosion` builds a position from the entity (`func_0x00055cd4`,
the same helper `FUN_08867b50`'s sweep uses) and calls the already-named
`Psys_Spawn_q` (`0x08915484`) with the fourcc tag `MIEX` (`0x5845494d`) and a
string-pointer argument. Read *as a pointer* rather than trusted as a
plausible-looking constant: `func_0x00141890` - `Psys_Spawn_q`'s hash
step - is a CRC32 over `(pointer, strlen(pointer))`, matching
`oag_formats::wad::hash_name`'s own mechanism, so the argument has to be a
string address. The constant the decompiler prints (`0x278904`) is the same
kind of misrendering `weapon-fire.md` already documents for `jal` targets in
this binary, and adding the image base back (`0x278904 + 0x08804000 =
0x08a7c904`) lands inside `.rodata`. A direct memory read there returns the
literal bytes `57 4F 5F 4D 49 4E 45 5F 45 58 50 4C 4F 00` -
**`"WO_MINE_EXPLO"`**, null-terminated at 13 characters.

**The `MIEX` tag is not the Missile's.** `Missile_SpawnExplosion`
(`0x08868d50`, already on this book at 85 from `missile.md`) is structurally
near-identical - same matrix-copy boilerplate, same allocation call shape,
same `Psys_Spawn_q` call with the same `0x5845494d` tag - and its own string
argument, read the same way (`0x278970 + 0x08804000 = 0x08a7c970`), is
`"WO_MISSILE_EXPLO"`. So `MIEX` is a generic "this is an explosion instance"
class tag the engine reuses across weapons, not a Missile-specific label as
`missile.md` read it in isolation; what actually selects the `.pob` is the
string this second reading resolves. Both functions are independently
compiled instances of the same explosion-spawn template, not one calling the
other.

**Reached from both ways a mine can die.** The destroy bit is set from two
places in this same subsystem: `FUN_08867370`'s own fuse-timeout branch (the
first loop, `fVar6 <= 0.0`), and `FUN_08867b50`'s trigger_radius sweep - both
just raise the bit and let the second loop's uniform teardown call
`Mine_SpawnExplosion`, so a mine that times out and a mine a craft walks into
play the identical explosion.

**`+0x164` is the pool's live count, kept exact by a swap-with-last removal in
the same second loop.** Recovered 2026-09-04, confidence 90 - a direct read of
`FUN_08867370`'s decompiled second loop, no inference needed. Once the destroy
bit is set and `Mine_SpawnExplosion` has been called for a slot, the same pass
clears the entity's flags, sets its pool entry to `0xffffffff` and then:

```c
live = *(param_2 + 0x164) - 1;
*(param_2 + 0x164) = live;
if (live != 0) {
    // move the last live pointer into the vacated slot
    entity_ptrs[removed_index] = entity_ptrs[live];
}
```

That is an ordinary swap-with-last compaction over the `+0x64` pointer array:
`+0x164` is decremented exactly once per removal and the array's tail is moved
down to fill the gap, so it stays a dense count of *live* entries rather than a
high-water mark or a rotating cursor. A prior live breakpoint measured
`craftArray->0x164 == 1` mid-race, before any mine had been dropped by that
session's own account - read against the mechanism above, a swap-with-last
live count reading `1` mid-race is exactly what one already-active mine
produces, so the measurement and the mechanism now agree rather than merely
coexisting. This is also the counter-example to `+0xa4`/`FUN_0886a920` being the same
field under a different name (`contact-response.md`'s own read already noted
the two write different offsets): `+0xa4` gates a capacity check before an
*allocation* and never decrements, where `+0x164` here demonstrably both
increments (elsewhere, at spawn) and decrements (here, at removal) - a live
count and a write-cursor are different shapes even when they bound the same
kind of pool.

**The Bomb's own explosion call is not chased here.** `mine.md`'s own reading
elsewhere records the Bomb keeps a separate pool cursor (`+0xc4`/cap 32,
against the Mine's `+0x164`/`+0x64`), so `Weapon_FireBomb`'s teardown is very
likely a distinct function from `FUN_08867370`, structurally similar but
unread. Whether it calls this same `Mine_SpawnExplosion` (playing
`WO_MINE_EXPLO` at a larger scale - which would match a maintainer's own
description, "the bomb should be static, like the mines, just a single big
mine", extended from motion to the explosion too) or its own equivalent
naming `WO_BOMB_SMOKERING` is open.

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
`craft+0x1b8` after every handler. **The counter starts at 5** - measured on
the running original on 2026-09-15, and read off the arm function the same
day; see [the cluster is five](#2026-09-15-the-cluster-is-five-measured-live-and-the-counters-writer-found). The pool cap here is `0x40`, not the `0x2e`
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
  projectile constructor read in this tree has one. **Corrected 2026-09-24:
  it is not a drift, it is the mine's own spin axis** (`entity+0xc0`), which
  `Mine_PoseNode` turns the model about every tick - measured live, see
  [2026-09-24: a laid mine spins](#2026-09-24-a-laid-mine-spins-and-the-bomb-sits-square-to-the-world).

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

## The Bomb is the same weapon, one size up

Everything above about the Mine is the Bomb's too, and the two differ in exactly
one structural place. Set out here rather than on a page of its own, because a
second page would be nine tenths this one.

**What is recovered.**

- **A press lays one, not a cluster.** `Weapon_FireBomb` (`0x08863a20`) makes a
  single spawn call with no reload timer anywhere in it, where
  `Weapon_DropMines` reloads `craft+0x1b0` and comes back next tick. Read end to
  end in
  [contact-response.md](contact-response.md#fun_08863a20-0x08863a20-0x08863ba3-weapons_dispatchfires-world0x44-handler-and-it-looks-like-the-mines-own-fire-handler),
  which also has the pool cursor (`+0xc4`, cap 32 - a **different** pair from
  the Mine's `+0x164`/`+0x64`, so nothing about one subsystem's pool transfers
  to the other) and `+0x40` being the owning craft's index.
- **The same rear anchor**, `craft+0xa0`, which these two weapons and nothing
  else use. See the jump table above.
- **Weapon id 9, fire bit `0x100`**, announcement index 8 - `bomb`.
- **Its own `.rodata` group**, at `0x08a7c748`: the cues `BOMBLAUNCH`,
  `~BOMBRADAR`, `BOMBEXPL` and `BOMBEXPL_PC`, and the model
  `Data\Weapons\Pulse_Bomb.vex`, whose string is materialised by
  `FUN_08862fc0` at `0x088630a8`. That group is what pairs the bit-`0x100`
  handler with this weapon, exactly as `MINELAUNCH` pairs the bit-`0x2` one with
  the Mine.
- **Eight `<Stats>` attributes**, six of them the Mine's six and every one of
  those larger on both shipped tables.

**What is not.**

- **`WeaponStats_ParseBomb`'s offsets.** The name is settled at 90 from the
  dispatch chain; the *offsets* are not read - see the note under the Mine's
  `<Stats>` table for why the function resisted one. Nothing needs them.
- **`damageradius`.** The Bomb is the only weapon of the thirteen that authors a
  second radius, and no consumer of one has been found - the blast path that
  *was* read spends `blastradius` for both damage and impulse. Left undecoded
  and named, rather than decoded on a guess about which half it governs.
- **Whether a bomb moves.** `Weapon_FireBomb` stages the craft's forward row
  **negated** (**corrected 2026-09-24**: the vector is the craft's `+0xb10`
  row, and live it reads as the craft's *up* - see the 2026-09-24 section) and hands it to a spawn helper at `0x0885f188` that
  [contact-response.md](contact-response.md) could not resolve statically - two
  callers jump past its prologue. So the direction is recovered and any *speed*
  is not. **This engine lays it static**, which is the same conservative reading
  the Mine takes and is what a maintainer who plays Pulse describes: "the bomb
  should be static, like the mines, just a single big mine". A negated forward
  row is a unit vector, so read as a velocity it would be one unit a second
  anyway - indistinguishable from static at racing speed.
- **What arms the bomb's fuse.** `Weapon_FireBomb` writes nothing to the
  entity's `+0x48`, which contact-response.md records as a clean negative
  result. This engine takes the fuse from `<Bomb> timetodie` because that is
  what `Mine_Init` does with `<Mine> timetodie`, and because twenty seconds is
  not a number with another plausible home.

## What is still ours, and one clean negative result

- ~~**How many mines a drop releases.** `craft+0x1ac` is decremented by the
  handler and **written by nothing else in the binary** - a `search_instructions`
  sweep for every `sw` to a `+0x1ac` offset returns exactly one store on a craft
  base, the decrement itself. The shipped `<Stats>` authors no count for the
  Mine either. This is the same negative result
  [weapon-fire.md](weapon-fire.md#what-is-not-verified) recorded and it survives
  a second search. The engine picks a number and says so; see
  `crates/weapons/src/projectile/mine.rs`.~~ **Measured 2026-09-15: five**,
  and the writer both sweeps missed is `WeaponPickup_ArmMine` (`0x0886759c`) -
  see [below](#2026-09-15-the-cluster-is-five-measured-live-and-the-counters-writer-found).
  The negative result was wrong, not merely incomplete: a third
  `search_instructions` sweep (`sw`, operand `0x1ac(`) on the relocated
  2026-09-07 database lists `0x088675c0` in `FUN_0886759c` alongside the
  decrement, so the earlier sweeps were run against a database in which that
  function was not yet defined, or read only the rows whose base register was
  already known to be a craft.
- ~~**What a mine does when a craft reaches it.**~~ **Recovered 2026-08-26** -
  see [`Mine_SpawnExplosion` plays `WO_MINE_EXPLO`](#mine_spawnexplosion-plays-wo_mine_explo)
  above: `blastradius`'s consumer is `Weapon_PostBlastImpulse_q`, and the
  visual both `trigger_radius` and the fuse timing out reach is
  `Mine_SpawnExplosion`. **And 2026-09-16, the credit is single-target and
  the fuse spends nothing** - see the section at the end of this page.
- ~~**The Bomb's own explosion call.** Whether `Weapon_FireBomb`'s teardown
  reaches `Mine_SpawnExplosion` too (the same `WO_MINE_EXPLO`, larger) or its
  own equivalent is unchased - see the note at the end of the section above.~~
  **Read 2026-09-15**: its own - `Bomb_Detonate` builds a `BombBlast` of
  `explosion_hemisphere.vex`, `WO_BOMB_SMOKERING` and `Bomb_Shockwave.vex`
  and plays `BOMBEXPL` or `BOMBEXPL_PC`. See
  [the Bomb's teardown](#2026-09-15-the-bombs-teardown-read---its-own-blast-not-the-mines).
- **The `+10.0` fuse branch**, above.
- ~~**`slowdown_time`**, which shares the unspent `<Global slowdown_limit>`
  mechanic's fate on every other weapon.~~ **Recovered 2026-09-06**: the
  mechanic's law is on [engine.md](engine.md), `Ship_AddSlowdown`
  (`0x08848690`) is its clamp, and `oag_tables::weapons` decodes the
  attribute.

## 2026-09-06: `MINELAUNCH` is a plain, positional, craft-emitter cue - `MINERADAR` is not

The evidence-in-one-line section above establishes *that* `Mine_Init` plays
both cues; it does not say *how*. Both calls were decompiled directly this
session (`decompile_function` on `0x08859ac8`, confidence 92 already):

```c
func_0x001352b0(0x3f800000, param_6, _DAT_002bddf8, 0, _DAT_00277ffc, 0);
// ... allocates a 0x70-byte object, points its own +0x50 at param_1+0x60
// (the entity's own matrix) ...
func_0x001352b0(0x3f800000, *(param_1 + 0x4c), _DAT_002bddf8, 0,
                _DAT_0027800c, param_1 + 0x50);
```

Both calls are the same function, `func_0x001352b0` - one call, one emitter
argument each, not two different play/loop functions. The two differ in that
argument, and that argument is what a maintainer wiring this needs, not what
either call's own line has read.

**`MINELAUNCH`'s emitter is `param_6`, and `param_6` traces back to the
firing craft's own `+0x50`.** `Weapon_DropMines` (`0x088675cc`) is the only
caller, and its call site (also decompiled directly this session) reads:

```c
func_0x00055ac8(iVar2, uVar4, param_3 + 0x10, param_4, uVar1,
                *(int *)(*(int *)(*(int *)(iVar3 + 0x44) + 0xf0) + 0x50), 0);
```

The sixth argument (`param_6`) is `*(*(*(iVar3+0x44)) + 0xf0) + 0x50)` - two
pointer hops off the subsystem's per-craft slot, landing on a `+0x50` field.
`+0x50` is the same offset `ShipCollisionFx_Trigger` (`0x089246b4`) reads as
the craft's own emitter for `.COLLISIONS` and `Shield_Activate`
(`0x0883e544`) reads it for `~SHIELD` - see
[`Cue::Collision`/`Cue::Shield`'s placement reading](../../../../crates/sound/src/sfx.rs).
Two levels of indirection is more inferential than either of those single-hop
reads, so this is scored **78** (probable: strong structural fit, the same
offset three call sites now agree on, not runtime-verified) rather than
matching their 82-85. **So `MINELAUNCH` is a one-shot, positional cue off the
firing craft's own emitter** - the same placement `Cue::Craft` already gives
`Collision`, `Absorb` and `Shield` - fired once per mine as it leaves the
back of the craft, which for the Mine is once every `DROP_INTERVAL` while a
cluster is coming out.

**`MINERADAR` is a different shape and stays unwired.** Its emitter,
`*(param_1 + 0x4c)`, is not the craft at all - it is a new `0x70`-byte object
`Mine_Init` allocates on the spot and anchors to *the mine entity's own
matrix* (`param_1 + 0x60`, i.e. `entity->matrix`), and the handle this call
returns is stored at `entity + 0x50` - the mine's own field, not the craft's.
That is a per-projectile emitter with a held handle, the shape
`docs/`'s own audio module (`crates/sound/src/sfx.rs`) calls a *held*
voice - and every held voice this engine plays today (`Engine`, `Shield`,
`Blowup`) is anchored to a **craft**, keyed by grid slot; nothing in
`CueEvent` or `SfxVoices` can anchor one to a projectile pool slot instead.
Building that is real work on its own (a new held-voice table sized to the
projectile pool, opened at lay time and closed at detonation or expiry) and
is not done here. Worth noting for whoever picks it up: the bank's own data
says the single waveform bound to `MINERADAR` does **not** carry the loop bit
(`oag-wad sounds ...:PSP_GAME/USRDIR/Data.wad --cue MINERADAR` reports "0
looping"), so the held handle is not for a continuous drone - what recurring
or one-shot use the original makes of that handle is unread, and is exactly
why this stays silent rather than guessed at.

**Confirmed against the disc's own bank, not just the executable's strings**:
`oag-wad sounds <image>:PSP_GAME/USRDIR/Data.wad --cue <name>` finds
`MINELAUNCH`, `MINERADAR`, `BOMBLAUNCH` and `~BOMBRADAR` all in bank `#866`
(hash `01bec824`), which is `Data\Sound\weapons.bnk` per this project's own
table in `docs/formats/psp-audio.md`.

**`BOMBLAUNCH`/`~BOMBRADAR` are not wired, and this session's own read of
`Weapon_FireBomb` (`0x08863a20`, decompiled directly) is why: it never calls
`func_0x001352b0` at all.** Its only calls inside the pool-allocation branch
are `func_0x0005f188` (the spawn helper contact-response.md already
documents as unresolved past its prologue) and `func_0x0005abc4` (the shared
`< 14` side-effect call every function on this page has now shown at least
once). So the two Bomb cues' *presence* in the same `.rodata` group as the
model string is confirmed, but their *call site* is not - either it is inside
`func_0x0005f188` itself, or it is not played from the fire path at all. The
distinction the "evidence in one line" section already drew between "the
`.rodata` group pairs a handler with a weapon" and "the handler plays the
cue" turns out to matter here: the Mine's own pairing was later confirmed by
a direct decompile and the Bomb's was not, so treating them as equally solid
would have been wrong.

## What this changes for the docs

- [weapon-fire.md](weapon-fire.md): `Weapon_UpdateBurstFire_q` (`0x088675cc`) is
  **not** the Cannon. It is `Weapon_DropMines`, and the row moves to this page.
- [contact-response.md](contact-response.md): `FUN_08863a20` (`0x08863a20`) is
  **not** the Mine. It is `Weapon_FireBomb` - the negated-forward spawn, the
  rear `craft+0xa0` anchor and announcement index 8 all agree, and its own
  `.rodata` group carries `BOMBLAUNCH`, `~BOMBRADAR` and `Pulse_Bomb.vex`.
- [missile.md](missile.md)'s weapon-id table stays correct as printed - it never
  claimed which weapon ids 8 and 9 were, only which bits they set.

## 2026-09-05: re-read live for a drop-time effect, and a live-database caveat

Before wiring `MINE_MODEL_ENTRY`/`BOMB_MODEL_ENTRY` (`oag_raceplay`), the
handover thread's own instruction was to read `Mine_Init` and `Mine_Construct`
again rather than assume the reading above still covered "does dropping a
mine trigger a particle effect". It does: `Mine_Init`'s full call list, read
straight off a live decompile, is two sound-cue calls (matching `MINELAUNCH`
and `MINERADAR` above), an allocator and an init for the second cue's tracked
handle. No call in it resolves to `Psys_Spawn_q` (`0x08915484`). **A mine or
a bomb is laid with no drop-time visual effect**, which the port now draws as
exactly that: nothing, on purpose.

**A live-database caveat surfaced by the same read, worth flagging rather
than acting on.** This session's Ghidra instance resolves `Mine_Init`'s and
`Mine_Construct`'s own function *entry addresses* differently from the two
rows above (`0x08859ac8` and `0x08859930`): decompiling those exact addresses
returns two other functions - a small teardown-shaped routine and a
20-instruction sound-effect fragment - and the matrix-copy/velocity/fuse/
drift logic this page describes as `Mine_Init` is, in this instance, reached
at `0x08859954` instead. Disassembling around `0x08859930` byte-for-byte
confirms it sits mid-instruction inside the neighbouring function rather
than at a function boundary. The **content** at both documented addresses
still matches this page's reading exactly - every offset, every literal,
every call shape - which is what today's finding rests on rather than the
addresses lining up. The likely cause is that `just apply-names` has not
been replayed into this particular Ghidra project (a fresh import re-derives
function boundaries from scratch; see `CLAUDE.md`'s note on this), not that
either reading here was wrong. Left as a caveat rather than a correction:
nobody has a confident replacement address, and there is nothing here that
contradicts the confidence-90/92 rows above.

## 2026-09-15: the cluster is five, measured live, and the counter's writer found

**`CLUSTER = 5` was invented and turns out to be right.** Measured on the
running original (PPSSPP v1.20.4, `pulse-psp-usa.chd`, a VENOM Single Race
on Talon's Junction, weapons on, audio off) with `scripts/psp-count-mines.py`,
which breaks on `Weapon_DropMines` (`0x088675cc`) every frame a cluster is
coming out and reads the firing record's `+0x1ac` at each hit. Nothing was
written to game memory: the clusters counted are the ones the AI collected
and fired through the game's own grant path, which is the only way the count
can be trusted - setting bit `0x2` by hand (`psp-fire-weapon.py burst`) skips
the arm below and counts down from whatever the word held.

**Three clusters, three different craft, the same trail every time**
(`frame` is the PSP cycle counter over `222e6/59.94`; `rounds` is `+0x1ac`
read at handler entry, before its own decrement; `pool` is the subsystem's
`+0x164`):

```text
frame     craft  rounds  reload  pool        frame     craft  rounds  reload  pool
10284.3   3      5       0.0000  0           15180.2   5      5       0.0000  0
10285.2   3      4       0.1000  1           15181.2   5      4       0.1000  1
10291.2   3      3       0.1000  2           15187.2   5      3       0.1000  2
10298.3   3      2       0.1000  3           15193.2   5      2       0.1000  3
10304.3   3      1       0.1000  4           15199.2   5      1       0.1000  4
```

(craft 0's cluster at frames 13175-13199 is identical to craft 5's.) The
counter reads **5** on the first hit of every cluster, counts down by one per
drop, and the fire word reads `0x2` throughout and is gone the frame after the
fifth mine. The write watch on every record's `+0x1ac` counted exactly **6**
hits per cluster - one arm plus five decrements - beside a control on the
player's rigid body counting 1,642 in the first 40-second run, so a silent
record was a record that did not fire rather than a dead instrument. (The
six-minute second run's control read **0** on the same body address: by then
the HUD was gone and the camera was on the post-race fly-by, so the parked
player craft was no longer being integrated - and record 0 fired one of the
three clusters in that state, so the player's slot is AI-driven once the race
is over for it. The two target records that fired counted their six hits each
regardless, which is what a positive control exists to certify.)

**The spacing is `0.1 s` at runtime too**: drops at 6-frame intervals in two
clusters (15180, 15186, 15192, 15198, 15204) and 7-7-6-6 in the first, where
the reload read `0.0002` on one frame - the handler's `reload -= dt` missing
zero by one frame of `dt` jitter and dropping a frame later. `DROP_INTERVAL`
stays `0.1`, now measured rather than only read. **The first mine leaves on
the press frame** (`reload 0.0000` at the first hit, `pool` `0` to `1` in the
same frame) - see the arm function for why.

### `WeaponPickup_ArmMine` (`0x0886759c`, EU `0x088673f8`) writes the five

PPSSPP's own watch log names the writer of the arming store, and it is the
function immediately before `Weapon_DropMines`:

```text
CHK Write32(CPU) at 09ba165c, PC=088675c0 (z_un_0886759c)   ; the arm, once
CHK Write32(CPU) at 09ba165c, PC=08867648 (z_un_088675cc)   ; the decrement, five times
```

Disassembled on both pressings, byte-identical (`sw a0,0x1ac(a1)` is the
store the watch caught; `a1` is the craft's weapon record):

```text
0886759c: li    a0,0x8              ; weapon id 8, the Mine
088675a0: lw    a2,0x1b8(a1)
088675a4: mtc1  zero,f12
088675a8: sw    a0,0x1bc(a1)        ; held weapon
088675ac: sw    a0,0x1c0(a1)        ; the second copy the grant's no-repeat rule reads
088675b0: li    a0,-0x2
088675b4: and   a0,a2,a0
088675b8: sw    a0,0x1b8(a1)        ; fire word &= ~0x1 - bit 0x1, which no weapon in the
                                    ;   jump table above owns; what it flags is unread
088675bc: li    a0,0x5
088675c0: sw    a0,0x1ac(a1)        ; rounds = 5        <- the cluster
088675c4: jr    ra
088675c8: _swc1 f12,0x1b0(a1)       ; reload = 0.0      <- the first drops on the press frame
```

It is the Mine's entry in the arm-function family [missile.md](missile.md)
already names two members of (`WeaponPickup_ArmRocket` writes id `0`,
`WeaponPickup_ArmMissile` writes id `1`, both two instructions), and Ghidra's
own xrefs put both its callers inside `WeaponPickup_Grant` (`0x08861d20`):
the AI branch (`0x08862074`) and the human branch (`0x0886268c`), each
reached when the `<Pickupodds>` roll lands in the Mine's weight (base
`0x238`) - so **the count is armed when the pickup is granted, not when fire
is pressed.** `Weapon_RequestFire`'s id-8 case only `ori`s bit `0x2` into the
fire word. Confidence **92**: a live write caught at this PC on three
independent clusters, a direct disassembly on both pressings, and the
callers resolved by xref rather than by hand. EU at 87 by
[exact-hash-transfer.md](../psp-pulse-eu/exact-hash-transfer.md)'s `-5`.

**What this settles beyond the number:**

- **A re-press mid-cluster is a no-op in the original**, and that is now
  recovered rather than chosen. The only writer of `+0x1ac` outside the
  handler's decrement is on the grant path, and `Weapon_RequestFire` cannot
  reach it, so a second press while bit `0x2` is already set changes nothing
  - which is what `oag_weapons::pickup::Held::begin_drop`'s guard does.
- **The reload timer's initial value** - listed as unread on
  [weapon-fire.md](weapon-fire.md#what-is-not-verified) - is `0.0`, written
  by the arm's delay slot, so the first mine leaves on the frame fire is
  pressed and the `0.1 s` gaps are between the first and the second onward.
  The live trail shows exactly that.
- **The Bomb has no arm function and no count.** `WeaponPickup_Grant` writes
  `9` straight into `+0x22c`/`+0x230` (`record+0x1bc`/`+0x1c0`) inline for
  the Bomb's weight (base `0x178`) and calls nothing, which is the grant-side
  half of "a press lays one" - `Weapon_FireBomb` never reads `+0x1ac`.
- **`+0x1c0` is the no-repeat memory.** The grant's human branch compares its
  last-held copy (`iVar7+0x230`) against each candidate id and loops until it
  draws something else; the arm writes both copies.

**A trap for whoever reads the grant next.** `WeaponPickup_Grant` receives
`(world, craft_index)` and computes the record as `world + index*0x1f0 + 0x70`
- the same inline array `Weapons_DispatchFire` walks and
`scripts/psp-fire-weapon.py` documents. Its `+0x22c`/`+0x230`/`+0x23c`/`+0x240`
offsets are therefore `record+0x1bc`/`+0x1c0`/`+0x1cc`/`+0x1d0`, not new
fields: `craft+0x23c` ("human") on [missile.md](missile.md) is
`record+0x1cc`.

## 2026-09-15: the Bomb's teardown read - its own blast, not the Mine's

The stretch item on the same session as the cluster measurement, read-only.
The Bomb-pool walker the earlier sections predicted "structurally like
`FUN_08867370`, cursor `+0xc4`/cap 32" exists, is exactly that shape, and sits
on either side of `Weapon_FireBomb` (`0x08863a20`) in the same function group.
Found by `search_instructions(lw, "0xc4(")` scoped to the group rather than by
xref: like `FUN_08867370`, none of these resolve a caller under either address
rendering. Every address below is real; the EU twin is given only where the
normalized opcode hash matched exactly (`BombPool_Update` did; the others
differ only in their `.rodata`/global immediates, same size and instruction
count, and are **not** transferred under the exact-hash rule).

**`BombPool_Update` (`0x088638b8`, EU `0x08863714`, 85).** Two passes over the
`+0x44` pointer array up to `+0xc4`: first `Bomb_UpdateTrigger(subsystem, i)`
per slot, then per slot `Bomb_AdvanceFuse(dt, bomb)`, raising destroy bit `4`
on the entity's `+0x3c` when it returns false, and for every slot with that bit
set `Bomb_Detonate(subsystem, bomb, 1, bomb->victim /* +0x74 */)`, clear bit
`8`, `+0x48 = 0xff`, then the same swap-with-last compaction of `+0xc4` the Mine
pool uses on `+0x164`. Same template, different pool - which is why nothing
about one transferred to the other.

**`Bomb_AdvanceFuse` (`0x088633d0`, 85)** is five instructions:
`bomb->age (+0xc0) += dt; return age < stats->0xe4`. **`+0xe4` is `<Bomb
timetodie>`** - `WeaponStats_ParseBomb` (`0x0880cef0`) decompiles cleanly on
the relocated database now, and its eight stores are:

| Offset | Attribute |
| --- | --- |
| `+0xc8` | `damage` |
| `+0xcc` | `damageradius` |
| `+0xd0` | `blastradius` |
| `+0xd4` | `blastforce` |
| `+0xd8` | `absorb` |
| `+0xdc` | `slowdown_time` |
| `+0xe0` | `trigger_radius` |
| `+0xe4` | `timetodie` |

So the Bomb's fuse **is** its own `timetodie`, which `oag_weapons::projectile::
mine::Drop::bomb` took by analogy and can now take as recovered - and the
reason `Weapon_FireBomb` writes nothing to `+0x48` is that the Bomb counts
**up** from zero at `+0xc0` where the Mine counts **down** at `+0x48`. The
block sits at `+0xc8..+0xe4`, immediately before the Mine's `+0xe8`, exactly
where the pool-order arithmetic above put it. Note the order is *not* the
Mine's: `damageradius` is second and `timetodie` last.

**`Bomb_UpdateTrigger` (`0x08863d7c`, 85)** reads `stats->0xe0`
(`trigger_radius`) once, then for every craft `i` in the field: skip the
**owner** (`bomb->+0x48 == i`) while `Bomb_InArmingDelay(bomb)`
(`0x08863440`, 75: `age < 0.5`, the literal at `0x08ab1054`) - so **the layer
is exempt from its own bomb for half a second and no longer**, which this
engine's former "never tripped by the craft that laid it" did not match (fixed 2026-10-07, see the last section); if the bomb
is armed (`+0x3c` bit `8`), take `craft->+0x90 - bomb_position`, reject
outside a `±trigger_radius` box on each axis, then `|d| < trigger_radius`
sets destroy bit `4` and calls `Bomb_ApplyBlast(subsystem, slot, i)`.
**After the craft loop it also detonates on the Quake**: it locates the bomb
on the track (`AiTrack_UpdateCursor(100.0, ...)`) and if
`Quake_SpanIntensityAt_q` at that point exceeds `0.1` it raises the destroy bit
with no victim - a passing quake wave sets off every bomb it rolls under.

**`Bomb_ApplyBlast` (`0x088643d0`, 85)** is `Weapon_PostBlastImpulse_q`'s
shape written out inline against the Bomb's own block: the tripping craft
takes `+0x120 += stats->0xc8` (`damage`, flat) and `+0x130 += stats->0xdc`
(`slowdown_time`), records the owner in `+0x13c` and **weapon kind `6` in
`+0x138`** - the `Ship_Damage` `weapon_kind` sub-bucket pickups.md lists as
unmapped now has one case named - and then **every** craft within
`stats->0xd0` (`blastradius`) gets `normalize(d) * (1 - |d|/blastradius) *
blastforce (+0xd4)` added to its pending impulse at `entity+0x110`, the
tripping craft included. If the owner is `DAT_08b36c08` the victim's `+0x124`
byte is set to `1` (a "hit by that craft" flag, unread further). **No read
of `+0xcc` (`damageradius`) anywhere in this chain**, and a `lwc1`/`lw`
sweep for a `0xcc(` operand finds none in the group either - it stays the
one authored Bomb attribute with no consumer.

**`Bomb_Detonate` (`0x088640c8`, 88)** is the teardown, `(subsystem, bomb,
play_visual, victim)`. With `play_visual` set it builds an orthonormal basis
from the bomb's `+0x60` direction against the world up at `0x08a907c0` and
the bomb's position (`FUN_088633c0`), allocates a `0x110`-byte object and
calls **`BombBlast_Construct(obj, &basis, bomb)`**. Then, on the bomb's own
sound emitter `+0x4c` (the same per-projectile held-emitter shape `MINERADAR`
uses): stop it (`FUN_089393b8`), set its `+0x38` to `600.0`, play
**`BOMBEXPL_PC`** (`0x08a7c7a0`) plus a dry `EAR_SWTNR` (`0x08a7c7b0`) at
`0x400` when `victim == DAT_08b36c08 && DAT_08b36c08 != 0` and the session
is local (`DAT_08b31048 < 0xd`) or `DAT_08ab07e3` is set, otherwise
**`BOMBEXPL`** (`0x08a7c790`); release the emitter, clear `+0x4c`, clear bit
`4` of the bomb's `+0x2c`, zero `+0x74`. `DAT_08b36c08` is compared against
craft indices in both functions and reads as **the player's craft index**
(hypothesis, 65 - it is not read here beyond those two uses), which would make
`BOMBEXPL_PC` the "player craft" variant heard when the player is the one hit,
with an ear-sweetener layered on. A bomb that times out has `+0x74 == 0`, so
it takes the `BOMBEXPL` branch whenever the player's index is non-zero.

**`BombBlast_Construct` (`0x08872078`, 90)** is the Bomb's answer to
`PlasmaBlast_Construct` (`0x0885fd90`) and settles the two candidates the
section above left open - it is **neither** `WO_MINE_EXPLO` rescaled **nor**
`WO_BOMB_SMOKERING` alone, it is three things at once:

1. `Vex_LoadModel(..., "Data\Weapons\explosion_hemisphere.vex", 2048.0,
   0xfdb2, 0x3e9, 0)` at the basis (`+0xd4`, string at `0x08a7cba0`);
2. `Psys_Spawn_q(..., "WO_BOMB_SMOKERING", 'BOSM' /* 0x4d534f42 */, &basis,
   0, 0)` at the same basis (string at `0x08a7cbc8`);
3. `Vex_LoadModel(..., "Data\Weapons\Bomb_Shockwave.vex", ...)` (`+0xd8`,
   string at `0x08a7cbdc`) at a **second** copy of the basis whose position
   is pulled back by `1.0` (`DAT_08ab1070`) along the basis's own row index
   `1` (0-based) - **read 2026-09-23, and it is the direction row, not the
   orthogonalised-up row**; see below.

Its ramp constants, read 2026-09-23 (was "unread as to meaning"): `+0xdc
2.0`, `+0xe0 4.0`, `+0xe4 0.1`, `+0xe8 0`, `+0xec 12.0`, `+0xf0 0.075`, `+0xf4
1.0`, `+0xf8 0`, `+0xfc = DAT_08ab1074 (0.1)`; vtable `0x08acafd8`. See
[the per-tick animator, below](#2026-09-23-the-blasts-own-per-tick-animator-read).
**The `Bomb_Shockwave.vex` string the handover thread found at `0x08a7c190`
next to the plasma models is a second copy**; the one this constructor loads
is at `0x08a7cbdc`, in the Bomb blast's own `.rodata` group with the alloc
tag `"file"` at `0x08a7cb98`.

**What this changes for the engine, none of it done here:** the Bomb's fuse
label moves from "by analogy" to recovered; its layer-exemption is `0.5 s`,
not forever; its blast is `damage` + `slowdown_time` on the tripping craft
and a linear-falloff impulse on everyone inside `blastradius`; its detonation
draws a hemisphere model, a smoke-ring `.pob` and a shockwave model, and plays
`BOMBEXPL`; and a Quake wave detonates it. `MINE_EXPLO_EFFECT` reused for the
Bomb would be a stand-in, and so would be `WO_BOMB_SMOKERING` on its own.

## 2026-09-23: the blast's own per-tick animator, read

The stretch item the section above named and left for later: what
`BombBlast`'s own vtable Update slot does with the nine ramp constants and
the two child objects. Found the same way `plasma.md`'s own vtable read
was - `inspect_memory_content` on `0x08acafd8` lists seven `(0, address)`
pairs, six in the `0x0892xxxx`/`0x0894xxxx` shared-object ranges and one,
`0x0887250c`, inside this weapon's own module range. `decompile_function` on
it decompiles cleanly with nothing unreachable in its main body. Confidence
**85** - a direct decompile, and every field it touches was already named by
`BombBlast_Construct`'s own writes (below), which line up exactly.

**Renamed `BombBlast_Update` (`0x0887250c`, 85).** Signature `(dt, blast)`,
called once a tick through the vtable, same shape as `PlasmaBlast_Update`.

**The basis math, resolved from `Bomb_Detonate`'s own preamble.** The
"orthonormal basis from the bomb's `+0x60` direction against the world up"
line above was written from a structural read; the arithmetic is now decoded
in full, and it is ordinary Gram-Schmidt against a reference vector, the same
shape `docs/ghidra/functions/psp-pulse-usa/plasma.md`'s own basis-build
independently uses:

```c
dir   = normalize(bomb->0x60);              // the bomb's own frozen direction
up    = normalize(WORLD_UP - dir * dot(dir, WORLD_UP));  // Gram-Schmidt against world up
right = cross(dir, up);
basis = { row0: right, row1: dir, row2: up, row3: bomb->0xb0 (position, w=1.0) };
```

`WORLD_UP` is the vec4 at `0x08a907c0`, read directly: `(0.0, 0.0, 1.0,
0.0)` - **not this engine's own world-up axis**, see the porting note below
for why the literal constant carries over unchanged. **Row index `1` (the
`dir` row) is what `BombBlast_Update`'s pull-back reads** -
`vscl_q(basis.row1, DAT_08ab1070) ` then `shockwave.position -=` that -
which corrects the section above's looser "along the basis's second row":
the row read is the direction row itself, not the orthogonalised-up row, and
`DAT_08ab1070` reads `1.0` directly (confirmed by memory inspection).

**"One unit below" is the right plain-language description after all -
settled from the meshes, not from `bomb->0x60`'s own unread semantics.**
`oag-view --mesh` on both `.vex` files (2026-09-23, same session) shows
`explosion_hemisphere.vex` as a dome resting pole-up and `Bomb_Shockwave.vex`
as a ring lying flat, each on its own local `Y` - so whichever real-world
direction `bomb->0x60` carries on the original's own entity, the basis's
row `1` is the slot both authored models expect their own vertical axis to
occupy. That is a measured fact about the assets, independent of what
`+0x60` itself turns out to mean; see `Bomb_Detonate`'s own "chosen, not
measured" note for the part that is still a substitution.

**Per tick, sub-stepped in fixed `1/60 s` increments** (`dt / 0.016666668`
iterations, each stepping every ramp once) - which is this project's own
tick rate exactly, so a 60 Hz port takes one sub-step per tick with no loop
needed:

- **The hemisphere's own uniform scale**, `+0xdc` easing toward `+0xe0` at
  rate `+0xe4` (`2.0 -> 4.0` at `0.1`/tick) - applied to all three basis
  rows before the transform is written onto the hemisphere object at `+0xd4`
  via `FUN_08945284` (an already-shared "set local transform" call, not
  itself chased here).
- **The shockwave's own radial-only scale**, `+0xe8` easing toward `+0xec`
  at rate `+0xf0` (`0.0 -> 12.0` at `0.075`/tick), **gated to start only
  once `age > 0.1 s`** (the `if (0.1 < age)` guard wrapping this ease
  alone). Applied to basis rows `0` and `2` (`right` and the orthogonalised
  `up`) and **not** row `1` (`dir`) - the shockwave grows in the plane
  perpendicular to the bomb's own direction and keeps its extent along that
  direction fixed, which is the "ring" shape its name predicts.
- **The shockwave's own fading alpha**, `+0xf4` easing toward `+0xf8` at
  rate `+0xfc` (`1.0 -> 0.0` at `0.1`/tick, `DAT_08ab1074`), packed
  `alpha*255` into an opaque-white ARGB word (`0xffffff` RGB) and applied
  through `Image_SetVertexColours` on the shockwave object at `+0xd8`.
  **2026-10-01: it reaches the draw, as the ambient light's alpha** (`0x5d` written before each strip of the same
  `.vex`'s ship-explosion ring: `0xf4`, `0xb8`; lighting on, vertex colours the material), and is applied
  (`Drawable::tint([1, 1, 1, alpha])`, `ship-shockwave.md`).
- **The hemisphere hides at `age > 1.55 s`** (a node-flag clear on `+0xd4`,
  bits `0x2`/`0x4`) and **the whole blast object is torn down at `age > 4.0
  s`** - the shockwave's own flag clear plus the same
  `g_pending_destroy_count`/`FUN_08944a38` release path `PlasmaBlast_Update`
  uses at its own, shorter, `1.5 s`. **`4.0 s` is `BombBlast`'s own total
  lifetime**, corroborated by `+0xd0`'s own doc comment above this session's
  read as "age" and nothing else in the function writing it.
- The smoke-ring (`WO_BOMB_SMOKERING`) is spawned once, at construction, at
  the hemisphere's own unscaled basis - `BombBlast_Update` never touches it
  again; its own authored particle behaviour is the entirety of its motion,
  the same as every other `Data\Psys\*.POB` effect this engine already
  plays through `oag_fx::psys::Stage`.

**Porting note, chosen rather than measured**: `WORLD_UP` above needs no
axis swap at all - `Rocket_HitCraft`'s own `y - 2.5` drop
([rocket-visuals.md](rocket-visuals.md), ported as `CRAFT_BLAST_DROP`
subtracted along `Vec3::Y`) already establishes this engine's world axes
agree with the executable's own with no translation, so `(0.0, 0.0, 1.0)`
carries over as `Vec3::Z` directly - a horizontal reference, not an up
vector, which is what the meshes above say it has to be too. `dir` itself
substitutes the frozen craft orientation's **up** axis
(`orientation * Vec3::Y`) for the unlocated rear-emitter's own `+0x60` row,
on the same footing `mine::frozen_pose`'s own doc comment already states -
no new confidence score, carried forward from that existing hedge. (An
earlier pass of this note read `dir` as the craft's *forward* axis crossed
against `Vec3::Y`, matching `blast_models::billboard_matrix`'s own column
convention rather than these two meshes' own authored vertical axis; caught
by comparing a rendered capture against the `oag-view --mesh` screenshots
above, not by re-reading this page.)

### 2026-10-05: the blast's two models play their texture on the blast's age

`BombBlast_Construct` seeds each model with `Node_SetAnimTimeTree(0.0)` (`0x08872268` hemisphere,
`0x08872470` shockwave) and never calls it again, so the `Mesh` update that adds the clock's delta
leaves `mesh+0x40` equal to the blast's age at rate 1. Read at the instruction and measured live on two
boots (spawn clock = detonation clock, slope `1.0000`); confidence 88. The derivation and the table are in
[`anim-transform.md`](anim-transform.md#a-meshs-texture-time-seeded-per-spawn-so-object-age-2026-10-05).
`BombBlast_Update` scrubs no *node* animation, but the models' texture tracks do play, and
`write_bomb_blasts` uploads them.

## 2026-09-16: the fuse and the trip spend differently, and the pool is named

`FUN_08867370` and `FUN_08867b50` decompile cleanly on the relocated database
and are read in full; both are named, the "no caller resolves" reservation
notwithstanding, because the bodies leave nothing to guess and the pool is
reached through a node's update virtual the way every pool in this engine is
([resource-loading.md](resource-loading.md)).

**`MinePool_Update` (`0x08867370`, 85)** is the Rocket pool's template on
the `+0x164` count: pass one runs `FUN_08859ecc(dt, mine)` (the per-mine
animator) and the fuse `mine+0x48 -= dt`, and when the fuse reaches zero on a
live mine (bit `0`) it raises **only the destroy bit `4`** - under a network
mode it also posts a `0xc`-byte "mine gone" record. Then
`Mine_SweepCraftTrigger(pool, i)` per mine. Pass two, for every slot with bit
`4`: `Mine_SpawnExplosion`, clear bits `2`/`3`, `+0x40 = -1`, swap with last.
**Nothing on the fuse path or the teardown touches a craft**: a mine that
times out plays `WO_MINE_EXPLO` and hurts nobody, the same shape as a
Missile's expiry. The earlier "both ways out spend a blast" reading in
`crates/weapons/src/projectile/mine.rs` was an inference from the chain's
shape and is retracted; the port now expires a mine quietly.

**`Mine_SweepCraftTrigger` (`0x08867b50`, 88)** is the only caller of
`Weapon_PostBlastImpulse` (`0x0886794c`), and it calls it for **one craft**:

```c
r = stats->trigger_radius;                            // +0x100
for each craft (the firer only once FUN_08859f04(mine) says the mine is armed for it):
    if (!(mine->flags & 8)) continue;                 // armed
    if (Ship_IsTargetable(craft) /* FUN_08862d4c */) {
        d = craft->pos (+0x90) - mine->pos;
        if (|d.x|,|d.y|,|d.z| < r && |d| < r) {       // cube, then sphere
            mine->flags |= 4;                         // detonate
            if (|d| < stats->blastradius /* +0xec */)
                Weapon_PostBlastImpulse(pool, mine, craft);   // THIS craft: damage, slowdown, impulse
            (network record)
        }
    }
}
if (Quake_SpanIntensityAt(track cursor at the mine) > 0.1) mine->flags |= 4;   // a Quake sets mines off
```

So the blast is **not** a radius sweep: the craft that trips the mine is the
only one credited, and only if it is also inside `blastradius` - a craft
tripping it from the ring between the two radii sets it off for nothing, and
a bystander standing inside `blastradius` takes nothing. `Weapon_PostBlastImpulse`'s
own reading on [contact-response.md](contact-response.md) had it as a
single-target function all along; what was missing was that nobody loops it.
Two more things fall out: **a Quake wave passing under a mine detonates it**
(no credit to anyone), and `Rocket_SweepProjectiles`
([rocket-visuals.md](rocket-visuals.md)) raising bit `4` on a mine a rocket
flies through has the same quiet result. Ported: `blast::blast_mine_trip`
credits the tripping craft alone, gated on `blastradius`, and
`Projectiles::sweep_rockets_through_laid` is the rocket trip, and
`Race::advance_quake` (`crates/raceplay/src/weapons/single_instance.rs`) is
the Quake trip - after the wave's own hits it retires every laid mine whose
located progress is inside the wave's radius and ignites the effect with no
craft credited (`crates/game/tests/mine_ground_truth.rs`,
`a_quake_wave_under_a_mine_sets_it_off_quietly`). The original's `> 0.1`
intensity threshold is not modelled: `Quake_SpanIntensityAt` is read off the
deformation table this port does not build yet, so the radius stands in for
the wave's footprint.

`FUN_08859f04` (the firer-arming test, 60) and `FUN_08862d4c` (targetable,
shared with `Rocket_HitCraft`'s shield check, 60) stay unnamed.

## 2026-09-24: a laid mine spins, and the Bomb sits square to the world

Read on the bridge, then measured live on PPSSPP (`pulse-psp-usa.chd`,
Single Race, the player's own weapon record) by setting the fire bit inside
`Weapons_DispatchFire` and breaking on the node hand-off. Raw samples are in
`data/scratch/weapon-pose/psp/` (not committed).

### The Mine: `0.6 x Rot(axis, -4 x fuse)`, rebuilt every tick

`Mine_Init` does copy the craft's rear anchor into `entity+0x60..+0x9c`, as
this page said - measured, that anchor is the craft's display matrix, rows
`(up x f, up, f)` scaled by `g_craft_scale` `0.75`. **But nothing draws it.**
`Mine_Init` then calls `Mine_PoseNode` (`0x08859ce4`), which
`Mine_Update` (`0x08859ecc`, called per live mine from `MinePool_Update`) calls
again every tick, and it overwrites the whole 3x3:

```c
angle = entity->fuse * 4.0;                       // +0x48, counts down
wrap angle into (-pi, pi];
Math_BuildAxisAngleMatrix(angle, &entity->matrix,  // +0x60, the 4x4
                          &entity->axis);          // +0xc0, Mine_Init's random unit vector
entity->matrix.row3 = saved position;             // +0x90 survives
entity->matrix.rows0..2 *= entity->scale;         // +0xd0
Node_SetMatrix(entity->node, &entity->matrix);    // +0xb4, 0x08945284
```

`entity->scale` is the code literal `0x3f19999a` = **`0.6`**, written by
`Mine_Construct` at `0x08859a30`. `Mine_Update` itself only adds `dt` to
`+0xd4` (an age) and runs the owner-gone check at `0x0885eca4` before posing.

**Measured.** Forty `Mine_PoseNode` hand-offs across the cluster: every
row has length `0.600`, the position never moves, and each mine's matrix
equals `0.6 x` the right-handed rotation by **`-4 x fuse`** about its own
`+0xc0` axis to four decimal places in every element (the `+4 x fuse` reading
misses by 0.4 to 1.1). The fuse starts at `7.0` (`<Mine> timetodie`) and falls
at one per second, so a laid mine **turns at 4 rad/s about its own random axis
for its whole life**. Confidence **92**.

So the Mine is not drawn in the craft's pose at all, and it is drawn at
three-fifths of its authored size. This engine drew it static, in the firing
craft's body orientation, at full size - see `mine::frozen_pose`, which was
chosen, not measured.

### The Bomb: up from the craft, yaw from the world

`Bomb_Construct` (`0x08862fc0`) loads `Data\Weapons\Pulse_Bomb.vex` and sets
`+0x80..+0xbc` to the identity. `Bomb_Init` (`0x08863188`, called from
`Weapon_FireBomb`) then:

1. raycasts ten units down from the drop point;
2. writes `+0x90` (row 1) = the normalised vector `Weapon_FireBomb` staged
   (`-craft[+0xb10]`);
3. orthogonalises whatever `+0xa0` (row 2) already holds against it and
   normalises - `(0, 0, 1)` from `Bomb_Construct` on a fresh entity, the
   previous drop's row 2 on a pooled one;
4. writes `+0x80` (row 0) = row 1 x row 2 (`vcrsp.t`), and hands `+0x80` to the
   node (`0x08863390`).

**Measured**: the matrix handed over at the drop was rows `(1.0, -0.0002, 0)`,
`(0.0002, 0.9998, -0.0206)`, `(0, 0.0206, 0.9998)` - a rotation, determinant
`+1`, **scale 1** - and the node held exactly that matrix `0.5 s` and `1.5 s`
later, so the Bomb is posed once and never again. The craft was heading `+X`
at the time, so this measurement cannot by itself tell world `+Z` from the
craft's right; the static read can, and it says row 2 is the pooled entity's
own previous row 2 starting from `(0, 0, 1)`, never the craft's. A second drop
on the same pooled entity read `+0xa0 = (-0.033, 0.077, 0.997)` before the
orthogonalisation, which is that residue. And row 1 on a flat straight cannot
separate the craft's up from world up; the read says it is the craft's
`+0xb10` row, negated. Confidence **88**.

`Weapon_FireBomb`'s vector was recorded on this page as "the craft's forward
row, negated". It is `-craft[+0xb10]`, and live it is `(0.0002, 0.9998,
-0.0206)` - an up vector, not a forward one.

### What this engine now does

- **Pulse's Mine** draws `0.6 x Rot(axis, -4 x fuse)` at its position. The
  mechanism, the rate, the sign and the scale are measured; **the per-mine axis
  is not the original's** - it is drawn from a render-side seeded roll (the
  same shape, a normalised `U(-1, 1)^3`) because the simulation's own RNG must
  not advance for a picture. Chosen, not measured.
- **Pulse's Bomb** draws `(up x Z', up, Z')` with `up` the frozen pose's own
  `Y` and `Z'` world `+Z` orthogonalised against it - `Bomb_Init`'s shape,
  without the pooled entity's residue.
- **HD's Mine and Bomb** keep the frozen craft pose: HD is a different
  binary and neither of its poses was read. Chosen, not measured.

## 2026-10-01: seen from the original's own chase camera, and what the first frames show

The 2026-09-24 pass had no picture of the original's charges because they are
laid at the craft's position and left behind. They are visible for **two
frames**, as the craft moves off them and before the camera passes over them:
fire at speed (106.2 u/s, 120 frames after GO, Time Trial, Talon's Junction,
Venom/Assegai; `scripts/psp-weapon-pair.py mine --set-word 0x1ac=5`, the round
counter a hand-set fire bit does not arm) and photograph every frame. The
camera is 11.4 behind and 2.6 above the craft (read off the camera node,
`camera.md`), so a charge laid at the craft's centre passes under the lens
about 6 frames after the drop. Frames: `data/scratch/pulse-weapons/mine-orig-mv1`,
`bomb-orig-mv1` (not committed); ours `mine-ours-tt480`, `bomb-ours-tt480`.

**Measured (a stationary craft, `Mine_PoseNode` probe, 5 mines, fuse 7.00
each):** every mine's matrix translation equals the craft's body position to the
hundredth (`6.08, -50.07, -196.03` both), so **the original lays a Mine at the
craft's own position, with no push back along the hull**, and its row lengths
are 0.75 at the first hit (the anchor's `g_craft_scale`, as above) before
`Mine_PoseNode` scales it to 0.6. The cluster leaves at stop frames 0, 7, 13, 19,
25 (6-7 apart). A stationary craft cannot see them (they are inside the hull);
the frames show the blast, a full-screen yellow wash and debris, between
fire+24 (clear) and fire+32, on the craft that laid them (the 2026-09-30
section above, 29 frames, consistent).

**Mine, side by side at native size.** The model matches: a dark grey
triangular prism with a grille, scaled and tumbling as drawn. It is visible for
two frames in the original (fire+5 and +6, emerging from under the hull) and for
three in ours (from fire+0, because ours is laid `hull_extent` behind the craft
centre and so is already clear of the hull). The second mine follows 6-7
frames later on both sides. Difference, named: **laid-position offset**, which is
`projectile::mine::drop_point`, documented there as chosen ("pushed back by the
hull's own extent"). It is now measured against; the same applies to the Bomb
(`Weapon_FireBomb` takes the same anchor).

**Bomb, side by side.** *(Both differences below are resolved: see the
2026-10-01 second pass at the end of this page.)* The same canister model on both sides (olive hexagonal
body, hex plate on top, yellow bands near the base), laid at the same place
(ours about 5 units behind the craft centre, the same offset as above) and out of sight within
three frames. Two differences, neither fixed:

1. *A ring.* Ours shows a wide, flat yellow ring around the canister for its first
   three or four frames (about 2.5 x the canister's width). The original shows
   none; at fire+3 it shows a thin vertical shaft of white-yellow light through the
   craft, and two yellow bands at the canister's own width at fire+5. The original
   spawns **no particle effect at all** for a Bomb launch: a `Psys_Spawn_q` probe
   (`0x08915484`, 120 frames after the fire) caught nothing but a wall-scrape
   spark. So the shaft and the bands are the model's own geometry or material,
   and ours' ring is either that same geometry drawn at a different scale or
   orientation, or something this engine adds. **Not isolated**; the first step is
   to view `Pulse_Bomb.vex`'s meshes and compare their extents with the ring.
2. *No detonation in the original's frames.* A moving bomb is left behind and
   its `BombBlast` is never in view within 58 frames; ours also holds it (fuse
   20 s, never detonates in 300 ticks). No difference.

## The Mine trips on its own layer too, measured live (2026-09-30)

On PPSSPP (Talon's Junction, the player stationary on the start line) a Mine
laid with the fire bit went off **29 frames** later and a Bomb **30**, each
centred on the craft that laid it, each calling `ScreenFlash_Start` from its own
explosion function (`Mine_SpawnExplosion` kind 8, `BombBlast_Construct` kind 3;
[screen-flash-callers.md](screen-flash-callers.md), "Measured live"). Nobody else
was near. So the layer trips its own charge after about 0.5 s: the Bomb's
`Bomb_InArmingDelay` is the recovered reason (above), and this is the first
observation of the same for the Mine, whose `Mine_SweepCraftTrigger` was not
read for the point.

`oag_weapons::projectile::mine::triggered_by` still excludes the owner for good,
labelled as this engine's choice. **That choice now has a measurement against
it.** Changing it is a gameplay change (a craft reversing into its own cluster
takes a blast in the original) and needs the racing gate; it is left for a lane
that owns it.

## 2026-10-01, second pass: the Bomb's ring is an animated node, and the drop point is the craft's own

Two of the differences the section above named, closed. Same state and method
(`psp-weapon-pair.py`, Time Trial, Talon's Junction, Venom/Assegai, 106.2 u/s at
`--go-offset 120`); frames and logs under
`data/scratch/pulse-weapon-fx/` (not committed).

### The wide flat ring was a node that never moved

`Pulse_Bomb.vex` (`oag-vex --example bomb_nodes_probe`) is one tree under `world`:
`orbit` (an `Anim Transform`, `0x3c0`) with `orbitShape`, a mesh of 79 triangles - the
yellow ring - and `bomb` (another `Anim Transform`) with `bombShape`, the 92-triangle
canister, plus a shadow hull. **Both transforms are keyframed rotations**, 180 frames at
60 Hz, one turn in three seconds: `orbit` turns about its own `X` (matrix at 0.25 s:
`[1 0 0; 0 .882 .472; 0 -.472 .882]`), `bomb` about `Y`. The ring lies flat at time zero,
and **the loaders and `oag-view` drew every laid Mine and Bomb at that time-zero pose**, so
ours was the ring at rest: wide, flat, 2.5 x the canister.

Turned about `X`, the ring's plane always contains `X`. The bomb is posed square to the
world with `X` along the heading (`Bomb_Init`, above), so from the chase camera the ring is
**always edge-on**: it reads as a thin vertical shaft when its plane is upright and as a
tilted arc between. That is the original's "thin vertical light shaft"; the "two bands at
the canister's width" are the canister's own yellow texture bands. No particle effect is
involved (the 2026-10-01 `Psys_Spawn_q` probe stands).

**The clock is the one animation clock.** `AnimTransform_Update` (`0x088fe0a8`) reads
`g_ingame->0x40` (`anim-transform.md`) and integrates `dt = clock - node->0x80` into the
node's own time; a node whose `+0x80` starts at zero takes the whole clock on its first
update, so its time is `clock mod LoopEnd` whenever it runs. The Bomb entities are built
once at the race's start (`FUN_088634e8` makes 32 of them, flags cleared; `Bomb_Init`
sets `|= 6`). **Measured:** `g_ingame->0x40` read 90.676 at GO and 92.695 at the fire
frame of one run, **49.52 s in the countdown of another** - the clock counts from the
game's own start of the session, so the ring's phase at a launch is not a constant of the
launch. Two launches of this harness at different clocks show different ring poses (a
vertical shaft in the first pass's frames, whose clock was not logged, and a tilted arc in
the second pass's, clock 92.695 = 2.695 s into the loop, ring turned 5.6 rad). Ours, pinned to that clock with `--anim-seconds`, draws the same
tilted arc at the same frames.

What this engine does now: `Scene::write_weapon_models` writes the Mine's and the Bomb's
node-animation tables at the race's `tick / 60` like the scenery's
(`write_node_anims`), so both play their authored `Anim Transform`s. The Mine's one
animated node (`mine`, a rotation tilted 30 degrees off `Y`, one turn in 2 s) plays too
**by analogy with the Bomb, not seen**: the original's two frames of a laid Mine are too
few to show a second spin, and the Mine's own node was never probed. The `Mine_PoseNode`
spin is the matrix handed to that node and is unchanged, so the two compose. **The phase is the engine's tick clock, not the original's
session clock** - which carries an offset this engine has no source for (the same offset the
scenery already lacks), so an exact phase match is not claimable; the shape, the rate and
the mechanism are the original's. Pinned by
`crates/game/tests/bomb_orbit_ground_truth.rs`: two frames at clocks 0 and 0.75 s differ
at 18,778 pixels outside the control pair's own difference with the write, 2,201 without it.

### The drop point is the craft's own position, measured

`Bomb_Init` (`0x08863188`) takes the drop point in `a1`. Probed on the running original
(`--probe bomb`): stationary, `a1 = (6.0764699, -50.0664253, -196.0265045)` and the rigid
body's position (`body+0x30`) on the fire frame is **the same to the last bit**; at
106.2 u/s `a1 = (124.4720764, -47.9094696, -196.9474182)` and the body position on that
frame is again identical - not advanced by the velocity. Together with the Mine's five
stationary `Mine_PoseNode` matrices (above) the law is: **a Mine and a Bomb are laid at
the craft's own position, no push back along the hull.** Confidence 90 (two states, both
exact; the Mine's moving-craft case rests on the shared `Weapon_FireBomb` shape and the
frame series, not a Mine probe at speed).

`oag_weapons::projectile::mine::drop_point` took the hull's own extent as a chosen
offset; it now returns the body position. It moves simulation state (a laid charge's
position, hashed per projectile); no committed golden hash covers a laid charge. The
`mine_ground_truth` tests that asserted "behind the craft" now assert "at the craft's
own position". With it a laid Mine is under the hull for its first frames and emerges as
the craft leaves, as in the original's frames (ours t406 against the original's k5).


## 2026-10-01, third pass: the Bomb re-looked at with its textures decoding, and its detonation seen for the first time

Same state as the second pass (Venom, Assegai, Talon's Junction, Time Trial, fire at speed
106.2, native 480x272, the original's own animation clock read per frame and pinned on our
side with `--anim-seconds`, which puts the ring at the phase the original's frame shows).
Frames and probes are under `data/scratch/pulse-rocket-look/` (not committed).

### The canister at launch

Compared at fire+4 to +6, after the swizzle fix made `Pulse_Bomb.vex`'s textures decode.
**Fixed**: the canister's mask. The weapon bodies were not stamping the glow mask, so the
body kept the road's `255` under it and bloomed into a pale grey-green wash; the original
writes `4` over the road there and `0xba` on its lamp batch, and ours now reads the same
counts - see [`glow-mask.md`](../../../rendering/glow-mask.md#the-weapon-bodies-and-the-bombs-dome-stamp-too-2026-10-01).
The body is now the original's darker olive. **Still different, not isolated**:

- *The ring's brightness.* The original's ring at this phase is a thick, solid, saturated
  yellow band round the canister's lower half; ours is thinner, dimmer, three yellow lines
  on brown. The batch's GE state in the original was read off a dump (one GE dump, prim 668: 81 vertices,
  `vt 0x13d`, texture format `T4`): additive `SRC_ALPHA, FIX 0xffffff`, colour test
  `NOTEQUAL` black, depth write off, culling off, lighting off, stencil off, filter
  `LINEAR_MIPMAP_LINEAR`, **texture level mode 2 with bias 2.875** (`TEXLEVEL` byte 46 / 16:
  a per-texture bias, where this engine uses the one `1.0`), vertex colours all `0xffffffff`,
  `u` and `v` in `0..128`. Those match ours but the bias, and at the bomb's 12 to 15 units of
  view depth the slope law gives level 0 either way; the gap is therefore not the level.
  Candidates left: the texel alpha (`237`) used as a vertex-times-texel factor, and the
  authored mip chain.
- *The top plate.* The original's hexagonal plate on the canister is an orange hatch; ours
  is brighter and yellower. Same texture family, not isolated.

### The detonation, pictured

`psp-weapon-pair.py bomb --detonate-bomb-at 8 --detonate-ahead 120`: the first laid Bomb
(pool `*0x08b3bf90`, slots at `+0x44`, count `+0xc4`) is moved 120 units ahead of the craft
along its travel by writing its position row (`+0xb0`, the one `FUN_088633c0` reads) and
its age (`+0xc0`) set far past `timetodie`, so `BombPool_Update` detonates it on the next
tick with the camera 120 units away. The frames (`bomb-det-a/`) show, from the blast:

1. **A full-screen orange wash, fire+10 to about +25**, fading - `ScreenFlash` kind 3
   (`(1, 0.5, 0, 0.7)`, 0.75 s, near 100, far 300 -> 0.9 at 120 units), which
   `BombBlast_Construct` starts. Ours draws the same wash at the same frames and colour.
2. **A bright yellow-white fireball dome** at the blast, from fire+14, growing to a 100 px
   hemisphere by +40, yellow at the rim and white at the core.
3. **A brown smoke wall** rolling out from it from +32, 100 px and more across by +50,
   with **rock debris** flying (the `debris` emitter's 4x4 rock sprites).
4. A shockwave that passes through the dome and fades by +30.

Ours, built from the same recovered animator, matched in kind and timing and differed in
size: the smoke was a 25 px puff and the dome a faint grey outline. **Two causes fixed**:
the smoke ring spawns on a ring round the blast that **widens from 12.94 to 23.3 units over
the emitter's twenty ticks** (`psys` shape 3, `ParticleSystem_EmitRing`, and the animated
attribute that scales its extent; [`particle-system.md`](particle-system.md#shape-3-is-a-ring-or-a-disc-particlesystem_emitring-2026-10-01),
read live on this effect, two boots: 13.1 to 23.4 units from the centre) where ours spawned
it at the anchor, and the dome stamps the glow mask (`hemisphere_disperse1_ADD_GLOW`, stencil `80`
in the GE list). The debris emitter's rock sprites now exist at all (4-bit textures,
[`pob.md`](../../../formats/pob.md)). **Still different**: the
dome reads yellow and opaque in the original and white and thin in ours, and the original's
smoke is denser and a little larger at fire+50; neither cause is isolated. The dome's palette
is read (`(255,253,238,80)` through `(252,151,0,80)` and then alpha `0`: every visible texel
is alpha `80`, so it is additive at 31 %), which is the same in both, so the brightness the
original shows is its bloom (mask `80`) and the sky behind it; our bloom of the same mask is
not separated from the extra white.

Confidence: **88** for the smoke ring's placement and widening (a particle pool read on two
boots); the dome's mask and the ring's and dome's GE states are **seen once** (one GE dump,
one boot - no score); the wash is a visual match of colour and frame, with the formula read
earlier (`particle-system.md`); the rest above is a description, not a measurement. The detonation was moved by a debugger
write, and the blast's own position is therefore the written one; the blast force is not
applied (that is `Bomb_ApplyBlast`, a separate call the write does not reach).

## 2026-10-01: the Mine's explosion, pictured against ours (pulse-fx-3)

Method: a stationary craft in a Single Race on Talon's Junction fires a Mine at stop frame 300 (`scripts/psp-weapon-pair.py mine --no-hold
--fire-frame 300 --set-word 0x1ac=5 --shots ...`, PPSSPP 1.20.4, software renderer, native 480x272; frames `data/scratch/pulse-fx-3/mineB`,
a GE dump at fire+31 and +33 in `mineGE31`, `mineGE33`, the pool probes `mineP_rolled`).

- **A stationary craft trips its own Mine at once**: the explosion starts at fire+30/31 (the arming delay, `screen-flash-callers.md`'s
  29 frames), not at the 7 s fuse. Ours kept a permanent owner exclusion until 2026-10-07 (retired, see the bomb-owner section below). The
  comparison below ignites ours at the craft's position 29 ticks after the press with a scratch-only hook (`OAG_SCRATCH_IGNITE`, not committed).
- The original's picture, fire+31 to +60: a yellow-white wash from +31, radial rays and orange burning debris across the screen to
  about +50 (the mean brightness of the playfield rows holds at 175-215 for twenty frames, then falls to 96 at +60), the player's shield bar goes red
  and the camera shakes. **It is a cluster**: the Mine scatters several charges and each goes off, so the wash has three bumps (+31, +38 to +40, +44);
  ours ignites one blast (peak 194, back to baseline in sixteen frames). In the GE dump at +31 the template quads sit at three different view depths
  (`16.5`, `11.2`, `9.3`: the `ring`'s texture at two of them), more than one explosion's worth (`mineGE31`; the cluster reading is an inference, the count of charges was not read).
- The effect's pools agree with ours at first order (`WO_MINE_EXPLO`'s smoke ring: two particles, half-size `4.0, 5.31, 6.61, 7.92 ...`, colour
  `1d2863` rising in alpha; the `Debris` pool: ten white quads of half-size `0.58-1.0`; ours `4.00, 5.1 ...`, `0.64-1.15`). The ring's colour `(29, 39, 98)` is
  authored, so a cyan-blue haze in the middle of the burst is the original's too.
- **The Mine's two templates were drawn with the wrong sprite** until this change (`ring` and `BANG` bound their parent's 64x64; their own are 64x64 and
  32x32, [particle-system.md](particle-system.md#a-sprite-templates-own-sprite-and-what-the-explosions-first-five-frames-are-2026-10-01-pulse-fx-3)).
  The dump's template quads: the `ring` 64x64 at half-extents `14 x 9.4` (aspect `1.5`) at fire+31, the `BANG` 32x32 as a bar `85` units wide, rotated, at
  depth `11.2`.
- **Not isolated:** the mean red of the burst reads `215` on the original and `194` on ours with green `218`/`212` (ours leans green-cyan, the original
  yellow-white); the original's debris are orange and large where ours are small and brown-grey; ours has no cluster. The flash's colour (kind 8, yellow) and
  duration (`0.4 s`) were not re-read. Confidence: the template-sprite finding **90**; the picture comparison is a description, one boot.
- Pointer: the Mine pool is `*0x08b3bf88` (count `+0x164`, slots from `+0x44`; `+0x48` the fuse), found by reading the candidates around the Bomb's `0x08b3bf90`
  (a candidate, not confirmed). The position row is **unverified**: a live mine read `(566, -18.7, 5.8)` at `+0x90` with 0.6 s of fuse, which may simply be another
  craft's mine elsewhere on the track; a detonation-by-write for the Mine needs that settled first.


## Bomb and Mine owner exemption is a 0.5 s window (bomb-owner lane, 2026-10-07)

**Law (Pulse, confidence 85).** Both laid weapons skip the laying craft in their
trip sweep only while the charge is younger than `0.5 s`, then trip on it like any
craft:

| Weapon | Function | Test | Age field |
| --- | --- | --- | --- |
| Bomb | `Bomb_UpdateTrigger` (`0x08863d7c`) | `i == bomb+0x48` and `Bomb_InArmingDelay` skips | `bomb+0xc0`, added by `Bomb_AdvanceFuse` after the trigger pass |
| Mine | `Mine_SweepCraftTrigger` (`0x08867b50`) | `i == mine+0x40` and `Mine_InArmingDelay` (`0x08859f04`) skips | `mine+0xd4`, added by `Mine_Update` before the sweep |

`0x08859f04` decompiles to `return *(float *)(mine + 0xd4) < DAT_08ab0efc;` and
`DAT_08ab0efc` reads `00 00 00 3f` = `0.5` (read with `read_memory` 2026-10-07).
`Bomb_InArmingDelay`'s literal at `0x08ab1054` reads the same word. Two sites, one
literal, plus the live Pulse Mine run above (a stationary craft trips its own Mine at
fire+30 frames, `0.5 s`): the Mine's `0x08859f04` is named `Mine_InArmingDelay`
at 75 (the sibling's confidence; the age field's role rests on `Mine_Update` adding
`dt` to `+0xd4`).

**HD (confidence 70).** `NormalBomb_Update` (`0x001443f8`) contains **no trigger or
owner test**, a clean negative: it does `age (+0xe8) += dt`, and when the age reaches
`stats+0x104` (`timetodie`) or flag `0x80` of `+0x40` is already set it runs the
ship-in-radius loop over `0x002d64d0` (a track/AI query at `-1`, not an owner test),
`NormalBomb_Detonate` and the blast. So the bomb's trip (the writer of flag `0x80`)
is elsewhere, not found; next address to try is the HD pool update that owns the
`NormalBomb` instances (the OPD at `0x00877238` is the update's vtable slot, so its
owner is the vtable's constructor, `NormalBomb_Construct` `0x00144a48`). The window
therefore rests on the film, not on code: the owner's own bomb trips about `0.6 s`
after laying in bomb age (`rpcs3-capture.md`, "Video time is the bomb's age"), which
is Pulse's `0.5 s` plus a frame or two. One shared constant, no per-title field.

**Ported.** `oag_weapons::projectile::mine::OWNER_EXEMPT_SECONDS = 0.5` and
`triggered_by(mine, owner, age, slot, position, radius)`; `Projectile::age` (hashed)
is raised by `advance_laid` (Mine before the trip test, Bomb after, as the two
originals order it). Tests: `projectile::tests::owner_exempt`. The AI's hazard
scan (`Race::hazard_for`) skips its own charge only while it is exempt, so it
steers round its own old mine like anyone's.

**Not modelled.** The cube-then-sphere test is the sphere alone here; the
"armed" bit `8`; `FUN_08862d4c`'s targetable gate. 2048/Omega: *not checkable*,
neither title's bomb trigger is located, and the weapon table has no equivalent
(`checked, differs` is not claimable either).

## 2026-10-08: the cluster is the round counter, and our drop is one tick too slow (pulse-weapon-look)

Settles the 2026-10-01 pulse-fx-3 section's open questions on the Mine's explosion.
Method: PPSSPP 1.20.4, software renderer, native 480x272, a stationary craft in a Time Trial
on Talon's Junction fires at stop frame 300 (`scripts/psp-weapon-pair.py mine --no-hold
--fire-frame 300 --set-word 0x1ac=R`), a frame every one to two frames from fire+29 to fire+64.
Ours: `oag-game --race --mode time_trial --give mine --input-script
verification/scenarios/mine-stationary-fire.inputs`, tick `304 + k`. The number plotted is the mean
brightness of the top 200 rows (`R+G+B)/3`). Frames and logs: `data/scratch/pulse-weapon-look/`
(`mine-orig-r1`, `mine-orig-r5`, `mine-orig-r5b`, `mine-ours*`).

**Is the three-bump wash the cluster? Yes (confidence 80: one PPSSPP boot, the five-round run restarted twice in it and the one-round run once - restarts, not reboots).** The discriminator written before the capture: one round (`0x1ac=1`) must show one
bump, five rounds several.

| round counter | brightness, fire+k | reading |
| --- | --- | --- |
| 1 | 98 at k=0, 88 at k=30, **186** at 32, 144 at 38, 100 at 46, back to 90 by 52 | one blast, one decaying bump, 20 frames |
| 5 | 88 at k=30, 192 at 32, **210** at 38, **205** at 44, **210** at 50, 159 at 54, 110 at 60 | bumps at 32, 38, 44, 51 (both restarts agree): one per laid mine |

Each mine trips 0.5 s after it is laid (`Mine_InArmingDelay`, the owner-exemption section below), the
first mine leaves on the press frame and the rest six frames apart, so the blasts go off at
`+30, +36, +42, +48` and render two frames later. The picture is the sum of the mines' bursts. Two
corollaries closed:

- **The "hue" gap (red 215 against 194) was a single blast against a five-mine stack.** Cluster against
  cluster, the mean RGB over fire+32..64 is `171 186 142` on the original and `168 181 138` on ours: within 3 %.
  A single blast on the original reads `200 220 138` at k=32 against ours `227 235 154`: ours is the
  brighter single burst by 8 to 14 %, the one place a gap is left (see below).
- **The fifth mine does not go off on the original either.** The owner is pushed outward (0.2 units at +33,
  1.8 at +45, 3.2 at +51, 4.8 at +57, read from the log's body position) and the trigger radius is 3, so
  mine 5 (tripping at +54) finds the owner out of reach: four blasts.

**Where ours diverged: the drop interval (landed 2026-10-08, lane `pulse-mine-drop`, see the end of this section).** Ours lays the cluster at seven-tick spacing, so
its blasts are at k=32, 39, 46 and a **third** only; the fourth is lost to the owner leaving the trigger radius first
(`MINEDBG`, a temporary print: trips at age 0.5000 with the owner 0.002, 0.57 and 1.69 units off, the fourth
would have been past 3). `Held::advance_drop` tests its `0.1` s reload with a strict `> 0`, and at the engine's
exact `1/60` s step `0.1f32` less six `dt`s leaves `+1e-9`, so every gap is seven. The original's own `dt` is the
display's `1/59.94` s with jitter, which carries six frames past zero and makes a seventh only after a short
frame (the 2026-09-15 measurement: six frames, seven now and then). Passing the test at a slack of `1e-5` s
(`DROP_TIMER_SLACK`, **chosen, not measured**: far below one tick, far below the jitter) gave blasts at k=32, 38, 44, 50
against the original's 32, 38, 44, 51 with the brightness tail agreeing (123 against 124 at k=59), and a unit test
(`a_cluster_leaves_every_six_ticks_at_sixty_hertz`, drops on ticks 0, 6, 12, 18, 24). **It was held back at first**: it moved `ai_dekonstruct_black_ground_truth` (`phantom_seed_3` forward 3 destroyed against a ceiling of 1,
`rapier_seed_1` and `rapier_seed_3` over theirs too; all 13 cells pass with slack 0). The seven AI Aces play with weapons on, so
mine kills are the obvious suspect, but **whether the extra deaths are mine kills or the per-seed divergence the test's own
notes call chaotic was not checked**. Those ceilings "may only fall" and are the AI lane's, so raising
them is the maintainer's or the lead's call. The ready patch is `data/scratch/pulse-weapon-look/mine-drop-six-ticks.patch`
(commit `9dc9b01b8` on the lane's history, reverted in the next commit); whether the AI should lay or avoid
mines the way the original does is the open question it carries.

**Still open on the Mine:** ours' blasts peak about 10 % brighter (205 to 221 against 197 to 204) and a
single one 8 to 14 % brighter; the original's debris reads as larger tan chunks and ours as smaller
orange sparks plus a few chunks (frames `mine-late.png`, one boot, a description not a measurement);
the original's camera shakes and the craft is pushed, ours' shield bar differs (not this lane's).
Pure's mine uses this same timer: **checked, applies if landed (the timer is shared engine code), not measured on
Pure's own binary**, so the six is Pulse's.

### Landed 2026-10-08 (pulse-mine-drop)

`DROP_TIMER_SLACK = 1e-5` is in `Held::advance_drop`, with `a_cluster_leaves_every_six_ticks_at_sixty_hertz`
(fails with slack 0). The three held ceilings were separated before being raised: with the fix, PHANTOM 3
forward loses 3 craft (old spacing 1), RAPIER 1 reversed 1 (0), RAPIER 3 forward 3 (1). The extra deaths are
weapon-dominant by shield accounting (weapon loss over wall plus roll) in all but one, but a mine blast leaves no
projectile to attribute (`unattributed`), so "mine kill" is not shown; weapons-off, all three cells lose nobody
under either spacing (not a wall-only control, see `ai_loss_attribution_board`). The wide control is decisive:
PHANTOM and RAPIER seeds 4-13, forward and reversed (80 races): **40 destroyed with the fix, 41 with seven-tick
spacing** (forward 18 against 19, reversed 22 against 22). Read as per-seed divergence, the same reading as the
2026-10-07 `weapon-tilt` note. Only the exceeded half of each of the three rows was raised.

Eliminator credit for a Mine is intact and now pinned (`a_tripped_mine_that_finishes_the_craft_credits_the_layer`):
`Impact::struck` records the layer as the victim's last damager and `credit_kill` pays it; the splash path
(`credit_blast`) covers a second craft inside the radius.

Cross-title: `advance_drop` and `DROP_INTERVAL` are shared by every title that lays a charge, so **Pure (Mine and Bomb)
and HD (`HD_Mine`) now drop at six ticks too**. That is Pulse's law, **unmeasured on Pure's and HD's own binaries**
(HD's drop timer is unread); the nearest evidence is that all three run the same fixed 60 Hz step, where a seven-tick
gap is an artefact of float rounding, not of any authored value. Status: checked, applies, not measured on Pure/HD.
