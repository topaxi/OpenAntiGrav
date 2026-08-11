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

## What is not determined

- **Units, and what any of these numbers multiply.** The parser stores the
  attribute; it does not use it. This needs the consumer, and the consumer is
  not identified.
- **The consumer itself.** Normally reached from the caller of
  `AiStats_LoadAll`, which the unrelocated-call-graph problem above makes
  invisible to `get_function_callers`. The remaining route is a byte search for
  the record base or a live read under PPSSPP.
- **`Data\XML\WeaponAIstats.xml`** (`0x08a7be38`, tag `WeaponAIStats` at
  `0x08a7bda4`) is a sixth AI file that `AiStats_LoadAll` does **not** load, so
  it has a separate loader that has not been looked for. It pairs with the
  `WEAPON AI %d` string and is the weapon-side AI.
- **`SkillScaleValue`** (`0x08a8134c`) appears in the string table and in none of
  the nine functions here. Reads like a computed runtime quantity; unchased.
