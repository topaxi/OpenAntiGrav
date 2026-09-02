# Weapons: Repulser and Rocket construct, and two owning classes found but not read

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
  each instance `WEAPON_SYSTEM_%d` - and every instance owns up to ten member
  objects in sequence, sized `0xc4, 0x3a8, 0x78, 0x334` or `0x268`,
  `0xd4, 0x118, 0x13c` (**this is where `RocketManager_Construct` is called**,
  on a freshly `0x13c`-allocated, newly-registered child), `0x78, 0x90`. So
  `WeaponManager` is exactly what its name suggests: the per-craft umbrella
  that owns one `RocketManager` alongside (presumably) one manager per other
  weapon type - none of the other nine member constructors
  (`_opd_FUN_0012f430`, `_opd_FUN_0011b8e8`, `_opd_FUN_00113288`,
  `_opd_FUN_00138700`, `_opd_FUN_00112128`, `_opd_FUN_001457b8`,
  `_opd_FUN_00141e70`, `_opd_FUN_001483a0`, `_opd_FUN_0013d520`,
  `_opd_FUN_00118dc0`) are `__FILE__`-tagged in this map, so identifying the
  rest needs a different technique - not chased further this pass.
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
