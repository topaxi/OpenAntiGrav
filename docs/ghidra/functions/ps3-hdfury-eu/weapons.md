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

## 2026-09-23: where the Missile's two HD models load, and what is not read

A bounded read for the Missile's two HD-only models, done after the Plasma
explosion landed ([plasma.md](plasma.md)'s 2026-09-23 section). **Neither
is wired; no trigger was read.** Everything is through the TOC `0x008ad4d8`
all four functions below carry in their `.opd` entries.

- **`HD_missile_ball_bloomring` is the flying Missile's own head, not part
  of its explosion.** Its `.vex` path (`0x007836c0`) is TOC slot `-0x3624`
  (`0x008a9eb4`), loaded by `0x0011ccc8` (OPD `0x00876010`) - a constructor
  that stamps `"Missile.cpp"` (`PTR_s_Missile_cpp_008a9eac`) into the object
  and keeps the model node at `param_1[0x70]`. `0x0011cfa8` (OPD
  `0x00876018`) is its twin, the same load at `0x0011d0a0`/`0x0011d144`.
  Confidence 75 that this is the per-missile object: the source-file string
  and a per-object model are what say so; its caller was not followed. Its
  material is named `hd_leachbeam_ball_glow` / `hd_leachbeam_bloomring` on
  the disc - the LeachBeam ball shares it.
- **`HD_missile_explosion` belongs to class "Z" of
  [renderer.md](renderer.md)'s `SortRoot` note**: `0x00154cf0` (OPD
  `0x00877a08`, called from `MissileManager_Construct` at `0x00142114` and
  from `0x00141a88`) and its twin `0x00155088` load TOC slot `-0x2240`
  (`0x00785828`) into `param_1[0x61]`, then bind **two** named shader
  constants on it through `FUN_00677018` to `_opd_FUN_002c11c8(model)`:
  `UV_offset` (`0x00785880`) and `Shockwave_scalar` (`0x00785890`). The same
  binding shape `WeaponExplosions_Construct` uses for the Plasma trio.
  The vtable is `0x00864b38`; slot 5 is `0x00155420` (OPD `0x00877a18`),
  which only writes the per-viewport matrix at `this + 0xf0 + v * 0x40` into
  the model node at `this + 0x184` - no scale ease of its own, unlike the
  Plasma's `Draw`. The growth is the model's: its `Anim Transform` nodes key
  scale 1 -> 18 over the first second (and a `0.0977`-scaled node to 44),
  measured by `crates/render/examples/hd_weapon_extents.rs`. Slot 3 is the
  generic `0x00327050`. Confidence 70 on the class reading.
- **What starts it is not read**: no function that writes `this + 0xf0` or
  sets the object's anim time was found in this pass. The next step is the
  Missile's hit path - `MissileManager`'s per-tick walker, the way
  `PlasmaManager_Update` led to `WeaponExplosions_Start` - and an xref sweep
  for loads of the manager's own member that holds the "Z" object.

No names are applied: the constructors sit at 70-75 and would carry `_q`,
and the trigger that would make them useful is the missing half.

## 2026-09-23: the LeachBeam's own HD pieces, read, not wired

A bounded read, done after Pulse's ribbon was rebuilt
([cannon-quake-leachbeam.md](../psp-pulse-usa/cannon-quake-leachbeam.md),
"2026-09-23: the ribbon re-read"). **HD's LeachBeam is not Pulse's ribbon
repainted.** It adds a model that travels along the beam and five effects
of its own. Nothing below is wired: this engine still draws Pulse's ribbon
and Pulse's two effects on HD. Every function here sits below `0x32d5e0`, so
Ghidra's TOC `0x008ad4d8` is right for all of them
([memory.md](memory.md)).

| Address | Name | Confidence | Evidence |
| --- | --- | --- | --- |
| `0x00153fb0` | `LeachBeam_Construct` | 85 | Stamps `"LeachBeam.cpp"` (`0x007857b8`, TOC slot `0x008ab224`) and installs vtable `0x00864af8`. Called from `LeachBeamManager_Construct` on the one `0xc9c0`-byte beam object. |
| `0x00114448` | `LeachBall_LoadModel` | 85 | Loads `Data\Weapons\hd_leachbeam_ball_bloomring.vex` (`0x007833e0`, slot `0x008a9c54`) through `_opd_FUN_002c1ec8` into `*param_1`. |
| `0x00114b40` | `LeachBeam_SpawnLaunchEffect` | 82 | Spawns `WO_LEACHBEAM_LAUNCH` (slot `-0x385c`), fourcc `0x454c424c`, on the matrix it is handed. |
| `0x001148c0` | `LeachBeam_SpawnHitTargetEffect` | 82 | Spawns `WO_LEACHBEAM_HIT_TARGET` (slot `-0x3864`), fourcc `0x5448424c`, on a non-null matrix. |
| `0x00114708` | `LeachBeam_SpawnBreakEffect` | 78 | Spawns `WO_LEACHBEAM_BREAK` (slot `-0x3868`, fourcc `0x4542424c`) when `_opd_FUN_00116308`, sampled at the ball's progress `+0x24`, returns true. |
| `0x00114c78` | `LeachBall_Advance` | 70 | See below. |

**The ball travels, and every arrival is a drain.** `LeachBall_Advance`
accumulates time at `+0x1c` against a period at `+0x20`. The period is
remapped (`_opd_FUN_002a3718`) from the magnitude of `param_4[4]`, clamped
between `DAT_008a9c88` and `DAT_008a9c8c`. Each time the accumulator passes
the period it wraps, calls `0x0013c8e8` on the beam, and spawns
`WO_LEACHBEAM_ABSORB` (fourcc `0x4541424c`) at `*(beam + 0x6944) + 0x1d0`.
Then it stores `+0x24 = accumulator / period` and asks
`_opd_FUN_00116308(+0x24, ...)` for a point along the beam at that fraction.
That point goes into the ball model node at `+0xc0` (its matrix at `+0xd0`).
So on HD the energy is a **model** carried along the beam, one trip per
drain - the part Pulse does with `WO_LEACHBEAM_ENERGY` walking the chain.
Confidence **70**: the loop and the spawn are direct reads. That
`0x0013c8e8` is the drain, and that `+0x1d0` is the shooter's matrix, come
from the Pulse analogue rather than a read of either.

**The beam object holds four balls and two strips.** `0x00153218` (called
from `LeachBeamManager_Construct`) initialises two `0x63d0`-byte strip
objects at `+0x50` and `+0x6420` (`_opd_FUN_00115770`; their constructors
`0x001153c0`/`0x00115cc0` take colour `0xffffff00`). It also builds four ball
models at `+0xc820 + 0x30/0x60/0x90/0xc0` through `0x001145b0`. Neither the
strips' geometry nor their draw was read.

**The rest of HD's LeachBeam vocabulary, located and not read.**
`%s\leacheffect.vex` (`0x00782298`), loaded per team by
`Ship_ReloadModelForSkin` (`0x000dc154`), is HD's per-hull effect: the
analogue of Pulse's `leachbeam_surface.mip` hull overlay. `LeachFader` and
`LeachScroller` (`0x007b32a0`/`0x007b32c0`) are its shader parameter names.
`leachbeam_triangle` (`0x0079c200`), `WO_SHIP_SPARK_DAMAGE_LEACHBEAM`
(`0x0079bdb8`) and the `LEACHFAIL` cue (`0x007857d8`) are also located.
`WO_LEACHBEAM_CHARGING` has a TOC slot (`0x008a9bd4`). The disc's
`WO_LEACHBEAM_EMIT`/`_HITSHELL`/`_BALL_SPARKS`/`_CHARGING_SPARKS`/`_ENERGY_SPRAY`
have no string in the executable, so nothing in the code names them. None
of the three spawners above had a caller found by `bl` or `get_xrefs_to`,
so they are reached through a function pointer. Their triggers are **not
recovered**, and that is why nothing here is wired.

## 2026-09-25: the LeachBall's period, its trigger search, and the ball wired

Picking up the "LeachBeam next" item this page's own 2026-09-23 section left
open: find `LAUNCH`/`HIT_TARGET`/`BREAK`'s callers, read `LeachBall_Advance`'s
period and placement, and wire what is recovered. Read headless against
`EBOOT.elf`, TOC `0x008ad4d8` throughout.

### The three spawners' trigger search, extended and still empty

The 2026-09-23 pass found no `bl`/`get_xrefs_to` caller and hypothesised a
function-pointer table. This pass tested that hypothesis three ways rather
than leaving it a hypothesis:

1. **The OPD block at `0x00875a60` is not a vtable.** It is the
   `LeachBeam.cpp` translation unit's own `.opd` section, laid out in the same
   order as the functions themselves (`LeachBall_LoadModel`,
   `LeachBeam_SpawnBreakEffect`, `LeachBeam_SpawnHitTargetEffect`,
   `LeachBeam_SpawnLaunchEffect`, `LeachBall_Advance`, ...) - confirmed by
   decompiling the neighbours: `_opd_FUN_001145b0` (the OPD slot right before
   `SpawnBreakEffect`) calls `LeachBall_LoadModel` **directly, four times**,
   which a vtable slot never would. `get_xrefs_to` on each spawner's own OPD
   *entry address* (`0x00875a78`/`80`/`90`, not the function) returns nothing
   either.
2. **The one real vtable this class installs doesn't reach them.**
   `LeachBeam_Construct` (and four sibling constructor-shaped functions -
   `0x001540c8`/`0x001541b0`/`0x00154288`/`0x00153e98` - the usual GCC
   duplication this directory already documents) all install the same
   vtable, `0x00864af8`. Its 14 slots were read directly
   (`inspect_memory_content`): all resolve to the generic render-node OPD
   family (`0x00323510`-`0x00323550`, `0x00327040`-`0x00327228`) this
   directory already reads as ordinary scene-node plumbing (see cannon.md's
   `0x00327500`/plasma.md's `0x00327050`), none to a LeachBeam-specific
   function. This vtable is the beam's own generic node interface, not the
   dispatch for its three effects.
3. **No literal 4-byte reference to any of the three spawners' OPD addresses
   exists anywhere in the file**, checked two ways: `search_byte_patterns`
   over the whole program (validated first against a byte pattern **known**
   to exist - `00 88 5A 80`, a slot of the vtable above, which returned 259
   hits - so the tool itself is not the reason for a negative), and a raw
   `python3` scan of `EBOOT.elf` on disk for the big-endian words
   `0x00875a78`/`80`/`90`/`98`/`a0` (the three spawners' and
   `LeachBall_Advance`'s own OPD entries), independent of Ghidra's memory
   map. Zero occurrences of all five, where the control (`0x00864af8`) found
   its one known occurrence. A `search_instructions` sweep for
   `ori rX,rX,0x424c` (the low halfword every LeachBeam fourcc shares, since
   all four end `..424c`) finds exactly the four spawns already known
   (`SpawnBreakEffect`, `SpawnHitTargetEffect`, `SpawnLaunchEffect`, and the
   inlined `ABSORB` copy in `LeachBall_Advance` - see below) and nothing
   else, so none of the three is inlined a second time elsewhere either.

**Conclusion: `LAUNCH`/`HIT_TARGET`/`BREAK` have no discoverable caller in
this retail build**, the same shape `WO_CANNON_MUZZLEFLASH`/`_HOTSPOT`
already have on cannon.md at confidence 80. Confidence **78** here (one
notch under Cannon's, since Cannon's page adds a `.pob`-corpus grep this
pass did not repeat) that the retail `EBOOT.elf` never plays these three.
They stay unwired.

### `LeachBall_Advance`'s inlined `ABSORB` is the live path

`LeachBeam_SpawnAbsorbEffect` (`0x00114a00`, confidence 82 - same shape as
its three named siblings: `_opd_FUN_002c9108(node, texture_slot, 0x4541424c,
matrix + 0x1d0, 1, 0)`, matching `WO_LEACHBEAM_ABSORB`'s own file name on
disc, `/data/psys/wo_leachbeam_absorb.pob`) exists **standalone and
uncalled** - `get_xrefs_to` finds only its own `[DATA]` OPD self-reference,
same as the other three. But `LeachBall_Advance` (`0x00114c78`) carries an
**identical, inlined copy of the same body** at its own wrap point (same
`0x180`-byte allocation, same `-0x3860` texture slot, same fourcc, same
`+0x1d0` offset), executed directly, every time the accumulator wraps. So
the retail EBOOT does play `WO_LEACHBEAM_ABSORB` - through the inlined copy,
not the named function - once per drain trip, for as long as the beam holds.
`LeachBeam_SpawnAbsorbEffect` itself is dead code (GCC's usual habit of
generating both the shared helper and an inlined callsite; see this page's
"Not named" section for the same pattern elsewhere in this file), but its
existence is what let this pass name and rate the inlined copy's own
mechanics with confidence.

### The drain-trip period, read off `LeachBall_Advance`'s own constants

`LeachBall_Advance` remaps `|param_4[4]|`, clamped to `[20.0, 100.0]`
(`_DAT_008a9c88`/`DAT_008a9c8c`, read directly), linearly onto
`[0.3, 1.0]` seconds (`_DAT_008a9c80`/`DAT_008a9c90`, read directly) through
`_opd_FUN_002a3718` - a remap-with-clamp call, the same shape this directory
already reads elsewhere. Confidence **85** for the arithmetic (a direct
decompile plus a direct memory read of all four constants).

**What `param_4[4]` itself is is not confirmed.** `param_4` is the strip
object `LeachBeam_Construct` builds (`_opd_FUN_00115770`, at beam `+0x50`
and `+0x6420`); `_opd_FUN_00116308` (the fraction-to-point lookup, below)
reads `param_4[5]` as the strip's own point count and `param_4[0x1778..]`
as its cumulative arc lengths, so `param_4[4]` is field `strip+0x10` -
plausibly the strip's own total length (the beam's length, in effect,
since the strip runs shooter to target), but nothing pins what writes it.
**Chosen, not measured**: this build reads `param_4[4]` as the straight-line
beam length (`|target - owner|`), which gives the sensible "short beam
drains fast, long beam drains slow" shape the constants themselves suggest,
but the strip could in principle hold something else.

### The ball's own placement, read and not fully resolved

At each wrap, `LeachBall_Advance` calls `_opd_FUN_00116308(fraction, strip)`
for a point along the strip's own chain (not a straight line - it walks the
strip's per-point cumulative-length table and interpolates between the two
points bracketing the fraction, `_opd_FUN_002a3bf0`), then
`_opd_FUN_001141d8` places the ball's model node with it through the same
generic per-tick placement call `0x00327500` cannon.md's `Draw` and this
page's own generic-vtable slots already use, gated on the model node's flag
bit `2`. Confidence 70 for `_opd_FUN_001141d8`'s role (a direct decompile,
heavily AltiVec-shuffled, of what reads as "orthonormalise/place a matrix
onto the node"); left unrenamed per the confidence rubric rather than
carrying a guess about exactly what beyond placement it might also do.

**Which end of the strip is fraction `0` is not resolved.**
`_opd_FUN_00116308` flips its own fraction (`dVar11 = dVar9 - dVar11`) when
`param_2[2] == *param_2` - a condition this pass did not chase to a
concrete craft identity. This engine's own implementation
(`oag_fx::beam::hd_ball::position`) picks target-at-`0`,
owner-at-`1` (the wrap = arrival at the shooter) by analogy with Pulse's own
`WO_LEACHBEAM_ENERGY`, which the ribbon's `Ribbon::energy_point` already
recovers as arriving at the shooter the same way - not itself a measurement
of `_opd_FUN_00116308`'s own flip condition. **Chosen, not measured**, and
recorded as such in `hd_ball::position`'s own doc comment.

### What the engine now does with this

`oag_fx::beam::hd_ball` (`crates/fx/src/beam.rs`) carries the period
law and the chosen straight-line placement law above, and
`Race::advance_leach_beam_ribbon` (`crates/raceplay/src/weapons/visuals.rs`)
drives a render-side accumulator (`RaceView::leach_ball_elapsed`) off it
alongside Pulse's ribbon, firing `WO_LEACHBEAM_ABSORB` as a one-shot burst
(`stage.play`, not attach-and-follow - the original allocates a fresh
instance at every wrap rather than reusing one) at the wrap point. Verified
present on the HD disc: `/data/psys/wo_leachbeam_absorb.pob` (`DATA02`,
`scripts/psarc.py list`).

**The ball's own model (`hd_leachbeam_ball_bloomring.vex`) loads and places
correctly as of 2026-09-25 (second pass), but still does not draw - and now
for a *confirmed* reason, not an untested worry.** `race/scene.rs` and
`race/load.rs` were split first (`race/scene/boost_flare.rs`,
`race/load/gantry_visibility.rs` - a pure move, no behaviour change) to buy
the headroom `scripts/check-file-size.py`'s 1,000-line ceiling had none of,
rather than folding this weapon into the unrelated
`blast_models::PlasmaBlastModels`/`PlasmaBlastDrawables` container the way
Plasma's own bolt head already does (rejected for the same
"compounding, not reusing" reason the first pass gave). `Scene` now carries
one `leach_ball: Option<Drawable>` - a single drawable, not a
`MAX_PROJECTILES` pool, since `oag_gameplay::World::leach_beam` is one
`Option`, never more than one live beam - written and drawn each frame a
beam is `Kind::Locked` (`Race::leach_ball_model_matrix`,
`crates/raceplay/src/weapons/visuals.rs`) at
`oag_fx::beam::hd_ball::position`, on a translation-only matrix
(identity rotation and scale - **chosen, not measured**, since
`_opd_FUN_001141d8` was never resolved past "orthonormalise/place a matrix
onto the node" at confidence 70).

**Checked directly with `oag-view --draws`, not assumed: this is the same
missing-shader-mode wall `HD_plasma_ball` hits, not a missing texture or a
missing blend state.** Against
`Data\Weapons\hd_leachbeam_ball_bloomring.vex`:

```
Data\Weapons\hd_leachbeam_ball_bloomring.vex: 0 opaque, 0 cutout, 2 blend draw(s), 2 texture slot(s) (2 decoded)
  blend  node    2 pSphereShape8         760 tri  tex 0: hd_leechbeam_ball.gtf 256x128 ... rgba 0.00,0.00,0.00,1.00
  blend  node    3 polySurfaceShape17     40 tri  tex 1: hd_leechbeam_ball_bloomring_ramp_128x32.gtf 256x128 ... rgba 0.00,0.00,0.00,1.00
  0 draw(s) resolve no texture and therefore bind the white 1x1
```

Both draws resolve real, bound textures - not `oag-view`'s "resolves no
texture" case - and both carry their own material's authored blend factor
pair (`hd_leachbeam_ball_glow.rcsmaterial`/`hd_leachbeam_bloomring.rcsmaterial`,
`/data/weapons/materials/` on the disc - read by
`oag_rcs::rcsmodel::Material::blend`, the same decode `cull_as_authored`
already trusts). **Both draws' own vertex colour is `rgba 0.00,0.00,0.00,1.00`
- black - on every one of 800 triangles.** That is an authoring pattern that
only makes sense under an unlit/emissive shader (the texture alone is the
picture; diffuse lighting is meant to contribute nothing), and this engine's
one shared `mesh::rcs` shader (`mesh.wgsl`, the "lit race pass" `oag-view`
itself names) has no such mode - it always multiplies texture by vertex
colour by lighting, the same gap the Plasma bolt's head had until 2026-10-06
(`docs/rendering/hd-unlit-programs.md`, "Gates").

**Forced on and measured live, to see what that gap actually produces
here**, since `HD_plasma_ball`'s own failure (an opaque black shard) is not
automatically this one's: `--race --mode single_race --give leachbeam
--hold cross --press square --ticks 15 --screenshot` against
`data/images/hdfury-ps3-eu-dec.iso` (VENOM class, default team, Talon's
Junction - a lock is available from tick 0, `lock_min_dist=10`/
`lock_max_dist=200` read off this disc's own weapon table, same as Pulse's)
shows a **flat, opaque, uniformly-lit grey sphere** at tick 15 - the
ambient+specular response this shader still computes against a black
diffuse, not the "bloomring" glow the name promises and not a black shard
either. That is exactly a plausible-looking stand-in for what the asset
should look like, so per `CLAUDE.md`'s rule against drawing one:
`load::weapon_models::LEACH_BALL_DRAWN` stays `false`. The model still
loads and reports what it found (see that constant's own doc comment for
the load report line); nothing is drawn. Screenshots and the `--draws`
capture above are `data/scratch/` (gitignored), not committed.

**Superseded the same day (third pass): the ball draws.** The grey sphere had
two causes, neither of them the black vertex colour, which no program here
reads. The sphere is an inline stride-18 chunk whose last four bytes are a
colour, so its `Uv1` was read as two `NaN` halves (fixed in
`oag_rcs::rcsmodel::Mesh::texcoords`); and its material's program -
`(0.9 (1 - rim^5))^5` fading a noise-displaced, clock-scrolled tap expanded by
`c / (1 - c)` - had no shading path. Both are read and closed on
[hd-unlit-programs.md](../../../rendering/hd-unlit-programs.md), and
`load::weapon_models::LEACH_BALL_DRAWN` is deleted rather than flipped. The
bloomring's program is one texture tap and already drew as `slots::EMISSIVE`.

### A related finding: `WO_LEACHBEAM_ENERGY` has no string in this EBOOT

`strings -a EBOOT.elf | grep -i LEACHBEAM_ENERGY` returns nothing, though
the asset ships (`/data/psys/wo_leachbeam_energy.pob`, `DATA02`). This
engine's own `Race::advance_leach_beam_ribbon` currently plays
`LEACHBEAM_ENERGY_EFFECT` (Pulse's own recovered trigger) unconditionally
on every title, HD included - so on HD this build likely fires an effect
the retail executable's own code never names, alongside the ball/`ABSORB`
wiring landing here. Not fixed this pass (out of this lane's own scope -
gating the whole ribbon-and-energy mechanism to Pulse/PS2 and drawing HD's
own ball+`ABSORB` instead is a larger, separate change); recorded here so
the next LeachBeam pass on HD does not have to re-derive it.

## 2026-10-07: the HD Bomb's detonation object, read (`hd-weapon-blasts`)

Raw bytes disassembled with capstone (`ppc64`, AltiVec) because Ghidra stops at
the VMX blocks of these functions and marks `0x00327500` no-return, so its
decompiles of the blast truncate. TOC `0x008ad4d8` throughout. Confidence
figures are for the reading, not for any name (none applied yet).

**The set is four models, not five.** `NormalBomb.cpp` (constructor
`0x00144a48`, vtable `0x00864640`) owns two models of its own, `[0x2d]`
`HD_Bomb` and `[0x2e]` `HD_bomb_halo` (the armed bomb's glow, registered with
flag `0x400`), and builds one **blast object** (`0x00151ad8`, `0x4c0` bytes,
vtable `0x00864ab8`) at `+0xe4`, only when `RaceManager_GetInstance()` is
non-null. The blast loads, from TOC `0x008ab18c..`: `HD_bomb_sphere` (`+0x2e4`),
`HD_bomb_sphere_white` (`+0x2e8`), `hd_bomb_sphere_bloomring` (`+0x2ec`) and
`hd_bomb_shockwaves` **eight times** (`+0x2f0..+0x30c`). `HD_Mine_halo` is the
Mine's. None of the four carries a moving Anim Transform key (one constant
scale and one constant translation each, `data/scratch/hd-weapon-blasts/keys.txt`
via `crates/render/examples/hd_weapon_anim_keys.rs`), so every size and fade is
the blast's own code, not the file's.

**Material parameters** (`FUN_00676ff8` hashes of `0x00785768..`):
`AlphaAnim`, `ColourAnim`, `V_Anim`, `Shockwave_scalar`, bound by pointer
(`FUN_00677018`). `AlphaAnim`, `V_Anim` and `Shockwave_scalar` go to
`AnimNode_FindTransformValueField(model)`, i.e. the model's clock, which the
blast sets with `AnimNode_UpdateTransformTree` (`0x002c1b30`) each tick;
`ColourAnim` goes to the blast's own `+0x2dc` (`+0x2e0` for `sphere_white`).

**Call shape.** `NormalBomb` update `0x001443f8(dt)`: `age (+0xe8) += dt`, then
`0x00144040(bomb, 1)` on a hit (the ship-in-radius loop through `0x002d64d0`),
which orthonormalises the bomb's own `+0x100`/`+0x110` rows into the matrix at
`+0xf0` (AltiVec, the same Gram-Schmidt shape as Pulse's `bomb_blast_basis`),
calls **`0x00151538(blast, bomb + 0xf0)`** (start: copies the 4x4 to blast
`+0x290`, sets `+0x490 = 1`, ORs 4 into the first four models' flags) and plays
`BOMBEXPL`. While `+0x490` is set, `0x001443f8` calls **`0x001503d8(blast, dt)`**
every tick and retires the bomb when it returns 0. Slot 5 of the blast's vtable
(`0x001512f8`) is its **draw**: sphere with the matrix at `+0x150`, sphere_white
with that matrix scaled, bloomring with `+0x1d0`, then two `0x006778c8` point
lights.

**Tunables** are a table at `0x008c1aa4` (initialised in the file): life `3.0`
(`+0`), `+0x64 = 1.5`, `+0x68 = 0.6`, `+0xa4 = 3.0`, `+0xa8 = 0.1`, and three
curve lists at `+0x08`, `+0x24`, `+0x40`.

### The per-tick law, and how it was checked

`NormalBombBlast_Update` (`0x001503d8`, with `0x0014fff0` for the ripple rings
and `0x0014fa08` building the curves) runs for `age` seconds, `age > 3.0`
returning false (the table's `+0`). It sets each model's clock with
`AnimNode_UpdateTransformTree` and places two groups of matrices; the table at
`0x008c1aa4` is initialised data and nothing found writes it (confidence 85).

| Age (s) | Fireball + core (`+0x2e4`, `+0x2e8`) | Bloom disc (`+0x2ec`) | First ring (`+0x2f0`) | Ripple rings (`+0x2f4..+0x30c`) |
| --- | --- | --- | --- | --- |
| `0 ..= 0.75` | clock `1.0` | clock `0.625 age` | clock `min(age / 0.6, 1)`, size `0.1 -> 53.33` over 0.6 | hidden |
| `0.5` | `WO_BOMB_RAYS` once (`0x150fc8`, tag `'BORS'`, at the blast's matrix) | | | |
| `0.75 .. 1.5` | clock `1 - (age - 0.75)^2` | clock `((age - 0.8) / 0.7 * 0.5)^2 + 0.5` once past 0.8 | held at 1 | |
| `1.2 .. 1.5` | rows squash: sideways `x (1 + 0.2 t^2)`, up `x (1 - t^2)`, `t = (age - 1.2) / 0.3` | same | | |
| `1.4 .. 2.3` | hidden from 1.5 | hidden from 1.5 | hidden from 1.5 | seven windows `[start, start + window]`, below |
| `3.0` | the object retires | | | |

- **Size** of the fireball, core and bloom: `curve(age) + cur`, `curve` linear
  through `(0, 2.0)`, `(0.1, 6.66)`, `(1.5, 10.0)` (`0x14fa08`: TOC floats
  `-0x23d4`, `-0x23e4`, `10.0`) and `cur` easing `cur += (3.33 - cur) * 0.05`
  once a tick from `0.1` (`+0x60`, `+0x5c`, `-0x23e8`; the loop runs
  `trunc(dt * 59.999996)` times, one at 60 Hz). The core is the fireball's
  matrix times `0.99` (`-0x2360`); its `ColourAnim` is the constant `+0xac = 0.9`.
- **`ColourAnim` of the fireball** is blast `+0x2dc`: `1.0` until 0.1 s, then
  `|x|` for `x < 0` and `x^(1/4)` (`0x677868`, the executable's `pow`) for
  `x >= 0`, `x = 2 (age - 0.1) / 1.4 - 1`; `(1 - ColourAnim)^-1` multiplies the
  fireball's colour in the shader, so it starts white-hot, cools to its texture
  at 0.8 s and flashes white again as it fades.
- **The fireball's frame.** `0x00151538` writes the base frame per viewport:
  `z` the third *column* of the camera table `0x00987780`, `y` the bomb matrix's
  up row with `z` removed, `x = y x z`. `0x001503d8` then tilts it by
  `-pi/2 (1 - age / 3) + pi/4` about the **unnormalised** `y x d`
  (`d = normalize(position + camera row 3)`) through `0x006ca6b0`
  (`0x006ca538` builds the rows `(x^2 k + c, xy k - z s, xz k + y s)`, ...,
  `k = 1 - c`, from the raw components, so a non-unit axis shears). The bloom
  disc's frame is `(-(y x d), y - d (y . d), -d)`: facing the viewer.
- **Ripple rings** `k = 0..7` (lists at `+0x24` start, `+0x88` window, `+0x08`
  and `+0x40` radius from and to, `+0x6c` offset): start `1.4, 1.42, 1.43, 1.45,
  1.43, 1.42, 1.4`; window `0.6, 0.7, 0.8, 0.9, 0.8, 0.7, 0.6`; radius `1.33 ->
  2.0, 3.33 -> 5.33, 5.33 -> 8.66, 8.0 -> 10.66, ...` linear in
  `(age - start) / start`; offsets along up `-5.33, -4, -2, 0, 2, 4, 5.33`
  (the position is `base - offset up`); the up row is further scaled by
  `3 - 25 (1 - u)^4`, `u = (age - start) / window`, which is **negative** for
  the first 0.41 of the window (an inside-out ring); clock `u`.
- **Draw** (`0x001512f8`): the fireball with the matrix at `+0x150`, the core,
  the bloom disc with `+0x1d0`, then two point lights at the blast's centre
  three units up (`0x006778c8`, ranges 40 and 100, colour `500, 200 -> 50...`,
  `5, 0.5`). **Played since 2026-10-08**, see "Weapon point lights" below.

**How it was checked.** Ghidra stops at the AltiVec in these functions and marks
`0x00327500` no-return, so a decompile of the start and the update is
truncated. A scratch interpreter over the raw bytes (capstone, `ppc64`
AltiVec, the Cell's `lvlx`/`lvrx`/`stvlx`/`stvrx` decoded by hand) ran
`0x00151538`, `0x001503d8` and `0x001512f8` with stubs for the draw and
set-time calls over random cameras, positions and orientations. The
arithmetic above reproduces its matrices to 4e-3 on rows of about ten units
(the executable's own polynomial `sin`/`cos`) and its clocks, windows and
`ColourAnim` to 2e-4, for every tick of the run. Scratch:
`data/scratch/hd-weapon-blasts/` (`emu.py`, `runblast.py`, `spec.py`; not
committed - the oracle numbers that matter are in `bomb_blast::hd::tests`).

**Materials.** `hd_bombfire_glow` (`@0x1d90`, fireball and core, cut-out),
`hd_bombfire_bloomring` (`@0x16a0`, texture-only, `slots::EMISSIVE`) and
`hd_bombfire_shockwaves_glow` (`@0x1950`): read in
[hd-unlit-programs.md](../../../rendering/hd-unlit-programs.md), "The Bomb's
fireball and shockwaves". The shockwave's vertices carry **two colours** after
the coordinate (stride 22, `ff 9f 00 4c` twice); `oag_rcs::rcsmodel` read the
coordinate out of them as `NaN` until the same day.

**Not read / chosen.** What the camera table holds at run time (confidence 65:
read as `-eye` in row 3 and the back axis in column 2, under which the
executable's own output is a camera-facing frame); the bomb entity's own up row
(this engine's frozen-pose substitute, as for Pulse); the Missile's pair
(next, below). **Confidence 85** for the law, 65 for the camera reading.

### Names recovered

| Address | Name | Confidence | Basis |
| --- | --- | --- | --- |
| `0x00144a48` | `NormalBomb_Construct` | 80 | writes `"NormalBomb.cpp"` (TOC `0x008aad60`) at `+0x30`, loads `HD_Bomb` and `HD_bomb_halo`, builds the blast |
| `0x001443f8` | `NormalBomb_Update` | 80 | `age (+0xe8) += dt`, the ship-in-radius loop, calls `NormalBombBlast_Update` |
| `0x00144040` | `NormalBomb_Detonate` | 80 | builds the matrix at `+0xf0`, `NormalBombBlast_Start`, `BOMBEXPL` |
| `0x00151ad8` | `NormalBombBlast_Construct` | 80 | loads the four models (eleven instances), binds the four material parameters |
| `0x00151538` | `NormalBombBlast_Start` | 85 | copies the matrix to `+0x290`, `+0x490 = 1`, frames, `WO_BOMB_SMOKERING`; run in the interpreter |
| `0x001503d8` | `NormalBombBlast_Update` | 85 | the per-tick law above; run in the interpreter |
| `0x0014fff0` | `NormalBombBlast_UpdateRipples` | 85 | the seven ripple rings; run in the interpreter |
| `0x0014fa08` | `NormalBombBlast_Reset` | 80 | zeroes age and `+0x2dc`, builds the curves from the table |
| `0x001512f8` | `NormalBombBlast_Draw` | 80 | the fireball, the core and the bloom disc, two lights |
| `0x006ca6b0` | `Matrix_RotateRowsByAxisAngle` | 75 | rows `x R(axis, angle)`, the axis taken raw |

**Lineage.** Omega's `data00.psarc` ships the same four `.vex`/`.rcsmodel` names
(`HD_bomb_sphere`, `HD_bomb_sphere_white`, `hd_bomb_sphere_bloomring`,
`hd_bomb_shockwaves`) beside `BombFire*.rcsmaterial` files, **different
materials**, and its executable names `NormalBomb.cpp`: *checked, applies, not
wired* - Omega's `WeaponModels` is `EMPTY`, its blast code and its materials'
programs are unread.

## 2026-10-07: the Bomb against the running original, and `HD_bomb_halo` (`hd-weapon-ref`)

The first RPCS3 reference for the detonation (recipe and the weapon-state table:
[rpcs3-capture.md](../../../reverse-engineering/rpcs3-capture.md), "Giving the player a
weapon"; frames `data/scratch/hd-weapon-ref/pair_bomb_stationary.png`, ours on the
right). Held state **9 is the Bomb**.

**What the original shows that this build does not.** The laid Bomb carries a pink ring and a
pulse (`e12/ring.png`); this build draws neither. Ours at ticks 402..418 shows the bomb
body alone. `HD_bomb_halo` is `NormalBomb`'s second model (`[0x2e]`, flag `0x400`), a
382-vertex, 760-triangle sphere shell of radius 0.60 with `hd_bomb_halo.rcsmaterial`
(state `0x79`, **SrcAlpha / One**, `pulse_bombflash_glow.gtf`); `hd_bomb.vex` carries
the same material on its sixth chunk. Both are additive and neither is drawn by `scene/
weapon_models.rs`.

**The scale law, read from `0x001443f8`** (confidence **45**: the instructions are
read, but the pulse it gives does not match the film, see below). While `age < the class's
fuse` (`*(class + 0x104)`, `timetodie` 20 s) and the bomb is not yet done, the update takes
`w = tab[0x10]` (`2.0`, or `tab[0]` = `6.0` once `age > tab[0x14]` = `18.0`), and
`s = fmodf(w * age, tab[4]) * tab[8] + tab[0xc]` with the table at `0x008c1a84`:
`{6.0, 1.0, 13.0, 3.0, 2.0, 18.0, 0.4, 0.35}`, so `s = 3 + 13 * frac(2 age)`: **a sawtooth
from 3 to 16 every 0.5 s**, applied to the bomb's rows `+0x70..+0xa0` (then selected by
the AltiVec mask `lis/lwz` from `-0x27b8(TOC)`, which was not decoded) and handed to the
halo's set-matrix call `0x00327500`. At 0.6 units of shell radius that is a ring 3.6 to
19 units across, which agrees with the ring's size in `e12` (about 18 units by angular
size), **and the film shows a steady outer ring as well as the growing one, and a flash
about every 0.3 s, not 0.5 s**. Whether the second ring is the `hd_bomb.vex` chunk or
the mask's other lanes, and whether alpha follows `s`, is unread. Not wired: a draw built
on a law that disagrees with the picture in period would be the plausible stand-in the
project rule forbids.

**The owner trips its own bomb.** The standing player's Bomb detonated 0.35 game seconds
after it was laid, from 0 km/h. The decompile has the ship loop (`0x002d64d0` per viewpoint)
with no owner test, and this build's `force_bomb_trip` skips the owner by design. **The
simulation is not this lane's**; the thread names it.

*Update 2026-10-07 (bomb-owner lane):* re-read statically, `NormalBomb_Update`
(`0x001443f8`) holds no trigger or owner test at all - the ship loop runs only once the
fuse or flag `0x80` of `+0x40` says the bomb is going off, so the writer of that flag is
the trip and is not found. The sim now exempts the owner for Pulse's `0.5 s` and trips on
it after, which the film's `0.6 s` supports (confidence 70). See the last section of
[mine.md](../psp-pulse-usa/mine.md).

**The first fireball frames do not match, and nothing was tuned.** The original is whited-out
yellow-white marbling filling the frame for 0.9 s with the camera 7 units behind a craft
that is itself shoved to 155 km/h; ours at +0.2 s has the craft shoved to 180 km/h within two
ticks and the camera outside the sphere, with a brown haze and an orange rim the original does
not show. The two runs differ in hull, camera distance and the bomb's own position (ours is
thrown ahead of the craft, the original's is under it), so **the size, brightness and
burn-away timing are not settled either way**; what is settled is the absence of the halo and
of the owner trip. Fixing the scenario (a pinned `--camera-pose` taken from the original's
chase camera, the bomb laid at the craft's own position) is the next step.

**Lineage (Omega):** not checked for `HD_bomb_halo` (the entry above counted four blast
models, not the halo); the RPCS3 recipe is HD-only, no PS4 emulator exists here.

## 2026-10-07: the Bomb matched against the film, in video time (`hd-bomb-match`)

**The film's clock is the bomb's age** (four timings, `rpcs3-capture.md`, "Video time is the
bomb's age"). The previous section's "0.35 game s" owner trip, the 0.9 s whiteout and the 0.3 s
halo period were HUD-clock readings; in recorder time they are 0.6 s, 1.63 s and 0.496 s, which
is `Bomb_InArmingDelay`'s 0.5 s, the fireball phase's 1.5 s, and the halo law's 0.5 s. **The halo
law `s = 3 + 13 frac(2 age)` is raised from 45 to 80 for its period**; the AltiVec mask that selects
which rows are scaled stays undecoded (the shell is a sphere, so the choice cannot show).

**Matched pairs.** Same circuit (Talon's Junction), craft (`feisar_c1`, `--variant concept1`), the
standing player's bomb laid at the craft, `--size 1280x720`, our eye pinned (`--camera-pose
-143.08,-46.6,-175.2,1,-0.05,0,0,1,0`, the far camera: 3 up and 11.25 back of the hull per the
`ExternalCameraFar` load line, checked against the original's pre-lay frame, `pre_cmp3.png`), the
rival put on the bomb to trip it (`--force-bomb-trip`; the sim's owner exclusion was a permanent one when these were taken and is a 0.5 s window since 2026-10-07). Frames at
age 0.2, 0.5, 0.8, 1.1, 1.4, 1.7, 2.0, 2.6 s, original left:
`data/scratch/hd-bomb-match/frames/pairA.png`, `pairB.png`.

Read as a player would:

- **Fill (the falsifier): passes.** At age 0.2 s the frame is a marbled pale-yellow-white fireball
  filling about nine tenths of the picture, as the film's; the marbling, the colour and the HUD on
  top agree. The fireball radius is `size` times a unit sphere (`hd_bomb_sphere` radius 1.011), 8.3
  at 0.2 s and 11.9 at 1.1 s, against an eye 11.64 away, so the eye is just outside the sphere
  until about age 0.5 s and just inside after.
- **The white core is the visible difference (age 0.5 to 1.1 s).** With the eye between the
  core (0.99 R, `ColourAnim` 0.9, x10) and the fireball, ours shows a hard-edged pure-white
  wedge and then a white-out the film does not (`pairA.png` rows 3-4); with the core not drawn the
  same frames are marbled yellow like the film (`nc_sheet.png`). Not changed: the core's draw
  condition is unread, and removing it would be an invention. Open question, with the
  experiment above: does the original draw the core at all while the eye is inside it?
- **After age 0.5 s the frames are not like for like**: the original's owner is flung to about
  125 km/h within 0.2 s and its chase camera follows through the fireball, so the eye stays
  inside it to 1.4 s with the hull visible; ours is shoved to 50 km/h and the pinned eye stays put.
  The sim's blast impulse on the owner differs from the original's (queued with the trip window).
- **The late rings differ**: at 1.7 s ours is an orange ellipse and horizon haze (shockwave
  rings seen from a static eye), the film's is a white ball at the right edge with a yellow band.
  Same cause, not separable until the camera follows.

**`HD_bomb_halo`'s program, read** (`scripts/ps3-microcode.py fp-file`, block `@0x19c0`, the
lit-race variant with fog; `hd_bomb_programs.rs` now lists it):

```text
a = TC0 . TC1 / sqrt(|TC0|^2 |TC1|^2)           rim = sat(1 - a)
ramp = tex(TC0.w, TC1.w)                         4x16, pulse_bombflash_glow.gtf
alpha = ramp.a * 50 * rim^(10 - 10 ramp.a) * ramp.a * 0.1
rgb = ramp.rgb ; then the circuit's fog
```

A Fresnel shell: bright at the silhouette, the ramp's alpha choosing the exponent. Confidence 75
(instruction by instruction, the vertex program's `TC0.w`/`TC1.w` unread). With the law above and the
pool already shaped (`halo-wiring-unfinished.patch`), drawing it needs the `rim_glow.rs` shape
and a `shade.wesl` branch. Not drawn: the generic lit program shows nothing.

**Omega:** the four blast models are checked, applies, not wired (above); `HD_bomb_halo` itself
was not looked up in Omega's archive this lane (no PS4 emulator for the film), so: not checked.

## 2026-10-08: `HD_bomb_halo` drawn, and the core's draw read (`hd-bomb-halo`)

**The halo program and the vertex program, both read.** Fragment block `@0x19c0` (above) is paired
with vertex block `@0x1860`:

```text
MOV o[TC0].w, v[2].x                    ; u
ADD o[TC1].w, v[2].y, c[208].x          ; v + Speed   (c[208] = register 464 = parameter 0x31182e0d)
ADD o[TC0].xyz, -(v[0] * c[210] + c[211]), c[209]   ; eye - position
MOV o[TC1].xyz, v[1].xyz                ; normal
```

`Speed` is authored on the material as **6.428** and nothing animates it (no clock in either
program), so the ramp row is `fract(v + 0.428)` under a repeat sampler. The draw state is `0x79`:
blend on, alpha test off (the `alpha_func 0x0201` the record also carries is not consumed), additive
`SrcAlpha`/`One`. Confidence 80 for the program and the constant (instructions read, constant read off
the record); the repeat sampler is inferred from the film's ring, not read.

**Wired.** `slots::BOMB_HALO` (bit 21, free since the glass pass was retired), a `rim_glow.rs` shape
fingerprinted on the 27 mnemonics, five literals and two parameters, a `shade.wesl` branch
(`ramp.rgb`, alpha `50 a^2 rim^(10-10a) 0.1`), `Speed` added to `v` when the mesh is built, and a pool
drawn per laid Bomb at `halo_scale(age)` (`p.age`, the seconds since it was laid). **Chosen, not
measured:** the alpha is held at 1 (the RSX wrote an 8-bit target; our float target would multiply
the ramp's top rows several times over). `hd_bomb.vex`'s sixth chunk carries the same material, so
the bomb body now draws a scale-1 ring as well; test
`hd_rim_glow_ground_truth::the_laid_bombs_halo_earns_its_bit_and_its_v_offset` fails if the shape or
the offset is dropped.

**Read as a player would** (Talon's Junction, `feisar_c1`, far camera pinned 40 units behind the bomb,
`data/scratch/hd-bomb-halo/frames/sheet3.png`, `sheet4.png`; the owner window was widened locally to
keep the bomb past 0.5 s, and restored): a pink Fresnel shell grows from about 2 to 19 units across
every 0.5 s and resets, with a smaller steady ring inside it (the body's chunk), as the film shows
(`hd-weapon-ref/e12/ring.png`). **Not reproduced:** the film's white disc at the reset.

**The core's draw is unconditional.** `NormalBombBlast_Draw` (`0x001512f8`) sets the matrix on `+0x2e4`
(fireball), `+0x2e8` (core) and `+0x2ec` (bloom) back to back; the only branch is the viewport flag
that picks `+0x150` or `+0x1d0`. There is no eye-inside or age test, so the core is drawn while the eye
is inside it. What differs from the film is therefore model flags (`0x00151538` ORs 4 into the first
four) or state, not a skipped draw. Confidence 80. Our whiteout is not changed.

**Omega:** `Data\Weapons\HD_bomb_halo.vex` (784 bytes), `hd_bomb_halo.rcsmodel` and
`materials\hd_bomb_halo.rcsmaterial` (31,112 bytes against HD's 26,480) are all in the PS4 extraction,
and `HD_bomb.vex` and `HD_bomb_sphere.vex` too: checked, applies, not wired (Omega's own program
bytes differ, so the shape would need its own fingerprint).

## 2026-10-07 (`hd-weapon-fx`): the Missile's explosion pool, read statically and live

The pool is `MissileManager + 0xcc + 4 i` (16 pointers to objects of vtable `0x00864b38`, stride
`0x2760` in the 2026-10-07 boots), the count at `+0x10c`. Method and the live numbers:
`docs/reverse-engineering/rpcs3-capture.md`, "Polling guest memory live". Confidences below are
per claim; no name is applied (nothing here is above 70 that was not already named).

- **Who enters the pool.** `0x00141288` (`pool_take`: `if count < 16 { Start(pool[count], matrix); count += 1 }`)
  is a thin entry; `Start` (`0x00155568`) is called from three places only: `0x001412c0` (that
  entry), `0x00142704` inside `0x001423a8` and `0x00143c28` inside `0x00143580`. `0x001423a8`
  walks the missile list (`this + 0x84`, count `+0xc4`) and, for each missile where the
  hit test `0x00126b78(arg, missile)` is true, sets the missile's flags (`|= 4`, or `0x24`
  with a velocity-scaled position bump) and then, for each of `GameState + 0xe4` viewports
  where the point is visible (`0x002d64d0`), takes a pool entry. **It is the craft-hit branch;
  a missile that dies on a wall never reaches `Start`.** Live: a state 1 missile (5 bounces,
  gone at 5.5 s) left the count at 0 and all 16 objects byte-identical. Confidence 70.
- **What retires an entry, and the law, measured on two boots.** On `m5` (the player put 12 units behind
  the nearest rival and state 1 fired, `scripts/rpcs3-mem-poll.py --behind-rival 12`) the missile list count
  `manager + 0xc4` fell 1 to 0 at host 4.47 s and the pool count `+0x10c` rose 0 to 1 in the same 10 ms
  sample; one pool object (index 0) filled in: `+0x34` flag word, `+0xd0/+0xd4/+0xd8` the hit position
  (`-268.5, 41.5, 110.7`, the rival's own place), the matrix at `+0xf0..+0x12f` (rotation rows and that
  position with `w = 1`, the rows `Start` copied from its argument), copies at `+0x130`/`+0x170` of the
  basis, `+0xe0 = 0x01000000` (the flag `Start` writes at `0x001558f4`) and then three floats that run
  with the object's age `p`:
  `+0x170 = p` (`0 -> 0.967`, linear, about `1.0 / s`: `0.0334, 0.0673, 0.1172, ... 0.9667` at 40 ms
  steps), `+0x17c = (1 - p)^2` and `+0x180 = 2 (1 - p)` (`0.9344 = 0.9666^2`, `1.9333 = 2 x 0.9667`,
  checked at all 25 samples to the fourth decimal). `+0x174 = 5.0` while alive and `-1.0` when free.
  `Draw` reads `+0x17c` as the light's range (`* 150`) and falloff exponent (`* 7 + 1.5`)
  (`0x006778c8(position, colour, D, w)`, no slot - see "Weapon point lights"), so the light dies quadratically. **The entry
  retires when `p` reaches 1: the count fell back to 0 at 5.45 s, 0.98 s after it rose** (`m3`: 0.96 s,
  the same boot's first observation), and every field is rewritten to its free value in one step.
  `HD_missile_explosion.vex`'s keyed `Anim Transform` nodes (`sphere`, `bloom`, `rays`, `shockwave`)
  end at key 60 (`60 / 60 Hz = 1.0 s`; scale `256 -> 4608`, 1x to 18x, ease-out: `779, 1250, 1671,
  2045, 2375, 2663, ...`, `hd_weapon_anim_keys`). **Lifetime 1.0 s, confidence 80** (two boots, the age
  field linear to 1, the keys ending together). Which function advances `p` and frees the entry was
  not found: no `stw ...,0x10c` decrements (the five stores to a `+0x10c` offset are `0x001412d0`, the
  increment in `pool_take`; `0x00141d7c`/`0x00142164`, zeroing in the twin `MissileManager`
  constructors `0x00141a88`/`0x00141e70`; and `0x00155024`/`0x001553bc`, the explosion class's own field
  init), so it runs through a computed store.
- **`Draw` (`0x00155420`, vtable slot 5) is also the per-frame body**: it draws the model at
  `this + 0xf0 + viewport * 0x40`, writes a point light `0x006778c8(light_slot, 1/f, radius)` whose
  size comes from a float at `this + 0x17c` (a per-object value never stored by this class's
  own functions: it is zero unless `Start`'s ranged randoms or the base class set it), and returns
  `1`. There is no age clock in the object: `Start` ends with `SetTime(model, 0)` (`0x002c1b30`)
  and the node clock runs on the render tree's own tick. Confidence 70.
- **The 16 entries at `this + 0x190`** (stride `0x50`, a matrix copy then `ranged_random` rotations
  `0x28c660` about the three axes, `0x677688`) are written by `Start` and **read by nothing in
  `0x154000-0x156000`** other than `Start` itself: they look like the rays' random orientations,
  which the `rays` node's mesh would take. Confidence 45; unresolved.
- **State identity** (`rpcs3-capture.md`): state 1 is the Missile (the list count `manager + 0xc4`
  is 1 while it flies and 0 when it ends; a counter in the block after the manager, `manager + 0x198`,
  reads 1, 2, 3, 4 at the bounces and 5 as it ends, against the Missile's `MAX_BOUNCES = 5`; confidence 70); state 2 is a different projectile with a large yellow-white
  burst that does not use this pool (unresolved, not the Quake's wave in the film either way);
  state 10 does nothing at the grid.
- **The picture, original only.** `m5` (known firer, rival 12 units ahead, `data/scratch/hd-weapon-fx/runs/m5`,
  contact sheet `burst.png`, 30 fps frames `f/h_*.png`): the count rose at host 4.47 s, which is video
  9.6 s (offset +5.1 s), and the frame's mean luma goes 135 (9.60 s) to 218 (9.67), 246 (9.80), 241
  (9.93), 209 (10.00), 162 (10.13), 150 (10.20). So the white-out starts within 0.1 s of the rise, is
  white to the frame edge for about 0.3 s with the struck rival's red-and-black hull in the middle of it
  and curved yellow-orange arcs (the rays and the shockwave rings) at the top corners, and is gone by
  0.55 s while the entry lives to 1.0 s (the additive shells thin out as the keyed scale decelerates and
  the colour term fades; the light falls as `(1 - p)^2`). `m3`'s earlier white-out (player in the pack,
  13 frames above luma 225 from video 48.57 s) is the same effect, its start again at the count's rise.
  Our side is **not drawn**. The trigger (a craft hit, visible in the viewport) and the clock (the
  model's own keys, `p` over 1.0 s) are known, and a keyed node clock exists
  (`Drawable::write_node_anims(seconds)`, used by the gantry and the adverts). Each of the three
  materials is **not** the generic glow, so wiring needs a program per node, read this lane
  (`scripts/ps3-microcode.py fp-file`, the first fogless variant of each):
  `core_glow` is `tex(TC) x vertex colour` with the alpha from `TC.z` (block 1); `lightrays_glow` is a
  Fresnel shell, `sat(1 - dot(TC0, TC1) / sqrt(|TC0|^2 |TC1|^2))` raised to 5, then `1 - 0.9 x`
  raised to 5 (the halo's shape with other exponents); `shockwaves_glow` is a two-tap ramp lookup that
  adds `Shockwave_scalar * 0.45` to a `0.01`-scaled `v` and a `0.05`-scaled first tap (so the clock
  enters through `Shockwave_scalar` and `UV_offset`, both the entry's age). Confidence 70 for those
  reads (instruction listings, the vertex programs' `TC` outputs unread). Open: the three programs
  in `shade.wesl`, one drawable per live instance, and the matched pair.

**Omega:** `HD_missile_explosion.vex`/`.rcsmodel` ships in Omega's archives (see
`ps4-omega-eu/weapons.md`); the pool, trigger and lifetime above were not looked up in Omega's
executable: not checkable here (no PS4 emulator).

## 2026-10-08 (`hd-missile-blast`): the Missile's explosion drawn

`oag_raceplay::missile_blast` plays `HD_missile_explosion` when a Missile reaches a craft
(`Race::ignite_blast`, `Missile` with `struck`), on HD only (`WeaponModels::missile_blast_hd`).

- **Programs** (fogged blocks, vertex programs read with `scripts/ps3-microcode.py vp-file`; each earns a
  pair of existing `slots` bits, the way the Bomb's did, `MISSILE_CORE`/`_RAYS`/`_SHOCK`). `U` = `UV_offset`
  (vertex `c[208]`) and `S` = `Shockwave_scalar`, both the age in seconds (the entry lives 1.0 s). Confidence 70
  for the reads, 80 for the age clock:
  - core (`@0x17f0`): `rgb = tex(u, v + 0.5 U).rgb * colour.rgb`, `a = tex.a * colour.a`. The colour is the
    chunk's inline last four bytes (`ff ff ff ff`, stride 18); no vertex colour decoded meant black and a
    core that never drew.
  - rays (`@0x17f0`): `rim = sat(1 - N.V)`, `f = sat(0.9 - 0.9 rim^5)^5`, `out = (f tex(u, U).rgb, tex.a)`; the mesh's `v` is unused.
  - shockwave (`@0x19b0`): the Bomb ring's program with `c = 0.45 S - (v + 0.05 n)` and `a = colour2.a * sum(tex(c,c).rgb) * (1 - S)`.
- **Node clock.** `write_node_anims(age)` scales `sphere`/`bloom` 1x to 18x (model radius 6.44, so 9.3x = 60 units at 0.1 s).
- **Chosen, not measured:** the orientation (the struck craft's own; `Start`'s source matrix unresolved), the
  per-viewport visibility gate (not reproduced), alphas held to 1, the 16 rotated entries at `+0x190` not drawn.
- **Matched pair** (Talon's Junction, `--force-missile-hit`, `data/scratch/hd-missile-blast/frames/pair_d.png`, film `m5`
  `h_011..h_028`): ours draws the cream-yellow core, orange ray arcs and ring discs, shape and colour like the film, but mean luma
  is 136/175/200/192/178 at ages 0.03/0.1/0.2/0.37/0.5 s against the film's 217/224/242/208/182. **Not reproduced:** the film's
  white to the frame edge that also washes the HUD and the player's craft, jumping 139 to 217 in one 30 fps frame. The model does not
  explain a HUD wash, so it is a screen-space part (bloom/exposure, the point light, or a flash object) still unread.
- **Omega:** `HD_missile_explosion` ships in Omega; same asset lineage as HD, so checked, applies, not wired
  (`oag-omega` has no `WEAPON_MODELS` entry, and its PS4 programs are unread).

## 2026-10-08 (`hd-whiteout`): what the Missile's white-out is, read off RSX frames on RPCS3

**Question.** When a Missile reaches a craft the film's frame goes white across everything, the HUD and the
player's craft included. Is that a screen-flash object, the bloom/exposure, or the point light? **Answer: none
of the three is a separate pass that washes the HUD.** The white is the explosion's own geometry around the
eye, lit and then bloomed by the ordinary chain; the HUD is drawn last and is not touched. Confidence 80 for
the structure (point 1 and 3 from one boot's dumped frames, point 2 from another boot's targets, the screenshots of
all three agreeing), 70 for "no flash object" (one static callee check, below).

**Method.** `scripts/rpcs3-hd-whiteout.py` (new): boots HD, gives the player a Missile (state 1), puts the player 12
units behind the nearest rival (`rpcs3-mem-poll.py`'s recipe), waits for the pool count at `manager + 0x10c` to rise,
sleeps a delay, pauses, takes the screenshot, then dumps the frame the RSX has queued (`rpcs3_draw_hook.py`) and/or the
render targets. `scripts/rsx-draw-list.py` prints a dumped frame's draws with the state each was issued under.
Raw captures `data/scratch/hd-whiteout/{run1,run2,run3}` (gitignored). Three boots: `run1` draws and screenshots at
pool ages 0.083 and 0.517 s, `run2` screenshots at 0.062/0.216/0.367/0.502, `run3` targets at 0.033/0.267/0.500.

**1. Nothing is drawn after the HUD, and the HUD's programs do not change.** In all three dumped frames (`base`,
`hit0`, `hit1`) the last pass is the target `00010000` (the screen): a full-screen quad of the finished image
(`fp 00744141`, sampling `00cc0000`) followed by the HUD's draws, `fp 00744181` (33 draws at rest, 40 and 32 in the
two hits), and then the next frame's first clear. No draw on any other target follows and the same two programs draw
the HUD with or without an explosion. (Subchannels other than 0 hold 23-24 blits per frame; they are the bloom
chain's resolves or copies, not read further.) So the HUD is not washed by a later pass. It *looks* washed because most of it is
translucent: the opaque hexes (lap, position, the damage ring) keep their colours over the white in every
screenshot, while the lap-time panel and the speed readout fade (`data/scratch/hd-whiteout/run2/hit0.png`,
`run3/hit1.png`). The film's "HUD washed" is translucency over white, **confidence 85**.

**2. The wash is already in the scene target, the bloom adds to it.** `run3` read the scene target `00f50000`
(A8R8G8B8, pitch `0x2800` = 2560 texels wide, two samples across, with `Write Color Buffers` on and the read done
through the GDB stub, see `rpcs3-capture.md`) at the same pause as the screenshot. At pool age 0.267 s: scene luma
209, final 232; pixels at or above 250 are **22.7 % of the scene and 45.8 % of the final**. At 0.50 s: 96 to 113 and
5.0 % to 8.3 %. At 0.033 s: 88 to 92 (no wash yet) and at rest 89 to 91. The explosion in the scene target is
yellow-orange ring discs and long light streaks over lit walls, no flat white
(`data/scratch/hd-whiteout/sheet5.png`, left final, right scene). The composite and bloom chain (`00741f41` down to
320x180 and 160x90, blurs `00741fc1`/`00741881`/`00741c01`, composite `007434c1` onto `00cc0000`) is the same set of
programs with or without an explosion (their dumped bytes are identical in `base`, `hit0`, `hit1`), so **no exposure or
tone-map constant moves**: the bloom is the engine's own, applied to a much brighter scene. One extra `007434c1`
draw (three, against two at rest) appears while an explosion is live; its purpose is unread.

**3. The explosion's draws, and the eye inside them.** In `run1` `hit0` three draws carry the model's age in
`c[464].x` (`0.05`, three pushbuffer ticks; the CPU read of the pool age at the pause was `0.083`, the queue runs about
two ticks behind): `idx` 224, 96 and 140 (probably the core, the rays and the shockwave; not matched to the model's chunks). All three are blended `SrcAlpha`/`One`
(additive), depth test on `LEQUAL`, **depth mask off, face culling off, alpha test off**, and share one matrix block:
rows of scale 4.88 (the model's `sphere` node at about key 2, `1250 / 256 = 4.88`, a tick under the `UV_offset` age) at `(-207.0, -72.6, 61.1)`, with the eye
(`eyePositionWorldSpace`, `c[465]`) at `(-181.8, -65.8, 64.1)`: **26.2 units from the centre, inside the core's
radius** (4.88 x 6.44 = 31.4). So from the first frames the eye sits inside the additive sphere and every pixel
accumulates its inner surface, which is what the white-out is; with culling off the inner face is drawn, and the
depth mask off lets the rays and shockwave stack on it. Confidence 75 (one boot, one frame read whole, the scale, the
centre and the eye all read off the same draw's constants; the `hit1` dump at 0.517 s was a partial frame with no
explosion draw in it).

**4. No flash object, statically.** The hit branch `0x001423a8` calls only `0x00126b78` (the hit test),
`0x00142258` (a `GameState` flag read ending in bad data), `0x002d64d0` (the visibility test), `Start`
`0x00155568` and `0x00676208`; `Start` calls `0x0028c660` (a ranged random, the 16 rotated entries),
`0x002c1b30` (set time), `0x002c9108`, `0x003238f8`, `0x003265c8`, `0x00676218`, `0x006762b8`, `0x00677688`, the
light pair `0x006778c8`/`0x00677c78` and `RaceManager_GetInstance`. Nothing there reads as a screen or HUD flash
(Pulse's `ScreenFlash` analogue is not among the callees at depth one; the subtree was not walked). Confidence 70.

**What follows for our renderer.** The missing parts are the light and the bloom's strength on HD, not a pass:
the point light `(1 - p)^2` (the `hd-weapon-lights` lane is wiring it as an SPU vertex-light record), and the HD
bloom strength (an open item for HD). **Our side re-measured** with the same method
(whole-frame mean of a grey-scale screenshot, `--force-missile-hit 300:1`, `feisar_c1`, 1, 6, 12, 22, 30 ticks
after the hit = ages 0.03/0.1/0.2/0.37/0.5): `--camera-view close` 195/203/211/211/208, `far` 194/202/209/210/207.
The earlier "136/175/200/192/178" does not reproduce on this base (`hd-missile-blast`'s own frames `d_2`, `d_6`, `d_12`
read 196/216/228 under the same method). The film `m5` reads 217/224/242/208/182 and three RPCS3 grabs read 224
(0.06), 226 (0.22), 232 (0.27), 142 (0.37), 148 and 113 (0.50): **ours is 20 to 30 luma under at 0.03-0.2 s and 25 to
95 over from 0.37 s**. The late excess is largely kinematics: the original's player is travelling at 100+ units/s
and leaves the sphere within about 0.35 s, ours is parked at the grid and stays inside it. The pair is not matched
until the player is moving; not tuned here.

**Chosen, not measured / not done.** The bloom strength, the light, the matched kinematics, the `007434c1` extra
draw, the render-target format of `00cc0000` (a read of it came back mostly black: its write-back is lazy).
**Omega:** the same model (`HD_missile_explosion`) and the same composite design apply; PS4 programs unread and no PS4
emulator in the toolchain, so **not checkable**. Nothing renamed; no `names.tsv` rows.

### The Bomb's core, read on the original (`hd-whiteout`, `bomb1`: one boot, one complete frame)

`scripts/rpcs3-hd-whiteout.py --bomb` (state 9, the player standing, the bomb trips on its owner) dumped one
complete frame with the fireball live (`data/scratch/hd-whiteout/bomb1/bomb0*`). **The original does draw the core
while the eye is inside it.** Four draws sit on the scene target with a large world matrix:

| Draw | Scale | State | Reading |
| --- | --- | --- | --- |
| 638 | 10.60 | opaque, alpha test on `LESS` ref `0x7f`, depth mask on, `LEQUAL`, cull off | the fireball, `ColourAnim` constant 0.3649 |
| 639 | 10.49 (= 0.99 x 10.60) | the same | the core, `ColourAnim` constant 0.9 |
| 640 | 10.64 | additive `SrcAlpha`/`One`, no depth write, alpha test off | the bloom disc (`hd_bombfire_bloomring`) |
| 641 | 53.33 | additive, no depth write | the first ring (`0.1 -> 53.33`) |

The fireball's `ColourAnim` 0.3649 puts the blast at age about 0.55 s by the law above (`x = 2 (age - 0.1) / 1.4 - 1`,
`|x|` branch), the centre 9.5 units from the eye (`c[259]` against `c[465]`) and the fireball radius 10.6: **the eye is
inside both the fireball and the core**, the case the open question asked about. Both draws use the same program
(`fp 0074cf01`/`0074d3c1`, the 50-instruction `@0x1d90` block: its `RCP`s give the `(1 - ColourAnim)^-1` multiplier, 1.57 and
10) and the same state, face culling off. So the original cannot be skipping the core by a draw condition or by
culling. **A lead, not tested:** both programs carry the clock parameter `AlphaAnim` as `0.996105`, where the law table
above gives `1.0` for age 0 to 0.75 s and ours takes the `divisor == 0` branch of the shader (any texel with alpha above
zero is cut, `shade.wesl`). With `0.996` the divisor is 0.0039 instead of 0 and only texels with alpha above 0.992 are
cut. If the original's clock is not exactly 1 over that stretch, our fireball and core are cut far more than the
original's and the white wedge is a symptom of that. Confidence 60 that the constant is the clock (read in both
draws, same value); the effect on our picture is not measured here.

**Not settled:** how the original's frame *looks* at that age, and a second boot of the core's state. The first boot's
screenshots were taken before the retried dump (the dump follows a resume), so bomb0's picture shows an earlier
moment than its draws. The second boot (`bomb2`, delays 1.2, 1.7 and 2.2 s after the press, screenshot after the dump)
never showed a fireball: the standing craft sat at 0-20 km/h with no detonation in any of the three captures (one
carries RPCS3's "Press and hold the START button" toast), two of the three dumps never completed (`only 42 draws`) and
the third holds a single large draw (scale 9.5, additive). So the core's state above is **one boot**, no repeat. The film
(`hd-weapon-ref/e11`, `e12`) stays the reference for the look. `rpcs3-hd-whiteout.py` now takes the screenshot after the
dump.




## Weapon point lights (2026-10-08, `hd-weapon-lights`)

**The call has no light slot.** `0x006778c8` is a tail-call thunk to
`SpuLight_AddCandidate` (`0x0040d990`), and the candidate store writes
`record = (v2.x, v2.y, v2.z, f2 | v3.x, v3.y, v3.z, f1)`: **`v2` position, `f2`
the falloff exponent `w`, `v3` colour, `f1` the range `D`** (`0x0040da38`-
`0x0040dab0`, read again here, confidence 90). The earlier "`(slot, 1/f,
radius)`" wording above and in the 2026-10-07 entries is retracted. The lights
therefore ride the mechanism renderer.md already read end to end: `EdgeGeom`
sums `max(0, 1 - |d|/D)^w * max(0, N.L) * colour` per vertex and the `SVC1`
programs add it to the pre-albedo diffuse. This engine's `SpuLights` list and
`spu_light_sum` are that, and the weapons only add records
(`oag_raceplay::weapon_light`, chained after the engines' in
`Race::hd_spu_lights`).

| Producer | Function | Position | Colour | `D` | `w` | Confidence |
| --- | --- | --- | --- | --- | --- | --- |
| Missile explosion | `MissileExplosion_Draw` (`0x00155420`, call `0x00155540`) | matrix row 3 at `this + 0xf0 + viewport * 0x40` | `(500, 200, 50)` | `150 f` | `7 f + 1.5` | 85 |
| Rocket, per rocket per frame | `Rocket_SubmitLight` (`0x00123de8`, call `0x00123e90`) | row at `this + 0x90` | `(7, 5, 1)` | `50` (`0x008aa124`) | `1` | 85 |
| Bomb blast, light 1 | `NormalBombBlast_Draw` (`0x001512f8`, call `0x001514bc`) | centre + 3 up | `(500, 100 + 100 x, 50)` | `40 s` | `7 (1 - x) + 1.5` | 75 |
| Bomb blast, light 2 | same (`0x001514f8`) | centre + 3 up | `(20, 5, 0.5)` | `100 s` | same | 75 |

`f = +0x17c = (1 - p)^2` for the Missile. The constants are `0x008ab258`
(`150.0`), `0x008ab25c` (`7.0`), `0x008ab260` (`1.5`), `0x008ab0e8` (`40.0`),
`0x008ab0e4` (`100.0`), `0x008ab0e0` (`7.0`), `0x008ab0ec` (`1.5`).

**The Rocket's light is `0x00123de8`, not the one at `0x001246cc`.** Read live
on RPCS3 (Talon's Junction, one rocket volley, `scripts` driver
`data/scratch/hd-weapon-lights/lightdump.py`, six snapshots over the first
1.0 s of flight): the visible buffer held the player's engine light
`(4, 10, 40)` and **three** records `(7, 5, 1)`, `D = 50`, `w = 1` that moved
with the three rockets (`x` from -116 to +256 across the snapshots), and the
candidate list held the same four. **No `D = 100` record, so `Rocket_Update`'s
`0x006778c8` call at `0x001246cc` (`D = 100`, `w = 1`, colour `(14, 10, r15)`
with `r15 = 2.0` from `lis r15, 0x4000` at `0x00124540`, the only path into
that store) is not a flight light.** It sits in the block that allocates a
`0x180`-byte object after the `0x0007be58` trace, so an impact-side flash is the
guess (below 50, unmeasured). The "colour `(14, 10, ?, ?)` unread" in
`rocket-trail.md` is closed: `(14, 10, 2)`. Not wired.

**The Bomb's `x` and `s`** are read off the original's own update, run in the
emulator (`0x001503d8` stepped at 60 Hz then `0x001512f8`, which logs the
calls): `x = 1` to 0.1 s, `1 - (t - 0.1) / 0.7` to 0.8 s, `((t - 0.8) / 0.7)^0.25`
(`0x00677868` is `powf`, exponent `0x008ab168 = 0.25`) to 1.5 s, then
`(1 - (t - 1.5) / 0.5)^2` to 2.0 s and 0; `s = 1` until 1.5 s, then `x`.
Closed forms fitted to the emulated samples (four digits), breakpoints read
from them: confidence 75.

**Matched pairs** (Talon's Junction, `data/scratch/hd-weapon-lights/frames/`,
`sheet_missile.png`, `sheet_rocket.png`, `sheet_bomb.png`, light off left, on
right):

- **Missile** (`m5` film): mean luma at ages 0.03/0.1/0.2/0.37/0.5 s, light off
  130/146/162/164/159, light on **202/208/213/207/185**, film
  **217/224/242/208/182**. The walls and track around the blast go to the film's
  cream and white; the film's wash over the HUD and the player's hull remains
  (screen-space, `hd-whiteout`'s), and the point light alone does **not** reach
  the HUD.
- **Rocket** (`r4`/`r5` film): the launch frames take a warm cream wash on the
  track and walls near the craft, which the film's first frame also shows, and
  it fades within about 0.3 s as the rockets leave; ours is stronger than the
  film at the launch frame (mean luma 132 off, 178 on at tick 424).
- **Bomb** (`hd-bomb-match` film): walls and track near the blast brighten to
  white at 1.1 s as the film does; the film is whiter still (its craft is
  nearer the bomb).

**Chosen, not measured:** the Missile orientation is irrelevant to the light
(centre only) but its position is the struck craft's; the Rocket's position is
the projectile's own; the original keeps eight visible records and this engine
passes every record to the vertex stage (no cap, no per-chunk cull); one
viewport.

**Omega:** `HD_missile_explosion` and the weapon objects ship in Omega, but its
race path has no SPU `EdgeGeom` light stage that was measured (PS4, no
executable capture path), so this is **not checkable**; **2048** has no
`SpuLight_AddCandidate` equivalent read (`Authored::engine_light` is false there):
checked, differs.

