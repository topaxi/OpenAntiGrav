# `Zone_UpdateCraftClass`: per-craft Zone-mode ladder advance

2026-09-15. Full evidence and the negative on the `craftArray[n]->+0x640`
question this closes are in
[sound.md](sound.md#fun_0006c600-named-zone_updatecraftclass-and-its-callers-found)
- this page is the address/purpose/confidence record ADR-0005 requires for a
rename, not a second write-up of the analysis.

## Identity

| | |
| --- | --- |
| Address | `0x0006c600`-`0x0006cbaf` |
| Name | `Zone_UpdateCraftClass` |
| Confidence | 72 |
| Signature (recovered) | `void Zone_UpdateCraftClass(double dt, ZoneState *zoneState, Craft *craft)` |

`FUN_0006c600`'s own OPD entry (found at `0x008729a0` by byte search) reads
`{func=0x0006c600, toc=0x008ad4d8}` - the TOC Ghidra already assumes for every
function, and the function's address (`0x0006c600`) is below the `0x32d5e0`
break [`memory.md`](memory.md) documents, so every `PTR_*` name
`decompile_function` renders for it resolves against the correct TOC without
the three-read check being at risk of the wrong-TOC failure mode - checked
directly anyway, not assumed, per the addendum in `sound.md`.

## What it does

Called once per racer, every tick, from `FUN_0006d958` (which walks the
world's own eight-slot racer-pointer array at `+0xe8`), and once more for the
local player's own craft from `FUN_0003f6d8`. Per craft:

1. Computes `dVar16 = zoneState[cp].distanceAccumulated / zoneState.totalDistance`
   for the craft's current checkpoint (`cp = craft->+0x6284`).
2. On one branch (`targetObj->+0x200 & 0x40000 == 0`), gated by a per-checkpoint
   flag and `dVar16 >= zoneState->+0x334`, fires a scaled event through
   `FUN_000d9cf8` and resets the checkpoint's distance accumulator.
3. On the other branch (bit set), gated by the same threshold, increments a
   `0`-`14` wrapping ladder-rung counter at
   `zoneState + (craft->+0x7a60)*4 + 0x24` - one slot per racer - and fires
   `FUN_00310bd8(bank, subsys, "ZONEADVANCE", 0)` followed by
   `FUN_002ffa58(..., "ZONEBAR_TRANS", 0x400, ...)` (role of the second call
   still not settled to certainty, see `sound.md`'s prior addendum).
4. Unconditionally, springs a per-racer HUD bar value
   (`zoneState + racer*4 + 0x40`/`0x44`/`0x4c`) towards
   `zoneState[cp]+0x2c` and writes the result through a pointer field at
   `zoneState+0x20` into offsets `+0x530`/`+0x53c` of whatever that points at -
   a HUD-facing object, not touched by this page further.

## What it is not

Checked directly: no instruction in either branch, or in the unconditional
tail, writes displacement `0x640` on any base register, by literal
displacement or by an immediate-fed indexed store (`li`/`addi` with
`0x640`). So this function is **not** the still-missing writer of
`craftArray[n]->+0x640` that
[`zone-effectsettings-loader.md`](zone-effectsettings-loader.md#craftarrayn-0x640-still-no-writer-and-the-offset-sweep-is-now-genuinely-exhaustive-rather-than-blind)
tracks, and its own per-racer ladder array is a different object at a
different base than the global `H[e]` (`0x008c2cb8`) that
`Environment_UpdateStageBlend` reads its target stage from - different
address, and indexed by racer slot or checkpoint id rather than by
environment (`0`/`1`).

## Why 72 and not higher

- Decompilation is unambiguous and both real callers were read, not just
  found - `FUN_0006d958`'s own body corroborates the sub-object's field
  layout by exact arithmetic against four independent fields (see `sound.md`),
  which is stronger than "a plausible offset".
- The craft-identifying `+0x7a60` reading agrees with two independently
  written pages (`engine-trail.md`, `zone-effectsettings-loader.md`) that
  reached the same offset from different code paths.
- No runtime trace exists for this function, and `FUN_002ffa58`'s exact role
  is still open, which is what keeps it inside the 70-84 band rather than
  higher.

## Not recorded

- `FUN_002ffa58` and `FUN_00679688` - not decompiled this pass; see `sound.md`'s
  own "Next" for the eight-argument call's role.
- The HUD-facing object `zoneState+0x20` points at, and the meaning of its
  `+0x530`/`+0x53c` fields - plausibly the Zone bar widget itself, not
  confirmed.
- `targetObj`'s own type (`*(param_2+iVar5+0x134)`, checkpoint-indexed) - it
  gets a `+0x21c` write, `+0x200`/`+0x4` flag and pointer writes, but nothing
  here names the struct.
