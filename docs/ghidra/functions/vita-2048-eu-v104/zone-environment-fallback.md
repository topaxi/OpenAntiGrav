# A race's environment loader falls back to HD's own title-wide effectSettings, and the per-stage table it feeds runs every frame regardless

Functions in `eboot.elf` (WipEout 2048, Vita, `PCSF00007` patch v1.04), image
base `0x81000000`. Found while chasing
`docs/formats/effectsettings.md`'s open question of what selects a Zone
stage at runtime. Two passes: the first found why the four `zone_N` circuits
Wipeout HD's own DLC ports into this title have no Zone palette of their
own; the second traced the three functions that first pass left unread and
found the actual per-stage selection and blend logic.

## `Environment_Load` (`0x8102f6d0`), confidence 80

Called unconditionally from the track loader
(`docs/ghidra/functions/vita-2048-eu-v104/zone-audio.md`'s own
`FUN_8121bce2`, tagged `"Backend/World/Track.cpp"` - one call site,
`0x8121bf3e`) once per race, regardless of mode. Confidence 80: the control
flow is unambiguous decompilation and the branch this page is about is
independently corroborated against real disc content in all three packages
(below). `FUN_812e660e` (the path-exists probe) is read only by its call-site
usage; the other three helpers this function calls
(`Environment_LoadEffectSettingsFiles`, `Zone_InitStageState`,
`Zone_UpdateStage`) are now decompiled and documented below. There is still
no runtime trace.

## The circuit's own environment tree is tried first, native and DLC alike

```c
iVar4 = FUN_812e660e(auStack_1a0, ..., "art\\source\\environments", ...);
if (iVar4 < 0) {
    iVar4 = FUN_812e660e(auStack_1a0, 0, "art\\published\\environments", ..., 0);
    if (-1 < iVar4) goto LAB_8102fe12;   // found - load the circuit's own files
    ...                                    // not found - the fallback below
}
```

`FUN_812e660e` reads as a path-exists probe from how every call site here
uses it (negative on failure, non-negative on success, exactly the shape
every other candidate-path try in this function follows). Read this way, the
function first asks whether the *currently selected circuit* has its own
`art\source\environments` or `art\published\environments` tree at all.

## When it does not, the fallback assumes one of HD's four ported Zone circuits

```c
uVar5 = *(undefined4 *)(DAT_8153fe00 + 0x74);   // the circuit's own path string
FUN_812e6166(auStack_140, uVar5);
iVar4  = FUN_812e660e(auStack_140, ..., "Zone_1", ..., 0);
iVar6  = FUN_812e660e(auStack_140, ..., "Zone_2", ..., 0);
iVar16 = FUN_812e660e(auStack_140, ..., "Zone_3", ..., 0);
iVar14 = FUN_812e660e(auStack_140, ..., "Zone_4", ..., 0);
if (-1 < iVar14 || -1 < iVar16 || -1 < iVar6 || -1 < iVar4) {
    // circuit path names one of HD's own Zone_N circuits
    FUN_8102493c(DAT_8153fd24 == 0xe
        ? "data/ZoneEnvironmentHDFury/DetonatorModeHDFuryDLC3.effectSettings"
        : "data/ZoneEnvironmentHDFury/ZoneModeHDFuryDLC3.effectSettings");
} else {
    FUN_8102493c(DAT_8153fd24 == 0xe
        ? "data/ZoneEnvironmentHDFury/DetonatorModeHDFury.effectSettings"
        : "data/ZoneEnvironmentHDFury/ZoneModeHDFury.effectSettings");
}
```

`DAT_8153fe00 + 0x74` is checked for the literal substrings `"Zone_1"`
through `"Zone_4"` - the exact names of HD's own four ported Zone circuits
(`docs/gameplay/race-modes.md#zone`'s own table: `/data/environments/zone_N/`
on HD). `DAT_8153fd24 == 0xe` picks Detonator's pair over Zone's; whichever
pair, matching the substring picks the `DLC3` revision over the base one.
`FUN_8102493c` was not opened this pass - it is a *different* function from
`Environment_LoadEffectSettingsFiles` below, called directly with the raw
path string rather than through the two-path pattern the success branch
uses.

## Verified: this branch is genuinely reachable, not dead code

The primary check fails for exactly the circuits this fallback is written
for, because DLC2 ports them under an extra path segment the primary probe
does not try:

```sh
$ python3 scripts/psarc.py list data/extracted/vita/PCSF00007/dlc2/PSP2/dlc2.psarc | grep zone_1
data/art/published/DLC1/environments/zone_1/track.EnvSettings
...
```

`data/art/published/DLC1/environments/zone_1/`, not
`data/art/published/environments/zone_1/` - the `DLC1` segment is exactly
what makes `FUN_812e660e(..., "art\\published\\environments", ...)` fail for
this circuit, which is exactly what this function's own fallback branch
requires to fire. **And the four files that branch reaches for do not
exist anywhere on the disc** - checked directly against all three packages
(`base`, `dlc1`, `dlc2`):

```sh
$ python3 scripts/psarc.py list data/extracted/vita/PCSF00007/base/PSP2/data.psarc | grep -i ZoneEnvironmentHDFury
$ python3 scripts/psarc.py list data/extracted/vita/PCSF00007/dlc1/PSP2/dlc1.psarc | grep -i effectsettings
$ python3 scripts/psarc.py list data/extracted/vita/PCSF00007/dlc2/PSP2/dlc2.psarc | grep -i effectsettings
# all three: no output
```

So the code path that would supply a palette for Zone or Detonator on any of
the four ported `zone_N` circuits reaches for four literal paths, all of
them absent from the shipped disc. This isn't a decode gap in this project;
it is the original game's own data falling short of its own code's
expectations. **What actually happens when that load fails is now answered**
- see `Zone_UpdateStage` below.

## The normal path loads two files into a shared cache - not primary-then-fallback

```c
LAB_8102fe12:
    // <circuit>\ZoneMode2048.effectSettings
    FUN_812e684a(auStack_188, ..., local_298, ..., "\\ZoneMode2048.effectSettings", ...);
    FUN_812e747a(auStack_170, auStack_188);
    ...
    // a second, title-wide path built the same way, one directory up
    FUN_812e684a(auStack_188, ..., local_288, ..., "\\ZoneMode2048default.effectSettings", ...);
    FUN_812e747a(auStack_158, auStack_188);
    ...
    Environment_LoadEffectSettingsFiles(local_14c, local_164);
    Zone_InitStageState();
    Zone_UpdateStage(0);
```

Even on a circuit that *does* have its own `ZoneMode2048.effectSettings`
(every base-package circuit, per `docs/formats/effectsettings.md`), the
function also builds a second, title-wide path,
`ZoneMode2048default.effectSettings` - **this file does not exist on disc
either**, checked the same way as the four `ZoneEnvironmentHDFury` paths
above.

## `Environment_LoadEffectSettingsFiles` (`0x8104619e`), confidence 78

Called once, from `Environment_Load`, with the two paths built above. **Not
a primary-then-fallback pair** - both are loaded independently:

```c
piVar3 = FUN_81014390(param_1, 0x2001);      // resource lookup; NULL on a miss
if (piVar3 != NULL) {
    // read the resource's bytes through a vtable (GetSize at +0x3c, Read at +0x18)
    // insert into a shared cache keyed by name
    FUN_812f9368(&DAT_816c7148, size, data);
}
// exact same sequence, again, for param_2
```

Each of the two paths gets its own independent `FUN_81014390` lookup and,
only if that succeeds, its own insert into a shared resource cache
(`DAT_816c7148`). **A missing file is a silent no-op for that one insert**,
not a fallback to the other path and not an error - confirmed directly,
since `ZoneMode2048default.effectSettings` does not exist on disc and every
base-package circuit's Zone race still runs. Confidence 78: the
lookup-then-conditional-insert shape is unambiguous and repeats identically
for both parameters, but `FUN_81014390`, the vtable's own two slots, and
`FUN_812f9368` are read only by usage pattern, not independently confirmed
against a second call site (this function has exactly one caller).

## `Zone_InitStageState` (`0x81044202`), confidence 75

Called once, from `Environment_Load`, immediately after the loader above. A
plain lazy-init, guarded by its own done-flag:

```c
if (DAT_816c7068 == '\0') {
    // zero eleven DAT_816c6ba*/DAT_816c6bd* fields
    DAT_8151c6f4 = 1;            // "next stage" index
    DAT_8151c6f8 = 0xffffffff;   // "previous stage" sentinel, -1
    DAT_8151c6fc = 0.0075;
    DAT_8151c700 = 100.0;
    DAT_8151c704 = 5.0;
    DAT_8151c708 = 0.5;
    DAT_8151c70c = 0.75;
    DAT_8151c710 = 0.75;
    FUN_81043b1c(); FUN_81043608(); FUN_8104cb1e();   // unread
    DAT_816c7068 = '\x01';
}
```

Confidence 75: a clean, simple lazy-init idiom with an unambiguous guard, but
what the seven float constants and the three called functions actually tune
is not read - this page names the function by what it does (initialise the
Zone stage state block `Zone_UpdateStage` reads and writes), not by what
those numbers mean.

## `Zone_UpdateStage` (`0x81044cfc`), confidence 80 - the actual per-stage selection

**This is the function `docs/formats/effectsettings.md`'s Open section has
been asking about.** Two call sites, both with the literal argument `0`:

- `Environment_Load`, once, right after the two functions above (load-time
  priming).
- `FUN_8101bc7a` (the main per-frame render/update function - a dense
  matrix/culling/constant-buffer-binding body, one caller itself,
  `FUN_8102bfdc` at `0x813dbb42`), inside the branch gated on
  `DAT_816a4d80 != '\0'` - **every frame**, not once.

`param_1 = 0` at both sites is the local player's own craft/race slot -
consistent with a screen-space, camera-tied effect rather than something
each opponent needs independently.

```c
if (DAT_8153fc24 == '\0' && DAT_816c6ba0 == '\0' && -1 < param_1) {
    iVar4 = (&DAT_8151f0ec)[param_1];        // per-craft race-state pointer
    if (iVar4 != 0) {
        iVar5 = *(int *)(iVar4 + 0x634);      // raw stage index
        iVar4 = *(int *)(iVar4 + 0x638) / 5;  // a second, related counter
        if (iVar5 < 0)  iVar5 = 0;
        if (0xc < iVar5) iVar5 = 0xc;          // clamp to 12
        if (DAT_816c6bac != iVar5) {           // stage changed since last tick
            ... FUN_81263db8(..., "ZONE_Wave", ...);   // fires on a stage change
        }
        if (DAT_816c6bb4 != iVar4) {
            ... FUN_81263db8(..., "ZONE_Pulse", ...);  // fires on the /5 counter changing
        }
        ...
        DAT_8151c6f8 = DAT_816c6bac;   // previous
        DAT_816c6bac = iVar5;           // current stage, updated
        ...
        DAT_8151c6f4 = DAT_816c6bac + 1;
        if (0xc < DAT_8151c6f4) DAT_8151c6f4 = DAT_816c6bac;   // next, clamped the same way
    }
}
// blend current and next stage's rows of a per-stage table:
FloatVectorMult(&DAT_816c4890 + DAT_816c6bac * 0x59, 1.0 - blend, ...);
FloatVectorMultiplyAccumulate(&DAT_816c4890 + DAT_8151c6f4 * 0x59, blend, ...);
```

**The clamp is the load-bearing fact**: `0xc` is 12, and 2048's own
`ZoneMode2048.effectSettings` names exactly thirteen stages, `0`-`12`
(`docs/formats/effectsettings.md`). A raw stage index read off a per-craft
struct field, clamped to precisely the table's own maximum row, blended
between the current row and the next (`DAT_816c4890`, stride `0x59` words -
356 bytes, one row per stage) using a fade factor (`DAT_816c6bc8`) that
climbs while the field at `+0x638` sits in a band - this is the selection and
the cross-fade in one function. `+0x634`/`+0x638` are not independently named
elsewhere in this project's Ghidra work; their exact relationship to
`RaceState::zone`-shaped counters this project already tracks for other
titles is not established, only that `+0x634`'s clamped range matches this
title's own stage count exactly.

**What this settles about the four unshipped-palette circuits**:
`Zone_UpdateStage` runs every frame regardless of whether
`Environment_LoadEffectSettingsFiles` ever populated `DAT_816c4890` for the
current circuit - the per-frame call is gated on `DAT_816a4d80`, which
`Environment_Load` sets to `1` whenever *any* environment path was probed
successfully (the circuit's own tree **or** falling into the `Zone_1`-`4`
substring branch), not on whether the effectSettings file itself loaded. So
on the four ported `zone_N` circuits: no crash, no skip - the stage-blend
math runs against whatever is already resident at `DAT_816c4890`, which
nothing this pass traced ever writes for those circuits specifically. On a
cold boot that is very plausibly zero-initialised `.bss` (an all-black or
all-zero palette), and on a second Zone race in the same session it could
just as plausibly be whatever the previous circuit's own load left behind -
**neither is verified**; only a runtime trace (or a save-stated comparison
the way `ship-parts.md` used one) would settle which.

Confidence 80: two independent call sites both corroborate the same
`param_1 = 0` meaning and the same gating flag, the clamp-to-12 is an exact
match against a fact already established from the disc's own data (not
inference), and the table-stride arithmetic is unambiguous decompilation -
but `+0x634`/`+0x638`'s host struct is not independently identified, the
audio/effect cue names are read from usage rather than confirmed against a
played race, and there is no runtime trace.

## What this settles and what it does not

- **Settles**: the four ported HD Zone circuits are a genuinely incomplete
  port on the effectSettings axis, and racing Zone or Detonator on one of
  them does not crash - the same per-frame stage-blend code runs, against
  data nothing loaded for that circuit.
- **Settles**: a real, plausible stage-selection mechanism exists and is now
  named - `Zone_UpdateStage` reads a per-craft `+0x634` field, clamped to
  12, matching 2048's own 13-stage effectSettings table exactly.
- **Does not settle**: what `+0x634`/`+0x638` are fields *of* - the
  containing struct at `(&DAT_8151f0ec)[param_1]` was not identified this
  pass, so whether it is (or maps cleanly onto) this project's own
  `RaceState`-equivalent for 2048 is open.
- **Does not settle**: whether `+0x634` is driven the same way Pulse's
  `Zone_Update` drives its own zone counter (a plain accumulate-every-10-
  seconds), or something else - no write site for `+0x634` was found this
  pass.
- **Does not settle**: what the `DAT_816c4890` table's 0x59-word rows
  actually contain, or whether they are the parsed `ZoneMode2048.effectSettings`
  data at all rather than something built from it - `Environment_LoadEffectSettingsFiles`
  reads the raw file into a *different* cache (`DAT_816c7148`), and nothing
  this pass traced the link from that cache to `DAT_816c4890`.

## See also

- `docs/formats/effectsettings.md` - the file format this loader reaches for
- `docs/gameplay/race-modes.md#zone` - the four ported `zone_N` circuits this
  page's finding is about
- `zone-audio.md` - the track loader (`FUN_8121bce2`) `Environment_Load` is
  called from, and the sibling investigation into Zone's speech banks
- `docs/ghidra/functions/psp-pulse-usa/zone-mode.md` - Pulse's own
  `Zone_Update`, the closest analogue this project has already recovered,
  for whatever the eventual write-site trace for `+0x634` turns up
