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
- **Settled, fourth pass**: `+0x634`/`+0x638` are written by
  `Hud_UpdateZoneSpeedClassWidget` (`0x81197d6c`), the Zone-mode HUD's own
  Speed Class number/lights widget - not a race-progress timer, not
  `Zone_Update`'s own accumulate-every-10-seconds shape. See the dated
  section below for the full trace; the "third pass, still not found"
  history immediately above predates this and is kept for the record of
  what was actually checked, not as the current answer.
- **Sharpened, third pass**: `DAT_816c4890` is not the raw parsed file and
  not a passive read of `DAT_816c7148`'s cache entry - `Environment_LoadEffectSettingsFiles`
  writes it directly, inside a block that reverses a `0x816c4778`-`0x816c68d8`
  region in place, gated on a flag read off the current circuit record
  (plausibly "is this circuit reversed", not confirmed). **Still open**: how
  the raw file bytes in `DAT_816c7148` get *into* that region in the first
  place - no parser call was found in the traced block, so either it happens
  earlier (unread) or `DAT_816c4890`'s table is built some other way
  entirely.

## 2026-08-30, a fourth pass: `+0x634`/`+0x638`'s writer is found - it's the Zone-mode HUD's own Speed Class widget, not race/physics logic

**`Hud_UpdateZoneSpeedClassWidget`** (`0x81197d6c`, confidence 82) writes both
fields `Zone_UpdateStage` reads, in the same block:

```c
// piVar3 walks a 17-entry table (DAT_8151faf8, stride 0x10), each record's
// first word a descending threshold; iVar2 is a percentage-shaped progress
// value read off this widget's own instance struct
do {
    if (*piVar3 <= iVar2) {
        *(uint *)(DAT_8151f0ec + 0x634) = 0x11 - uVar6;   // the field Zone_UpdateStage clamps to 12
        *(int   *)(DAT_8151f0ec + 0x638) = (int)fVar10;   // the field Zone_UpdateStage divides by 5
        // ... +0x63c/+0x640/+0x644 also written here (not read by Zone_UpdateStage) ...
        FUN_810e33d4(*(undefined4 *)(param_2 + 0x704), auStack_30 /* "%d/%d" text */, 0, 0);
        break;
    }
    uVar6 = uVar6 + 1;
    piVar3 = piVar3 + 4;
} while (uVar6 < 0x11);
```

`DAT_8151f0ec` used bare (not indexed) matches `Zone_UpdateStage`'s own
`(&DAT_8151f0ec)[param_1]` for `param_1 = 0` exactly - both of
`Zone_UpdateStage`'s call sites already hardcode `param_1 = 0`, confirmed by
re-decompiling it this pass. Independently byte/decompile-verified in this
session (not taken from a single trace) before either rename.

**Identified via its init sibling, `Hud_InitZoneSpeedClassWidget`**
(`0x81197c24`, confidence 82, same vtable-slot group), which looks up scene
nodes by name - `"ZoneNumber"`, `"ZoneLight%d"` (0-9), `"ZoneSpeedLogo"` -
storing the results at the exact offsets `Hud_UpdateZoneSpeedClassWidget`
reads back (`param_1+0x704`/`+0x70c...`/`+0x734`). Unambiguous: this is the
Zone-mode HUD's own speed-class number/lights display widget, not a
race-progress or timer subsystem. Both functions are reached only through an
indirect per-frame widget-update pointer table (`0x815112xx`-`0x815119xx`,
Thumb-bit-set function pointers); `get_xrefs_to`/`get_function_callers` find
no static caller for either, which is normal for this kind of HUD dispatch
and not a blocker to the finding.

**A real Mach-number Speed Class ladder exists near the threshold table, but
its exact per-entry pairing is not nailed down this pass.** Strings
`MX_CLASS`, `M1_9_CLASS`, `M1_8_CLASS`, `M1_7_CLASS`, ... down through
`AP_CLASS`/`A_CLASS`/`B_CLASS`/`C_CLASS`/`D_CLASS` exist as a contiguous
null-terminated blob at `0x8148a4a0`, confirmed directly
(`inspect_memory_content`). `search_strings` for `CLASS` elsewhere in this
binary independently corroborates the same ladder by name
(`Speed_Class_A_Plus_0`/`_B_0`/`_C_0`/`_D_0`, `SpeedClass`/`NextSpeedClass`
scene-node strings, `data/fe/newimages/speedclass/{a,b,c,d,ap}_class.tga`) -
this is a real, title-wide Speed Class concept, not a one-off. **What is not
settled**: reading the threshold table's own second word as a per-record
string pointer (`DAT_8151faf8`'s 16-byte stride, word `+4`) gives an
inconsistent result on direct decode - the record for threshold `90`
(expected `M1_9_CLASS`) carries the *same* pointer as the record for
threshold `9999` (`MX_CLASS`), which doesn't fit a clean one-pointer-per-name
reading and was not resolved further this pass (possibly a sentinel/unused
first entry, an off-by-one in which word is the "real" pointer, or the
strings are addressed by some other index entirely). **So: the mechanism and
its writer are confirmed; a specific threshold-value-to-class-name table is
not** - do not cite a specific number-to-name pairing from this pass without
re-deriving it.

**This is the general Speed Class field, not a Zone-specific counter** -
which matches HD/Fury's own side of this question: HD's `speech_zone.bnk`
`MR_*` cue names and 2048's `ZoneMode2048.effectSettings` stage list
(`Subsonic, Mach 1, Supersonic`, ...) were already read as the same ladder by
name; this pass confirms from 2048's *executable* (not just file/audio
content) that Zone's stage-blend index rides on the same field this Speed
Class HUD widget maintains, not a bespoke Zone timer. **What this gives HD's
own still-unsolved search for its stage-index writer** (see
`docs/ghidra/functions/ps3-hdfury-eu/zone-effectsettings-loader.md`, which as
of the same day found its own write site by a different route and did not
end up needing this): if HD's write site had not already been found, the
thematic target would have been HD's own Speed Class HUD widget - scene
node names or a Mach-class string table - not a race-progress or physics
system.

**A second, unresolved writer of the same field, flagged not chased down**:
`FUN_811687e2` (an unrelated `"IG_HUD_TARGET"` opponent-lock HUD widget) also
writes `+0x634` on a *variably*-indexed craft (`param_2+0xd0`, not a
hardcoded 0), which was not traced to confirm whether it can ever target
slot 0 and interact with the Speed Class writer above. Not ruled out, but
lower-confidence than the Speed Class explanation (no named sibling, no
verified string ladder, no confirmed always-0 index). Two unrelated
functions (`FUN_811538c2`, a medal-tracker HUD widget; `FUN_813261be`, a TLS
session struct with a `"RC4-SHA"` literal) also write a `+0x634` field of
their own - both ruled out by full decompile as coincidental offset
collisions on unrelated structs, not craft-slot-0 at all.

**Two live tooling notes, confirmed on this bridge/binary and worth carrying
forward**: `search_instructions`' mnemonic/operand filter returned zero
matches even against a *known*-positive address on this ARM/Thumb2 binary
(`ldr.w r6,[r5,#0x634]` inside `Zone_UpdateStage` itself) - the same
exact-match trap `toolchain.md` already documents for MIPS and PowerPC,
confirmed a third architecture. And `search_byte_patterns`' `mask` parameter
does not filter at all on this bridge (a masked query returns either the
exact-pattern match or nothing, regardless of what the mask says) -
workaround: drop the mask and enumerate the one truly-unknown nibble as a
small set of exact patterns instead of relying on wildcarding.

Nothing wired into Rust from this pass - this settles who writes the field
2048's own Zone stage-blend reads, not what triggers a render change from
it, which stays open per the existing "does not settle" bullets above.

## 2026-08-30, a fifth pass: the threshold table is read in full, the "inconsistency" is a sentinel, and the value it is walked against is the zone number

The fourth pass found `Hud_UpdateZoneSpeedClassWidget` (`0x81197d6c`) and
stopped one step short: it read the per-record name pointer as inconsistent
(threshold `90` and threshold `9999` carrying the same pointer), called the
number-to-name pairing unsettled, and left the thresholds themselves
uncited. All three are closed here, and the table is now the live trigger
`crates/game/src/race/zone_grade.rs` runs a 2048 Zone race off.

### The table, read with `read_memory` rather than inferred

`DAT_8151faf8` (`0x8151faf8`), 17 records of `0x10` bytes, **renamed
`g_zone_speed_class_thresholds`** (confidence 82). Word `+0` is the threshold,
word `+4` a pointer into the name blob at `0x8148a4a0` - **renamed
`g_zone_speed_class_names`** (confidence 82) - and words `+8`/`+0xc` are a
sprite cell, each a multiple of `83` drawn from `{0, 83, 166, 249, 332, 415}`
and `{0, 83, 166}`, which is what the `ZoneSpeedLogo` node the init sibling
looks up needs and nothing a gameplay rule would.

| i | threshold | name | class written (`0x11 - i`) |
| ---: | ---: | --- | ---: |
| 0 | 9999 | `MX_CLASS` | 17 |
| 1 | 90 | `MX_CLASS` | 16 |
| 2 | 85 | `M1_9_CLASS` | 15 |
| 3 | 80 | `M1_8_CLASS` | 14 |
| 4 | 75 | `M1_7_CLASS` | 13 |
| 5 | 70 | `M1_6_CLASS` | 12 |
| 6 | 65 | `M1_5_CLASS` | 11 |
| 7 | 60 | `M1_4_CLASS` | 10 |
| 8 | 55 | `M1_3_CLASS` | 9 |
| 9 | 50 | `M1_2_CLASS` | 8 |
| 10 | 45 | `M1_1_CLASS` | 7 |
| 11 | 40 | `M1_CLASS` | 6 |
| 12 | 33 | `AP_CLASS` | 5 |
| 13 | 17 | `A_CLASS` | 4 |
| 14 | 9 | `B_CLASS` | 3 |
| 15 | 2 | `C_CLASS` | 2 |
| 16 | 0 | `D_CLASS` | 1 |

**The "inconsistency" is a sentinel.** There are seventeen records and the
name blob holds exactly sixteen null-terminated strings; record `0`'s
threshold `9999` is unreachable by any run and its pointer is record `1`'s.
Every record below it is strictly one name apiece, and the pointers step 12
bytes apart through the twelve-byte names (`MX_CLASS` .. `AP_CLASS`) and 8
apart through the four short ones (`A_`/`B_`/`C_`/`D_CLASS`) - so the
pairing is checked by the blob's own layout, not assumed from the order.

### What the value walked against them is

`iVar2 = *(*(param_2 + 0xc) + 0x20)`, and the same block `sprintf`s it as the
**first `%d` of `"%d/%d"`** into the node `Hud_InitZoneSpeedClassWidget`
(`0x81197c24`) looked up by the authored name `ZoneNumber`. The second `%d`
is `FUN_812c4e0c`'s return, and that function is three lines: it picks a word
off the widget's event descriptor by an event-type field (`+0x2d8` reading
`4`, `3` or anything else, giving `+0x3bc`, `+0x1cc` or `+0x1b4`) - a
per-event *target*, so the text reads `progress / goal`.

Two more writes in the same block corroborate the units. Besides `+0x634`
(the class) and `+0x638`, it stores the raw walked value into the race state
at `+0x63c`, the **matched record's own threshold** at `+0x640`, and the
**record above it** at `+0x644`. That triple is a progress bar between two
class boundaries, and it only means anything if the walked value climbs in
the thresholds' own units.

**Confidence 78** on "these thresholds are zone numbers": the authored node
name, the `%d` formatting against an event target, the neighbour-threshold
pair, and the field being read whole rather than computed from speed or
position anywhere in this function. What is *not* here is a traced increment
of `*(widget + 0xc) + 0x20` itself - the widget's `+0xc` was not resolved to
its owning object, so this rests on four converging reads rather than on a
found writer.

### What the numbers say about the play-based lead

The lead this work started from was "every few zones, especially the named
ones." The bands are `0`-`1`, `2`-`8`, `9`-`16`, `17`-`32`, `33`-`39`, then
every five to `90`. **"Every few zones" is corroborated**; "especially the
named ones" is **not** - every boundary is one class step, the `Sub`/named
stage names alternate straight up the ladder, and nothing is skipped or
singled out. Recorded as a split result rather than reshaped to fit.

### The 17-vs-13 question, answered rather than left tidy

`0x11 - i` yields `1`-`17` and `Zone_UpdateStage` clamps to `0xc`. So classes
`13`-`17` (thresholds `75` and up) all squash onto stage `12`, `Supersonic` -
the top of 2048's own thirteen-stage table. That is a real lossy squash at
the top of the ladder, not a clean fit, and it is stated here rather than let
the clamp's tidiness stand in for evidence. Classes `1`-`12` map one-for-one
onto stages `1`-`12`; stage `0` (`Start`) is never selected during a live
race, since the last record's threshold is `0`.

### Wired

`oag_title::ZoneStages`, `oag_2048::race::ZONE_STAGES` and
`ZoneGrade::show_zone`; checked against the real Vita package in
`crates/game/tests/zone_grade_ground_truth.rs`.

## 2026-08-30, a sixth pass: 2048's own effectSettings schema registrar

**`Environment_RegisterStageSchema`** (`0x8104714c`, confidence 80) - the
counterpart of HD's function of the same name
([zone-effectsettings-loader.md](../ps3-hdfury-eu/zone-effectsettings-loader.md#2026-08-30-a-twelfth-pass-the-schemas-registrar-is-found-on-both-titles---and-the-compile-time-array-framing-was-wrong)),
found from this title's own key templates rather than from its loader.

`search_strings` for `Growing Texture` and `Environment Fog Colour` returns
`"Zone %s.Growing Texture.{Colour,Scale Bias,Factors}"` (`0x8150a5c0`,
`0x8150a5e0`, `0x8150a604`) and `"Zone %s.Fog.Environment Fog Colour"`
(`0x8150a680`) - and `get_xrefs_to` on those puts every reference inside this
one function. Its body is the same format-then-register idiom HD uses:

```
8104bbe0  movw/movt r1, #0x8150a5c0   ; "Zone %s.Growing Texture.Colour"
8104bbe4  mov  r0, r8                 ; the formatted-name buffer
8104bbe6  mov  r2, sp                 ; the stage's own name
8104bbec  blx  0x813f38e0             ; buffer = format(template, stageName)
8104bbf0  ldr.w r1, [sp,#0xcc8]       ; the destination pointer
8104bbfa  bl   0x812f9dac             ; a typed registration helper
```

Confidence 80 rather than HD's 85: the templates and the idiom are read
directly, but this function has no static caller (`get_xrefs_to` returns
nothing - normal for this binary's indirect dispatch) and no loop bound was
read, so "fifteen stages" is HD's fact, not yet this title's.

**Its destinations are harder to read than HD's**, which matters for anyone
picking this up: they arrive from precomputed stack slots (`[sp,#0xcc8]`,
`[sp,#0xccc]`, `[sp,#0xcc4]`) and long-lived registers rather than from
`base + imm` literals, so the key-to-offset table needs the prologue that
fills those slots, not a peephole read at each call. Four typed helpers show
in the sampled window: `0x812f9dac`, `0x812f9ea4`, `0x812f9d30`,
`0x812f9d6e`.

**Also surfaced, unchased**: `"Debug.Reload Growing Textures"` (`0x81509e3c`)
- 2048 ships a named Growing Texture subsystem with its own debug reload,
which is a better handle on that effect than anything the file side has
offered so far.

**Nothing wired from this pass.** The offsets are not read, and even read they
would say where a parsed colour lands, not which material or mesh it
recolours - which is the actual blocker behind "the grade should affect all
textures, not just the fog".

## 2026-08-30, a seventh pass: 2048 *does* have a consumer - the Zone grade is a full-screen composite shader, not a per-material recolour

HD's side of this question closed as a negative (`Scene.*`/`Track.*` have no
located consumer -
[zone-effectsettings-loader.md](../ps3-hdfury-eu/zone-effectsettings-loader.md)).
**2048's does not.** The chain is complete, and it explains why HD's search
kept coming up empty: the two titles apply the grade in structurally
different places.

### The chain, end to end

```
Zone_UpdateStage (0x81044cfc)
  reads craft +0x634 -> clamp 0..12 -> DAT_816c6bac              the stage
  blends DAT_816c4890[stage] against DAT_816c4890[stage+1]
  hands the 16-byte result to
Zone_SetBlendedStageColour (0x8103876e)                          renamed, 85
  g_zone_blended_stage_colour = result                           0x816af070, renamed, 85
                    |
                    v  read at 0x810394b2
FUN_810387b4  (11,902 bytes)
  calls 33 distinct SceGxm_* entry points - a GXM draw/render function
```

`Zone_SetBlendedStageColour` is two stores and a return, which is what makes
it safe to name at 85: it exists only to publish that vector.

### The shader names settle what kind of pass it is

`FUN_8103d2ae` - the other writer of `g_zone_blended_stage_colour` - carries
a shader-name table, read directly off its string references:

```
default              wo_blur_fp             wo_composite_vp
wo_bloom_gate_fp     wo_blur_vp             wo_composite_nocc_fp
wo_bloom_gate_vp     wo_blur_ds_fp          wo_composite_notonemap_fp
wo_composite_zone_fp        wo_composite_zone_vp
wo_composite_zone_hdfury_fp wo_composite_zone_hdfury_vp
```

Bloom, blur and composite - a post-processing chain - **with two
Zone-specific composite programs in it**, and a second pair named
`hdfury` for the ported HD circuits this page's own fallback section is
about.

**So 2048 applies the Zone colour grade as a full-screen composite pass**,
fed by the blended stage colour, rather than by recolouring individual
materials. Confidence **80**: the data flow is traced instruction by
instruction, the reader is unambiguously a GXM render function, and the
shader names are the disc's own - what is *not* read is the shader binary
itself, so exactly how `wo_composite_zone_fp` combines the colour with the
frame is unrecovered.

### This reframes the user-facing question

The observation that started this work - "it only changes the fog, it should
affect all textures and such" - now has a mechanism behind it. The original
does affect everything, because it grades the **whole frame** in a composite
pass. It does not do it by binding `Colour 3` to some material, which is the
reading that made the problem look intractable.

**What that changes for wiring**: a full-screen grade needs no per-material
binding, which was the blocker. What it still needs is the composite's own
arithmetic - which of the stage's colours the pass consumes, and how it
combines them - and that is in `wo_composite_zone_fp`, a shader this project
has not extracted.

### The cross-fade rate, recovered for 2048

`Zone_UpdateStage`'s own weight, read from the same decompile:

- `DAT_816c6bc8` (the blend factor) `= DAT_816c6bc4 * 0.0002`, clamped to
  `1.0`, and `0.0` unless a transition is running (`DAT_816c6bcc`).
- `DAT_816c6bc4` is a transition timer accumulating from the frame delta.
- **The stage only commits once `DAT_816c6bc4 > 5000.0`**, and the `/5`
  counter at craft `+0x638` commits at `> 1000.0`.
- The blend runs current -> **next** (`DAT_8151c6f4 = stage + 1`, clamped to
  `0xc`), not against the previous stage the way HD's does.

That is the direction-and-rate question this thread has carried since the
render wiring landed, answered for this title. **It is not transferable to
HD**, which blends against `stage - 1` from a different field.

### What is still not read: 2048's key-to-offset table

The Thumb tracker was replaced with the decompiler's own dataflow
(`DecompInterface` + `HighFunction`, walking `PcodeOp.CALL` and expanding
each destination varnode). That fixed the template pairing - all 51
per-stage key names now sit against their own registration call - and
resolved the **16 non-stage globals** (`0x816c710c`, `0x816c7110`,
`0x8151c71c`, `0x8151c740`, `0x816c7028`, `0x816c6ba0`, `0x816c6bac`,
`0x816c6ba8`, `0x8151c708`, `0x8151c70c`, `0x816c6ba4`, `0x8151c6fc`) plus
two per-stage ones (`EQ.Mid-Band Position` -> `0x8151c738`,
`EQ.BG Mid-Band Position` -> `0x8151c73c`).

**The rest come back `expr:INDIRECT`** - the decompiler models them as
call-clobber effects on stack-held pointers, so the destination is not
recoverable from the call site alone. That is a sharper obstacle than the
first attempt's naive register tracking, and it is recorded rather than
worked around: **no 2048 offset table is published.** It would need the
prologue's stack-slot fills resolved, or a different vantage point entirely.

## 2026-08-30, an eighth pass: the composite shader is read - and the stage colour is `zoneEdgeColour`, not the frame tint

The previous pass ended by naming `wo_composite_zone_fp` as the next thing to
extract. It is extracted, and it **corrects that pass's own framing**.

### The shaders are in the executable, not on disc

`data.psarc`'s full 18,430-entry directory has **no shader entries** - the
census is `.gxt`, `.at9`, `.vex`, `.rcsmodel` and twenty others, and the
`.xfx` files that look like a candidate by name are ship engine audio. The
shaders are **111 `GXP\0` blobs embedded in `eboot.elf`** from file offset
`0x515f70`. Format, parser and validation on the new
[gxp.md](../../../formats/gxp.md).

### Blob #77 is the Zone composite

Enumerating every uniform and sampler name across all 111 programs, exactly
**one** declares `zoneEdgeColour`: blob #77 at file `0x51eac8`, whose full
parameter list is `bloomFactor[4]`, `accumFactor[1]`, `screenTintColour[3]`,
`zoneEdgeColour[3]` with samplers `mainTex`, `alphaTex`, `bloomTex`. Its two
siblings (#75, #76) are the same composite without the Zone uniform.

### The correction: the stage colour is the *edge* colour

`Zone_SetBlendedStageColour` publishes the cross-faded stage palette to
`g_zone_blended_stage_colour` (`0x816af070`). Traced from there to the GPU:

- `FUN_81037670` looks both uniforms up by name (`0x81424e18`
  `screenTintColour`, `0x81424e38` `zoneEdgeColour`) and caches their
  resource indices at `0x8151c650` and `0x8151c654`.
- `FUN_810387b4` stages `0x816af060` into `sp+0x78` and
  `g_zone_blended_stage_colour` into `sp+0x80` (`0x810394a0`, `0x810394b2`).
- It then writes `sp+0x78` against index `0x8151c650` and **`sp+0x80`
  against index `0x8151c654`** (`0x8103a9b8`-`0x8103a9ee`).

**So the Zone stage colour lands in `zoneEdgeColour`.** Confidence 85. The
frame-wide `screenTintColour` takes a *different* global, `0x816af060`,
written by `FUN_8103875c`/`FUN_81038780` - unchased, so **whether the
whole-frame tint is also Zone-driven is not established**.

> **This corrects [the seventh pass](#2026-08-30-a-seventh-pass-2048-does-have-a-consumer---the-zone-grade-is-a-full-screen-composite-shader-not-a-per-material-recolour)**,
> which said 2048 "grades the whole frame". What is now traced is narrower:
> the stage colour reaches the composite pass as **one of two** colour
> uniforms, and it is the one named *edge*. The pass being full-screen is
> still true; the stage colour being the frame's tint is not established and
> should not be repeated.

### What is still unread, and why it blocks wiring

The USSE bytecode. So `zoneEdgeColour`'s **arithmetic** - how it combines
with `mainTex`, `bloomTex` and `alphaTex` - is unrecovered. Its name says
edge, and an edge or rim term is not a flat tint, so **wiring a full-screen
tint from this would be an invention**, and a more tempting one now than
before because the plumbing is all traced. It is exactly the case
`CLAUDE.md`'s rule covers.

**What would close it**: a USSE disassembler (Vita3K has a recompiler; this
project has no decoder), or observing the pass's output directly.

## See also

- `docs/formats/effectsettings.md` - the file format this loader reaches for
- `docs/gameplay/race-modes.md#zone` - the four ported `zone_N` circuits this
  page's finding is about
- `zone-audio.md` - the track loader (`FUN_8121bce2`) `Environment_Load` is
  called from, and the sibling investigation into Zone's speech banks
- `docs/ghidra/functions/psp-pulse-usa/zone-mode.md` - Pulse's own
  `Zone_Update`, the closest analogue this project has already recovered,
  for whatever the eventual write-site trace for `+0x634` turns up
