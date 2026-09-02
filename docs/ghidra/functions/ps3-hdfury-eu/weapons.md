# Weapons: Repulser and Rocket construct, `WeaponManager`'s ten other members, and two owning classes found but not read

2026-09-02, a targeted pass following the breadth survey below. Located via
`batch_string_anchor_report(".cpp")` (a bulk read of every `.cpp` `__FILE__`
tag and the functions attributed to it - see
[the breadth-survey note](#breadth-survey-batch_string_anchor_report-and-its-one-trap)
at the end of this page for the tool's one trap), then read with the same
`scripts/ps3-toc.py toc`-first discipline every other page in this directory
uses. All addresses below are TOC-`exact`.

## The names

| Address | Name | Confidence | Evidence |
| --- | --- | ---: | --- |
| `0x00156d88` | `Repulser_Construct` | 88 | `"Repulser.cpp"` tag; called from both `RepulserManager` constructor pair members |
| `0x001254c8` | `Rocket_Construct` | 88 | `"Rocket.cpp"` tag; called from both `RocketManager` constructor pair members |
| `0x0014aea8` | `RocketManager_Construct` | 85 | `"RocketManager.cpp"` tag; the constructor actually called, 2 real call sites (both `WeaponManager.cpp`, constructing it as a member - see below) |
| `0x00128ab0` | `WeaponExplosions_Construct` | 85 | `"WeaponExplosions.cpp"` tag; the constructor actually called, 2 real call sites (both `Plasma.cpp`, attaching it elsewhere - see below) |
| `0x0012f430` | `BombManager_Construct` | 85 | `"BombManager.cpp"`, read raw at `0x007843d8`; the constructor actually called from `WeaponManager.cpp` - see [below](#weaponmanagers-other-ten-members-nine-named-one-inherits-from-cannon) |
| `0x0011b8e8` | `MineManager_Construct` | 85 | `"MineManager.cpp"`, read raw at `0x007835e8` |
| `0x00112128` | `CannonManager_Construct` | 85 | `"CannonManager.cpp"`, read raw at `0x00783210`; the complete-object member of a pair the same pass separated - see below |
| `0x001457b8` | `PlasmaManager_Construct` | 85 | `"PlasmaManager.cpp"`, read raw at `0x00785290` |
| `0x00141e70` | `MissileManager_Construct` | 85 | `"MissileManager.cpp"`, read raw at `0x00785098` |
| `0x001483a0` | `QuakeManager_Construct` | 85 | `"QuakeManager.cpp"`, read raw at `0x00785390` |
| `0x0013d520` | `LeachBeamManager_Construct` | 85 | `"LeachBeamManager.cpp"`, read raw at `0x00784d48` |
| `0x00113288` | `EMPManager_Construct` | 85 | `"EMPManager.cpp"`, read raw at `0x007832b0`; no PSP-side weapon of this name, see below |
| `0x00118dc0` | `LightBarrierManager_Construct` | 85 | `"LightBarrierManager.cpp"`, read raw at `0x007834e0`; not a weapon at all, see below |

`Rocket_Construct` and `Repulser_Construct` each load a per-weapon `.vex`
model and register it as a child of the newly-constructed instance
(`_opd_FUN_002c1ec8`, the same render-node-attach helper both use). Read
directly off `Repulser_Construct`'s own string operands
(`inspect_memory_content` on the two pointers its format call takes):
`snprintf(buf, 0x80, "%s\pulse_repulsorwave.vex", "Data\Weapons")` - the `%s`
takes the base directory as its one argument, not a per-team folder, so every
team loads the identical `Data\Weapons\pulse_repulsorwave.vex`.
`Rocket_Construct`'s own asset load was not chased to a string this pass.

`RocketManager_Construct` builds a fixed pool of **48** (`0x2f + 1`) `Rocket`
instances (`0x150` bytes each) per weapon system, each linked to the manager via
`_opd_FUN_003238f8` and constructed through `Rocket_Construct`.
`WeaponExplosions_Construct` builds **three** render sub-nodes per instance
(not a pool - three sibling members of one object) and initialises a large
block of hardcoded floats immediately after: colours, sizes and timers for
what reads as a three-layer fireball/smoke effect, the same shape
`ExhaustFlare_Init` uses on `psp-pulse-usa` for its own hardcoded defaults
([`exhaust.md`](../psp-pulse-usa/exhaust.md)).

Cross-reference: `psp-pulse-usa`'s own `Weapon_FireRepulser` (`0x0886ce8c`,
weapon id `0x10000`) and the Rocket-firing path (`Weapon_FireRocket`
`0x0886e104`, weapon id `0x80`) are already documented in
[`weapon-fire.md`](../psp-pulse-usa/weapon-fire.md) and
[`rocket-visuals.md`](../psp-pulse-usa/rocket-visuals.md) - this page is the
PS3/Fury side of the same two weapons, not previously read on this binary.

## Not named

Both constructor pairs' *other* half - the complete-object variant paired
with each name above - decompiles identically to its named sibling (the
usual GCC-family duplication this directory has seen before, e.g.
[`mode-manager.md`](mode-manager.md)) but has **no code caller at all**, only
a `[DATA]` (vtable) reference:

- `0x0014a3d0` (`RocketManager`'s pair), `0x00128518` (`WeaponExplosions`'s
  pair).
- `RepulserManager`'s own constructor pair, `0x001499e0`/`0x00148be0`, is
  worse than these two: **neither** member has a code caller, so unlike the
  two rows above there is no evidence at all for which is base-object and
  which is complete-object. Both stay unnamed.
- A second `WeaponExplosions` constructor overload, `0x00128418`/`0x00126f40`
  (identical to each other, both taking a `param_2` position/quaternion and
  copying four 16-byte-aligned blocks into place) - a placement variant, not
  chased further.
- `Rocket`'s and `Repulser`'s own unused sibling constructors, `0x00125330`
  and `0x00156a80` - each has only a `[DATA]` reference, same as the pattern
  above.

## Two owning classes found, not fully read

The two real callers of `RocketManager_Construct` and `WeaponExplosions_Construct`
are themselves `__FILE__`-tagged, and neither tag is one already documented
anywhere in this directory. **Checked directly, not assumed**: read the call
site around each `RocketManager_Construct`/`WeaponExplosions_Construct` call
rather than trusting the base-object-constructor-chaining reading an initial
pass reached for. Both are the ordinary allocate-then-construct-as-member
idiom this page already uses throughout (`FUN_006762b8` allocate,
`FUN_00676218` zero, `_opd_FUN_003238f8(parent, child)` register, then the
member's own constructor) - composition, not inheritance:

- **`WeaponManager.cpp`** (`0x00783dd0`): `0x0012cce8`/`0x0012c220`
  (identical bodies, the usual pair) each build what reads as **one weapon
  system per active craft** - a loop over `GameState`'s craft count naming
  each instance `WEAPON_SYSTEM_%d` - and every instance owns up to eleven member
  objects, **all now identified** - see
  [below](#weaponmanagers-other-ten-members-nine-named-one-inherits-from-cannon)
  for the full sequence, its mode-gating, and each member's name.
- **`Plasma.cpp`** (`0x00783788`): `0x0011f288`/`0x00120570`, a different
  shape again - **not** a member of the `Plasma` object being constructed.
  The call is `iVar11 = RaceManager_GetInstance(); ...
  WeaponExplosions_Construct(child)` with `child`'s parent pointer set to
  `iVar11`, the **race-wide `RaceManager` singleton**, not `param_1` (the
  `Plasma` instance under construction). Reads as `Plasma`'s constructor
  lazily creating a **shared, race-wide** explosion-effect pool owned by
  `RaceManager` the first time a `Plasma` is built, rather than `Plasma`
  owning or deriving from `WeaponExplosions` itself. Matches `psp-pulse-usa`'s
  own `Weapon_FirePlasma` (weapon id `0x4`, [`plasma.md`](../psp-pulse-usa/plasma.md))
  existing as a real, distinct weapon - a shared per-race explosion pool a
  weapon's own constructor lazily creates is a plausible shape for it, but
  which other weapons also reach for the same `RaceManager`-owned instance
  (and whether `Plasma`'s own explosions actually use it, rather than
  something else entirely) is unread.

## `WeaponManager`'s other ten members: nine named, one inherits from Cannon

2026-09-02, a second pass on the same page, re-decompiling
`WeaponManager_Construct` (`0x0012cce8`/`0x0012c220`, identical bodies) in
full rather than reading only the `RocketManager_Construct` call site. All
nine targets are TOC-`exact` (below `0x32d5e0`), so their string references
are read directly with no `scripts/ps3-toc.py` step needed.

**The sequence is not a flat list of ten unconditional members - it is gated
on `g_GameState.GetMode()` (`+0xe0`, established in
[mode-manager.md](mode-manager.md#the-mode-enum-22-ids-eleven-of-them-named))
and on one more global byte**, read at `0x009384e1` and confirmed the same
address across `WeaponManager_Construct` and three of its members
(`BombManager_Construct` via `PTR_DAT_008aa5e0`, `MineManager_Construct` via
`PTR_DAT_008a9e18`, `CannonManager_Construct`'s base member via
`PTR_DAT_008a9af8` - each a different TOC-relative slot resolving to the
identical absolute address). `get_xrefs_to` on `0x009384e1` returns only
reads (weapon managers among many other subsystems); no write site was
chased this pass, so what the flag actually means (dedicated-server mode,
some prototype/template-object marker, or something else) is **unread**.
Every claim below holds when that byte is `0`, which is the ordinary
in-race case - the byte is a corroborated real gate, not a hypothesised one,
but its *meaning* is not established.

With that byte `0`, `WeaponManager_Construct` builds:

| Alloc size | Member | Gate | Confidence |
| ---: | --- | --- | ---: |
| `0xc4` | `BombManager_Construct` | always | 85 |
| `0x3a8` | `MineManager_Construct` | always | 85 |
| `0x78` | `EMPManager_Construct` | mode `0xe` (Detonator) only | 85 |
| `0x334` | *(unnamed, inherits `CannonManager`)* | mode `0xe` (Detonator) only | below 50, not renamed |
| `0x268` | `CannonManager_Construct` | every mode **except** `0xe` | 85 |
| `0xd4` | `PlasmaManager_Construct` | every mode except `0xe` | 85 |
| `0x118` | `MissileManager_Construct` | every mode except `0xe` | 85 |
| `0x13c` | `RocketManager_Construct` (already named) | every mode except `0xe` | 85 |
| `0x78` | `QuakeManager_Construct` | every mode except `0xe` | 85 |
| `0x90` | `LeachBeamManager_Construct` | every mode except `0xe` | 85 |
| `0xd0` | `LightBarrierManager_Construct` | mode `0xd` or `0x15` only | 85 |

This confirms and refines what an earlier pass of this page recorded as
"sized `0xc4, 0x3a8, 0x78, 0x334` or `0x268, 0xd4, 0x118, 0x13c, 0x78, 0x90`"
- the "or" was mode `0xe` (Detonator) branching to a two-member reduced
roster instead of the six-member normal one, not an arbitrary alternative
reading. That earlier list was also **incomplete, not just ambiguous**: it
recorded ten members total, and `LightBarrierManager` (`0xd0`) is an
eleventh, built by a separate, later check the first read did not reach (see
the mode-`6`/`0xd`/`0x15` paragraph below).

**Two modes get nothing from this list at all.** Modes `6`, `0xd` (`13`) and
`0x15` (`21`) skip the `0xc4`/`0x3a8` always-members too (the whole branch is
gated on the same `*pcVar2 == '\0'` check), and only `0xd`/`0x15` pick
`LightBarrierManager_Construct` back up afterward in a separate, later check.
So: mode `6` builds no member on this page's list at all; modes `0xd`/`0x15`
build only `LightBarrierManager`; mode `0xe` (Detonator, confirmed in
mode-manager.md) builds `BombManager`, `MineManager`, `EMPManager` and the
unnamed Cannon-derived class; every other mode builds the full eight
(`BombManager`, `MineManager`, `CannonManager`, `PlasmaManager`,
`MissileManager`, `RocketManager`, `QuakeManager`, `LeachBeamManager`).
Modes `6`, `0xd` and `0x15` are exactly [mode-manager.md](mode-manager.md)'s
own "Zone, Zone Battle, Detonator" bucket (minus `0xe`, already resolved
there as Detonator) - this page's finding narrows but does not close that
open question: it says mode `6` gets no weapon-adjacent system at all, and
`0xd`/`0x15` share exactly one (`LightBarrierManager`, not a weapon - see
below), which is consistent with `6` being single-player Zone and `0xd`/`0x15`
being a Zone Battle pair (matching mode-manager.md's own "per-viewport,
two-player" reading of those two ids), but does not assign which id is which.

### The nine direct `.cpp` tags, confirmed in raw memory

Every name above except the Cannon-derived one carries its own `__FILE__`
tag, stored at object offset `0x30` right after chaining to the shared base
constructor `0x003271d8` - the same pattern this directory always uses. Each
was **read directly out of memory** (`inspect_memory_content` on the
dereferenced pointer, not trusted from Ghidra's own auto-generated symbol
name), matching the confidence band `RocketManager_Construct` already set on
this page:

- `BombManager.cpp` at `0x007843d8`
- `MineManager.cpp` at `0x007835e8`
- `CannonManager.cpp` at `0x00783210`
- `PlasmaManager.cpp` at `0x00785290`
- `MissileManager.cpp` at `0x00785098`
- `QuakeManager.cpp` at `0x00785390`
- `LeachBeamManager.cpp` at `0x00784d48`
- `EMPManager.cpp` at `0x007832b0`
- `LightBarrierManager.cpp` at `0x007834e0`

Seven of the nine tagged names (`Bomb`, `Mine`, `Cannon`, `Plasma`, `Missile`,
`Quake`, `LeachBeam`) match `psp-pulse-usa`'s own canonical roster directly:
`Weapon_RequestFire`'s thirteen-entry jump table
([`mine.md`](../psp-pulse-usa/mine.md#weapon_requestfires-jump-table-read-as-a-table))
lists exactly these names. With `RocketManager` already named on this page,
that is eight of the PSP's thirteen roster weapons now confirmed with a
PS3-side manager class. `CannonManager_Construct` is a pleasing cross-title
echo: `plasma.md` notes the PSP's own Cannon is "dispatched by nothing" in
`Weapons_DispatchFire`, an oddity documented there and not explained; here
Cannon is a real, `__FILE__`-tagged manager class same as every other
weapon, just one whose own PS3-side dispatch was not chased this pass
either.

**The gap is the more interesting result.** Turbo, Shield, Autopilot,
Shuriken and **Repulser** are in `mine.md`'s thirteen and have no member in
`WeaponManager`'s list above. Turbo, Shield and Autopilot are plausibly
per-craft state rather than a pooled pickup manager (matching how
`Weapons_DispatchFire` treats them on the PSP side - state bits, not fire
handlers with a payload), and Shuriken is simply unaccounted for this pass.
**Repulser is the specific open question**: `Repulser_Construct` and
`RepulserManager` are already named on *this very page*, so a working
`RepulserManager` class exists in this binary, yet `WeaponManager_Construct`
- the per-craft weapon-system umbrella - does not construct one anywhere in
the sequence read here. Either something else owns and constructs
`RepulserManager` (unread this pass), or its member sits in a part of
`WeaponManager`'s prototype (`param_2`/`param_3`, the two arguments most
member constructors above ignore) not chased here. Not resolved.

`EMPManager` and `LightBarrierManager` have **no PSP-side counterpart in the
roster above** - both are either new to HD/Fury or are not player-facing
"weapons" at all in the pickup sense. `LightBarrierManager` is the stronger
case for the latter: it is gated on the same two mode ids
(`0xd`/`0x15`) mode-manager.md already flagged as the two-player,
per-viewport pair, is built **without** any of the other weapon managers
alongside it, and its name reads as a track/mode hazard (a barrier of light)
rather than a craft pickup. `EMPManager` is gated on Detonator specifically
and paired with the unnamed Cannon-derived class below - both plausible as
Detonator-specific mechanics (Detonator's bombs are shot with a weapon;
EMP/an enhanced-Cannon variant tuned for hitting them is a reasonable guess)
but neither claim is established, only the tag and the gate are.

### The `0x334` member: inherits from `CannonManager`, not named

`_opd_FUN_00138700` (and its complete-object pair `_opd_FUN_00138778`,
identical body, same `[DATA]`-only extra reference) does **not** chain to
the shared base constructor `0x003271d8` directly - it chains to
`_opd_FUN_00111a30`, which decompiles **byte-for-byte identical** to
`CannonManager_Construct` (`0x00112128`): same `"CannonManager.cpp"` tag
store, same field layout, same pool-size logic. `get_xrefs_to 0x00111a30`
returns only the two calls from `0x00138700`/`0x00138778` (plus one `[DATA]`
vtable reference) and no allocate-then-construct site of its own - the
opposite of `CannonManager_Construct`'s two real call sites from
`WeaponManager_Construct`. Under the C++ ABI (the same discriminator
[mode-manager.md](mode-manager.md#the-names) used) that makes `0x00111a30`
**`CannonManager`'s base-object constructor** and `0x00112128` its
**complete-object constructor** - the rare case, on this binary, where both
pair members are separately attested. Only `0x00112128` is renamed here,
following this page's existing convention of naming the member that is
actually constructed as a weapon-system member (`RocketManager_Construct`'s
own precedent); `0x00111a30` is left as a documented-but-unrenamed base-ctor
attribution rather than renamed, since renaming it risks colliding with
mode-manager.md's opposite convention (`_Construct` for the base member,
`_ConstructComplete` for the complete one) without a driving reason to prefer
either scheme here.

After the base-chain call, `_opd_FUN_00138700` overwrites the vtable pointer
(`*param_1 = puVar1`, a different table than `CannonManager`'s own) and
constructs one further member at object offset `param_1 + 0x9a`. `0x9a * 4 =
0x268` - **exactly `CannonManager`'s own allocation size** - so this
member's storage begins precisely where `CannonManager`'s own fields end,
independent arithmetic corroboration (on top of the vtable override and the
base-chain call) that this is real inheritance from `CannonManager`, not the
composition idiom (`FUN_006762b8` allocate + `_opd_FUN_003238f8` register)
this page uses everywhere else for ordinary member construction. The two
calls that follow (`_opd_FUN_00147ac8`/`_opd_FUN_00147a18`) write the same
set of fields at that offset, differing only in one flag byte, and are field
initialisers rather than constructors of a further class - no additional
`__FILE__` tag there.

One time-boxed attempt was made to identify this class independently of a
`.cpp` tag: the new vtable's first slot (`PTR_PTR_008aa8d0`) was read raw
(`inspect_memory_content`, 64 bytes) and does not resolve to a function
address recognised by `get_function_by_address` - it sits in what reads as a
mixed data/constant table (adjacent IEEE-754 floats, one recognisably
`pi` at `+0x30`), not a clean vtable this tool can walk further without more
work. **Left unnamed per the confidence rubric**: it has no `__FILE__` tag
of its own, and "Detonator-only Cannon subclass" is a real, evidenced shape
but not a specific weapon identity - a name here would be exactly the "guess
dressed as a name" the project's naming rule exists to prevent.

### Not swept this pass

The base/complete-object pair for the other eight named members (only
`CannonManager`'s pair was chased, because its base member was the one
answering the `0x334` mystery) was not searched for - if a distinguishable
pair exists for `BombManager`, `MineManager`, `PlasmaManager`,
`MissileManager`, `QuakeManager`, `LeachBeamManager`, `EMPManager` or
`LightBarrierManager` the same way `mode-manager.md` and this section found
for `CannonManager`, it is unread, not ruled out.

## Also found, and ruled out

Four more `.opd.FUN_*` addresses `batch_string_anchor_report` bucketed under
`RepulserManager.cpp` (`0x00463968`, `0x00463908`, `0x00463d80`, `0x00463eb0`)
and two under `RocketManager.cpp` (`0x006d8008`, `0x006d8090`) are not weapon
code at all: the first four are a generic `sys_lwmutex`-backed mutex registry
(create/lock/destroy against a shared linked list), and the second two are a
bounds-checked vtable dispatch shaped like a memory-protection-fault handler.
Both are almost certainly library code that happens to sit in the address
range the tool attributes to the nearest preceding tag, not members of either
class - see the trap note below.

## Breadth survey: `batch_string_anchor_report` and its one trap

Run once each against `ps3-hdfury-eu` and `vita-2048-eu-v104` this session as
a breadth pass (full output too large to inline; saved and queried with `jq`).
Both binaries carry the same `__FILE__`-tag convention this directory already
relies on, confirmed independently on the Vita binary too - among other
things, `vita-2048-eu-v104` ships a `System/Render/Mesh_Importer.ps3.cpp`
(literally `.ps3.` in the filename), a sharper piece of lineage evidence than
anything the confirmed-lineage finding in
[`vita-2048-eu-v104/README.md`](../vita-2048-eu-v104/README.md#the-lineage-question-is-answered-confirmed)
already cites.

**The trap, found chasing this page's four ruled-out addresses above.** On
`vita-2048-eu-v104` the tool correctly splits each class's functions into
`documented`/`undocumented` lists. On `ps3-hdfury-eu` it does not: every
`.opd.FUN_*` label counts as "documented" there (an OPD label is technically
a name, just not a real one), so `ps3-hdfury-eu`'s own `documented_count`
summed to 944 and `undocumented_count` to 0 across all 308 anchors - useless
as a ranking signal on its own. Rank by grepping the `documented` arrays for
`^\.opd\.FUN_|^FUN_` instead (what this page's own survey did). Separately,
and independent of that quirk: **proximity is not membership.** A function
this tool buckets under a `.cpp` tag can still be unrelated library code that
happens to sit in the linked address range after that tag's own functions
end - exactly what this page's six ruled-out addresses turned out to be.
Treat every bucketed address as a lead to verify by decompiling it, never as
a name to apply from the bucket alone.
