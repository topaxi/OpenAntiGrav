# Detonator: a stage is a lap, fourteen of them end the race

**Binary:** `EBOOT.elf` (`/hdfury/EBOOT-ps3-hdfury-eu.elf`), TOC `r2 = 0x008ad4d8`. Static reading only;
nothing here ran. Read first: [zone-effectsettings-loader.md](zone-effectsettings-loader.md), "Detonator's
own escalation, recovered end to end", which found `Detonator_UpdateRace` (`0x00067b40`) and its counter
`RaceManager+0x2e10` but left open what steps it.

## What steps the stage: the player's own start-line crossing

`Detonator_UpdateRace` adds one to `RaceManager+0x2e10` when `*(rm+0x13e8)+0x7001 != 0` and
`*(rm+0x13e8)+0x7810 > 1`. `rm+0x13e8` is the local player's craft (the pointer
`ai-controls.md` and `hud-readouts.md` already read as such). Both fields belong to
`Craft_UpdateLapProgress` (`0x000e4a30`, [hud-message-triggers.md](hud-message-triggers.md)), whose
Ghidra body stops at `0x000e4e7b`, so `search_instructions` cannot see the rest of it. A raw scan of the
real extent, `0x000e4a30`-`0x000e6060`:

```text
$ scripts/ps3-dform-scan.py EBOOT.elf 0xe4a30 0xe6060 --disp 0x7001
0xe4ac8 stb r9, 0x7001(r31)     ; r9 = 0 (li r9,0 at 0x000e4a50): cleared on entry, every tick
0xe5104 stb r0, 0x7001(r31)     ; r0 = 1: a new forward line crossing
0xe5960 stb r7, 0x7001(r31)     ; r7 = 1: the race's first crossing
```

- `0x000e4f3c` stores the crossing count `+0x7810` (`endrace-loyalty.md` reads `+0x7810 - 1` as laps
  completed) and `0x000e4fa4` reloads it as `r7`.
- `0x000e4fac`-`0x000e4fb4`: when the count is `1` the branch goes to `0x000e5938` (the only branch there,
  `--target 0xe5938`), which sets `+0x7000 = 1`, `+0x7814 = 2` and `+0x7001 = 1`: the crossing that
  starts lap 1.
- Every later crossing that reaches a new count reaches `0x000e50e8` (directly, or through
  `Craft_RecordSplit(rm, 0)` at `0x000e5a38`), which stores `+0x7814 = count + 1` and `+0x7001 = 1`.

So `+0x7001` is a one-tick "crossed the line forwards" pulse, and the `+0x7810 > 1` guard drops the
first crossing. **The stage steps once per completed lap**: it is constructed at `1` (`SPDetonator`'s
two constructors, `li r9,1; stw r9,0x2e10`), reads `2` after lap 1, and reaches `15` at the end of lap
14, where the end-of-race path (`if (stage < 0xf) return;`) lets the race finish. Fourteen laps,
fourteen stages. Confidence **78**: three writers read at instruction level and consistent with the
lap counter two other pages read, short of a runtime read.

The disc says the same thing in words. 2048's English `MAN_3_DETONATOR` (`data2.psarc`
`data/plugins/languages/english/entries.xml`, v1.04 patch): "Blast your way through as many of the
**14 stages** as you can ... Fresh mines (up to a stage maximum) and one new bomb will be laid **every
time you cross the start line to begin a new stage** ... You must face this one alone - it's single
player only." HD's `DATA06` English table carries the mode's name (`Detonator` = `DETONATOR`,
`ER_DETONATOR_STAGE` = `STAGE CLEARED`) but not that manual page.

## What `Detonator_UpdateRace` does that is not wired

Read off the decompile of `0x00067b40`; none of it is in this engine yet.

| Piece | Reading | Confidence |
| --- | --- | ---: |
| Second race end | the same end path runs when the player craft's `+0x6a64 & 0x1000` is set (the out-of-race bit [eliminator-flip.md](eliminator-flip.md) cites): a destroyed craft ends the run. What sets the bit: mine and bomb damage drains the energy pool, `Ship_ApplyDamage` at zero calls `Ship_SetState(4)`, which sets it ([detonator-mine.md](detonator-mine.md#what-the-craft-does-with-it), 72). | 70 |
| Mines and bombs per stage | `Detonator_PlaceStageObjects` (`0x000654e0`, `(rm, oldStage)`) runs once at race start with `oldStage = 0` and on every step while the new stage is under `15`. Read from the disassembly on [detonator-placement.md](detonator-placement.md): pattern groups from the global `/data/xml/DetonatorLayouts/PatternFile.xml`, stage counts `MineCountStage1..14` from `GamePlayValues.xml` as a target total including surviving mines, chunked placement round a surveyed lap, single-mine top-up, one bomb, through `Detonator_SpawnObject` (`0x00064f70`) into a 200-mine pool. | 80 |
| EMP charge | a craft flag `+0x6a64 & 0x8000000` (cleared on read) bumps `rm+0x2dd8`; below `table+0x1bc` the gauge `craft+0x7da0` is `0`, above it `(n - (hi - lo)) / (hi - lo)` clamped to `1`, with the `EMPREADY` and `EMPFULL` announcer cues on the two edges. The manual's "fly over the special EMP pads to charge up your EMP weapon" is this. Read whole and live since: [detonator-emp.md](detonator-emp.md). | 70 |
| End-of-run percentage | `rm+0x2ddc = float(rm+0x6de3c) / float(MineCountStage1 + .. + MineCountStage14) * 100.0` (f32, `k = 100.0` at TOC `-0x6830`; the disc's sum is 1860); `rm+0x6de3c` counts mines destroyed by a cannon round or the EMP, not chain-reaction mines or bombs; `rm+0x2de0 = rm+0x6de40` (bombs). `PERCENT_MINES_CLEARED` is the result screen's label ([detonator-cannon.md](detonator-cannon.md#the-destroyed-count-and-the-end-of-the-run)). | 82 |
| Top block | while the craft's speed `+0x6944 -> +0x4c4` is over `10.0`: `Detonator_UpdateHud` (`0x00067120`: reload gauge, HUD stage, medals, two trophies, `+1000` on a craft flag; not a score add), then `Detonator_IsBombAhead(500.0, rm, craft)` (`0x000642a0`) once per stage: a live bomb up to 500 ahead raises HUD message `8` and the `"bomb"` cue, a warning, not a hit ([detonator-cannon.md](detonator-cannon.md#the-top-block-a-bomb-warning-the-hud-and-three-trophies)). | 55-80 |
| Pulse cannon and score | the craft's fixed cannon: a clip of 15 two-round volleys at 0.1 s, rounds at 500 units/s that skim the track and rebound once, a mine in one hit, a bomb in four, a chain reaction round a shot-down bomb; the score from `DetonatorScoring.xml` into `rm+0x2df0`, streak bonuses 1000/3000/5000 per clip ([detonator-cannon.md](detonator-cannon.md)). | 72-82 |

**What the manual says and no code here was read for**: "Like Zone mode, acceleration is automatic
and ever-increasing". So this engine flies a Detonator race on the player's own throttle: the
auto-speed law is not read, and Zone's is not borrowed for it. (Read since, 2026-11-22: it is
Zone's branch with the Detonator stage as `n`, [detonator-speed.md](detonator-speed.md).) The pulse cannon, its reload, the
shot-chain bonus and the score values are read on [detonator-cannon.md](detonator-cannon.md).

## The Zone-modes mask carries Detonator

HD tests one mask, `0x206040` (game modes 6, 13, 14 and 21), in `Craft_Update` and
`Craft_UpdateHover`, and mode 14 is Detonator. So a Detonator craft, like a Zone craft, flies the
Zone hover rig with its trailing ray, has no brakes, and waits on the grid at the ray's height
([hover-four-point.md](hover-four-point.md), "Zone, read live"; the mask is read statically, and
the laws were observed live for mode 6 only). Wired as `oag_race::Mode::on_zone_mask`; on 2048 and
Omega those title fields are `None`, so nothing changes there.

**The handling file is not Zone's.** `Team_LoadHandlingStats` tests mode `0xe` before the mask
(`0x001e7050`) and reads `Data\Ships\Detonator\handlingstats.xml`. That file has the same engine as
Zone's, but turns harder (`Turning` 2.2/1200 against 1.8/1000), has less airbrake and tilt, is
narrower (`Misc.width` 4.0 against 5.0), and carries its own cameras. Confidence 82, and the same
test is on 2048 and Omega ([detonator-hull.md](detonator-hull.md), "The handling file"). This page
once said a Detonator craft flies Zone's file, and the wiring followed it:
`oag_hd::race::ZONE_HANDLING` applies to every `on_zone_mask` mode. Engine lights are Zone's
(`Data\Ships\Zone\EngineLightData.xml`, under the mask). The hull is Detonator's own,
`Data\Ships\Detonator\Ship.vex`, also on [detonator-hull.md](detonator-hull.md). **Departure, named**: the same mask skips `Craft_RecordSplit`
inside `Craft_UpdateLapProgress` (`0x000e50c4`-`0x000e50dc`), so the original keeps no split gaps in
Detonator; this build's HUD still shows its lap times.

## 2048's grade on its own circuits

`Environment_Load` (`0x8102f6d0`, v1.04), inside its Zone-family gate, loads
`<circuit>\ZoneMode2048.effectSettings` (and `ZoneMode2048default`) when the circuit has an
`art\source\environments` or `art\published\environments` tree, with no mode test on that path;
only the fallback for HD's ported circuits tests mode `0xe` and picks `DetonatorModeHDFury[DLC3]`.
Read 2026-11-13 off the decompile. So a Detonator race on a native 2048 circuit grades with that
circuit's Zone table, and on a ported circuit with HD/Fury's Detonator table: what
`oag_2048::race::DETONATOR` loads (`palette`, then `fallback_palette` when the circuit's file is
absent, which is the ported circuits). `detonator_ground_truth.rs` checks the `zone_1` case.

## The campaign's Detonator cells

Every `mode="Detonator"` cell on `DATA00`'s eight Fury grids (`grid_08`..`grid_15`, 14 cells) authors
`Weapons="on"`, `laps="0"`, no `AICount`, `ShipChoice="Yes"`, class `Venom` or `Zone`, and six-figure
score medals (`EasyGold` 40,000 to 150,000). Tracks `17_Track`..`28_Track`, the Fury circuits and the
four Zone circuits. `laps="0"` agrees with the stage law ending the race rather than a lap count.

## The same law on 2048 and Omega

**Ported**, with a load test each (`detonator_ground_truth.rs`: 2048 `zone_1`, Omega). Each
title's Detonator update is found by its `EMPREADY` cue and has the
same shape: a one-tick craft latch, a crossing count over `1`, a counter that starts at `1`, a placement
call while the counter is under `15`, and the end at `15` or the out-of-race bit.

| Title | Update | Latch, crossing count | Stage | Placement |
| --- | --- | --- | --- | --- |
| HD/Fury | `0x00067b40` | `craft+0x7001`, `+0x7810` | `rm+0x2e10` | `FUN_000654e0(rm, old)` |
| 2048 v1.04 | `FUN_8117e9c2` | `craft+0x6641`, `+0x6e50` | `rm+0x2b238` | `FUN_8117cf22(rm, old)` |
| Omega | `FUN_012a0fc0` (state `2` of a phase switch) | `craft+0x77e1`, `+0x7ff0` | `rm[0x6017]` | `FUN_012a2300(rm)`, also once at race start with the opening stage |

Omega adds a per-stage event list (`rm[0x6019]`, entries skipped on `"BLANKEVENT"`) and writes the stage,
capped at `14`, into a craft field `+0x5e4`. On 2048 and Omega the crossing-count reading is structural
(the fields were not read back to their writers), confidence **70**.
