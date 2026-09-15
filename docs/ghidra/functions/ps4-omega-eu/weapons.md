# Weapon and race/mode-manager constructors, transferred from `ps3-hdfury-eu`'s own names

2026-09-15. Functions in `eboot.bin` (WipEout: Omega Collection, PS4,
`CUSA05670`, EU), `x86:LE:64:default`, image base `0x01000000`. Answers the
follow-up to [`game-boot.md`](game-boot.md): now that this binary's own boot
chain is named, does the same `.cpp`-tag transfer technique
[`README.md`](README.md) already used for `MagstripWake_Construct` reach past
the boot chain into gameplay code proper? It does - every weapon-manager
`.cpp` tag `ps3-hdfury-eu/weapons.md` already names
(`docs/ghidra/functions/ps3-hdfury-eu/names.tsv`) exists verbatim in this
binary's own string table, under `Backend/Weapons/...` rather than HD's flat
layout, and the technique extends cleanly to the `RaceManager`/`ModeManager`
family once the weapon managers established the constructor idiom.

**The names here are applied**, from [names.tsv](names.tsv). 28 functions
named this pass: 9 weapon-manager constructors, the base `RaceManager` and
`ModeManager` roots, 16 `_RaceManager` subclasses, and the intermediate
`MPRaceManager_Construct`.

## Method

For each class HD already names (`BombManager`, `CannonManager`,
`EMPManager`, `LeachBeamManager`, `LightBarrierManager`, `MineManager`,
`MissileManager`, `PlasmaManager`, `QuakeManager`, `Rocket`,
`RocketManager`, `WeaponExplosions`, all confidence 85-88 on
`ps3-hdfury-eu`): `search_strings` its `.cpp` tag on this binary, then
`get_xrefs_to` the string to find the one function that writes it into a
`param_1[0xb]`-style tag field, the same idiom `game-boot.md` already
established for this binary's constructors. Every one of the twelve tags
exists here too, verbatim (`C:\WOPS4\Wipeout\Code\Backend\Weapons\<Name>.cpp`).
Nine decompiled cleanly and are named below; three more were located but
not decompiled, and two (`LeachBeamManager`/`MineManager`) turned out to
share one unnamed composite function - see "Not yet verified" below.

## Named this pass

| Address | Name | Pool size | Confidence |
| --- | --- | ---: | ---: |
| `0x01366f70` | `QuakeManager_Construct` | - | 87 |
| `0x01368610` | `Rocket_Construct` | - | 87 |
| `0x0136cd70` | `RocketManager_Construct` | 0x30 (48) | 87 |
| `0x013574d0` | `MissileManager_Construct` | 0x10 (16) | 87 |
| `0x0133df00` | `EMPManager_Construct` | - | 87 |
| `0x013275a0` | `BombManager_Construct` | - | 87 |
| `0x01333f90` | `CannonManager_Construct` | - | 87 |
| `0x0134aae0` | `LightBarrierManager_Construct` | 0x10 (16) | 87 |
| `0x01361b00` | `PlasmaManager_Construct` | 0x10 (16) | 87 |

**Confidence: 87** for all nine (one band above `ps3-hdfury-eu`'s own 85 for
the same nine classes: same evidence class - decompiled, tag verbatim, vtable
and flags shape identical to this binary's own already-confirmed constructor
idiom - corroborated by a second, independently-compiled binary naming the
same class, the same reasoning that raised `vita-2048-eu-v104`'s boot-chain
functions when `ps3-hdfury-eu` corroborated them).

Four more, beyond the five already written up individually above:

- **`BombManager_Construct`** (`0x013275a0`): tag
  `Backend\Weapons\BombManager.cpp` written directly. Owns **two** distinct
  nested item tags - `Backend\Weapons\DetonatorBomb.cpp` and
  `Backend\Weapons\NormalBomb.cpp` - matching the Detonator race mode's own
  bomb variant existing as a distinct class from the standard weapon, not a
  flag on one class.
- **`CannonManager_Construct`** (`0x01333f90`): tag
  `Backend\Weapons\CannonManager.cpp` - written from **two separate call
  sites** within the function (accounting for the four xrefs found during
  triage), both writing the identical tag text; not chased further, but
  consistent with two constructor overloads (e.g. a default and a
  parameterized form) rather than two different classes. Owns a nested
  `Backend\Weapons\CannonBullet.cpp` item tag, the same manager/item split
  `RocketManager`/`Rocket` and `LightBarrierManager`/`LightBarrier` (below)
  also show.
- **`LightBarrierManager_Construct`** (`0x0134aae0`): tag
  `Backend\Weapons\LightBarrierManager.cpp`. Pool of 16
  `Backend\Weapons\LightBarrier.cpp`-tagged items, each loading
  `lightbarrier_new.vex`/`.rcsmodel` and a second
  `lightbarrier_new_shockwave.vex`/`.rcsmodel` pair through the same
  `Data/Weapons` vs `Data/Weapons2048` runtime switch `Rocket_Construct`
  uses.
- **`PlasmaManager_Construct`** (`0x01361b00`): tag
  `Backend\Weapons\PlasmaManager.cpp`. Pool of 16
  `Backend\Weapons\Plasma.cpp`-tagged items; each also constructs a nested
  `Backend\Weapons\WeaponExplosions.cpp`-tagged object **inline, in the same
  body** - the same inlining `game-boot.md` already documents for
  `Game_Main`'s own `GameRoot`/`SystemRoot`/`MusicManager` constructors.
  **`WeaponExplosions` therefore gets no `names.tsv` row of its own on this
  binary** - there is no separate function to name, only this inlined
  occurrence (which loads `HD_plasma_ball`/`_ring`/`_sphere`/`_halo`
  `.vex`/`.rcsmodel` pairs). `ps3-hdfury-eu/weapons.md` names
  `WeaponExplosions_Construct` as its own function (`0x00128ab0`) - this is a
  genuine cross-binary structural difference, not a failure to find it here.

Evidence, each following the same shape `game-boot.md`'s
`SoundManager_Construct`/`FrontendRoot_Construct` already established
(allocate, zero, `*param_1 = &PTR_FUN_...` vtable, `param_1[0xb] = "<tag>"`,
`param_1[1] = <destructor fn>`), plus what is specific to each:

- **`QuakeManager_Construct`** (`0x01366f70`): tag
  `Backend\Weapons\QuakeManager.cpp` written directly (no `"Unknown"`
  placeholder step, unlike the others below). Allocates one nested `0x88`-byte
  object linked onto the manager via the same intrusive-list splice code
  `game-boot.md`'s managers use, then looks its own object up in a CRC-keyed
  table using `"WEAPON SYSTEM %d"`-formatted names against track data - the
  same "resolve myself against a per-track table" shape recurring across
  every weapon manager below.
- **`Rocket_Construct`** (`0x01368610`): tag `Backend\Weapons\Rocket.cpp`.
  Loads `hd_Rocket.vex`/`hd_Rocket.rcsmodel` through a path built by
  `snprintf(..., "Data/Weapons/%s", ...)` or `"Data/Weapons2048/%s"`,
  **selected by `DAT_01f999e4 < 0x17`** - the first evidence this project has
  found of the binary picking between HD's and 2048's own asset roots at
  runtime rather than shipping one fixed set, consistent with
  `omega-ps4-patch-adds-four-archives-not-in-the-base-pkg.md`'s own finding
  that the patch PSARCs bundle both titles' assets side by side.
- **`RocketManager_Construct`** (`0x0136cd70`): tag
  `Backend\Weapons\RocketManager.cpp`. Constructs a pool of **48**
  `Rocket_Construct` instances in a loop (`FUN_01368610` called with
  `lVar14` from 0 to `0x30`), plus two `"System\Render\SortRoot.cpp"`-tagged
  render sub-objects - a manager-owns-a-fixed-pool-of-its-item-class shape.
- **`MissileManager_Construct`** (`0x013574d0`): tag
  `Backend\Weapons\MissileManager.cpp`. Same shape, a pool of **16** missile
  objects (`FUN_013506f0`), plus one `SortRoot.cpp`-tagged explosion effect
  object that loads `HD_missile_explosion.vex`/`.rcsmodel` through the same
  `Data/Weapons` vs `Data/Weapons2048` runtime switch `Rocket_Construct` uses.
- **`EMPManager_Construct`** (`0x0133df00`): tag
  `Backend\Weapons\EMPManager.cpp`. Allocates one nested object tagged
  `Backend\Weapons\EMP.cpp` - **a source file not previously named on any
  binary in this project** (HD's own `weapons.md` names only
  `EMPManager_Construct`, not a separate `EMP` class) - consistent with the
  manager/item split `RocketManager`/`Rocket` already show explicitly. Not
  named here: the nested `EMP.cpp` object's own constructor address was not
  isolated from `EMPManager_Construct`'s body (the nested allocation is
  inlined, the same inlining `game-boot.md` already documents for this
  binary's `SystemRoot`/`MusicManager`).

Every one of the five also shows the `"Unknown"`-then-real-tag two-step
`game-boot.md`'s `SoundManager_Construct` already documents (`RocketManager`
and `MissileManager`), or writes the real tag directly with no placeholder
(`QuakeManager`, `Rocket`, `EMPManager`) - both idioms already attested on
this binary, so the split is not itself new evidence either way.

## Not yet verified: `LeachBeamManager.cpp` and `MineManager.cpp` share one composite function

`0x013781e0` decompiles, but not to a single constructor: it builds **three**
tagged objects in sequence, not one -

1. An object tagged not with a `.cpp` string but with `&DAT_0183a20e` - the
   same non-file string block (`"file\0Feisar\0Up\0Down\0Left\0Right\0..."`)
   `game-boot.md` already documents at the same field offset inside
   `Game_Main`'s inlined `SpeechManager`-shaped construction. Whatever this
   shared block actually is, it is reused as a stand-in tag in at least two
   unrelated places on this binary, not unique to speech.
2. A `Backend\Weapons\MineManager.cpp`-tagged object, constructed inline (no
   separate `CALL` - the same inlining pattern as `PlasmaManager_Construct`'s
   own nested `WeaponExplosions` object above).
3. A `Backend\Weapons\LeachBeamManager.cpp`-tagged object, also inline.

This reads as one composite "owns several weapon-adjacent objects" function
rather than `MineManager_Construct` or `LeachBeamManager_Construct` in the
single-class sense every other name on this page is. **It is also not a
weapon-manager top-level object at all**: while reading the `_RaceManager`
family below, `0x013781e0` turned up as a *member* constructor called
directly by most of them (`SPArcadeRaceManager_Construct`,
`DemoRaceManager_Construct`, `MPArcadeRaceManager_Construct`,
`MPTournamentRaceManager_Construct`, `SPFreePlayRaceManager_Construct`,
`SPTimeTrialRaceManager_Construct`, `SPTournamentRaceManager_Construct`,
`SPDetonatorRaceManager_Construct` all confirmed calling it directly, each
against a freshly allocated `0x1620`-byte block) - the reason `get_xrefs_to`
on either tag found only this one writer despite it constructing two
distinct classes: it is a shared composite every race owns one of, not code
reached from `Backend/Weapons/` at all. That still does not resolve what the
composite as a whole represents, since it never writes its own tag (the
placeholder in item 1 above is never overwritten) - naming `0x013781e0`
itself risks the same overclaim the confidence rubric warns against, so
**nothing at this address is named**, and neither `MineManager_Construct`
nor `LeachBeamManager_Construct` gets a `names.tsv`
row on this binary: like `WeaponExplosions` above, there is no separate
function address for either, only an inlined occurrence inside a
larger, unnamed owner.

`Repulser.cpp` does not exist anywhere in this binary's string table
(`search_strings` returns zero matches) - not a miss, since every other HD
weapon tag checked this pass returned exactly one hit. Genuinely absent, or
present under a different name/path; not chased further.

## `RaceManager_Construct` - `0x0128e550`

**Confidence: 87** (same evidence class and band as the nine weapon managers
above: clean tag, vtable, cross-binary corroboration with `ps3-hdfury-eu`'s
own `RaceManager_Construct`, confidence 80.)

Decompiles cleanly. Tagged `"Unknown"` on entry, then overwritten with
`C:\WOPS4\Wipeout\Code\Backend\General\RaceManager.cpp` before return, vtable
at `&PTR_FUN_01913f50`, destructor `FUN_01290c40`. The object itself is
enormous - the constructor touches offsets past `param_1[0x35ff]`, i.e. well
over 100 KiB per instance - consistent with a race manager owning large
per-track/per-ship state arrays rather than being a thin coordinator.
Prepends itself onto a global singly-linked list (`DAT_01a05440`) and
increments an instance counter (`_DAT_01a0545c`) - the shape of "every
constructed `RaceManager` (base and subclass alike) registers itself here",
matching the one-instance-per-mode roster the tag census below finds. Also
references a nested `System\Render\ParticleSystem_Importer.cpp`-tagged
object, not itself decompiled.

## `ModeManager_ConstructByMode` - `0x0124fe10`

**Confidence: 80** (Probable band: strong, unambiguous structural evidence -
decompiled, every branch tag-consistent - but no cross-binary corroboration
for this specific shape, since neither `ps3-hdfury-eu` nor `vita-2048-eu-v104`
has an equivalent single dispatcher to compare against, and no runtime trace.)

Decompiles (2,431 instructions, 482 basic blocks, cyclomatic complexity 336 -
by far the most complex function read on this binary so far). It is not one
class's constructor: the body writes the base
`Backend\General\ModeManager.cpp` tag and a shared vtable
(`&PTR_FUN_01913338`) **twelve separate times**, each immediately followed by
one of twelve distinct subclass tags overwriting it and a subclass-specific
vtable -
`GameModes\GameMode_TournamentModeManager.cpp`,
`GameModes\GameMode_ModeManager.cpp`,
`Backend\General\AIBatch_ModeManager.cpp`,
`Backend\General\Attract_ModeManager.cpp`,
`Backend\General\SPArcade_ModeManager.cpp`,
`Backend\General\SPTournament_ModeManager.cpp`,
`Backend\General\SPTimeTrial_ModeManager.cpp`,
`Backend\General\SPElimination_ModeManager.cpp`,
`Backend\General\MPArcade_ModeManager.cpp`,
`Backend\General\MPTournament_ModeManager.cpp`,
`Backend\General\MPTimeTrial_ModeManager.cpp`,
`Backend\General\MPElimination_ModeManager.cpp` - all twelve subclass tags
the earlier string census found alongside the base `ModeManager.cpp` tag
(thirteen total), accounted for. Named for what it demonstrably does - build the right `ModeManager` subclass
out of one shared body - rather than guessed at a specific verb; `_ByMode`
marks that this is a dispatcher, not a single class's plain constructor, the
same way `ModeManager_ConstructComplete` on `ps3-hdfury-eu` marks a
base-plus-derived two-stage constructor rather than a plain one.

Ghidra's decompiler reports `param_count: 0` and `get_function_callers`
finds none - consistent with the mode selector arriving through a register
or a computed/indirect call (a dispatch table indexed by mode enum) that
static analysis does not resolve here, not with this being unreachable code:
its shape is exactly what four other already-confirmed manager constructors
on this same binary look like, just repeated twelve times over twelve
different literal tags in a row.

## The `_RaceManager` family: sixteen separate functions, not a `ModeManager`-style dispatcher

Unlike `ModeManager.cpp`, `RaceManager.cpp`'s own tag resolves to exactly one
function (`RaceManager_Construct`, above) - and every one of the sixteen
`_RaceManager.cpp` subclass tags the string census found resolves to its
*own* dedicated function, each with a clean, single-writer xref pair: the
`ps3-hdfury-eu`/`vita-2048-eu-v104` shape (separate functions per subclass),
not `ModeManager_ConstructByMode`'s one-dispatcher-many-tags shape. All
sixteen decompile cleanly and open by calling an already-identified base
constructor before overwriting the vtable and tag with their own -
structurally the clearest evidence class on this page, since it proves the
inheritance relationship directly rather than through tag matching alone.

Fourteen call `RaceManager_Construct` directly:

| Address | Name | Confidence | Notably |
| --- | --- | ---: | --- |
| `0x0124f040` | `AIBatchRaceManager_Construct` | 84 | Loads `TimeTrial_HUD.xml` |
| `0x0125ba00` | `DemoRaceManager_Construct` | 87 | Loads `Arcade_HUD.xml` - matches `vita-2048-eu-v104`'s own `DemoRaceManager_Construct` by class name |
| `0x012a0b90` | `SPArcadeRaceManager_Construct` | 87 | Matches `vita-2048-eu-v104`'s own `SpArcadeRaceManager_Construct` by class name |
| `0x012a4610` | `SPDetonatorRaceManager_Construct` | 84 | - |
| `0x012a7e60` | `SPEliminationRaceManager_Construct` | 84 | Region-aware HUD selection (`WIP3OUT`/`2097` build-name checks, see below) |
| `0x012a98f0` | `SPFreePlayRaceManager_Construct` | 84 | Loads `TimeTrial_HUD.xml` |
| `0x012ad580` | `SPNitroRaceManager_Construct` | 84 | Also calls `FUN_012ac060(param_1 + 0x600e)`, a second helper not chased |
| `0x012af230` | `SPTimeTrialRaceManager_Construct` | 84 | Ghost/replay-slot allocation gated on `DAT_01f999e4 == 10 \|\| == 5` (track-count-dependent) |
| `0x012b1e60` | `SPTournamentRaceManager_Construct` | 84 | Also calls `FUN_0129c120(param_1)`, a second helper not chased |
| `0x012b4960` | `SPZoneRaceManager_Construct` | 84 | Loads region-specific `Zone_HUD.xml` variants (`wo3_HUD`, `2097_HUD` - see below) |
| `0x015a7020` | `GameModeRaceManager_Construct` | Loads `InGame2048`-namespaced frontend widgets (`InGameWipeout2048Logo`, `EndPreRaceButton`) - ties this `GameModes/` tag concretely to 2048 content, not just a name |

Two call a second, unnamed helper (`FUN_0129c120`/`FUN_012ac060`) between
`RaceManager_Construct()` and their own tag write; neither helper was
chased.

Four call `MPRaceManager_Construct` (`0x01284690`, below) instead of
`RaceManager_Construct` directly, plus `MPNitroRaceManager_Construct`
(`0x0127d940`, confidence 84), which calls `FUN_01284690()`
(`MPRaceManager_Construct`) then `FUN_012ac060(param_1 + 0x6148)`, a second
helper also called by `SPNitroRaceManager_Construct` above, before writing
its own tag - consistent with the same class hierarchy, not chased further:

| Address | Name | Confidence |
| --- | --- | ---: |
| `0x01279ed0` | `MPArcadeRaceManager_Construct` | 84 |
| `0x0127ad50` | `MPEliminationRaceManager_Construct` | 84 |
| `0x0127ecd0` | `MPTimeTrialRaceManager_Construct` | 84 |
| `0x01280490` | `MPTournamentRaceManager_Construct` | 84 |

**Confidence: 84 for fourteen of the sixteen, 87 for two.** All sixteen share
the same structural evidence - tag, vtable, and an explicit call to an
already-identified base constructor, stronger evidence than tag-matching
alone gave the weapon managers above, since it demonstrates the class
hierarchy directly rather than inferring it from parallel structure - but
that evidence is internal to this one binary. Per the confidence rubric,
decompilation-only evidence with consistent call sites caps at 84 ("Probable")
without a second binary naming the same class; `DemoRaceManager_Construct`
and `SPArcadeRaceManager_Construct` reach 87 because `vita-2048-eu-v104`
independently names those two exact classes (`DemoRaceManager_Construct`,
`SpArcadeRaceManager_Construct`), the same cross-binary-corroboration bar
`RaceManager_Construct` and `MPRaceManager_Construct` themselves clear
against `ps3-hdfury-eu`. The other fourteen have no such match in either
sibling binary's own `names.tsv` and stay at 84 until one turns up.

### `MPRaceManager_Construct` - `0x01284690`

**Confidence: 87** (matches `ps3-hdfury-eu`'s own `MPRaceManager_Construct`,
confidence 78, by class name and role - the multiplayer-specific
intermediate base between `RaceManager` and the four `MP*RaceManager`
leaves above.)

Calls `RaceManager_Construct()` itself, then sets its own vtable
(`&PTR_FUN_01913ab8`, distinct from `RaceManager_Construct`'s own
`&PTR_FUN_01913f50`) **without writing a `.cpp` tag of its own** - the base
class's tag is left standing until the concrete leaf class overwrites it,
the same two-level pattern `ModeManager_ConstructByMode`'s "write base tag,
then overwrite with subclass tag" shows, just without a repeated tag write
at this middle layer. Sets up an `"InGame2048"`-namespaced pre-race-callout
frontend widget and a redirect lookup keyed `"InGame"`/`"Redirect"`.

### Region-aware HUD selection, found while reading the `_RaceManager` family

Three of the sixteen (`SPEliminationRaceManager_Construct`,
`SPZoneRaceManager_Construct`, and the `MPElimination`/`SPTimeTrial`
siblings) branch their HUD XML path on a `strcasecmp` against the current
loaded title's own name string, matched against two literal build-codename
constants: `"WIP3OUT"` and a second string at `&DAT_018244b6` (not read this
pass, likely `"2097"` given the resulting path). The three resulting paths
are `Data\XML\<Name>_HUD.xml` (default), `Data\XML\wo3_HUD\<Name>_HUD.xml`,
and `Data\XML\2097_HUD\<Name>_HUD.xml` - `WIP3OUT` and `2097` are two of the
franchise's own historical titles (*wipE'out''3* and *WipEout 2097*/*XL*),
so this reads as the frontend picking a HUD skin to match whichever earlier
title's assets a given build/region is presenting under, not a gameplay
distinction. Not chased further - which builds actually set this and why is
a frontend/localization question, not a `RaceManager` one.

## `GameModes/` is a real, distinct directory - not a rename

`GameModes/GameMode_ModeManager.cpp`, `GameMode_RaceManager.cpp` and
`GameMode_TournamentModeManager.cpp` exist in this binary's string table
in their own `GameModes/` directory, distinct from `Backend/General/` - this
resolves the open question of whether `GameModes/*.cpp` is a PS4-only
wrapper layer over the per-mode managers `Backend/General/` already has: it
is not a rename of anything there, since both the plain and
`GameMode_`-prefixed tags exist side by side in the same string table.
`ModeManager_ConstructByMode` builds both `GameMode_ModeManager` and
`GameMode_TournamentModeManager` as distinct branches alongside every
`Backend/General/` one, and `GameModeRaceManager_Construct` (above) is a
real, separate, decompiled function - not a guess from the tag alone.

## Fuzzy matching does not bridge architectures here - use tags, not hashes

Before doing the tag-string search above, `find_similar_functions_fuzzy` and
`bulk_fuzzy_match` were tried directly, seeded from confirmed pairs:

- `vita-2048-eu-v104` (ARM Thumb-2) → `ps4-omega-eu` (x86-64), seeded from
  `MagstripWake_Construct`: at threshold 0.05, 10,279 candidates returned and
  the true target (`0x012e38d0`) is not among them at all. Pure noise.
- `ps3-hdfury-eu` (PowerPC64) → `ps4-omega-eu` (x86-64), seeded from
  `Ship_DispatchCollisionFx`: the true target ranks first (score 0.4682,
  next candidate 0.4551 - a thin margin). Seeded from
  `ShipCollisionFx_Trigger` on the same binary pair: the true target
  (`0x01723ae0`) does not appear in the top 14 matches above threshold 0.2 at
  all.
- `bulk_fuzzy_match`'s own `filter` argument does not appear to restrict the
  candidate source-function set as its description implies - two calls with
  different filter strings (`"named"`, `"Ship_"`) against the same program
  pair returned the same first-50-by-address block of `.opd.FUN_*` PS3
  relocation-descriptor symbols either time, not the requested subset.

One true hit out of three probes, with a thin score margin even on the hit,
means this tool cannot be trusted to drive naming on this project's
architecture spread (ARM/PowerPC64/x86-64) - it did not reach one match in
three even between the two closest-related ISAs here. The `.cpp` debug-tag
technique `README.md` already established for `MagstripWake_Construct` is
what actually transferred every name on this page, and should stay the
default for any future cross-binary work rather than the fuzzy tools.

## Next Steps

- `0x013781e0`'s own composite role is still unclear even knowing it is a
  per-race member every `_RaceManager` subclass constructs (see above) -
  read what else it owns beyond `MineManager`/`LeachBeamManager` before
  attempting a name; a guess here would be exactly the kind of name a future
  reader can't verify.
- `RaceManager_Construct`/`ModeManager_ConstructByMode` and the full
  `_RaceManager` subclass family are now named - `oag-race`'s own scope is
  the natural next reader, not further RE on this binary, unless the
  `ModeManager`-family subclasses (still unread beyond the dispatcher's own
  tag-write shape) turn out to matter for it too.
