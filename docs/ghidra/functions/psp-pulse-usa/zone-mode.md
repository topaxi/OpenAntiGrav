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

## The ship model is not the player's own hull

`Ship_LoadModel` (`0x08843258`) switches on `local_38`, set to `DAT_08b31048`
when `DAT_08ab07e3 == 0` and to `0` otherwise - **the same expression** this
page's identification section already fixes at confidence 84. Every other
case builds the ordinary `%s\%s\%s.vex` or `%s\Ship.vex` path, but:

```c
case 6:
    local_40 = (char *)0x0;
    FUN_08972550(auStack_100, s__s_Zone_vex_08a7ba4c,
                 *(undefined4 *)(*(int *)(param_1 + 0x370) + 0x94));
    break;
```

`case 6` builds `%s\Zone.vex` instead, and the same `local_38 == 6` guard
further down swaps `%s\%swreck.vex` for `%s\zonewreck.vex`. Confidence **84**
for the branch itself: decompilation only, capped by the
[confidence rubric](../../../reverse-engineering/confidence-rubric.md), but
using the identical selector this page already verified end-to-end rather than
a second inference.

**The model this loads is not a per-team hull.** Rendered with `oag-view
--mesh` against the PSP disc, `AG_Systems\Zone.vex`, `Assegai\Zone.vex` and
`Feisar\Zone.vex` all decode to the same 8 meshes / 1213 vertices / 1149
triangles / radius 6.97, where their three `Ship.vex` files disagree on every
one of those numbers. **All eight were then decoded, and the inference no longer
rests on file size.** Every one of the eight `Zone.vex` files reports the same 8
meshes / 1213 vertices / 1149 triangles / radius 6.97 under `oag-view --mesh`.
Corrected 2026-08-09: this paragraph used to generalise from three decodes to
eight via "every `Zone.vex` is exactly 76944 bytes", and that equality is false -
`EGX\Zone.vex` is **76880**, the other seven are 76944. The conclusion survives
because the decodes were run; the size argument never supported it. So Zone mode
flies one shared hull regardless of team - the per-team folder only supplies that hull's livery, the same way
`Ship.vex`'s livery differs from team to team on the same underlying rig
elsewhere in the format. Not checked against the PS2 disc (a different WAD
layout, `WADS2.WAD` rather than `Data.wad`), so this is PSP-only for now.

Implemented in [`oag_game::race::ship_entry_name`](../../../../crates/game/src/race/assets.rs).

## A speed pad is worth 100 points

`Zone_Update` (`0x0882f5cc`) consumes a flag that only `Ship_ApplySpeedupPad`
sets, and only under this page's own selector:

```c
if (DAT_08b3435c != '\0') { DAT_08b3435c = '\0'; score += 100; }
```

`Ship_ApplySpeedupPad` sets `DAT_08b3435c` when a craft enters a **new** pad and
`DAT_08ab07e3 == 0 && DAT_08b31048 == 6` - so the bonus is Zone-only, once per
pad rather than once per tick on it. Confidence **85**: `Zone_Update`'s three
other constants (1 per tick, 500 per zone, 500 for a clean zone) are the ones
`crates/race/src/zone.rs` already carries from unrelated evidence, and all three
agree, which is what identifies `+0x1a1c` as the score in the first place.

See [pads.md](pads.md) and [engine.md](engine.md).

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

   **Independently corroborated by the disc's own text**, which is unusual enough
   to note: `MSC_EVENT_ZONE` describes the mode as one where *"the top speed
   increases after every ten second period, called a zone"*. Instruction stream
   and shipped English agree, and they were read a day apart from different
   places.
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

1. ~~**What *sets* bit 12.**~~ **Answered 2026-08-10: `Ship_SetState`
   (`0x08844100`) case 5.** The search that produced the paragraph below was
   looking at the wrong function - the bit is not set anywhere in the Zone code
   or on the contact path, but in the craft's own state machine, three states
   after the energy pool empties. The chain, end to end:

   | Step | Where | What |
   | --- | --- | --- |
   | Pool reaches zero | `Ship_Damage` (`0x088439ac`) | `if (new <= 0.0) Ship_SetState(entity, 4)` |
   | State 4, the explosion | `Ship_SetState` case 4 | plays `_BLOWUP`, hides the HUD, camera mode 5, and arms `entity+0x874 = 0.5` |
   | Half a second later | `0x088404c8`, state 4's update | four lines: `0x874 -= dt`, and at or below zero `Ship_SetState(entity, 5)` |
   | State 5 | `Ship_SetState` case 5 | **`entity+0x860 |= 0x1000`** - bit 12 - plus `0x874 = 1.5` and a render flag that hides the model |
   | The mode notices | `Zone_UpdateRacing` | tests bit 12, moves to state 3 |

   Confidence **88**: every step is a branch-clear decompile and the two halves
   were recovered from opposite ends - the pool from
   [`shield.md`](shield.md), the bit from this page - and met in the middle.
   No runtime leg. Implemented as `oag_physics::damage::CraftState` and
   `oag_race::RaceState::eliminate`.

   **Case 4 read at instruction level, 2026-08-24**, because the decompiler
   cannot render it: `Ship_SetState` dispatches through a nine-entry jump table
   the decompiler gives up on (`"Could not emulate address calculation"`), so
   the whole state machine reads as one indirect call. **The table is at
   `0x08a7bc18`** and its arms are `0x0884416c`, `0x0884417c`, `0x0884418c`,
   `0x0884419c`, **`0x0884430c`** (state 4), **`0x08844548`** (state 5),
   `0x088445a0`, `0x088445ec`, `0x088446ec`. Disassemble the arm; do not try to
   decompile the function.

   What case 4 actually does, in order:

   | | |
   | --- | --- |
   | first ~30 instructions | **multiplayer only** - a game-mode `>= 0xe` gate and two debug strings about Quake ownership on destroy (`0x08a7bacc`, `0x08a7bb08`). Nothing to port. |
   | `0x08844434` | `craft+0x874 = 0.5` - the state timer this page already had |
   | `0x08844448` | `FUN_0883e9b0(craft, hud_bank, "~BLOWUP", 0x400, 0)`, handle kept at `craft+0xcac` |
   | `0x08844454` | **`if (craft+0x368 != 0) return`** - everything below is the local player's alone |
   | `0x088444b4` | a call on `*(0x08ab0838)`, unidentified - the HUD hide is presumably here |
   | `0x088444c8` | `(*(0x08ab2120))+0x58 = 0`, a byte |
   | `0x088444e4` | `(*(0x08ab10e8))+0x2c &= ~6` |
   | `0x088444f0` | **`(*(0x08ab10b0))+0xe4 = 0.0`** - the active camera, and `+0xe4` is the *duration* field of the shake setter `FUN_08878750` writes, so this **cancels a running shake** |
   | `0x08844514` | `(*(0x088594cc))+0x2c \|= 6`, then `+0x26c = 0` |
   | `0x08844524` | a call with **`a1 = 5`** - the camera mode this page names |
   | `0x0884453c` | `(*(0x0885c01c))+0x1a08 = 4.5` |

   **`_BLOWUP` is `~BLOWUP`** - the string is at `0x08a7b6c0`, reached through
   the pointer at `0x08a7b6c8`, and `Data\Sound\hud.bnk` carries it as two
   waveforms, one looping, 0.37 s. It is played through the **no-emitter**
   path at volume `0x400` - the one
   [`positional-audio.md`](positional-audio.md) identifies as the local
   player's - and `FUN_0883e9b0` returns without playing when `craft+0x368` is
   non-zero, so **an opponent's destruction plays nothing here**. Whatever an
   opponent's explosion sounds like comes from somewhere this pass did not
   find. Confidence **85** on the cue and the path; the argument order is
   `FUN_0883e9b0`'s, read once here and once on the autopilot's warning.

   **Ported**: `oag_game::audio::sfx::Cue::Blowup`, held on
   `Race::craft_is_exploding` - the state's own `0.5 s`, because where the
   original releases `craft+0xcac` is unread and that is the shortest lifetime
   the evidence supports.

   **Still not ported, and now with addresses rather than a shrug**: the HUD
   hide and the camera mode. Three of the four globals above
   (`0x08ab0838`, `0x08ab2120`, `0x08ab10e8`, `0x088594cc`) are unidentified,
   and until they are, "hides the HUD" is a summary of a call rather than a
   reading of one. The camera-mode call at `0x08844524` passes `5` and is the
   most tractable of them.

   **One thing that came free**: the shake cancel at `0x088444f0` is a second,
   independent sighting of `FUN_08878750`'s field layout - see
   [`autopilot.md`](autopilot.md), which found the setter from the other side.

   The original text is kept below because its *negative* result is still
   correct and still useful: `Zone_UpdateRacing` tests bit 12 (`0x1000`) of
   `entity+0x860` and moves the mode to state 3 on it, but the `sw ...,0x860(...)`
   sites that were read set `0x200000` and `0x400000`, never `0x1000`, so the
   write is composed somewhere not resolved.

   **What the bit *means* is no longer open**, and the evidence is the disc's own
   text rather than the instruction stream. `ER_ZONE_DEST` reads
   `"Ship destroyed on zone"`, and `MSC_EVENT_ZONE` describes the mode as
   *"survive for as long as possible before crashing out"*. So a run ends when the
   craft is destroyed - confidence **80**, up from the **25** this page first gave
   "shield reached zero", which was a guess and is now corroborated.

   Still not implemented, but the reason has changed: it needs a shield pool that
   depletes, which is unimplemented project-wide, rather than more reverse
   engineering.
2. **What raises `IG_HUD_PERF_ZONE`, `IG_HUD_NEW_ZONE_RECORD` and
   `IG_HUD_NEW_SCORE_RECORD`**, and what drives the `Zone_Bar_*` widgets. The
   strings exist but are `idstring` values consumed by the HUD-XML loader, not
   referenced from mode code, so the trigger is inside the widget system.
3. **The milestone table's sound-ID column**, which reads as zero for every entry.
4. **The per-event target zone count.** `MSC_EVENT_ZONE` ends *"Clear the target
   number of zones to win the event"*, so a target exists and belongs to the event
   rather than to the mode. Where it is stored was not looked for - it is
   progression data, and progression is M7.

## What is implemented

`crates/race/src/zone.rs` and `crates/race/src/state.rs` carry the ten-second
step, the score, the perfect-zone bonus and the recharge.
`oag_physics::forces::Environment::auto_speed` carries the law into
`oag_physics::engine::engine`, whose four-corner branch is gated on groundedness
as above.

The run-end condition is **not** implemented, because item 1 is not determined -
see [race modes](../../../gameplay/race-modes.md).
