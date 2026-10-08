# `AIStats`: the AI tuning parser

**Binary:** `pulse-psp` `BOOT.BIN`, image base `0x08804000`.
**Status:** the parser chain is read end to end and the per-class struct layout
closes exactly. Confidence **88** for the nine functions and the field offsets,
**85** for the `BaseThrust` mismatch below, and the numbers' **units are not
determined** - that needs the consumer, which is not identified.

Everything here is decompilation plus shipped-data agreement. No runtime trace,
so nothing goes above 94 per the
[confidence rubric](../../../reverse-engineering/confidence-rubric.md); the
layout is scored at the top of the 85-94 band because it closes on an exact
arithmetic invariant, and the rubric names that case.

**These nine are read in the USA build only.** `psp-pulse-eu` is the project's
[target of record](../../workflow.md) and the rubric holds that a second binary
is worth more than a second reading, so EU cross-verification is owed and has
not been done. The shipped *data* is corroborated on the PS2 disc; the *code* is
not corroborated anywhere.

The design-level reading of what this data *does*, and what this project builds
instead, is [gameplay/ai.md](../../../gameplay/ai.md). This page is the
evidence.

## Read this first: this region's addresses are unrelocated

**`get_xrefs_to` and `get_function_callers` return nothing for any of the
addresses on this page, and neither does a search for the string addresses in
instruction operands.** That is not evidence of absence. The import carries
address constants in the *instruction bytes* unrebased, so the decompiler prints
`0x276c70` where the string is at `0x08a7ac70`:

```
add 0x08804000 to every constant the decompiler prints
```

[`HANDOVER.md`](../../../../HANDOVER.md) records the same wart for the Pure
imports, attributing it to analyse-then-rebase import order; it is now confirmed
on `psp-pulse-usa` as well. The consequence is that **the call graph does not
resolve in this region** - every call decompiles as `func_0x000318a4` rather than
as a named function - which is why the consumer hunt below is an open thread
rather than a result.

The way in that *does* work: take the string's rebased address, subtract
`0x08804000`, and search instruction operands for the low half.
`Data\XML\AIControlStats.xml` at `0x08a7ac70` becomes `0x276c70`, whose `addiu`
is the single hit that anchors everything on this page.

## The chain

Five XML entries are loaded, one per file, into one object:

| Address | Name | What it does |
| --- | --- | --- |
| `0x08835830` | `AiStats_LoadAll` | Calls the file parser five times, with `Data\XML\AIControlStats.xml` (`0x08a7ac70`) and the four `Data\XML\AIRaceStats_<class>.xml` paths (`0x08a7ac8c`, `ac`, `cc`, `ec`). |
| `0x088358a4` | `AiStats_ParseFile` | Finds the `AIStats` root (`0x08a7ad10`), then maps each child element name to a class index and hands it the class's own record. |
| `0x08835a88` | `AiStats_ParseClass` | Dispatches `RaceBalancing` (`0x08a7ad58`) and `Controller` (`0x08a7ad68`). |
| `0x08835b78` | `AiStats_ParseRaceBalancing` | Dispatches `StartStats`, `RubberBanding`, `PosBalancing`, `SkillScale`. |
| `0x08835ce0` | `AiStats_ParseStartStats` | `GridPlace<N>` rows. |
| `0x08835e50` | `AiStats_ParseRubberBanding` | `WhenLeading` / `WhenBehind` rows. |
| `0x08836084` | `AiStats_ParsePosBalancing` | `PlayerInPos<N>` rows. |
| `0x088361f0` | `AiStats_ParseSkillScale` | `SkillScalePoint<N>` and `AIPackSwapping<N>` rows. |
| `0x08836548` | `AiStats_ParseController` | The six steering constants. |

Attribute matching is exact throughout: every comparison goes through the
`strcmp(a, b) == 0` wrapper at `0x0895372c`, and the indexed rows go through a
`strncmp` against a fixed prefix plus a check that the tag is exactly one
character longer.

## The class index, and the dead `Vector` branch

`AiStats_ParseFile` maps the five class element names to an index:

| Element | Address | Index |
| --- | --- | --- |
| `VectorStats` | `0x08a7ad18` | **none - the comparison's result is discarded** |
| `VenomStats` | `0x08a7ad24` | 0 |
| `FlashStats` | `0x08a7ad30` | 1 |
| `RapierStats` | `0x08a7ad3c` | 2 |
| `PhantomStats` | `0x08a7ad48` | 3 |

The `VectorStats` comparison is made and its return value is **never assigned**,
so the index stays at its initial `-1` and the record pointer computes as
`base + 0x14 - 0x10c`. A shipped file containing a `VectorStats` element would
write one class-record's worth of fields *below* the array.

Nothing on either Pulse disc contains one: `AIControlStats.xml` carries only the
four classes above, and each `AIRaceStats_<class>.xml` carries only its own. So
the branch is unreachable with shipped data, which is consistent with Vector
having been dropped from Pulse's AI data while its name stayed in the string
table. This bears on the open
[fifth handling class](../../../formats/handling-stats.md) question and does not
settle it - it is a fact about the AI parser, not about the ladder.

**A third table shows the same four, checked while chasing that question
(2026-09-02).** The front-end's loading-screen message keys, contiguous at
`0x08a827c0`-`0x08a827f0`, are `MSC_LOAD_VENOM`, `MSC_LOAD_FLASH`,
`MSC_LOAD_RAPIER`, `MSC_LOAD_PHANTOM` - immediately preceded by a `"...Class
Help"` key and followed by `"Event Help"`, with no `MSC_LOAD_VECTOR` in the
run built for exactly this purpose. Confidence 85, same completeness
reasoning as the two class tables above: a missing fifth key would have to
sit somewhere other than the block authored to hold it.

## The per-class record

One record per class, indexed from `base + 0x14`, **stride `0x10c`** - the value
`AiStats_ParseFile` multiplies the class index by. Every offset below is a
32-bit field written straight from the XML attribute:

| Offset | Field | Source |
| --- | --- | --- |
| `+0x00` | `SteerMul` | `Controller` |
| `+0x04` | `SteerDamp` | `Controller` |
| `+0x08` | `LookAheadSecs` | `Controller` |
| `+0x0c` | `xtrackMul` | `Controller` |
| `+0x10` | `xtrackMax` | `Controller` |
| `+0x14` | `xtrackDamp` | `Controller` |
| `+0x18` | - | **unwritten by any of the nine functions**, 4 words |
| `+0x28` | `BaseThrust[8]` | `StartStats`, `GridPlace<N>` - **see below** |
| `+0x48` | `StartBoost[8]` | `StartStats`, `GridPlace<N>` |
| `+0x68` | `AIThrust[8]` | `PosBalancing`, `PlayerInPos<N>` |
| `+0x88` | `SpreadDist[8]` | `PosBalancing`, `PlayerInPos<N>` |
| `+0xa8` | `WhenLeading.SpeedMul` | `RubberBanding` |
| `+0xac` | `WhenLeading.MinSpeedChangeDist` | `RubberBanding` |
| `+0xb0` | `WhenLeading.MaxSpeedChange` | `RubberBanding` |
| `+0xb4` | `WhenBehind.SpeedMul` | `RubberBanding` |
| `+0xb8` | `WhenBehind.MinSpeedChangeDist` | `RubberBanding` |
| `+0xbc` | `WhenBehind.MaxSpeedChange` | `RubberBanding` |
| `+0xc0` | - | **unwritten by any of the nine functions**, 1 word |
| `+0xc4` | `ThrustOffset` | `SkillScale`, stride `0x18` |
| `+0xc8` | `ThrustMultiplier` | `SkillScale`, stride `0x18` |
| `+0xcc` | `SpreadMultiplier` | `SkillScale`, stride `0x18` |
| `+0xd0` | `numAIPositionSwaps` | `SkillScale`, stride `0x18` |
| `+0xd4` | `numIntermediatePositions` | `SkillScale`, stride `0x18` |
| `+0xd8` | `intermediateOffsetSize` | `SkillScale`, stride `0x18` |

**The layout closes exactly, and that is the evidence for it.** The four
eight-entry arrays are contiguous and abutting (`0x28`, `0x48`, `0x68`, `0x88`,
each `0x20` long), the six rubber-banding scalars follow immediately at `0xa8`,
and the difficulty block runs `0xc4 + i * 0x18` for `i` in `0..2`, ending at
`0xc4 + 3 * 0x18 = 0x10c` - **the stride the file parser independently
multiplies by.** Two numbers derived from different functions meeting on the
nose is what puts this at 88 rather than at a reading.

**Two holes**, neither written by any of the nine: `+0x18`'s four words, and the
single word at `+0xc0` between the rubber-banding block and the first difficulty
record. For the first, speculative at confidence **35** and not acted on: Pure
authors `GridSpread` and `ShipSpacingVariation` in the same position in its own
file and Pulse's parser has no branch for either, so the gap may be their
vestige. `+0xc0` has no hypothesis; alignment is as likely as anything.

### `SkillScalePoint<N>` and `AIPackSwapping<N>` are one record

Both families share the `0x18` stride and the same index, so difficulty level
`N`'s thrust offset and its pack-swapping counts are **six fields of one
struct**, split across two XML elements for authoring convenience. That is worth
knowing before treating "pack swapping" as a separate system: it is part of the
difficulty setting, not an independent behaviour.

## `BaseStartThrust` is authored and never read

`AiStats_ParseStartStats` compares each `GridPlace<N>` attribute against exactly
two names:

| Address | String the parser looks for | What the shipped files author |
| --- | --- | --- |
| `0x08a7adb8` | `BaseThrust` | `BaseStartThrust` |
| `0x08a7adc4` | `StartBoost` | `StartBoost` |

So **`BaseStartThrust` matches nothing and `+0x28` is never written from shipped
data.** Only `StartBoost` actually varies per grid slot.

Confidence **90**, resting on three legs, none of which is the comparison
function's identity:

1. **`BaseStartThrust` does not exist as a literal anywhere in the binary.** A
   string search returns `BaseThrust` at `0x08a7adb8` and nothing else, so there
   is no second reader elsewhere that could pick the attribute up.
2. **The two names diverge at index 4** (`BaseS` against `BaseT`), so exact,
   prefix and case-insensitive matching all fail alike. The conclusion does not
   depend on which of those `0x0895372c` implements - it wraps
   `func_0x0016f2d8`, whose `== 0` return convention reads as `strcmp`, but that
   identification is an inference and is not needed here.
3. **The parser's names and the shipped names agree character for character
   everywhere else** - all six `Controller` names, both `PosBalancing` names,
   all three `RubberBanding` names on both rows, and all six `SkillScale` names.
   One mismatch against fourteen matches reads as a rename that landed on one
   side only.

What would take it higher is a runtime leg, and there are two: read `+0x28`
after load under PPSSPP, or author a file using `BaseThrust` and watch the
launch change. Neither has been done.

**A second binary agrees (2026-10-08).** HD's EBOOT carries the same parser
with the same mismatch: `BaseThrust` is a string there and `BaseStartThrust` is
not, its attribute compare is `strcasecmp` (which still cannot match a different
letter), and HD ships Pulse's own files. HD also confirms the layout above: its
record is this one plus three `SplitScreenMultiplier` floats at `+0xc4`, stride
`0x118`. See [ps3-hdfury-eu/ai-stats.md](../ps3-hdfury-eu/ai-stats.md).

## `StartBoost[8]` has a consumer: the AI's grade 3

Added 2026-10-01 (`pulse-launch-boost`). `Ship_UpdateStartBoost` (`0x0883fdec`) case 3 -
`craft+0x294 = boostMul * *(float *)(g_race_manager + DAT_08b31040 * 0x10c +
player+0x914 * 4 + 800)` - reads a per-grid-slot table out of the race manager, and
`Race_UpdateLaunchGrade` (`0x0882773c`) writes grade 3 to every craft with
`player+0x368 != 0` on every frame of the perfect window. That table is this block's
`StartBoost[8]` on the strongest reading (the same eight-per-block shape, indexed by
slot) - **read from the decompile, not watched, confidence 75**; the copy from the
block into the manager was not followed. So the "staggered launch" this page calls
the point of `StartBoost` is the original's AI getting `boostMul` times a slot figure
for the rest of the launch window without a thrust edge, which `oag-ai` does not copy:
see [the launch boost](../../../physics/launch-boost.md).

## What is not determined

- **Units, and what most of these numbers multiply.** The parser stores the
  attribute; only the `SkillScale` block's own consumer is found (below) -
  `AIThrust`/`SpreadDist`/the `RubberBanding` six and `StartStats` still have
  none identified.
- ~~**The consumer itself.** Normally reached from the caller of
  `AiStats_LoadAll`, which the unrelocated-call-graph problem above makes
  invisible to `get_function_callers`.~~ **Found 2026-09-28 - see [the section
  below](#the-skillscale-consumer-ai_computeopponentthrust).** Reached from
  the *other* direction: a live caller of `AI_ResolveSkillScale`
  (`race-campaign.md`), not a caller of the loader.
- **`Data\XML\WeaponAIstats.xml`** (`0x08a7be38`, tag `WeaponAIStats` at
  `0x08a7bda4`) is a sixth AI file that `AiStats_LoadAll` does **not** load.
  ~~It has a separate loader that has not been looked for.~~ **Found 2026-08-17 -
  see [the section below](#the-sixth-ai-file-weaponaistatsxml).** It pairs with
  the `WEAPON AI %d` string and is the weapon-side AI.
- ~~**`SkillScaleValue`** (`0x08a8134c`) appears in the string table and in none
  of the nine functions here. Reads like a computed runtime quantity;
  unchased.~~ **Found 2026-09-28**, off `FEData.wad`'s own `stats.xml` rather
  than off this page's nine functions - see
  [`race-campaign.md`](race-campaign.md#where-fedatawads-per-track-records-are-actually-read)
  and [the section below](#the-skillscale-consumer-ai_computeopponentthrust).


## The `SkillScale` consumer: `AI_ComputeOpponentThrust`

**Found 2026-09-28**, while wiring `oag-ai` to the campaign's own difficulty
rung (`crates/tables/src/track_stats.rs`). `AI_ResolveSkillScale`
(`0x08834df4`, `race-campaign.md`) is called from exactly one site,
`AI_ComputeOpponentThrust` (`0x08855904`, confidence 82 - clean decompile,
cross-checked below), the per-tick AI thrust update for one opponent slot.

```text
fVar13 = AI_ResolveSkillScale()                 # 1.0..3.0-ish, campaign or ambient
if fVar13 < 2.0:
    (offset, mult) = (class->SkillScalePoint1, class->SkillScalePoint2)  # +0xc4, +0xdc
else:
    (offset, mult) = (class->SkillScalePoint2, class->SkillScalePoint3)  # +0xdc, +0xf4
(thrustOffset, thrustMul, spreadMul) =
    AI_InterpolateThrustPoint(offset[i], mult[i], fVar13 - floor) for i in 0..3

spread = spreadMul * class->SpreadDist[grid_position]     # +0x88, PosBalancing - player-coupled
...pack-position-matching loop over every ship's own rank...
rubberband = leading/behind speed adjustment against DAT_08b34410  # the PLAYER's own ship,
                                                                    # RubberBanding - player-coupled
thrust = thrustOffset + (posBalancingThrust + rubberband - base) * thrustMul + base
```

Three independent offset checks against `ai-stats.md`'s own already-scored-88
struct layout close on this function, which is the evidence for the address:
the `SkillScalePoint1/2/3` stride (`+0xc4`, `+0xdc`, `+0xf4` - exactly
`0x18` apart, the stride `AiStats_ParseFile` itself multiplies by), the
`AIThrust`/`SpreadDist` array reads at `+0x68`/`+0x88`, and all six
`RubberBanding` fields (`+0xa8`..`+0xbc`) in the exact `WhenLeading`/
`WhenBehind` shape `ai-stats.md`'s own table already has.

**Why this project still does not port it, campaign wiring included**: the
`SkillScale` triple is not read alone here - it is one input added inside the
identical per-tick computation as `PosBalancing`'s `SpreadDist` lookup (keyed
on the whole field's grid rank) and `RubberBanding`'s leading/behind term
(keyed on the gap to `DAT_08b34410`, confirmed elsewhere on this page to be
the player's own snapshot slot). `docs/gameplay/ai.md`'s own table already
classifies `PosBalancing`/`RubberBanding` as player-coupled and refuses to
port either - finding their consumer does not change that refusal, it only
confirms `SkillScale`'s raw number cannot be extracted from this function
without carrying the player-coupled terms along with it. What this project
*does* carry is the faithful half one level up: `AI_ResolveSkillScale`'s own
campaign-cell branch resolves a position on the track's `SkillScaleValue`
curve, and `oag_ai::Difficulty::tune_at_scale` is this project's own
(**chosen, not measured**) reading of what a position on that curve should
mean for its own four-axis AI - see `crates/tables/src/track_stats.rs` and
`crates/ai/src/difficulty.rs`.

**It also flies the finished player** (2026-10-04, measured): once the racer record's `+0x82` (`finished`)
is set, the player's own driver runs this law with spread `50` and no rubber band, and the `56.7` the
post-finish captures read is `ThrustOffset + 0.7 * AIThrust[place]` at skill `0.9`. Read live off the
running race, the per-class block holds exactly the shipped `AIThrust` and `SkillScale` rows, the first
runtime leg this page's layout has. See
[race-finish.md](race-finish.md#the-finished-players-thrust-ai_computeopponentthrust-with-the-players-own-rank-2026-10-04).

`AI_InterpolateThrustPoint` (`0x08852d14`) and `AI_InterpolateSkillScale`
(`0x088347b8`, `AI_ResolveSkillScale`'s own helper) are the same one-line
lerp (`a*(1-t) + t*b`), confidence 95 each - unambiguous decompile, no second
reading possible.

## The sixth AI file: `WeaponAIstats.xml`

**Found 2026-08-17**, and the reason nobody had found it is worth recording:
nothing was hiding it. `AiStats_LoadAll` (`0x08835830`) genuinely does not load
it, which this page recorded correctly - but its *caller* does, on the very next
call.

| Address | Name | Confidence |
| --- | --- | --- |
| `0x08851d88` | `WeaponAiStats_Load` | 90 |
| `0x08852178` | `WeaponAiStats_ParseWeapon` | 90 |
| `0x08851d3c` | `WeaponAiStats_Reset` | 90 |

`FUN_08829124` is the race-setup function, and it calls the two back to back:

```text
088293fc: jal 0x08835830     ; AiStats_LoadAll  - the five files this page covers
08829404: jal 0x08851d88     ; WeaponAiStats_Load - the sixth
```

So "it has its own loader" was right, and the loader is a sibling eight bytes
later. **The lesson is the cheap one**: when a page says a file has no loader,
read the caller of the loader it *does* have before concluding the loader is
hidden.

### The schema is three floats a weapon

`WeaponAiStats_Load` opens `Data\XML\WeaponAIstats.xml` (`0x08a7be38`), matches
the root `WeaponAIStats` (`0x08a7be28`), and dispatches each child element to
`WeaponAiStats_ParseWeapon(record, element, weapon_id)`. That function is short
enough to state whole:

```c
record += weapon_id * 0xc;
while (Xml_NextAttribute(element, attr)) {
    if      (name == "useAgainstPlayer") record[0x08] = as_float(attr);
    else if (name == "useAgainstAI")     record[0x0c] = as_float(attr);
    else if (name == "absorb")           record[0x10] = as_float(attr);
}
```

Three attributes, three floats, `0xc` bytes a weapon, **indexed by weapon id**.
Confidence 90 - the stores are unambiguous and the strings are contiguous.

There is also a `Template` element (`0x08a7bed0`) which the loader matches and
then **discards** - no parse call. The shipped file authors no such element, so it
is an authoring convention rather than data.

### It confirms the weapon-id space, from a second direction

The element-to-id mapping falls straight out of the dispatch chain, and it is an
**independent confirmation** of the ids
[missile.md](missile.md#weapon_requestfire-and-the-weapon-id-to-bit-map) anchored
from sound cues and lock distances - plus the five it left as bare numbers:

| id | element | | id | element | | id | element |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 0 | `Rockets` | | 5 | `Cannon` | | 10 | `Leachbeam` |
| 1 | `Missiles` | | 6 | `Autopilot` | | 11 | `Repulser` |
| 2 | `Quake` | | 7 | `Plasma` | | 12 | `Shuriken` |
| 3 | `Turbo` | | 8 | `Bomb` | | | |
| 4 | `Shield` | | 9 | `Mines` | | | |

`Bomb` at id 8 is read out of the image directly (`0x08a7be64` is
`42 6f 6d 62 00`), because Ghidra had not typed it as a string.

**This disagrees with `oag_tables::weapons::Weapon::ALL` at three positions**,
and the enum is the one that is wrong about ids: it has `Cannon, Turbo, Shield`
at 3/4/5 where the id space has `Turbo, Shield, Cannon`. The enum's order is the
*string-pool layout* order at `0x08a78c00`, which is a different thing and is
what its own doc comment says it is - and that comment already warned the mapping
to ids was unconfirmed. It now is confirmed, and it differs. Nothing in this
project reads `Weapon::ALL`'s index as a weapon id, so this is a documentation
correction rather than a bug; `oag_gameplay::hash::write_weapon` uses the index
only as a hash discriminant, where any stable order does.

### One thing this does *not* settle, and a conflict it opens - **resolved 2026-08-26**

**Read [mine.md](mine.md) for the resolution.** The short version, because the
paragraphs below are kept as the record of how the conflict was posed:
`FUN_08871ddc` has been read - it is `Weapon_AnnounceIncoming` (`0x08871ddc`) -
and it *is* the remap the last paragraph here suspected. **The id space this
section's table describes is the shipped file's element order, not
`craft+0x1bc`.** The tell is Turbo and Shield: this loader's own literals put
them at 3 and 4, while [shield-pickup.md](shield-pickup.md) measured
`craft+0x1bc` reaching the Turbo at 4 and the Shield at 5. So the correction two
paragraphs up - "`Weapon::ALL` is the one that is wrong about ids" - is itself
wrong about *which* ids: `Weapon::ALL`'s pool order is right about the weapon id
at eleven of thirteen positions, and this file's order is a fourth thing that
`WeaponAiStats_Load` and the announcement table both index by.

And the conflict it opened is settled the other way from the guess below: **bit
`0x2` is the Mine's, not the Bomb's and not the Cannon's.** Its spawn plays
`MINELAUNCH` and loads `Data\Weapons\Pulse_Mine.vex`, which needs no ordering
argument at all.

The original posing, kept:

Combining the id map above with `Weapon_RequestFire`'s id-to-bit switch puts
**fire-request bit `0x2` on the Bomb**. That bit's handler is
`Weapon_DropMines` (`0x088675cc`), which
[weapon-fire.md](weapon-fire.md#the-other-multi-shot-weapon-a-staggered-burst-and-it-is-the-mine)
read as the **Cannon** at confidence 72 on the grounds that `rounds` and `rate`
are the Cannon's `<Stats>` and nobody else's. A Bomb that fires thirty rounds at
a tenth of a second apart makes no sense.

Three loose ends said the id-to-bit switch was not as simple as it looks: bit
`0x2000` (case 3) is dispatched by *nothing* in `Weapons_DispatchFire`; three of
the cases make no call to `FUN_08871ddc` where the rest do; and that call's
argument is a **second** id space which remaps 3 to 5, 8 to 9 and 9 to 8. Two of
the three are now explained - see [mine.md](mine.md). The first is not: nothing
dispatches bit `0x2000`, and where the Cannon's fire actually happens is still
unread.

### The shipped values are nearly uniform, and that is the finding

Read off `pulse-psp-usa.chd`. Every weapon authors `absorb="1.0"`; every weapon
except `Plasma` and `Quake` authors `useAgainstAI="1.2" useAgainstPlayer="1.1"`,
and those two author `1.0`/`1.0`.

**That nearly-uniform table led to a wrong conclusion here, twice.** Because no
weapon is marked absorb-only or fire-only, this page previously argued the file
could not be the fire-or-absorb decision. It is. The consumer compares these
values against a normalised random draw, so `1.1` versus `1.2` is a *probability*
and near-uniform weights are exactly what a designer would author for "usually
fire, slightly more readily at another AI". See the retraction below.

### RETRACTED: it *is* read, about 400 times a second

**This section previously said the record was parsed and never read, at
confidence 92. That was wrong, and it was committed and pushed before it was
checked properly.** Corrected 2026-08-17 by a runtime measurement in Eliminator
and in Single Race.

The reader is `WeaponAi_DecideFireOrAbsorb` (`0x088518b4`), called by
`WeaponAi_Update` (`0x08851550`) - both unnamed and undocumented until this pass,
which is why three static sweeps and a watchpoint run all missed it. **The whole
decision is on [weapon-ai.md](weapon-ai.md)**; what follows is only the part that
concerns this record. Read out of memory at a halted watchpoint and
confirmed against Ghidra:

```text
08851ae8: sll   a3, a3, 2        ; a3 = weapon_id * 12
08851aec: addu  a0, a0, a3
08851af0: addiu a0, a0, 8        ; a0 = record + id*0xc + 8
08851af4: lwc1  f15, 8(a0)       ; record[id].absorb           (+0x10)
08851af8: beq   a1, zero, +3
08851b04: lwc1  f16, 0(a0)       ; record[id].useAgainstPlayer (+0x08)
08851b08: lwc1  f16, 4(a0)       ; record[id].useAgainstAI     (+0x0c)
```

**And it is the fire-or-absorb decision after all.** It compares those floats
against `FUN_089731c4() * 4.656613e-10` - a normalised random draw - and writes
two request bytes at `*(param_1 + 0x10) + 0x15` and `+0x17`. So this file *is*
what decides whether an opponent fires or absorbs, which the previous text also
denied.

### Why every sweep missed it, and the lesson

**The reader computes the field address into a register first**: `record +
id*0xc + 8` lands in `a0`, and the loads are `0(a0)`, `4(a0)`, `8(a0)`. Every
static sweep in this file's history keyed on the operand text `0x7...`, so none
of them could ever have seen it.

**That exact hole was written down as residual gap 1 and gap 4 of the previous
verdict, and then the conclusion was reported at 92 anyway.** Naming a gap is not
closing it. If a sweep's method cannot see a whole class of access, the
confidence has to be bounded by that, not by how many variations of the same
blind method were run.

### The measurement

The record is located from the singleton rather than by scanning:
`table = *(0x08b317b4) + 0x710`. Fingerprint verified before every arming, and
still intact after each, so no run measured a recycled heap block.

| Mode | Player | Armed | Table hits | Control hits |
| --- | --- | --- | --- | --- |
| Eliminator (`DAT_08b31048 = 8`) | driving | 44 s | **1,130** | 272,443 |
| Eliminator | driving | 45 s | **5,742** | 346,359 |
| Single Race (`= 3`) | driving | 52 s | **21,170** | 458,669 |

Every non-zero reproduced. Two arithmetic identities corroborate a single code
path: per weapon row `absorb = useAgainstPlayer + useAgainstAI` (Single Race
Shield: 687 + 5,989 = 6,676 exactly; 14 of 15 rows exact, one off by one at a
sampling-window edge), and each caught PC maps to exactly one field offset in
every row. Only weapons actually in play appear - six rows in Eliminator, nine in
Single Race.

### The earlier zero was a false negative, and the mode was not the cause

The previous run's watch address (`0x09881900`) and control craft
(`0x09b72610`) are byte-identical to the Single Race run above, so a wrong
address is ruled out. What differed is that **the player was parked**. The
reader gates on a signed along-track range test (`ahead < 150`, `behind > -200`)
and on a random draw, and a stationary player never satisfies it. A second,
undiscriminated candidate is that the earlier race had `WEAPONS Off`.

So the lesson is not "test more modes" - it is that **a scenario where the thing
under test cannot happen produces a zero that looks like evidence.** The positive
control proved the *instrument* worked and said nothing about whether the
*scenario* exercised the code.

### The anomaly is explained, and it was the instrument

The unreproduced 9,706 hits recorded here previously are almost certainly the
`log=True` trap: a watchpoint with logging on writes a line per hit, a 256-byte
control fires about 6,000 times a second, and the log grew to roughly 6 GB,
**filled `/tmp` and wedged the emulator** - after which `memory.read` times out
and counters freeze. That also explains one run's counter stalling at 8,944.

**But the same log is the prize.** Its lines read
`CHK Read32(CPU) at 09881944 ((09881944)), PC=08851af4` - address *and* program
counter, at full emulation speed, with no halting. That is strictly more than the
`hits` counter, and it is how the reader was finally caught. See
[ppsspp-debugger.md](../../../reverse-engineering/ppsspp-debugger.md#memory-watchpoints-and-the-control-that-makes-a-zero-mean-something).

### What is open now

**All three of the questions this section used to list are answered on
[weapon-ai.md](weapon-ai.md)**: both functions are named and landed in
`names.tsv`, the object carrying the record at `+0x18` and the weapon id at
`+0x20` is the per-craft weapon-AI context, and the `useAgainstPlayer` conflict
resolves in the attribute names' favour at confidence 75 - the range test is
against one particular craft's along-track gap, and reading it as the player's
makes the names correct.

What is left that touches *this* record:

- **What writes the gap at `self+0x60`.** It is the one step of the naming
  argument that is inference rather than reading, and if it turns out to be the
  nearest craft of any kind rather than the player, the shipped attribute names
  are misleading.
- **Weapons-off modes are untested** and should read nothing; Tournament and the
  three other race types were skipped on the grounds that two weapons-on modes
  both read, so a third cannot change the verdict.
- **A clean race-that-ends measurement.** One run crossed into `Race End Photo`
  armed, but the emulator was stalled by the log at the time, so that leg is
  void rather than negative.

**What this actually establishes.** `ai.md` and `crates/raceplay/src/field.rs`
recorded this file as where the original decides what an opponent does with a
pickup, and **they were right** - `FUN_088518b4` reads it every frame and writes
the fire request. So the weapon-AI hook is located, and what is left is naming the
two functions and reading the decision's other inputs, not hunting for the
mechanism. The `WEAPON AI %d` string is no longer the better lead.
