# Pure's `ExhaustFlare_Init` opens the same `~ENGINE` voice Pulse's does

**Binary:** `pure-psp` `BOOT.BIN`, image base `0x08804000`.
**Status:** decompilation only, no PPSSPP leg for either title.

Answers `oag_game::audio::sfx::Cue::Engine`. Structurally different from the
other `Cue` variants this project has chased: `Engine` is not a one-shot
trigger fired on an edge, it is a **voice opened once and updated every
tick** - `sfx.rs`'s own doc comment says so ("held for as long as the craft
is running") and points at two Pulse functions, not one:
`Exhaust_UpdateEngineSound` (`0x08904cf4`, writes pitch/volume per tick) and
the constructor that actually opens the voice, documented on
[`psp-pulse-usa/exhaust.md`](../psp-pulse-usa/exhaust.md) as
`ExhaustFlare_Init` (`0x08905308`, confidence 82). The **open** is the edge
this thread cares about; the per-tick write is a separate, ongoing behaviour
this page does not chase.

## Pulse's `ExhaustFlare_Init` (`0x08905308`)

Per `exhaust.md`: the constructor calls `Sound_Play(1.0, self+0x78, ...,
"~ENGINE", /* out */ self+0x7c)`, keeping the handle at `self+0x7c` so
`Exhaust_UpdateEngineSound` can write into it every tick. It also seeds
`self+0x80` (`base`, the pitch floor) to `rand(-127.0, 127.0) - 1143.0`, a
per-instance random spread around roughly **-1143**.

## Pure's `FUN_0886b924` (`0x0886b924`), found by searching for `~ENGINE`'s string address

`search_strings("~ENGINE")` lands on `0x08a49108`; `search_instructions`
for a direct `lui`/`addiu` pair building the corresponding offset
(`0x245108`) finds exactly one hit, `FUN_0886b924` at `0x0886b924`
(`0886bb48: _addiu a3,a3,0x5108`) - the same neighbourhood as the dry-play
chain's callers found for `SpeedupPad`/`Disengaging`
([`dry-play-cues.md`](dry-play-cues.md)), a different function family from
those (this one is not a `func_0x001209e0` caller).

**Decompiled in full, and two literal constants settle it beyond the call
shape alone:**

```c
func_0x0002dddc(0x3f800000, *(undefined4 *)(param_1 + 0x158), _DAT_00288f1c, 0,
                0x245108 /* "~ENGINE" */, param_1 + 0x15c);
fVar7 = (float)func_0x0002b70c(0xc2fe0000, 0x42fe0000);  // rand(-127.0, 127.0)
*(float *)(param_1 + 0x160) = fVar7 + -1143.0;
*(float *)(param_1 + 0x198) = fVar7 + -1143.0;
*(undefined4 *)(param_1 + 0x194) = 0x3c23d70a;  // 0.01
```

`func_0x0002dddc` is `Sound_Play` (`0x08831ddc`, renamed in
`rocket-and-collision-fx.md`), called with the same six-argument
handle-out-slot shape every looping cue on this page uses. `0x245108 +
0x08804000 = 0x08a49108` - the same address `search_strings` already
confirmed independently, not solely the arithmetic - reads `~ENGINE`,
matching `sfx.rs`'s literal and the disc's own `~ENGINE` cue (present in
three separate banks, `just wad sounds`).

**`rand(-127.0, 127.0) - 1143.0` and `0.01` (`0x3c23d70a`) are not just
structurally similar to Pulse's constants - they are the identical
literals**, `0xc2fe0000`/`0x42fe0000` being `-127.0`/`127.0` in IEEE-754 and
`0x3c23d70a` being exactly the lerp rate `exhaust.md` documents for
`Exhaust_UpdateEngineSound`'s own pitch smoothing. Two independently
plausible-but-arbitrary tuning constants matching bit-for-bit across a
two-year-older build is strong evidence this is genuinely the same
authored value carried forward, not a coincidental read.

**Renamed `ExhaustFlare_Init`, matching Pulse's own name for its
counterpart** (`FUN_0886b924` -> `ExhaustFlare_Init`).

## Confidence

**82**: decompilation only (no runtime leg, no verified caller - blocked on
the `jal` wart), but corroborated by an independently-recovered Pulse
function of matching shape *and* two exactly-matching literal constants,
not merely a structural echo - the same evidence class `Sound_Play`'s own
rename earned, and stronger on the constants than any other cue this thread
has found so far.

## What is not verified

- **`ExhaustFlare_Init`'s own caller**, blocked on the `jal` wart.
- **Whether Pure's `Exhaust_UpdateEngineSound` counterpart writes the same
  per-tick pitch/volume behaviour** - this page confirms only the voice's
  *opening*, not the ongoing per-tick write `exhaust.md` documents in detail
  for Pulse. A separate pass would be needed to find and read it.
- **Runtime verification.** No PPSSPP leg for either binary.

## History

- **2026-09-04.** Written answering `every-sfx-trigger-is-a-pulse-reading-applied.md`'s
  `Engine` cue for Pure.
