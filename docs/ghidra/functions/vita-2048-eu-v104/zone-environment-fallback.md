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
`FUN_8102493c` is called directly with the raw path string rather than
through the two-path pattern the success branch uses - it is a *different*
function from `Environment_LoadEffectSettingsFiles` below, now decompiled
and named as `Environment_LoadHDFuryContent`, see its own section.

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

## `Environment_LoadHDFuryContent` (`0x8102493c`), confidence 75 - a separate subsystem, not a shared one

**2026-08-28, fourth pass.** Called with the raw effectSettings path string
from the DLC fallback branch above (two call sites, both inside
`Environment_Load`, one per `DAT_8153fd24 == 0xe` arm - so one caller
function, two literal arguments). 1,664 decompiled lines; the relevant shape:

```c
piVar3 = FUN_81014390(param_1, 0x2001);     // same resource-lookup call
if (piVar3 != NULL) {                        // Environment_LoadEffectSettingsFiles uses
    ...                                       // read via vtable, same pattern
    FUN_812f9368(&DAT_815d3420, size, data);  // inserted into a DIFFERENT cache
}
if (*(char *)(DAT_8153fe00 + 0x1d9) != '\0') {
    // an in-place reversal of a 0x21c0-byte region at &DAT_81519e70,
    // 0x240-byte blocks, pointers walking toward each other - the same shape
    // Environment_LoadEffectSettingsFiles's own DAT_8153fe00+0x1d9 block has
}
if (DAT_8153fd24 == 0xe) {
    // FUN_812f05e0(&iStack_c0, "data/Tex/DetonatorModeTrack0.gxt", 0, 1);  ... through 14
} else {
    // FUN_812f05e0(&iStack_bc, "data/Tex/zoneModeTrack0.gxt", 0, 1);      ... through 14
}
// then: trig-heavy vertex/index-buffer construction (sin/cos via SceLibm,
// degrees-to-radians, a repeating 0x12-entry index stride) - a procedural
// ring/tunnel mesh, not decoded further this pass
```

**This confirms the `zonemodetrack{0..14}.gtf`/`DetonatorModeTrack{0..14}.gtf`
identification from `docs/formats/effectsettings.md` from the executable's
own control flow, not just file-content inference**: this function loads all
fifteen files of whichever "Track" set the active mode names, one texture
handle per index (`DAT_815d2d0c`, `DAT_815d2d10`, ... - reference-counted,
stride `4`, confirmed by reading the handle-release/retain bookkeeping around
three consecutive calls), gated `if`/`else` on `DAT_8153fd24 == 0xe` exactly
the way `Environment_Load`'s own effectSettings-path choice is. It then
builds what reads as procedural geometry (the "Growing Texture" mesh itself,
plausibly, since nothing else in this thread's evidence names a mesh for
that texture set to map onto) - genuinely unread past recognising the shape.

**Independently corroborates `Environment_LoadEffectSettingsFiles`'s own
`+0x1d9` reversal block**, on different data (`&DAT_81519e70`, not
`DAT_816c4778`-`DAT_816c68d8`) and a different byte layout (`0x240`-byte
blocks over `0x21c0` bytes, not `0x59`-word rows) - the same circuit-record
flag gating the same "walk two pointers toward each other and swap" shape in
two unrelated functions is real support for that flag meaning something
title-wide like "this circuit is reversed," not a coincidence of one
function's own local table.

**Does not reference `Zone_UpdateStage`, `Environment_LoadEffectSettingsFiles`,
`Zone_InitStageState`, the `+0x634`/`+0x638` offsets, or `DAT_816c4890`/
`DAT_816c7148` anywhere in its body** (checked directly against the full
decompile, not by sampling). So this is a genuinely separate, parallel
subsystem for the four ported `zone_N` circuits - it loads its own
effectSettings-shaped resource into its own cache and its own Track textures
into their own handle array, rather than feeding the native-circuit path's
tables. It does not identify `+0x634`'s struct and does not narrow the
write-site search from the section above.

**Confirmed against `Environment_Load`'s own full decompile, not a summary**:
the DLC/HDFury fallback branch that calls this function returns (via
`FUN_812e62cc(auStack_140)` then falling through to the outer function's own
tail) **without ever reaching `LAB_8102f7de`** - the label that calls
`Environment_LoadEffectSettingsFiles`/`Zone_InitStageState`/`Zone_UpdateStage(0)`.
That label is reachable only from the primary-probe success path
(`LAB_8102fe12`) or a prior-iteration flag (`DAT_816a4d84 != '\0'`), neither
of which the DLC branch sets before its own early return. **So on the four
ported `zone_N` circuits, `Zone_UpdateStage` is never called at load time at
all** - only from its other caller, the per-frame render loop, gated on
`DAT_816a4d80`, which the top of `Environment_Load` sets to `1` whenever its
outer gate (`FUN_810018d4(&DAT_8153fc40)`) passes, **regardless of which of
the two effectSettings branches runs afterward**. This turns the previous
pass's "plausibly zero-initialised `.bss`" hedge into a confirmed mechanism:
the per-frame blend runs every frame on these circuits with certainty, off
whatever `DAT_816c4890` already held, because nothing in either branch of
`Environment_Load` populates it for them.

Confidence 75: the resource-lookup and texture-preload shapes are
unambiguous and repeat the `Environment_LoadEffectSettingsFiles`/`Zone_UpdateStage`-family
pattern closely enough to read confidently, and the `Environment_Load`
control-flow check above is exact decompilation, not inference - but this is
a single-caller function whose full 1,664 lines were not exhaustively
walked (the geometry-building tail past the texture loads is characterised,
not decoded), and several inner helpers (`FUN_81014390`, `FUN_812f9368`,
`FUN_812f05e0`, `FUN_812f6bee`, `FUN_812f645c`, `SceLibm_4A496BC0`) are read
only by call-site usage.

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

## `DAT_816c4890` is built, not the raw file - and `+0x634`'s writer is still unfound

**2026-08-28, third pass.** `Environment_LoadEffectSettingsFiles` was
re-read in full (only its first third had been decompiled before): after the
two independent cache-inserts into `DAT_816c7148` (the primary and
title-wide-default paths, still true - "Not a primary-then-fallback pair"
above), a large block gated on `*(char *)(DAT_8153fe00 + 0x1d9)` walks a
region from `&DAT_816c4778` to `&DAT_816c68d8` with two pointers stepping
toward each other, `0x59` eight-byte words (712 bytes) per step, swapping
each pair - the shape of an in-place array reversal, not a copy. `DAT_816c4890`
sits `0x118` bytes into that same region and is written directly inside this
block (confirmed via `get_xrefs_to`: `Environment_LoadEffectSettingsFiles`
writes it at `0x810469de`, in addition to the read at `0x81046614` already
known). **So `DAT_816c4890` is not the raw parsed file and not a passive
alias of `DAT_816c7148`'s cache entry** - it is a table this same function
actively builds and conditionally reverses, gated on a flag read off the
currently-selected circuit record (`DAT_8153fe00`) at offset `0x1d9`. The
most likely reading - not confirmed - is a reversed-circuit flag: this
function already reads `DAT_8153fe00 + 0x74` elsewhere (the circuit's own
path string, in `Environment_Load` itself) as the same base object, and
`docs/gameplay/race-modes.md`'s track-reversal handling on other titles is
exactly the kind of thing that would need a stage table walked in the
opposite order. **Not settled**: what `+0x1d9` actually is, whether the
`0x816c4778`-`0x816c68d8` region is sized for 24 rows or some other count
(`(0x816c68d8 - 0x816c4778) / 712 = 12` swap-pairs, i.e. 24 rows at the
712-byte stride this loop uses - not the 356-byte stride `Zone_UpdateStage`
reads `DAT_816c4890` at, so whether a "row" here is one stage or two is
unread), and how `DAT_816c7148`'s raw bytes end up populating this region at
all - no call from `Environment_LoadEffectSettingsFiles` into a parser was
found in this block; it reads as rearranging data already present, not
decoding it fresh.

**The `+0x634` write site was searched for directly and not found.** Ruled
out this pass, each by full decompilation: `Zone_UpdateStage`'s own two call
sites (`Environment_Load` and `FUN_8101bc7a`, the per-frame render/update
body - neither writes the field, both only call `Zone_UpdateStage(0)`);
`Zone_InitStageState` (zeroes the module-level `DAT_816c6b*`/`DAT_816c6bd*`
blend globals only, never touches the per-craft struct); the craft state
machine `FUN_811c711e` (operates on a *different* object, at offsets in the
`0x5xxx`-`0x7xxx` range with no `0x634`/`0x638` anywhere in its body); an
EMP-bar HUD setup function, `FUN_8114b724` (also unrelated offsets). The base
pointer `(&DAT_8151f0ec)[0]` has dozens of direct readers across this
executable (`get_xrefs_to` on `DAT_8151f0ec` returns 29 hits in dead-reckoning
craft/HUD/camera code), and without a runtime watchpoint or exhaustively
decompiling most of them, the specific writer of `+0x634` was not found this
pass. Recorded as a real dead end rather than left silently unmentioned.

## What this settles and what it does not

- **Settles**: the four ported HD Zone circuits are a genuinely incomplete
  port on the effectSettings axis, and racing Zone or Detonator on one of
  them does not crash - the same per-frame stage-blend code runs, against
  data nothing loaded for that circuit.
- **Settles, fourth pass, confirmed against the full decompile**: the DLC
  fallback branch never reaches `Zone_UpdateStage`'s load-time call site at
  all - `Zone_UpdateStage` only ever runs for these circuits from its
  per-frame caller, unconditionally (the gating flag is set regardless of
  which effectSettings branch ran), against whatever `DAT_816c4890` already
  held. Not a hedge any more.
- **Settles**: the four ported circuits *do* get their own load-time
  subsystem - `Environment_LoadHDFuryContent` - but it is parallel to, not
  feeding, the native-circuit path: its own effectSettings-shaped resource
  cache, its own fifteen-texture Track-set preload (confirming
  `zonemodetrack{0..14}.gtf`'s identification from executable control flow,
  not just file-content inference), and what reads as its own procedural
  geometry. None of it touches `DAT_816c4890`, `Zone_UpdateStage`, or
  `+0x634`.
- **Settles**: a real, plausible stage-selection mechanism exists and is now
  named - `Zone_UpdateStage` reads a per-craft `+0x634` field, clamped to
  12, matching 2048's own 13-stage effectSettings table exactly.
- **Does not settle**: what `+0x634`/`+0x638` are fields *of* - the
  containing struct at `(&DAT_8151f0ec)[param_1]` was not identified this
  pass, so whether it is (or maps cleanly onto) this project's own
  `RaceState`-equivalent for 2048 is open.
- **Does not settle**: whether `+0x634` is driven the same way Pulse's
  `Zone_Update` drives its own zone counter (a plain accumulate-every-10-
  seconds), or something else. **Searched directly, third pass, and still
  not found**: `Zone_UpdateStage`'s own two callers, `Zone_InitStageState`,
  the craft state machine and an EMP-bar HUD function were all fully
  decompiled and ruled out - see the section above.
- **Sharpened, third pass**: `DAT_816c4890` is not the raw parsed file and
  not a passive read of `DAT_816c7148`'s cache entry - `Environment_LoadEffectSettingsFiles`
  writes it directly, inside a block that reverses a `0x816c4778`-`0x816c68d8`
  region in place, gated on a flag read off the current circuit record
  (plausibly "is this circuit reversed", not confirmed). **Still open**: how
  the raw file bytes in `DAT_816c7148` get *into* that region in the first
  place - no parser call was found in the traced block, so either it happens
  earlier (unread) or `DAT_816c4890`'s table is built some other way
  entirely.

## See also

- `docs/formats/effectsettings.md` - the file format this loader reaches for
- `docs/gameplay/race-modes.md#zone` - the four ported `zone_N` circuits this
  page's finding is about
- `zone-audio.md` - the track loader (`FUN_8121bce2`) `Environment_Load` is
  called from, and the sibling investigation into Zone's speech banks
- `docs/ghidra/functions/psp-pulse-usa/zone-mode.md` - Pulse's own
  `Zone_Update`, the closest analogue this project has already recovered,
  for whatever the eventual write-site trace for `+0x634` turns up
