# A race's environment loader falls back to HD's own title-wide effectSettings - and those files never shipped

Function in `eboot.elf` (WipEout 2048, Vita, `PCSF00007` patch v1.04), image
base `0x81000000`. Found while chasing
`docs/formats/effectsettings.md`'s open question of what selects a Zone
stage at runtime - a different question this function does not answer, but
along the way it explains something nobody had looked for: **why the four
`zone_N` circuits Wipeout HD's own DLC ports into this title have no Zone
palette of their own.**

## `Environment_Load` (`0x8102f6d0`), confidence 80

Called unconditionally from the track loader
(`docs/ghidra/functions/vita-2048-eu-v104/zone-audio.md`'s own
`FUN_8121bce2`, tagged `"Backend/World/Track.cpp"` - one call site,
`0x8121bf3e`) once per race, regardless of mode. Confidence 80: the control
flow is unambiguous decompilation and the branch this page is about is
independently corroborated against real disc content in all three packages
(below), which is stronger than a bare read - but several inner helpers
(`FUN_812e660e`, `FUN_8104619e`/`FUN_81044202`/`FUN_81044cfc`) are read only
by their call-site usage, not confirmed independently, and there is no
runtime trace.

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

So **racing Zone or Detonator on any of the four ported `zone_N` circuits
loads no effectSettings table at all** - the code that would supply one
reaches for four literal paths, all of them absent from the shipped disc.
This isn't a decode gap in this project; it is the original game's own data
falling short of its own code's expectations. Whatever the game does when
that load fails (skip the palette silently, fall back further, or something
this function's own later half was not read closely enough to say) is
unread.

## The normal path also carries an unshipped fallback of its own

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
    FUN_8104619e(local_14c, local_164);   // unread: plausibly primary+fallback load
```

Even on a circuit that *does* have its own `ZoneMode2048.effectSettings`
(every base-package circuit, per `docs/formats/effectsettings.md`), the
function also builds a second, title-wide path,
`ZoneMode2048default.effectSettings`, and calls a third function
(`FUN_8104619e`) with both. Read as a primary-then-fallback load by the same
`\<dir>\<name>` pattern the rest of this function uses, but not confirmed -
`FUN_8104619e` itself was not decompiled this pass. **This file does not
exist on disc either**, checked the same way as the four `ZoneEnvironmentHDFury`
paths above.

## What this settles and what it does not

- **Settles**: the four ported HD Zone circuits are a genuinely incomplete
  port on the effectSettings axis - the code path built for them is real and
  reachable, and the data it needs was never shipped.
- **Does not settle**: what selects a *stage* (the zone-number question
  `docs/formats/effectsettings.md`'s own Open section still carries).
  `FUN_8104619e`/`FUN_81044202`/`FUN_81044cfc` are the next functions in the
  chain and none were opened this pass.
- **Does not settle**: whether a failed load here crashes, skips silently, or
  falls back further - nothing downstream of the three unread calls above was
  traced.

## See also

- `docs/formats/effectsettings.md` - the file format this loader reaches for
- `docs/gameplay/race-modes.md#zone` - the four ported `zone_N` circuits this
  page's finding is about
- `zone-audio.md` - the track loader (`FUN_8121bce2`) this function is called
  from, and the sibling investigation into Zone's speech banks
