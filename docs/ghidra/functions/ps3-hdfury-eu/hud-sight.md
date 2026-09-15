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

## The LeachBeam's four reveal one at a time, gated by hold-time not distance

**2026-09-15, corrects this page's own earlier reading.** This section
previously read the four widgets' bit-`0x4` toggles as gated by "distance
breakpoints" off a resolved per-weapon stats pointer. Disassembling the
exact instructions that produce `iVar16 - 0x5d14` etc. shows they are
`lfs fN, -0x5d14(r2)` - **TOC-relative loads, not pointer-plus-offset reads
off a runtime struct**. `iVar16`/`iVar17` in the decompile is Ghidra's
label for whatever currently occupies the stack slot this function's own
TOC-preservation dance (`puVar10[5] = uVar14; ... = puVar10[5];`, needed
because callee TOCs differ from the caller's in this binary) has parked
there at that point - not a weapon-stats pointer at all. `0x00090d30` is
below `0x32d5e0`, so per [memory.md](memory.md) Ghidra's own TOC
(`0x008ad4d8`) is the right one here, confirmed directly:
`scripts/ps3-toc.py toc 0x00090d30` prints `exact`.

Resolving the three loads through that TOC and reading the slots
(`scripts/ps3-toc.py resolve 0x00090d30 -0x5c98` etc., cross-checked against
a direct `read_memory` at each computed address) gives three **literal,
static** IEEE-754 floats:

| Load site | Slot | Value |
| --- | --- | --- |
| `lfs f0,-0x5c98(r2)` @ `0x00091460` | `0x008a7840` | `0x3e000000` = **0.125** |
| `lfs f0,-0x5d14(r2)` @ `0x0009147c` | `0x008a77c4` | `0x3e800000` = **0.25** |
| `lfs f0,-0x5c94(r2)` @ `0x00091a9c` | `0x008a7844` | `0x3ec00000` = **0.375** |

**All three are exact quarters of `DAT_008a764c` (`0.5`), the same hold
constant this page's previous section already read.** The value the three
are compared against, `fVar2 = *(float *)(param_2 + 0x604)`, is the same
field the top-of-function lock check reads and increments by `param_1`
(the tick's delta seconds) before comparing it to `DAT_008a764c` - so this
is **the hold-time accumulator itself, in seconds**, not a screen or world
distance. The reveal is gated by *how far into the 0.5 s hold the lock
currently is*, at each quarter.

The bit-`0x4` direction reads unambiguously from the function's own
no-target early return: `*(uint*)(BG+0x34) &= ~4` is the last thing it does
before returning "nothing to show" - so **clearing bit `0x4` hides a widget
and setting it shows one**, the opposite of neither being obviously right
from the toggle sites alone. Restated as a table, `t` being the hold-time
accumulator in seconds and BG (`+0x590`) shown throughout once a target is
being tracked:

| `t` | Outer (`+0x594`) | Middle (`+0x598`) | Inner (`+0x59c`) |
| --- | --- | --- | --- |
| `t <= 0.125` (0-25% of hold) | shown | hidden | hidden |
| `0.125 < t <= 0.25` (25-50%) | hidden | shown | hidden |
| `0.25 < t <= 0.375` (50-75%) | hidden | hidden | shown |
| `t > 0.375` (75-100%) | hidden | hidden | hidden |

So the four rings do not snap on together and do not stay filled once
revealed: each of the three outer rings gets **one exclusive quarter** of
the hold window, and the last quarter (`t > 0.375`) shows only the BG
before the lock completes at `t > 0.5`. This is a LOD/reveal sequence in
the same spirit as the PSP's brackets closing in, but built from time
fractions rather than a closing bracket's own extent.

**Confidence 88** on the three values and the table above: the constants
are direct memory reads at TOC-exact-resolved addresses, corroborated by
`scripts/ps3-toc.py` independently of the MCP bridge, and the clean
quarter-of-`0.5` values are exactly the kind of authored-not-coincidental
number a hand-tuned reveal schedule would carry. What is not independently
confirmed: the outer state machine that decides *when* this whole block
runs versus the function's several other branches (still the per-frame
"is a target currently being tracked" gating this page's earlier section
already flagged as not fully decoded) - so the table above is right for the
frames it runs on, and exactly which frames those are is not proven past
"while the hold accumulator is live and below the completion threshold."

**Not adopted here either, for the same reason `HOLD_SECONDS` is not**:
`oag_race::sight::Sight` has no accessor for the hold-time accumulator or a
0-1 progress fraction derived from it - only `locked()` and `visible()`,
both hard booleans - so `oag_game::hud::sight_draw` cannot drive this table
without a new `crates/race` API. `oag_title::hud::Sights::Concentric::leach`
still draws all four together in code and its own doc comment still says
chosen, not measured, for the *runtime* behaviour; what moved is that the
reveal law itself is no longer unread. See this lane's report for the
concrete ask.

### The Missile shares one of the three constants, but was not fully re-derived

`Hud_UpdateMissileSight` (`0x0008a5b8`) reads `-0x5d14(r2)` too, at
`0x0008b234` - the **same TOC slot**, `0x008a77c4` = `0.25`. It is compared
against a value accumulated in that function's own `param_2+0xec` field at
its own per-frame rate, not against `param_2+0x604`, so the same `0.25`
constant is corroborated as a shared "halfway" reveal threshold without this
pass having proven the Missile's own accumulator is unit-for-unit the same
"seconds into the hold" quantity the LeachBeam's is. Structurally the
Missile's transition is a **single** step, not three: crossing this
threshold hides `MissileSightMiddle` (`+0x584`) once and sets
`LockedOnLines`/`LockedOnMiddle` (`+0x588`/`+0x58c`) visible, guarded by a
"have I already done this" check on `MissileSightMiddle`'s own hide bit
rather than being re-evaluated every frame the way the LeachBeam's three-way
table is. That is a real, structural answer to this page's own open question
about whether [`oag_title::hud::Sights::Concentric::locked`]'s "locked adds
two more" reading (confidence 70) is an all-or-nothing pair-add or a staged
reveal like the LeachBeam's: **it is closer to all-or-nothing than to a
three-stage reveal**, gated by one threshold rather than three. Confidence
**60** - the toggle statements and the shared constant are read directly,
but the full surrounding state machine in this larger, more heavily
vectorised function was not traced to the same depth as the LeachBeam's.

## Not read this pass

- The full outer state machine gating when `Hud_UpdateLeachBeamSight`'s
  widget-toggle block runs at all, across the function's several branches -
  see above.
- `Hud_UpdateMissileSight`'s own accumulator (`param_2+0xec`) and whether it
  is genuinely hold-seconds like the LeachBeam's, or a distance-derived
  ramp that happens to cross the same `0.25` constant.
- The far-target alpha (96/255 on the PSP) - the colour writes here go through
  `Image_SetVertexColours` with a computed alpha byte, and whether any of that
  arithmetic is the same 96/255 ratio was not checked against the PSP's
  constant.
- Which of the two functions - or a third, unfound one - runs when *no*
  lockable weapon is held at all (both refuse early on `*(int*)(iVar41 or
  iVar22 + 0x5edc) == 0`, a per-weapon-system pointer being null, but nothing
  here traces where that pointer comes from beyond "resolved earlier in the
  same eight-slot player lookup").
