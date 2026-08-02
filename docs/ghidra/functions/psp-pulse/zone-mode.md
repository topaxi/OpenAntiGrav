# Zone mode

**Binary:** `pulse-psp` `BOOT.BIN`, image base `0x08804000`.
**Status:** the mode's own update loop is read end to end. Confidence **84** for
the identification and the ten-second step, **82** for the scoring and the
recharge, and the run-end condition is **not determined**.

Everything below is decompilation plus instruction-stream reading. No runtime
trace, no emulator observation, so nothing here goes above 84 per the
[confidence rubric](../../../reverse-engineering/confidence-rubric.md).

## Identification

`docs/physics/README.md` and [`engine.md`](engine.md) recorded the selector
`DAT_08ab07e3 == 0 && DAT_08b31048 == 6` at confidence 84, but only **50** for it
being Zone rather than some other mode. That second figure is now **84**, and the
evidence is a chain rather than an inference:

| Step | Address | What it shows |
| --- | --- | --- |
| Mode factory | `0x0882112c` | `uVar3 = (DAT_08ab07e3 == 0) ? DAT_08b31048 : 0; switch (uVar3)`. **`case 6:`** allocates `0x1a30` bytes and calls the constructor below. |
| Constructor | `0x0882eee4` | Loads `Data\XML\Zone_HUD.xml`, installs vtable `0x08ac9c38`, zeroes the counters. |
| Vtable slot `+0x1c` | `0x0882f214` | The per-frame update, dispatching on the mode state at `obj+0x7c8`. |

The selector expression in `Ship_UpdateEngine` is **byte-for-byte the expression
the factory uses** to pick the constructor that loads the Zone HUD. That closes
it: the four-corner hover variant, the disabled brakes and the auto-speed law all
belong to Zone.

## Named here

| Address | Kind | Name | Confidence |
| --- | --- | --- | --- |
| `0x0882112c` | function | `Race_CreateModeObject` | 84 |
| `0x0882eee4` | function | `Zone_Create` | 84 |
| `0x0882f214` | function | `Zone_UpdateState` | 82 |
| `0x0882f37c` | function | `Zone_UpdateRacing` | 82 |
| `0x0882f5cc` | function | `Zone_Update` | 84 |
| `0x0882f438` | function | `Zone_UpdateResults` | 82 |
| `0x0883ddc8` | function | `Ship_AddShield` | 82 |
| `0x0883e6f4` | function | `Ship_SetShield` | 82 |
| `0x0883e68c` | function | `Ship_Shield` | 78 |
| `0x08827350` | function | `RaceMode_SetState` | 80 |
| `0x08827b08` | function | `RaceMode_PushState` | 75 |
| `0x0881a1c8` | function | `Hud_SetEnergyBar` | 75 |
| `0x088c291c` | function | `Ship_LoadHandlingStats` | 84 |
| `0x0894f6a8` | function | `SystemRoot_Create` | 78 |

Why the last seven:

- `Ship_Shield` (`0x0883e68c`) is the getter `Ship_AddShield` reads before adding;
  its result plus the amount is what goes to `Ship_SetShield`.
- `RaceMode_SetState` (`0x08827350`) writes the mode state at `obj+0x7c8` and
  zeroes `+0x7c0`, `+0x7c4` and `+0x7cc`. It is what moves a Zone run to state 3
  at the end.
- `RaceMode_PushState` (`0x08827b08`) is called with the literal
  `"EndRace_Results"` from `Zone_UpdateResults`. 75 rather than higher because
  only this one call site was read.
- `Hud_SetEnergyBar` (`0x0881a1c8`) is called by `Ship_SetShield` with
  `(shield / maxshield) * 100` and the HUD object at `DAT_08ab0838` - a
  percentage, which matches `ShieldBarText` reading `100%` on a reference frame.
- `Ship_LoadHandlingStats` (`0x088c291c`) builds `%s\handlingstats.xml` and calls
  `Handling_ParseStats`, guarded by a once-only flag at `+0xa8`.
- `SystemRoot_Create` (`0x0894f6a8`) is the other caller of `Handling_ParseStats`,
  passing the literal `Data\XML\HandlingStats.xml`, and closes by tracing
  `"After SystemRoot Create"` - which is where the name comes from. 78: the trace
  string names it, but the function does a great deal more than load that file.
| `0x08b36be0` | data | `g_autospeed_base` | 84 |
| `0x08b36be4` | data | `g_autospeed_step` | 84 |
| `0x08b34360` | data | `g_zone_recharge` | 84 |
| `0x08ab0bc8` | data | `g_zone_milestones` | 75 |

## The mode object

Offsets from the object the factory allocates. From the constructor's zero-init
list plus consistent use in `Zone_Update`. Confidence **80**.

| Offset | Type | Meaning |
| --- | --- | --- |
| `+0x1a10` | `u16` | zone number |
| `+0x1a12` | `u16` | perfect-zone count |
| `+0x1a14` | `u16` | a counter stepped on a lap-like condition |
| `+0x1a16` | `u16` | laps completed |
| `+0x1a18` | `u16` | written at run end |
| `+0x1a1a` | `u16` | best speed seen, `speed * 100` |
| `+0x1a1c` | `s32` | score |
| `+0x1a24` | `f32` | zone dwell timer |
| `+0x1a28` | `u8` | "this zone was dirty" |
| `+0x1a2c` | ptr | cursor into `g_zone_milestones` |

Two of these offsets are reused by *other* mode subclasses with different
meanings, so read them only through `Zone_Update`.

## The ten-second step

`Zone_Update`, the load-bearing finding:

```c
param_1 = *(float *)(obj + 0x1a24) + dt;
*(int *)(obj + 0x1a1c) += 1;                 // score, every tick
*(float *)(obj + 0x1a24) = param_1;
if (10.0 <= param_1) {                        // lui a1,0x4120 @ 0x0882f5e4
    if (*(char *)(obj + 0x1a28) == 0) {        // nothing hit this zone
        *(int *)(obj + 0x1a1c) += 500;
        *(short *)(obj + 0x1a12) += 1;
        Ship_AddShield(g_zone_recharge, *(void **)(obj + 0x2c0));
    }
    *(char *)(obj + 0x1a28) = 0;
    *(short *)(obj + 0x1a10) += 1;
    *(uint *)(*(int *)(entity + 0x94) + 0x28c) = *(ushort *)(obj + 0x1a10);
    *(float *)(obj + 0x1a24) = 0;              // reset, NOT -= 10.0
}
```

Four claims, each read from the instruction stream rather than the decompiler:

1. **The zone steps every 10.0 seconds of accumulated frame time**, and on
   nothing else - not distance, not laps, not score. The `10.0` is the immediate
   `lui a1,0x4120` at `0x0882f5e4`. Confidence **84**.
2. **`craft+0x28c` is assigned, not incremented.** The mode object owns the
   counter; the craft field is a copy for the engine to read. Confidence **84**.
3. **The accumulator resets to `0`**, not `-= 10.0`, so each step discards the
   frame overshoot and the zone clock runs fractionally slow. Confidence **82**.
   This is behaviour, not a rounding artefact - do not "fix" it.
4. **There is no cap and no threshold table.** `obj+0x1a10` is a `u16` with a bare
   `+= 1`, and the only other write to `craft+0x28c` anywhere is the reset in
   `Ship_InitCraft` at `0x08849554`. Confidence **80**, bounded because a write
   through a computed base with a different displacement would not have been
   caught by a displacement search.

`g_zone_milestones` at `0x08ab0bc8` is **not** a speed table. It is a list of
8-byte entries walked by `obj+0x1a2c`, whose first words are a rising sequence of
zone numbers and whose second word triggers the same call the "ready" sample uses
- an announcement list. Confidence **75**. Its all-zero sound-ID column and its
`{0,0}` / large-sentinel tail are **unexplained**.

## Correction to [`engine.md`](engine.md)

The auto-speed law recorded on that page is incomplete. The actual branch, at
`0x0884c810`:

```
0884c828: and   a1,a1,a2            ; gate = (flags & 1) && !(flags & 2)
0884c82c: beql  a1,zero,0x0884c86c
0884c830: _mov.s f12,f13            ; likely-delay: T = 0.0
0884c834: lw    a1,0x28c(a0)
0884c83c: bgez  a1,0x0884c850
0884c844: lui   a1,0x4f80           ; +2^32 if the word is negative
```

Three deltas, all confidence **84**:

- **There is a gate.** The law applies only when `(flags & 1) && !(flags & 2)`,
  and the target is `0.0` when it fails. Bit 0 is the ground-contact bit - the
  same one that gates `brakes` - so groundedness is what it turns on.
- **The conversion is unsigned**: `bgez` plus `lui 0x4f80` is the standard
  `(float)(u32)` idiom, so the law is `base + step * (float)(uint32)n`. The PS2
  page already writes `(uint)`; the PSP page was the odd one out.
- **There is no cap.** The ordinary branch computes `min(0.5 * speed + accelcap)`;
  this branch overwrites the slot instead.

## Where the two floats come from

`g_autospeed_base` and `g_autospeed_step` are in `.bss` and hold nothing in the
file. They are parsed at runtime by `Xml_ReadGlobalSettings` (`0x0883a970`) from

```xml
<Zone start="..." increment="..." recharge="..."/>
```

inside `<Handling><Global>`. `start` and `increment` are the two law floats;
`recharge` is `g_zone_recharge`, added to the shield for a clean zone.

**Which file** is the part worth writing down, because it is not the obvious one:
`Handling_ParseStats` (`0x0883a2f0`) has two callers, and the `<Global>` block
lives in the one at `0x0894f6a8`, which passes the literal
`Data\XML\HandlingStats.xml`. The per-team `Data\Ships\<Team>\handlingstats.xml`
files carry `<Stats>` and nothing else - measured across all sixteen shipped files,
eight teams on each of the PSP and PS2 discs, by
`which_top_level_elements_handlingstats_carries` in
`crates/formats/tests/handling_ground_truth.rs`. See
[handling stats](../../../formats/handling-stats.md).

## The shield recharge

`Ship_AddShield` (`0x0883ddc8`) adds to the pool and calls `Ship_SetShield`
(`0x0883e6f4`), which **clamps to the ship's own maximum** out of the stat block
and then drives the HUD energy bar. So a clean zone on an undamaged craft gains
nothing. Confidence **82**.

The dirty flag is set from bit 22 (`0x400000`) of `entity+0x860`, which reads as
"hit something this frame" - confidence **80**.

## Not determined

1. **What ends a run.** `Zone_UpdateRacing` tests bit 12 (`0x1000`) of
   `entity+0x860` and moves the mode to state 3 on it. **What sets bit 12 was not
   found**: the `sw ...,0x860(...)` sites that were read set `0x200000` and
   `0x400000`, never `0x1000`, so it is composed somewhere not resolved. The same
   bit ends every other mode's run, which makes it a generic "this craft is done"
   flag rather than anything Zone-specific. Confidence **60** for "craft
   eliminated", **25** for specifically "shield reached zero". **Do not implement
   it as shield depletion on this evidence.**
2. **What raises `IG_HUD_PERF_ZONE`, `IG_HUD_NEW_ZONE_RECORD` and
   `IG_HUD_NEW_SCORE_RECORD`**, and what drives the `Zone_Bar_*` widgets. The
   strings exist but are `idstring` values consumed by the HUD-XML loader, not
   referenced from mode code, so the trigger is inside the widget system.
3. **The milestone table's sound-ID column**, which reads as zero for every entry.

## What is implemented

`crates/race/src/zone.rs` and `crates/race/src/state.rs` carry the ten-second
step, the score, the perfect-zone bonus and the recharge.
`oag_physics::forces::Environment::auto_speed` carries the law into
`oag_physics::engine::engine`, whose four-corner branch is gated on groundedness
as above.

The run-end condition is **not** implemented, because item 1 is not determined -
see [race modes](../../../gameplay/race-modes.md).
