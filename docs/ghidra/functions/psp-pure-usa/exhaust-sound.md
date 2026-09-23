# Pure's `ExhaustFlare_Init` opens the same `~ENGINE` voice Pulse's does, and its `Exhaust_Update` grows the flare on boost too

**Binary:** `pure-psp` `BOOT.BIN`, image base `0x08804000`.
**Status:** decompilation only, no PPSSPP leg for either title.

**2026-09-23 update: the visual half is now read too, settling whether Pure's
boost is visually inert.** Pure's own `Exhaust_Update` (the sibling this
page's "What is not verified" section flagged as unread) carries the identical
`half_size = ((i * 0.6 + 0.4) * 2.5 + boost_timer * 8.0)` term Pulse's own
`exhaust.md` documents, bit-for-bit, on both pressings. So the always-on
`Engine Flare` billboard **does** grow while boosting on Pure, exactly as it
does on Pulse - it is only the separate `<Team>boost.vex` plume mesh that
Pure never had (`ship-models.md`). See "The visual half" section below.

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

## The visual half: `Exhaust_UpdateEngineSound` and `Exhaust_Update`, both found (2026-09-23)

Answers this page's own "not verified" bullet below, and the open rendering
question of whether Pure's always-on `Engine Flare` billboard does anything
visible when boosting, given it has no separate plume mesh to reveal - see
`docs/formats/pure-status.md`. Found by walking forward from
`ExhaustFlare_Init` (`0x0886b924` usa / `0x0886b71c` eu) to the two sibling
functions immediately after it in `.text` - the same "adjacent in source
order" pattern that placed `ExhaustFlare_Init` itself, `get_xrefs_to` still
being unusable for a locator string like `~ENGINE`'s neighbour `Engine Flare`
(the known relocation defect, "Technique" section of `ship-models.md`).

**`Exhaust_UpdateEngineSound`** - `0x0886b274` (usa), `0x0886b06c` (eu),
`xref_count` 1 each, called from the sibling below. Decompiled in full on
both pressings, byte-identical to Pulse's own reading
(`docs/ghidra/functions/psp-pulse-usa/exhaust.md`): `engine_on` sets when
`thrust > 0 || boost_timer > 0.2`, pitch lags toward `base + speed_kmh * 5.0`
at rate `0x3c23d70a` (`0.01`, the same literal `ExhaustFlare_Init` seeds), and
the function's only write to `boost_timer` is `boost_timer = max(0,
boost_timer - dt)` - a pure decay, never an arm.

**`Exhaust_Update`** - `0x0886bc6c` (usa), `0x0886ba6c` (eu), `xref_count` 0
on both (called only through a vtable slot, same as Pulse's). Decompiled in
full: three staggered layer-alpha ramps at `i = 0, 0.25, 0.5`, then

```c
*(float *)(param_2 + 0x1a8) =
     (*(float *)(param_2 + 0x1a0) * 0.6 + 0.4) * 2.5 + *(float *)(param_2 + 0x19c) * 8.0;
fVar5 = (float)FUN_0882f70c(0x3f400000, 0x3fa00000);  // rand(0.75, 1.25)
*(float *)(param_2 + 0x1a8) = *(float *)(param_2 + 0x1a8) * fVar5;
iVar3 = FUN_0882f5ac(200, 0xff);                      // rand_int(200, 255)
*(int *)(param_2 + 0x1ac) = iVar3 * 0x1000000 + 0xffffff;
```

identical on the usa pressing down to the literal hex, `+0x1a0` being the
intensity `i` and `+0x19c` being `boost_timer` - the same field
`Exhaust_UpdateEngineSound` only ever decays. This is Pulse's own
`half_size = ((i * 0.6 + 0.4) * 2.5 + boost_timer * 8.0)` and colour-alpha
formula (`exhaust.md`), reproduced bit for bit, both pressings.

**The writer, closing the loop.** `search_strings("SPEEDUPPAD")` (the sound
cue Pulse's `ExhaustFlare_OnSpeedupPad` plays from the same branch) lands on
`0x08a490b8` (usa) / `0x08a475b8` (eu); `get_xrefs_to` on that address - a
code reference, unaffected by the data-in-data relocation defect - returns
exactly one function each: `0x0886b548` (usa), `0x0886b340` (eu). Both open
with `*(undefined4 *)(param_1 + 0x19c) = 0x3f4ccccd;` - **`0.8f`**, the exact
bit pattern `oag_render::exhaust::BOOST_SECONDS` already carries, read off
Pulse. So the arm, the decay and the visual term all match Pulse's own
reading, on both pressings, and this engine's existing generic
`oag_render::exhaust::Exhaust` (title-agnostic, wired unconditionally in
`crates/game/src/race/pads.rs`) already reproduces Pure's boost visual
correctly - nothing to implement, only to cite.

**Confidence 88** for this section: decompiled in full on both pressings, four
independent literal constants matching Pulse's bit for bit (`0.6`, `0.4`,
`2.5`, `8.0`, the `0.75..1.25`/`200..255` rand ranges, and the `0.8` arm), no
runtime leg for either binary.

## Confidence

**82** for `ExhaustFlare_Init`: decompilation only (no runtime leg, no
verified caller - blocked on the `jal` wart), but corroborated by an
independently-recovered Pulse function of matching shape *and* two
exactly-matching literal constants, not merely a structural echo - the same
evidence class `Sound_Play`'s own rename earned, and stronger on the
constants than any other cue this thread has found so far.

## What is not verified

- **`ExhaustFlare_Init`'s own caller**, blocked on the `jal` wart.
- **Runtime verification.** No PPSSPP leg for either binary, for any function
  on this page.

## History

- **2026-09-04.** Written answering `every-sfx-trigger-is-a-pulse-reading-applied.md`'s
  `Engine` cue for Pure.
- **2026-09-23.** Added `Exhaust_UpdateEngineSound`, `Exhaust_Update` and
  `ExhaustFlare_OnSpeedupPad`, closing the "not verified" bullet on the
  sound-update sibling and the open question on whether Pure's boost visual
  is inert (`docs/formats/pure-status.md`).
