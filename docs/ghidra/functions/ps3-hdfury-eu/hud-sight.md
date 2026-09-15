# HD's own lock-on sight: two per-tick updates, one shared bind, and a hold time that is not the PSP's

2026-09-15. Located from the widget-name strings themselves (`search_strings`
on `"Sight"`), TOC-resolved with `scripts/ps3-toc.py` and decompiled through
the bridge. All three addresses below are TOC-`exact`.

Read [`race-hud.md`](race-hud.md) first for `Hud_LoadDefinition` and the
per-mode HUD file set; this page is the runtime side, once a HUD object
exists and is ticking.

## The names

| Address | Name | Confidence |
| --- | --- | --- |
| `0x0009bb30` | `Hud_BindWidgets` | 80 |
| `0x0008a5b8` | `Hud_UpdateMissileSight` | 78 |
| `0x00090d30` | `Hud_UpdateLeachBeamSight` | 78 |

## Finding the bind site: ten strings, one function

`search_strings("Sight")` on this binary returns exactly ten hits, all
contiguous in `.rodata`:

```
0077f890 MissileSightBG
0077f8a0 MissileSightOuter
0077f8b8 MissileSightInner
0077f8d0 MissileSightMiddle
0077f8e8 MissileSightLockedOnLines
0077f908 MissileSightLockedOnMiddle
0077f928 LeachBeamSightBG
0077f940 LeachBeamSightOuter
0077f958 LeachBeamSightMiddle
0077f970 LeachBeamSightInner
```

These addresses sit at `0x0077f890`-`0x0077f990`, which is inside this
binary's TOC overlap-and-beyond region (above `0x758110` per
[`memory.md`](memory.md#there-are-exactly-two-tocs-and-they-overlap-in-address-space)),
so `get_xrefs_to` on any of them returns nothing - not because they are
unused, but because Ghidra's own analysis resolved every TOC-relative load in
this range against the wrong TOC and never built the reference in the first
place. `scripts/ps3-toc.py attrib`, which resolves through each function's
*own* OPD-declared TOC rather than Ghidra's uniform one, finds the real
caller for all ten in one function: **`0x0009bb30`**.

That function is not sight-specific. It is a ~2,700-instruction, straight-line
sequence run once per HUD construction (guarded by a "did I already run" byte
at `param_1+0x161`) that resolves several hundred widget names to pointers -
`ForwardHUD`, `SpeedClassParent`, `PickupBackground`, the eight-wide
`LapBar%d`/`PosBar%d` progress-arc loop, the Zone ladder, and so on - of which
the ten sight widgets are one small unconditional run in the middle, no
different in shape from the widget resolution around it. Named `Hud_BindWidgets`
rather than anything sight-specific, since renaming it for the ten this page
cares about would misdescribe the other few hundred.

### The ten widgets' offsets in the HUD object

`search_byte_patterns` on each string's own address finds its TOC slot
directly - the ten are themselves contiguous, `0x008a7ac8`..`0x008a7aec` in
strict string order - and each slot's displacement from this function's own
TOC (`0x008ad4d8`, confirmed `exact`) lines up byte-for-byte with the sequence
of `_opd_FUN_0067e6d0(param_1, <name>, 0)` calls (the resolve-by-name helper)
and the offsets their results are stored at:

| Widget | Offset (`param_1 +`) |
| --- | --- |
| `MissileSightBG` | `0x578` |
| `MissileSightOuter` | `0x57c` |
| `MissileSightInner` | `0x580` |
| `MissileSightMiddle` | `0x584` |
| `MissileSightLockedOnLines` | `0x588` |
| `MissileSightLockedOnMiddle` | `0x58c` |
| `LeachBeamSightBG` | `0x590` |
| `LeachBeamSightOuter` | `0x594` |
| `LeachBeamSightMiddle` | `0x598` |
| `LeachBeamSightInner` | `0x59c` |

All ten resolve in one unconditional run with no branch between the Missile's
six and the LeachBeam's four - binding tells nothing about which is shown
when; that is the two functions below.

## Two separate per-tick updates, not one shared switch

An exhaustive `search_instructions` for `lwz ...,0x578(...)` through
`0x59c(...)` across all 1,829,838 instructions in the program turns up one
function that touches the Missile's six offsets in the sight-specific way
(lock search, easing, colour writes) - `0x0008a5b8` - and a **second, separate**
function doing the structurally identical thing to the LeachBeam's four -
`0x00090d30`. (Both offset ranges also turn up in a handful of unrelated
functions touching unrelated structs at the same small numeric offset - noise
inherent to a byte-offset search, not additional sight code; the two named
functions are the ones whose reads land inside the sight-shaped lock-search-
and-colour-write block, not a bare isolated load.)

So HD does not read a "which weapon is held" flag inside one shared sight
update the way this project's own `oag_race::sight::Held` does - it runs a
**whole second copy of the update**, keyed by which pool of eleven-plus-widget
weapon systems is active (the same `WeaponManager` gating
[`weapons.md`](weapons.md) already found branches on mode and a shared byte).
Both functions:

- resolve "my own" `RaceManager`/player-slot pointer through an identical
  eight-slot-then-fallback lookup (`+0xe8` through `+0x104`, then `+0x13e8`) -
  boilerplate, not sight law.
- run a **lock-target search** over live craft: pairwise squared-distance and
  an angle test built from `vectorReciprocalSquareRootEstimateFloatingPoint`
  and a `vectorCompareGreaterThanFloatingPoint` against a threshold constant -
  the RSX-shader-style vectorised equivalent of the PSP's scalar `w > 0` /
  screen-bounds gate in [`lock-sight.md`](../psp-pulse-usa/lock-sight.md).
- accumulate a **hold timer** (`param_1` is the per-tick delta time in
  seconds) and compare it against the *same shared constant address*,
  `DAT_008a764c` - see below.
- ease a per-widget "extent" value toward the resolved target and write it
  through each widget's own vtable setters (`+0x68`/`+0x70`, x/y) and
  `Image_SetVertexColours` for the tint - the same "chase and tint" shape
  `HudSight_Update` uses on the PSP, restated as PS3 render-object method
  calls instead of direct field writes.

Neither function ever touches the other's six/four offsets. This directly
answers the brief's (a): **the two dialects are not one function switching on
held weapon - they are two independent, structurally-mirrored update
functions**, each hardwired to its own six or four widget pointers.

## The hold time is 0.5 s, not the PSP's 0.8 s

Both functions gate their lock on the *same* constant address,
`0x008a764c`. Read directly (`read_memory`, 4 bytes, big-endian IEEE-754):

```
008a764c: 3f 00 00 00  =  0.5f
```

`bVar24 = DAT_008a764c <= fVar2;` where `fVar2` is the running hold
accumulator (`param_1 + previous accumulator`, reset on loss of target) - the
exact role [`oag_race::sight::HOLD_SECONDS`] plays for `0.8`. **This is a
concrete, directly-read value, not an inference**: one memory read, one
unambiguous bit pattern, referenced identically by both the Missile's and the
LeachBeam's update, so it is HD's one shared hold time for both weapons, and
it disagrees with the PSP's `0.8` measured in
[`lock-sight.md`](../psp-pulse-usa/lock-sight.md#the-hold-timer-and-where-it-lives).

**Not adopted here.** `oag_race::sight::HOLD_SECONDS` is shared, title-blind
engine code judged against the PSP's own recovered law; changing it for HD
alone is a simulation-behaviour change, and this lane's brief is
presentation-only and explicitly defers a change like this to a report rather
than a commit. The fix wants either a per-title hold constant or a measured
argument that `0.8` is close enough, and either is a `oag-race`/gameplay
change outside this lane's file ownership - recorded in this project's
handover tracker under the existing HD lock-on thread.

## The LeachBeam's four are not "all four, always" in the original

This project's own `oag_title::hud::Sights::Concentric::leach` currently
draws all four LeachBeam widgets whenever the reticle is up at all, labelled
chosen-not-measured for want of a reading. `Hud_UpdateLeachBeamSight` shows
that is not what the original does: once a lock is in progress
(`bVar24` true), it walks a chain of float comparisons against per-weapon
distance breakpoints (read off `iVar16 - 0x5d14`, `- 0x5c94`, `- 0x5c98`, and
similar small negative offsets from a table pointer this pass has not named)
and, depending on which threshold the current hold/lock progress clears,
toggles bit `0x4` at `widget + 0x34` - the same hide-this-widget bit idiom
`Hud_BindWidgets` clears and sets on dozens of unrelated widgets throughout
the HUD - individually on the Outer (`+0x594`), Middle (`+0x598`) and Inner
(`+0x59c`) widgets. The BG (`+0x590`) is set visible unconditionally in the
same branch. Read plainly: **the rings appear to fill in one at a time as the
lock progresses, rather than all four snapping on together** - a LOD/reveal
sequence closer in spirit to the PSP brackets closing in than to "on or off".

**Not fully decoded**: which literal breakpoint values gate which ring, and
therefore the exact order and timing of the reveal, needs the distance-
breakpoint table named and read - `iVar16`'s own source (a per-weapon stats
pointer resolved earlier in the function, likely the same shape
`oag_tables::weapons::LeachBeamStats` already decodes from the XML side) is
the next thing to chase. `Hud_UpdateMissileSight` runs the equivalent
distance-based bit-toggle on its own `LockedOnLines`/`LockedOnMiddle` pair
rather than an all-or-nothing pair-add, so the "locked adds two more" reading
[`oag_title::hud::Sights::Concentric::locked`] carries at confidence 70 is
itself worth re-checking against this same mechanism - not done this pass.

This is why `oag_title::hud::Sights::Concentric::leach`'s doc comment still
says chosen, not measured, rather than promoting the widget existence
question (measured, confidence 90) into a claim about the reveal order
(unmeasured past this page).

## Not read this pass

- The distance-breakpoint table(s) both functions index by small negative
  offsets from a resolved weapon-stats-shaped pointer - the next step for
  settling the reveal order above.
- Whether `Hud_UpdateMissileSight`'s own `LockedOnLines`/`LockedOnMiddle`
  toggle is genuinely the same distance-reveal mechanism as the LeachBeam's,
  or a simpler on/off.
- The far-target alpha (96/255 on the PSP) - the colour writes here go through
  `Image_SetVertexColours` with a computed alpha byte, and whether any of that
  arithmetic is the same 96/255 ratio was not checked against the PSP's
  constant.
- Which of the two functions - or a third, unfound one - runs when *no*
  lockable weapon is held at all (both refuse early on `*(int*)(iVar41 or
  iVar22 + 0x5edc) == 0`, a per-weapon-system pointer being null, but nothing
  here traces where that pointer comes from beyond "resolved earlier in the
  same eight-slot player lookup").
