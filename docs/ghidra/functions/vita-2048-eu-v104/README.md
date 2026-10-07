# WipEout 2048 Vita functions

Functions from `eboot.elf` (WipEout 2048, Vita, `PCSF00007`, EU, patch v1.04),
ARM Thumb-2. Import path and the SELF/NpDrm decryption this needed are in
[toolchain.md#vita](../../../reverse-engineering/toolchain.md#vita).

**Not a reverse-engineering target in its own right** - no milestone is open
on 2048. What this directory exists for is the same lineage question
[`ps3-hdfury-eu/`](../ps3-hdfury-eu/) exists for: 2048 ships HD/Fury content
as DLC, which raises the question of whether it is built off HD/Fury's own
codebase rather than a fresh build off the PSP/PS2 Pulse lineage this
project's other RE work is anchored to.

## The lineage question is answered: confirmed

Raised 2026-08-26 as a user hypothesis while naming `Game_Main`, confirmed
2026-09-01. `Game_Main` ([game-boot.md](game-boot.md)) sets the window/session
title to the literal
string `"WIPEOUT_HD"`, not `"WIPEOUT_2048"` - the first, narrow data point.
Confirmed properly by comparing literal debug-tag and asset-path strings
between this binary and `ps3-hdfury-eu/EBOOT.elf` via `search_strings`, which
is language-independent (PowerPC64 versus ARM Thumb-2, so no code shape could
match directly):

| String in `ps3-hdfury-eu` | String in this binary |
| --- | --- |
| `EngineTrail/TrailEffectManager.cpp` | `System/Render/EngineTrail/TrailEffectManager.cpp` |
| `Data/RibbonEffects/enginetrail_bluered_triangle.vex` (`Trail_ModelPath`) | `data/RibbonEffects/enginetrail_bluered_triangle.vex` |
| `%s\engineflare.vex` (`EngineFlare_ModelPathFormat`) | `%s\engineflare.vex` and `%s/engineflare.vex` |
| `GameRoot.cpp` | `Game/GameRoot.cpp` (`GameRoot_Construct`'s own debug tag, [game-boot.md](game-boot.md)) |
| `RaceManager.cpp`, plus 15 of its 16 per-mode variants (`SPArcade_`, `SPZone_`, `MPTournament_`, ...) | `Backend/General/RaceManager.cpp` and the same 15 variants under `Backend/General/`, plus one this binary has and `ps3-hdfury-eu` doesn't: `GameModes/GameMode_RaceManager.cpp` |
| `Collision.cpp` | `Backend/General/Collision/Collision.cpp` |

Every match is exact by path suffix (`ps3-hdfury-eu`'s tags are truncated to
the bare filename or a short relative path; this binary's carry the fuller
`Backend/General/...` tree). Nothing here is a coincidental filename: an
engine-vocabulary word like `lightmap` recurring across titles would not be
surprising, but a literal, multi-segment source path plus the exact same
15-of-16 enumeration of race-mode `RaceManager` subclasses is not something
two independently-built codebases converge on by chance. **2048 is HD/Fury's
own codebase retargeted for Vita, not a fresh build off the PSP/PS2 Pulse
lineage.**

Consequence for future work: when naming an unidentified 2048 function,
check `ps3-hdfury-eu/names.tsv` and its pages first - a same-role match there
raises confidence rather than requiring independent PSP/PS2-side evidence.
This does **not** retroactively change any name already applied here; the
seven boot-manager names in [game-boot.md](game-boot.md)
(`Memory_Alloc`, `Heap_Alloc`, `SystemRoot_Construct`, `SpeechManager_Construct`,
`SoundManager_Construct`, `FrontendRoot_Construct`, `MusicManager_Construct`)
were named on single-binary evidence only, at confidence 80-90, and none of
them assumed an HD/Fury shape - so a same-role match in `ps3-hdfury-eu` would
add corroboration, not correct a guess. That cross-check has not been done:
`ps3-hdfury-eu`'s own `names.tsv` has no `SystemRoot`/`SpeechManager`/
`SoundManager`/`FrontendRoot`/`MusicManager` equivalent named yet, so it
needs fresh RE work on the PS3 side before it can happen, not just a lookup.

## The lineage extends to Omega Collection's PS4 build too

`ps4-omega-eu/eboot.bin` (WipEout: Omega Collection, PS4) shares the same
`Backend/...` source tree as this binary and `ps3-hdfury-eu`'s: 74 of 76
`.cpp` debug-tag paths found in the PS4 binary match one here by suffix
exactly. That similarity held up at the code level too, not just the string
table: [`MagstripWake_Construct`](ships-effects.md) is the same constructor
in both binaries - same tagged-object field offset, same resource-name
lookup, same flag bits, same instance counter - despite one being ARM
Thumb-2 and the other x86-64. See
[`ps4-omega-eu/ships-effects.md`](../ps4-omega-eu/ships-effects.md) for the
full comparison.

## Pages

- [game-boot.md](game-boot.md) - `Game_Main` and the `GameRoot` singleton:
  the boot chain, the tagged-allocation idiom, and the lineage finding above.
- [track-and-collision-loaders.md](track-and-collision-loaders.md) - the
  track geometry and collision loaders.
- [ships-effects.md](ships-effects.md) - `MagstripWake_Construct`, found by
  cross-referencing this binary against `ps4-omega-eu`'s.
- [race-hud-selection.md](race-hud-selection.md) - which of the 26 shipped
  HUD layouts a real race actually constructs.
- [zone-audio.md](zone-audio.md) - Zone's sound banks: the bank-path gate is
  read, the announcer's own trigger is still not (all below the 70 naming
  threshold, cited by address only).
- [archive-mount.md](archive-mount.md) - the five PSARCs, their mount order, and
  which copy answers a path the base and the patch both carry.
- [zone-craft.md](zone-craft.md) - Zone loads one shared `hdships\Zone` hull for
  every craft; the pick selects a livery.
- [zone-environment-fallback.md](zone-environment-fallback.md) - a race's
  environment loader falls back to HD's own title-wide `effectSettings`, and
  the per-stage table it feeds runs every frame regardless.
- [pickup-icon-uv-table.md](pickup-icon-uv-table.md) - the held pickup's one
  `PickupIcon` widget gets its per-weapon look from a 12-slot UV table, not
  thirteen named widgets; found alongside the shield fill's own update
  function, which corrects two claims about `EnergyBg`/`EnergyBarDelay`.
- [game-mode-base-fields.md](game-mode-base-fields.md) - `GameModeBase`'s own
  field table: `M_PPLAYERSHIPMODELDATA`/`M_bPrevent*Ships` are the real
  craft-restriction mechanism `docs/formats/2048-campaign.md` had expected
  `WOShipCreatorParams` to be (authored on zero events).
- [weapon-type-bits.md](weapon-type-bits.md) - `WeaponType`'s own enum
  declaration, found by chasing the field-to-enum reflection link rather than
  a runtime consumer: settles `M_WEAPONAVAILABLEBITS` bit 4 as `Shield` and
  splits the prior joint `Mine`+`Bomb` pair into `Bomb`=8, `Mine`=9.

Add a row to [`names.tsv`](names.tsv) and the page it cites in the same
change: `scripts/apply-ghidra-names.py` refuses a row whose address and name
do not both still appear on the page named in its last column.
