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

Both are ported in `crates/gameplay/src/projectile/mine.rs` - one module,
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

## `Mine_SpawnExplosion` plays `WO_MINE_EXPLO`

**Recovered 2026-08-26, confidence 90 - direct instruction-level read, closed
the same day this page's own doc comment in `crates/gameplay/src/projectile/mine.rs`
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
  **negated** and hands it to a spawn helper at `0x0885f188` that
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

- **How many mines a drop releases.** `craft+0x1ac` is decremented by the
  handler and **written by nothing else in the binary** - a `search_instructions`
  sweep for every `sw` to a `+0x1ac` offset returns exactly one store on a craft
  base, the decrement itself. The shipped `<Stats>` authors no count for the
  Mine either. This is the same negative result
  [weapon-fire.md](weapon-fire.md#what-is-not-verified) recorded and it survives
  a second search. The engine picks a number and says so; see
  `crates/gameplay/src/projectile/mine.rs`.
- ~~**What a mine does when a craft reaches it.**~~ **Recovered 2026-08-26** -
  see [`Mine_SpawnExplosion` plays `WO_MINE_EXPLO`](#mine_spawnexplosion-plays-wo_mine_explo)
  above: `blastradius`'s consumer is `Weapon_PostBlastImpulse_q`, and the
  visual both `trigger_radius` and the fuse timing out reach is
  `Mine_SpawnExplosion`.
- **The Bomb's own explosion call.** Whether `Weapon_FireBomb`'s teardown
  reaches `Mine_SpawnExplosion` too (the same `WO_MINE_EXPLO`, larger) or its
  own equivalent is unchased - see the note at the end of the section above.
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
