# Detonator: the pads charge the EMP, the EMP clears the track ahead

**Binary:** `EBOOT.elf` (`/hdfury/EBOOT-ps3-hdfury-eu.elf`). Every function below is under
`0x32d5e0`, TOC `r2 = 0x008ad4d8` ([memory.md](memory.md)); constants and strings resolved with
`scripts/ps3-toc.py resolve`, AltiVec-truncated bodies read off `llvm-objdump -d
--mattr=+altivec,+vsx`, writers found with `scripts/ps3-dform-scan.py --disp`. Static reading
2026-11-22; nothing here ran. Read first: [detonator-mode.md](detonator-mode.md) (the gauge
paragraph this page replaces), [detonator-cannon.md](detonator-cannon.md) (`Ship_UpdateCannonTrigger`,
the score record), [detonator-mine.md](detonator-mine.md) (`MineManager_DestroyAtLane`).

## The answer

| Quantity | Value | Confidence |
| --- | --- | ---: |
| Speed pad, Detonator | **no boost**: no force, no timer; charges the EMP once per pad entered, plays `WEAPONPICKUP` | 90 (live) |
| Weapon pad, Detonator | **no weapon**: the award roll is skipped; charges the EMP once per pad, again only after a different pad or the next start-line crossing | 80 (one live charge) |
| Charge | `rm+0x2dd8 += 1` per charge, any number of pads | 82 |
| Ready | `rm+0x2dd8 >= minNumPadsToCollectEMP` (`table+0x1bc`, disc `4`) | 85 |
| Gauge | `craft+0x7da0 = min((n - (hi - lo)) / (hi - lo), 1)` with `lo = 4`, `hi = maxNumPadsToCollectEMP = 8`; `0` below `lo` | 92 (live, every `n` from 0 to 39) |
| Cues | `EMPREADY` on the first charge at or over `lo`, `EMPFULL` on the first at gauge `1`; both re-arm while under `lo` | 80 |
| Fire | `controls+0x1c` (the EMP button, **L1** on the default pad) while ready, mode 14 only: launch, then `n = 0`, gauge `0`; the cannon's clip is not refilled | 85 (live) |
| Wave extent | `minDistance + (maxDistance - minDistance) * gauge` = `0.25 .. 2.0` at gauge `0 .. 1`: **seconds the wave lives**, while it walks `speed` (`1`) AI-track points a frame, about `6` units each ([below](#the-wave-a-repulser-wave-that-clears-mines)) | 88 (live) |
| Look | the disc's `WO_QUAKE_DETONATOR` (a blue `WO_QUAKE`) on the wave's front, scaled to the road, and a `(0, 5, 10)` point light; the stepper is the Repulser's, copied ([below](#what-hd-draws-for-a-fired-emp-2026-11-24)) | 85 (live) |
| Wave kill | every mine and bomb in each track section the wave sweeps, through `MineManager_DestroyAtLane` / `BombManager_DestroyAtLane` (score `DetonatorScoring` `<Emp>`, counted in `rm+0x6de3c`) | 80 (live: 12 mines, `+25` each, over 1.8 s) |

## The table: `WeaponStats_ParseEmp` (`0x0002bbc8`)

The only reader of the strings `minNumPadsToCollectEMP` (`0x0077a940`) and `maxNumPadsToCollectEMP`
(`0x0077a958`) (`scripts/ps3-toc.py attrib`). Under the `Stats` element (TOC `-0x78b0`) it stores:

| Attribute (TOC) | Offset | Type | HD `DATA00` |
| --- | --- | --- | --- |
| `wave_time` (`-0x787c`) | `+0x1b4` | f32 | `1.0` |
| `speed` (`-0x7878`) | `+0x1b8` | int | `1` |
| `minNumPadsToCollectEMP` (`-0x7874`) | `+0x1bc` | int | `4` |
| `maxNumPadsToCollectEMP` (`-0x7870`) | `+0x1c0` | int | `8` |
| `minDistance` (`-0x786c`) | `+0x1c4` | f32 | `0.25` |
| `maxDistance` (`-0x7868`) | `+0x1c8` | f32 | `2.0` |

Values from `/data/xml/weaponstats_detonator.xml` (`<Weapon type="EMP"><Stats wave_time="1.0"
speed="1" minNumPadsToCollectEMP="4" maxNumPadsToCollectEMP="8" minDistance="0.25"
maxDistance="2.0"/>`). `oag_tables::weapons::DetonatorStats::emp` already decodes all six.
Confidence **85** (every store and name read; the table is the class-indexed weapon table that
`Detonator_UpdateRace` and `Ship_UpdateCannonTrigger` both reach as `*(T + 4 * *(T + 0xc))`).

## Pads charge, they do not boost

### Speed pads: `Craft_ApplySpeedPad` (`0x000eeb70`)

On a tick where the pad test (`0x000b6f68`, the craft against the speed-pad list for its section
`ship+0x6284`) hits, with the net byte clear and the game mode `0xe`:

```text
0x000eee7c  lwz r10,0x70(r1)          ; the pad hit
0x000eee80  lwz r0,0x274(r31); cmpw  ; craft+0x274: the pad it was on last tick
0x000eee88  beq 0x000eec88            ; same pad: return
0x000eeea4  stw r10,0x274(r31)        ; remember the pad
0x000eeeb4  oris r0,r0,0x800          ; ship+0x6a64 |= 0x8000000: the EMP charge flag
0x000eeebc  stw r0,0x6a64(r11)
0x000eeec0  bl SoundEmitter_PlayCue   ; ship+0x5ee0, "WEAPONPICKUP" (TOC -0x4320), 1.0
0x000eeec8  b 0x000eec88              ; the epilogue
```

`0x000eec88` is the function's epilogue (`ld r0,0xe0(r1)` ... `blr`). Ghidra's decompile shows the
cue call as not returning and stops there; the disassembly continues to the epilogue. Every other
mode instead loads the boost timer `craft+0x2e8` and strength `+0x2ec` from the class tables, and
the force is only applied while `+0x2e8 > 0` (`0x000eebec`-`0x000eebf8`); the Detonator arm sets
neither and returns before the force. A miss stores `+0x274 = 0` (`0x000eebe4`), so leaving a
pad and re-entering it charges again. **A speed pad gives no speed in Detonator.** Confidence
**80** (one arm, every instruction read; the mode literal is a `cmpwi 0xe`).

### Weapon pads: `Ship_TestWeaponPad` (`0x000d4078`)

Per tick, while `ship+0x7002` is clear, when the pickup slot `*(ship+0x5edc)` is empty
(`+0x204 == -1`) or, in mode `0xe`, holds the Cannon (`+0x204 == 3`, what `CannonManager_ArmSlot`
sets, [detonator-cannon.md](detonator-cannon.md)), the weapon-pad test (`0x000b7c20`, against
`ship+0x6284` and the last pad `ship+0x7960`) hits and the slot's `+0x200 & 0x800` is clear:

- `ship+0x7a64 = 1`, the pickup sound for the player, and `slot+0x20c = 1` (`0x0012d7c0`: a
  pending pickup);
- if the pad differs from `ship+0x7960`: `ship+0x7960 = pad`, and in mode `0xe` with the net byte
  clear `ship+0x6a64 |= 0x8000000` (`0x000d4260`).

`ship+0x7960` is cleared on every forward start-line crossing (`stw r0,0x7960(r31)` with `r0 = 0`
at `0x000e4fb0`, inside `Craft_UpdateLapProgress`) and at construction (`0x000df2e0`,
`0x000e1324`), so **a weapon pad charges once per lap unless another weapon pad is taken between**.

The pending pickup is spent in `WeaponManager_DispatchFire` (`0x0012b2c0`): at `0x0012b554`-`0x0012b568`
it skips the award call `WeaponManager_RollPickup` (`0x0012a3e8`, the weighted `Libc_Rand` roll
over the class's per-weapon odds that arms the slot with a weapon) when the net byte is clear and
the mode is `14`, and clears `slot+0x20c` either way. **A weapon pad hands out no weapon in
Detonator.** Confidence **75**.

### Do the circuit's own pads still draw?

**No, on all three titles; HD 80, Omega 65, 2048 65.** Measured 2026-11-24.

**The code.** Before it loads `padReplacement` and the `Detonator_Pad` models,
`WorldManager_Construct` (`0x000b8590`) runs, for modes `0xd`, `0xe` and `0x15` only, a loop
that calls vtable slot 13 (`+0x34`) on every entry of the two pad lists (`this+0x4c`, count
`+0x114`: the speed pads; `this+0x118`, count `+0x1e0`: the weapon pads) and then zeroes both
counts. Slot 13 of `WeaponPad_Importer_Vtable` (`0x006b5818`) and of `SpeedupPad_Importer_Vtable`
(`0x006b5f78`) is the deleting destructor (restore the vtable, base destructor `0x002bec80` /
`0x003265b8`, free `0x006761f8`). So the circuit's own pad objects are deleted, and
`WorldManager_CountPads` then counts the pads of `padReplacement.vex`, whose `.rcsmodel` has no
mesh at all (`docs/formats/rcsmodel.md`, 128 bytes, count 0). The `Detonator_Pad` (or, in modes
`0xd`/`0x15`, `Zone_Battle_Pad`) is then the only model on each pad.

**Same shape on the other two.** Omega `eboot.bin` `FUN_01393610` (string `0x018175f7`): the
mode gate is `0x206000 >> mode & 1` (modes 13, 14, 21), three lists (`+0x88`, `+0x3b0`,
`+0x6d8` words, counts at `+0x3a8`/`+0x6d0`/`+0x1358`), vtable slot `+0x50`, counts zeroed, then
`padReplacement`. 2048 v1.04 `FUN_8121e862` (string `0x814aa45c`): modes `0xe`/`0xd`/`0x15`,
three lists, vtable slot `+0x28`, counts zeroed, `padReplacement`, then `Detonator_Pad` for each
pad of the first two lists (mode 14) or `Zone_Battle_Pad` (modes 13, 21). The destructor targets
of those two were not read (only the HD pair above).

**The frame.** The stored RPCS3 grab of the HD `pad` pair (Talons Junction, the pad two craft
lengths ahead) shows the dark floor and nothing but the `Detonator_Pad` blocks. Ours with the
circuit's pad left drawn (hide switched off for the render) adds the Talons pad's cyan chevrons
and dark feet round and between the blocks, which the original frame lacks; the floor is not dark
enough to hide them. With the hide on, ours matches. One pose, one grab (80, not repeated).

**What is not proven.** The destroy loop removes the pad *objects*. HD's four original circuits
and Omega's HD-lineage circuits draw their pads as mesh shapes (`weapon_pads` material chunks, and on
Omega `tracksurface:speedup_pad_*` shapes on the start-line material), which the loop does not
obviously own; the frame is what settles HD, and Omega and 2048 inherit it. No original frame of
Omega or 2048 was taken (their Detonator mode needs HD's `Data\New_Mode_Pads` files and neither
emulator was driven to a pad).

**Wired** as `oag_title::DetonatorPads::replaces_circuit_pads` (`true` on HD, Omega, 2048): HD
drops the pad-material chunks of the world-space pass and the separate pad models
(`mesh::rcs::build_scene_views`' `hide_pads`; `zone_1` exactly 2093 triangles,
`detonator_hull_ground_truth.rs`); Omega and 2048 drop the circuit's `weapon_pads` and `speed_pads`
material submeshes and every shape named `speedup_pad` or `weapon_pad`
(`mesh::rcs::psp2::is_circuit_pad`; `psp2_circuit_pads_ground_truth.rs` holds the triangles
dropped equal to the file's own pad triangles on Omega's `talons_junction` and `arena` and 2048's
`altima`). A normal race on the same circuits is pixel-identical before and after. Modes 13 and 21
(Zone Battle) run the same loop but are not wired.

## Live

Run 1 (the setup on [detonator-speed.md](detonator-speed.md#live-rpcs3-2026-11-22)): every rise
of `rm+0x2dd8` was attributed to the pad that caused it, from `craft+0x274` (speed pad) and
`ship+0x7960` (weapon pad) in the samples before it.

| Read | Value |
| --- | --- |
| speed pads | four per lap on Vineta K (`0x30624060`, `0x306a5960`, `0x306237c0`, `0x30625ee0`), 37 observed entries, each followed by one charge, plus one charge whose entry fell between two 30 ms polls; `craft+0x274` back to `0` after each |
| weapon pad | one charge under player input (`ship+0x7960` `0 -> pad`), cleared to `0` on the next crossing; none under the Autopilot (its running bit `slot+0x200 & 0x800` is the test's own exclusion) |
| `craft+0x2e8` | `0.0` throughout: no boost timer on any pad |
| slot `+0x204` | `3` (the Cannon) for the whole race |
| gauge `ship+0x7da0` against `n` | `0..4` -> `0.0`, `5` -> `0.25`, `6` -> `0.5`, `7` -> `0.75`, `8..39` -> `1.0` |
| the flag | `ship+0x6a64 & 0x8000000` never seen set (set and cleared inside one frame) |

Run 2, L1 pressed for 0.5 s with `n = 12` (gauge `1.0`), the Autopilot bit cleared: `n` read `0`
and the gauge `0.0` on the next sample; `rm+0x6de3c` went `3 -> 15` and the score `150 -> 450`
(`+25` per mine, `<Emp> old_mine_score`, the dark-mine value; no live-mine `+50` was seen)
over the next 1.8 s; the frame after the press shows a blue wave ahead of the craft; the clip
read `09` before and after. Triangle and R1, pressed first with `n = 12`, changed nothing; R2 and
L2 came after the fire with `n` at 0 and 1, below `lo`, so they test nothing.

## The charge and the gauge: `Detonator_UpdateRace` (`0x00067b40`)

```text
if ship+0x6a64 & 0x8000000:                            ; 0x00067bb0
    rm+0x2dd8 += 1                                     ; 0x00067bc0-0x00067bcc
    ship+0x6a64 &= ~0x8000000                          ; 0x00067be8
    lo = table+0x1bc; hi = table+0x1c0                 ; 0x00067bf0, 0x00067bf4
    if rm+0x2dd8 < lo:
        rm+0x2e1e = 1; craft+0x7da0 = 0; rm+0x2e1d = 1 ; 0x00067c08-0x00067c10: re-arm both cues
    else:
        span = hi - lo
        g = f32(rm+0x2dd8 - span) / f32(span)          ; int to double, divided, rounded to f32
        if g > 1.0: g = 1.0                            ; TOC -0x68ec = 1.0
        craft+0x7da0 = g                               ; 0x00067d64, the craft is `rm+0x13e8`
        if g >= 0.0 and rm+0x2e1d: cue EMPREADY; rm+0x2e1d = 0
        if g >= 1.0 and rm+0x2e1e: cue EMPFULL;  rm+0x2e1e = 0
```

Cue ids: TOC `-0x67bc` `EMPREADY`, `-0x67b8` `EMPFULL`, through `Sound_QueueAnnouncerCue`. **Keep
the law literal**: `(n - (hi - lo)) / (hi - lo)` equals `(n - lo) / (hi - lo)` only because the
disc's `hi - lo == lo` (`8 - 4 == 4`); a table with other values gives a different gauge.
With the disc: `n = 4` gauge `0` and `EMPREADY`, `5` 0.25, `6` 0.5, `7` 0.75, `8` `1.0` and
`EMPFULL`; more pads keep it at `1`. The HUD's `Emp_0..7` pips read `rm+0x2dd8`
([detonator-hud.md](detonator-hud.md)). Confidence **82** (the decompile and the disassembly
agree; the flag's two writers are above).

The other `rm+0x2dd8` writers are mode-manager constructors and resets (`0x000648a0`,
`0x00064e20` in the two Detonator constructors' span; `0x00069b70`, `0x0006a2d8`, `0x0006c390`,
`0x0006c3e0`) and the fire below. `rm+0x2dd8` is a separate field from the score.

## Fire: `Ship_UpdateCannonTrigger` (`0x000e9cd0`), the `+0x1c` arm

When the controls' byte `*(craft+0x84)+0x1c` is set, the net byte is clear and the mode is `0xe`:

```text
if rm+0x2dd8 >= table+0x1bc:
    EMPManager_Fire(craft+0x7da0, wm+0x84, wm+0x90 /* slot 0 */, 0)
    CannonManager_ArmSlot(wm+0x70, wm+0x90)    ; re-arms the slot as the Cannon (+0x204 = 3) that
                                               ; EMPManager_Fire set to -1; slot+0x1f4 kept
    craft+0x7da0 = 0
    rm+0x2dd8 = 0
    lockout+0x16c = 0; lockout+0x174 = 1        ; the object at TOC -0x11dd, unread
```

`wm` is the weapon manager (`*TOC[-0x110b]`, slots of `0x230` from `+0x90`,
`WeaponManager_DispatchFire`). Under `0x1c`, a held button fires again as soon as `n` is back at
`lo`; whether `+0x1c` is a level or an edge is the controls' (unread). The HUD clip read `09` both
before and after a live fire, so the re-arm is the slot's weapon id, not a reload. Confidence **85**.

`EMPManager_Fire` (`0x00112730`; `EMPManager.cpp` spans `0x00112bf8`-`0x00113288` by its
`__FILE__`, this sits just before its constructor's span):

```text
slot+0x204 = -1; mgr+0x34 |= 2; slot+0x200 &= ~0x100000
if mgr+0x68 == 0:                                 ; one EMP at a time
    slot+0x1f8 = table+0x1b8                      ; speed (1)
    emp = mgr+0x64; emp+0x40 = 1; emp+0x44 = arg4; emp+0x48 = ++serial
    EMP_Launch(minDistance + (maxDistance - minDistance) * gauge, emp, arg4, slot+0xf0's ship, ship+0x5ee0)
    mgr+0x68 += 1
```

`EMP_Launch` (`0x0013c430`, in `EMP.cpp`'s span `0x0013b220`-...): `emp+0x54 = extent`,
`emp+0x44`, `emp+0x150 = ship`, `emp+0x158 = 0`, `emp+0x34 |= 6`, and the `QUAKELAUNCH` cue on
the ship's emitter. So **the EMP is a Quake-family wave** ([quake.md](quake.md)) whose length is
set by the gauge. Confidence 70 (fire path read; `wave_time` and `speed`'s units are unread).

## The wave's kill: `EMP_SweepSections` (`0x0013b930`)

Recursive over the track's section graph: it steps section by section (`0x000a54b8` forwards,
`0x000a5698` the other way, `param_7` steps left), and for each section it reaches looks up the
Detonator placement record for that section in the race manager's list (`rm+0x6de50`, count
`rm+0x6de4c`, stride `0x30`, `+0x18` the section, `+0x00` the lane count, `+0x04` the lane types):
**every lane of that section is destroyed**, a type `0` through `MineManager_DestroyAtLane`
(`0x0011a590`), a type `1` through `BombManager_DestroyAtLane` (`0x0012e260`). Those two score
through `DetonatorScoring`'s `<Emp>` record (`new_mine_score 50`, `old_mine_score 25`, `bomb_score
1000`, [detonator-cannon.md](detonator-cannon.md)) and the mine one counts into `rm+0x6de3c`.
The same lookup runs again for a neighbouring section (`0x000a96d8`), and a fork (`iVar11 == 1`)
spawns a second branch record. Ghidra's body stops at `0x0013c31f` on an AltiVec word.
Confidence **70** for "every mine and bomb in each swept section", **50** for how far it sweeps
per tick.

## What the wire lane needs

1. **Pads** (`oag_raceplay::pads`): in Detonator a speed pad applies no boost (no force, no
   timer) and a weapon pad awards nothing; each raises one charge: a speed pad once per entry
   (re-armed when the craft is off every pad), a weapon pad once per pad, its memory cleared on
   each start-line crossing. Cue `WEAPONPICKUP` on the speed-pad charge.
2. **Gauge**: `n` (race state, reset by fire), `gauge = 0` while `n < lo`, else
   `min((n - (hi - lo)) / (hi - lo), 1)` in f32 from the table's two ints; `EMPREADY`/`EMPFULL`
   edge cues with the two latches re-armed below `lo`. HUD: `Emp_0..7` from `n`, `EmpReady`.
3. **Fire**: a Detonator-only EMP button (the original's controls `+0x1c`, L1 on the default pad,
   live), `n >= lo`: launch with `extent = 0.25 + 1.75 * gauge`, `n = 0`, gauge `0`,
   `QUAKELAUNCH`; the cannon's clip is untouched.
4. **Kill**: every mine and bomb in the swept track sections, scored `<Emp>`, mines counted.
   **Unread**: the wave's reach in sections per unit of `extent`, its speed (`speed = 1`,
   `wave_time = 1.0`) and whether it sweeps forward only. Recipe: `EMP_SweepSections`'s caller
   (`EMP.cpp`'s update, the vtable of the object `EMP_Launch` arms) for how `emp+0x54` becomes
   `param_7`; or live: an RPCS3 breakpoint on `MineManager_DestroyAtLane` (`0x0011a590`) after an
   EMP fire, logging the section index against the craft's `ship+0x6284`.

## Wired (2026-11-22)

`oag_weapons::detonator::emp` holds the law; `oag_raceplay::detonator::advance_emp` runs it once a
tick after the cannon; the pad arms are in `oag_raceplay::pads`.

| Piece | Where | Note |
| --- | --- | --- |
| Speed pad | `Race::test_speedup_pads` | no force, no `pad_timer`, no plume or `SPEEDUPPAD`; the entry edge raises `Emp::pending` and plays `WEAPONPICKUP` (`Cue::WeaponPickup`) |
| Weapon pad | `SimRace::test_weapon_pads` | the race keeps its weapon pads with weapons off where the title's pads charge (`start.rs`); no grant; one charge per pad held in `Emp::weapon_pad`, cleared on every forward crossing (`player_step`); not while this project's autopilot flies the craft (it stands in for the original's Autopilot bit, **chosen**) |
| Charge, gauge, cues | `emp::step_charge`, `emp::gauge` | the literal law in `f32` (exact on every integer charge the HUD reaches); `Cue::EmpReady`/`EmpFull` loaded from `oag_title::DetonatorVoices` (HD `speech_det.bnk`, the one speech bank that names them) |
| Fire | `emp::fire` | `Button::Flip` (the pad's L1) as a press edge, **chosen**; gated as the cannon is (a person flying, past the countdown); the charge resets even while a wave runs |
| Wave | `emp::step_wave` | forward from the craft's nearest survey entry; ~~`extent * SECTIONS_PER_EXTENT` (40, chosen) entries over `wave_time`~~ one entry a tick while `age < extent` since 2026-11-24 (below); `<Emp>` scores; mines counted into the destroyed count; a bomb chains nothing |
| HUD | `oag_hud::detonator` | `Emp_0..7` while the charge is over `i`, `EmpReady` at `lo`; `EmpBarEffect` not drawn (an `<ImageModel>`) |
| Titles | `oag_title::Detonator::pads` | HD measured; Omega the speed pad measured, the weapon pad inherited from HD; 2048 `false` (its pads keep boost and pickup, so its EMP never charges) |

Tests: `crates/weapons/src/detonator/emp/tests.rs` (the live gauge table for `n = 0..39`, the
literal law, the cue edges, fire, the wave's `+25`/`+50`/`+1000`),
`crates/gameplay/src/hash.rs` (`the_emp_moves_the_hash_only_where_detonator_armed_it`), and
`crates/raceplay/tests/detonator_ground_truth.rs`: a lap's speed-pad entries each add one charge
and leave `pad_timer` at `0` (five on the default circuit); a weapon pad charges once, hands out
nothing and charges again only on another pad; a full EMP fired from a stand clears 11 mines for
`275` dark and `550` live; the three cues decode on HD. Omega: a lap charges six and never
boosts; 2048: a speed pad charges nothing and keeps its boost. The HUD pips are in
`detonator_hud_ground_truth.rs`.

## Open

- ~~The wave's reach and speed~~ (read and wired 2026-11-24, below). Whether it hurts the player.
- Whether the circuit's own pad geometry is drawn at all: four live frames show none (above);
  wired as not drawn at 60.
- Weapon pads under player input over several laps (one live charge only).

## What HD draws for a fired EMP (2026-11-24)

Lane `det-emp-look-re`. Static on `EBOOT.elf` (`llvm-objdump` over `EMP.cpp`'s span
`0x0013ae00`-`0x0013c700`, every TOC load resolved with `scripts/ps3-toc.py resolve 0x0013b220`),
then live on RPCS3 (Talon's Junction Detonator grid state, charge `12` and gauge `1.0` written,
L1 tapped at lap clock 6.0; `scripts/rpcs3-hd-detonator-shots.py --emp-charge 12 --emp-poll 2.6`,
103 paused samples of the `EMP` object, and four frames at lap clock 6.3, 6.8, 7.4 and 8.0).

**The answer.** A fired EMP is **the disc's particle system `WO_QUAKE_DETONATOR`** riding a
matrix that walks the AI track one point per frame. The matrix sits at the midpoint of the
track's two edges, its `Y` the track's up, and the instance's extent co-factor is the edges'
distance over `100`. One SPU point light rides the same centre: colour `(0, 5, 10)`, range `50`,
exponent `1`. No model, ribbon or shader of its own. Confidence **85** (static, plus the live
step, lifetime and colour below, plus four frames).

### The code is the Repulser's wave, the data is the Quake's

The maintainer asked whether the EMP shares code or data with Eliminator's Repulsor.
**Shared code by copy, not by call; no shared data.**

- **Code.** `EMP_SweepSections` (`0x0013b930`) is a clone of HD's Repulser wave stepper
  (`0x00157b20`, the twin of Pulse's `Repulser_AdvanceWave`,
  [repulser.md](../psp-pulse-usa/repulser.md)). `diff_functions` scores the two `0.53` similar.
  Both read the AI-track point's two half-widths (`lfs 0x44`, `lfs 0x48`) and average the edges.
  The fork block is byte for byte the same apart from the effect name: allocate a `0x180`-byte
  `LinkObj`, spawn by pointer, store the handle, copy the cursor. **The fork's spawn keeps the
  Repulser's fourcc `REP2`** (`0x32504552`, `lis r5,0x3250; ori r5,r5,0x4552` at `0x0013c16c`),
  next to the EMP's own `EMP1` (`0x0013afe8`) and `EMP2` (`0x0013b5e8`). The EMP calls nothing
  in `Repulser.cpp` (`0x00156a80`-`0x00158f00`) or `RepulserManager.cpp`, and loads no `.vex`
  (`pulse_repulsorwave.vex` is the Repulser's alone, [weapons.md](weapons.md)). What the copy
  adds: the per-point lookup of the Detonator placement records and the two `DestroyAtLane`
  calls.
- **Data.** `WO_QUAKE_DETONATOR` (HD `DATA00.PSARC /data/psys/wo_quake_detonator.pob`) is
  **`WO_QUAKE` recoloured blue**: the same root (`shape 1`, extent `30.196`, size `3..10`,
  alpha to `64`) and the same `molten_debris_colliding` and `debris` emitters. Where the Quake has
  `FIRE_STREAKS` (yellow fireball) and orange `SPARKS`, it has `EXPLOSIONS`
  (`explosion_fijets_8x8_grey`, colours `(67,167,255)` to `(62,90,255)`), blue `SPARKS`, and an
  extra `BLUE_GLOWS` (`blue_glow2`, size to `25`, the only local-space emitter, flag `0x2`).
  The smoke texture is the white variant. It shares **no texture** with `WO_REPULSER`
  (`pulse_repulsor_sprite1_ADD`) or `WO_REPULSER_BLAST` (`DC_Circle_rich`). The travel cue is
  `~MAGPULSETRAVEL`, not `~REPULSORTRAVEL`; the launch cue is `QUAKELAUNCH`.

So the play feel (a wall that runs down the track, forks at a split, sweeps what it passes) is
the Repulser's, and the look is a blue Quake.

### The wave: a Repulser wave that clears mines

`EMPManager_Update` (`0x00112e50`, `EMPManager`'s vtable `0x00863ac0` slot 3) calls, for each
live `EMP`, `EMP_UpdateWave` (`0x0013c320`), and resets and frees the EMP when it returns `0`:

```text
EMP_UpdateWave(emp, dt):
    emp+0x158 += dt                                  ; age, 0x0013c358-0x0013c394
    emp+0xf0 = emp+0xa0 ; emp+0x100 = emp+0xe0       ; last tick's two wave centres
    EMP_SweepSections(emp, cursor = emp+0x168, matrix = emp+0x70, handle = emp+0x5c, 0,
                      emp+0x190, steps = table+0x1b8 /* speed, 1 */, init = 0)
    travel emitter emp+0x4c
    return emp+0x158 < emp+0x54                     ; alive while age < extent
```

`EMP_Launch` arms the same call with `init = 1` (a tail `b 0x0013b930` at `0x0013c5b0`), after
`EMP_SpawnLaunchEffect` (`0x0013afe8`) copied the firing ship's AI-track cursor
(`ship+0x7820..+0x782c`) to `emp+0x168` and spawned the effect. `wave_time` (`table+0x1b4`) is
read nowhere in `EMP.cpp` or `EMPManager.cpp`.

**So `extent` is a lifetime in seconds and `speed` is AI-track points per frame.** Live, every
sample (confidence **88**):

| Read | Value |
| --- | --- |
| `emp+0x54` | `2.0` at gauge `1` (`minDistance + 1.75 * gauge`) |
| `emp+0x158` | grows by the frame's `dt` (`0.0167`, `0.0167`, `0.026`, `0.033` ... under the emulator's uneven frames) |
| cursor `emp+0x170` | the point index, `+1` every frame (`74`, `75`, `76`, ...), the point pointer `+0x70` with it (the AI track's stride) |
| wave centre `emp+0xa0` | `(466.3, -30.4, -67.2)` on the launch frame, 19 units ahead of the craft, then about `6.0` units per frame |
| end | age `2.0` reached at point `144`: `70` frames at the emulator's ~35 fps |

At HD's 60 frames a second a full EMP walks `120` points, about `720` units at `6` units a point.
The step is per frame and the lifetime per second, so the reach follows the frame rate; this
engine's fixed 60 Hz tick is the console's target. This engine's Detonator survey is one entry per
control point, `862` entries a lap on Talon's Junction, median spacing `5.99` units: the same
points (`oag_raceplay`'s `survey`).

### What it spawns and how it is posed

| Piece | Where | Read | Confidence |
| --- | --- | --- | ---: |
| `WO_QUAKE_DETONATOR`, fourcc `EMP1` | `EMP_SpawnLaunchEffect` `0x0013afe8` | `0x002c9108(obj, name, 'EMP1', emp+0x70, 1, 0)`: attached **by pointer** (`li r7,1`), so flag-`0x2` emitters ride the matrix; handle at `emp+0x5c` | 85 |
| the same, fourcc `EMP2` | `EMP_ForkAtJunction` `0x0013b5e8` | the launch call's fork (the Repulser's init-tick fork), at `emp+0xb0`, handle `emp+0x60`, cursor to `emp+0x178` | 60 |
| the same, fourcc `REP2` | inside `EMP_SweepSections`, `0x0013c170` | a fork met while walking: the same matrix, handle and cursor slots | 70 |
| pose | `EMP_SweepSections`, `0x0013bc00`-`0x0013bf78` | translation = midpoint of `pos -+ lateral * half_width` (`point+0x44`, `+0x48`); then `|edgeA - edgeB| * 0.01` (`0x3c23d70a`) into the instance through `0x002846d0` | 75 |
| light | `EMP_DrawLights` `0x0013b300` (vtable `0x00864348` slot 5) | `SpuLight_AddCandidate(emp+0xa0, *(vec4 *)0x00994490, 50.0, 1.0)` (`0x008c1a50`); the fork's at `emp+0xe0` when `emp+0x60` is set | 85 |
| light colour | `0x00994490` | `(0.0, 5.0, 10.0, 10.0)` on every live sample | 88 |
| kill effects | `MineManager_DestroyAtLane` / `BombManager_DestroyAtLane` | the mine's own `DetonatorMine_Explode` (live mine `WO_MINE_EXPLO_DETONATOR`, dark `_DEAD`, `MINEEXPL`) and the bomb's blast (`WO_BOMB_EXPLO_DETONATOR`, `BOMBEXPL`), [detonator-mine.md](detonator-mine.md), [detonator-bomb.md](detonator-bomb.md) | 70 |
| reset | `EMP_ResetWave` `0x0013ae68` | both handles released through `0x0027f7b0(mgr, handle, 1)`, the matrices back to identity | 70 |

`EMP_Rays`, the other EMP string, is a parameter of the HUD's `EmpBarEffect`
(`DetonatorHud_BindEmpWidgets`, [detonator-hud.md](detonator-hud.md)), not a world effect.

### The frames

Original, lap clock (frames after launch): 6.3 s (8): a wall of blue smoke spanning the whole
road just ahead, the scene around it washed cyan, four mines still glowing inside it. 6.8 s (24):
the wall about 100 units ahead, still the road's full width. 7.4 s (45): the wall far ahead,
`+50`/`+100`/`+200` score pops behind it, score `500`. 8.0 s (67): the wave gone over the rise,
mine debris falling, score `1050`. Ours, the same frame counts after an L1 press with
`--detonator-emp-charge 12`: the same blue wall and cyan wash at 8; at 24, 45 and 67 the wall
runs on and the mines behind it go. Ours is about 2x as far ahead at 24 and 45 frames, since the
emulator ran at ~35 fps and ours at 60. **What differs:** ours leaves rows of faint grey particles
on the road along the wave's path (the five world-space emitters' bursts, one row per tick across
the extent), which the original's frames at the same frame counts do not show. Not explained: the
emitters are world-space in the file (flags `0x02500099`, `0x84104205`, `0x8010020d`, `0x86000005`,
`0x80000001`), and the extent spread is not the cause (switched off, the frame is the same).

## Wired: the look (2026-11-24)

| Piece | Where |
| --- | --- |
| The wave's law | `oag_weapons::detonator::emp::step_wave`: `speed` entries a tick while `age < extent`; `extent` is the launch's lifetime. Replaces the chosen `SECTIONS_PER_EXTENT` and `wave_time` share |
| The effect | `Trigger::DetonatorEmp` (`oag_title::engine_effects::DETONATOR_EMP_EFFECT`, `WO_QUAKE_DETONATOR`), HD's table only; `RaceView::advance_emp_visual` attaches it on the front's midpoint, `Y` the track's up, extent co-factor `width / 100`, its local-space emitters riding; released when the wave ends. In HD's `SPRITES` and `EXTENTS`, as the Quake's wave is |
| The light | `weapon_light::detonator_emp_record`, `(0, 5, 10)`, `D = 50`, `w = 1` at the effect's anchor, beside the cannon's lights |
| The kills | `MineEmped` plays the mine explosion by liveness, `BombEmped` the bomb's; `MINEEXPL`/`BOMBEXPL` with them |
| Captures | `--detonator-emp-charge N` (`oag-game`) starts the race at `N` charges; the shots script's `--emp-charge`, `--emp-poll` |

Not wired: the fork (this engine's survey is one route); the travel cue `~MAGPULSETRAVEL`.

Tests: `emp::tests::the_wave_steps_one_entry_a_tick_for_extent_seconds` (15 entries at gauge `0`,
120 at `1`), `tests::detonator_fx::an_hd_emp_wave_carries_the_discs_effect_on_its_front_until_it_ends`
(the disc's effect rides the front, lights it, and is released with the wave),
`tests::detonator_fx::each_event_plays_the_effect_the_original_spawns_for_it` (the kill rows),
`weapon_light::tests::the_detonator_lights_are_the_values_the_render_queue_loads`.
Regression pairs, re-rendered: `pad` 8.17 (8.17 before), `mines-ahead` 14.65 (14.65).

### 2048 and Omega

- **2048: checked, applies, not wired.** `eboot-vita-2048-eu-v104.elf` names `Backend/Weapons/EMP.cpp`,
  `WO_QUAKE_DETONATOR` (spawned at three sites, `FUN_811e7c68`, `FUN_811e7dd6`, `FUN_811e841c`,
  as on HD) and `~MAGPULSETRAVEL`. `FUN_811e7dd6` is the same sweep: the `REP2` fourcc
  (`0x811e804c`-`0x811e8056`), the half-widths `+0x44`/`+0x48` averaged, and the width times
  `0x3c23d70a` into the instance (`0x811e83f4`). `data.psarc` ships `WO_QUAKE_DETONATOR` in
  `data/particles` and `data/particles2048`, emitter for emitter HD's. Not wired: 2048's pads
  never charge the EMP in this engine (`oag_title::Detonator::pads`), and its effect table
  answers none of the Detonator triggers.
- **Omega: checked, applies, not wired.** `eboot.bin` (1.07) names `EMP.cpp`, `EMPManager.cpp`,
  `WO_QUAKE_DETONATOR` and `_MAGPULSETRAVEL`; `data00.psarc` ships the file in both particle
  directories with HD's emitters. Its sweep was not read. Not wired: Omega's effect table answers
  none of the Detonator triggers yet (as for the four explosions, [pob.md](../../../formats/pob.md)).

## 2048 and Omega

- **Omega: checked, applies, not wired.** `/omega/eboot-ps4-omega-eu-v107.bin`: the Detonator
  update holding `EMPREADY` (`FUN_012b6c70` in this import) tests `ship+0x76f4 & 0x8000000`,
  counts into `rm+0x300c0`, reads min/max at table `+0x234`/`+0x238`, computes
  `min((n - (hi - lo)) / (hi - lo), 1)` into `ship+0x8c0c` and raises `EMPREADY`/`EMPFULL` on the
  same two latches. The speed-pad arm is in the craft update `FUN_0133fff0`: on a pad hit with the
  mode `0xe`, a pad other than `craft+0x2f0` sets `ship+0x76f4 |= 0x8000000` and plays
  `WEAPONPICKUP`. Omega's weapon-pad arm, fire and wave were not read.
- **2048: checked, applies, not wired.** `/2048/eboot-vita-2048-eu-v104.elf`: `FUN_8117e9c2`
  tests `ship+0x60a4 & 0x8000000`, counts into `rm+0x2b1f0`, reads table `+0x234`/`+0x238`, and
  computes the same gauge into `ship+0x7458`, with the divide done as a NEON reciprocal estimate
  and three Newton steps (so not bit-identical to an f32 divide), clamped to `1`, then
  `EMPREADY`/`EMPFULL`. Its pads, fire and wave were not read; its HUD widgets differ
  (`EmpCharge`/`EmpFull`, [detonator-hud.md](detonator-hud.md)).

## Names

| Address | Name | Confidence |
| --- | --- | ---: |
| `0x0002bbc8` | `WeaponStats_ParseEmp` | 85 |
| `0x000d4078` | `Ship_TestWeaponPad` | 72 |
| `0x0012a3e8` | `WeaponManager_RollPickup` | 75 |
| `0x00112730` | `EMPManager_Fire` | 72 |
| `0x0013c430` | `EMP_Launch` | 68 |
| `0x0013b930` | `EMP_SweepSections` | 75 |
| `0x00112e50` | `EMPManager_Update` | 78 |
| `0x0013c320` | `EMP_UpdateWave` | 85 |
| `0x0013afe8` | `EMP_SpawnLaunchEffect` | 82 |
| `0x0013b300` | `EMP_DrawLights` | 82 |
| `0x0013b5e8` | `EMP_ForkAtJunction` | 60 |
| `0x0013b220` | `EMP_Construct` | 80 |
| `0x0013ae68` | `EMP_ResetWave` | 70 |

`EMP_SweepSections` rises from 62: its caller is read (`EMP_UpdateWave` and `EMP_Launch`'s tail
call) and live it steps one point a frame. `EMP_ForkAtJunction` is named for its Repulser twin and
held under 70 (never seen to run: Talon's Junction forked nowhere in the run). `EMP_Construct`:
the `"EMP.cpp"` tag at `+0x30`, the vtable `0x00864348`.

Existing names cited: `Craft_ApplySpeedPad` (`0x000eeb70`, 65, [craft-flags.md](craft-flags.md)),
`Ship_UpdateCannonTrigger` (`0x000e9cd0`, 68), `CannonManager_ArmSlot` (`0x0010f2e8`, 65),
`Detonator_UpdateRace` (`0x00067b40`), `MineManager_DestroyAtLane` (`0x0011a590`),
`BombManager_DestroyAtLane` (`0x0012e260`), `WeaponManager_DispatchFire` (`0x0012b2c0`).
