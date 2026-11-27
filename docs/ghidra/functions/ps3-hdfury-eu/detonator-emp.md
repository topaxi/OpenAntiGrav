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
| Titles | `oag_title::Detonator::pads` | HD measured; Omega and 2048 both arms measured (2026-11-25, [`vita-2048-eu-v104/detonator-pads.md`](../vita-2048-eu-v104/detonator-pads.md)) |

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
| the same, fourcc `EMP2` | `EMP_ForkAtJunction` `0x0013b5e8` | the launch call's fork (the Repulser's init-tick fork), at `emp+0xb0`, handle `emp+0x60`, cursor to `emp+0x178` ("The launch-time fork") | 80 |
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
runs on and the mines behind it go. The wave itself agrees per frame: the original's front covered
about 143 units in its first 24 frames (`(466.3, -30.4, -67.2)` to `(570.8, -17.3, 29.7)`), ours
24 survey entries of about 6 units. Equal frame counts are equal game time: the poll's wave age
and race clock step `1/60` a frame (`0.0167` after the first), however slowly the emulator ran in
wall time. The gap between the craft and the wave differs because ours fired 28 ticks after GO,
still on the auto-speed ramp. **What differed:** ours left rows of soft grey half-domes on the road
along the wave's path; the original's road behind the wave is clean. Found 2026-11-25, next
section.

Before this section, main drew nothing for the wave (`EmpLaunch` played no effect) and swept a
chosen `40 * extent` entries over `wave_time`.

## The grey rows were sparks that never moved (2026-11-25)

**Cause (confidence 80).** `SPARKS` (`0x80000001`: looping, world-space, class 6) authors speed
`(0, 0)`, 8 particles a tick, a 137-tick life and one modifier, type `0x11`. That modifier is a
Perlin-noise force on the velocity ([particle-noise-force.md](../psp-pulse-usa/particle-noise-force.md),
`Modifier_NoiseForce`); it is the only thing that moves these sparks. Our player applied drag and
the weather box and skipped every other modifier type, so each spark sat where it was born for
137 ticks, drawn as a zero-length streak: a row of grey half-domes per tick along the path.

How it was found: one render per emitter muted, then one per emitter kept alone (a scratch
switch, not committed), road region of the 24-frame shot, mean grey: all muted `13.5`, root
`WO_QUAKE_DETONATOR` alone `22.0`, `SPARKS` alone `17.3`, the rest `13.3` to `13.6`. The rows are
`SPARKS`'; the root adds the soft wash under them. Then `pob` dumps of the record's flags and
modifier list (`kind 17`, params `0.3, 0.1, 0.3, 1.0, 0.1, 0.1, 0.1, 0, 0`).

**HD has the same law.** `0x0027e118` (`Modifier_NoiseForce`, 72: Ghidra cuts the body at
`0x0027e193`, `llvm-objdump` reads on) loads the same two offset vectors through its TOC
(`0x008b28f0` to `0x0079b490`: `135.13, -123.45, 12.39` and `3931.49, 11241.24, -141.89`), takes
the doubled points (`fadds x, x, x`) before the timed branch, and calls the noise (`0x0027dbc0`)
six times in pairs. Omega (`eboot.bin` file `0x800db0`) and 2048 (`eboot.elf` file `0x29b47c`) carry the same
constants once each: **ported** (2026-11-26, "2048 and Omega draw it"): both titles' tables list
the EMP's effect in `turbulence`; their functions are unread beyond the constants. Not live: no
capture reads a spark's velocity.

**Wired.** `oag_fx::psys::turbulence` (the law, Perlin's `noise3` on a seeded table, the
particle system's own seconds clock) applies it for an effect a title lists:
`oag_title::Effects::turbulence`, HD `[WO_QUAKE_DETONATOR]`. The other 13 HD emitters that author
it (census on the Pulse page: the mine and bomb explosions, the trail hits, the rocket's trail,
`WO_PLASMA_FLASH` and others) are not listed: each changes a picture a pair covers and is a
decision against its own capture. The EMP's sparks also draw as HD's mode-5 bar
(`Effects::far_bars`, [engine-trail.md](engine-trail.md), "The class-6 draw law"): with their
velocity they are streaks, and against the procedural profile's soft blotches the bar's thin
streaks are nearer the original's white-blue spikes at the front.

**Frames** (scratch, read by eye): at 45 and 67 frames the road behind the wave is clean, as in
the original; at 24 a few streaks remain between the craft and the front, which ours sees from
further back than the original does (the craft gap above). The front's size and brightness are
not compared at 24 to 67 for the same reason: the original's craft is right behind the wall, ours
is far back. At 8 frames both are the same blue wall across the road. Still differs: the
original's spikes are denser at the front and bluer; ours are fewer and whiter. Not chased.

Tests: `oag-fx` `psys::turbulence::tests` (a zero-speed emitter drifts only with the force on: red
without the integrate hook) and `oag-raceplay`
`hd_effect_sprites::the_emp_sparks_feel_their_noise_force_only_where_the_title_lists_it` (disc).
Pairs: `pad` 8.17 to 8.17, `mines-ahead` 14.65 to 14.65.

## Wired: the look (2026-11-24)

| Piece | Where |
| --- | --- |
| The wave's law | `oag_weapons::detonator::emp::step_wave`: `speed` entries a tick while `age < extent`; `extent` is the launch's lifetime. Replaces the chosen `SECTIONS_PER_EXTENT` and `wave_time` share |
| The effect | `Trigger::DetonatorEmp` (`oag_title::engine_effects::DETONATOR_EMP_EFFECT`, `WO_QUAKE_DETONATOR`), HD's table only; `RaceView::advance_emp_visual` attaches it on the front's midpoint, `Y` the track's up, extent co-factor `width / 100`, its local-space emitters riding; released when the wave ends. In HD's `SPRITES` and `EXTENTS`, as the Quake's wave is |
| The light | `weapon_light::detonator_emp_record`, `(0, 5, 10)`, `D = 50`, `w = 1` at the effect's anchor, beside the cannon's lights |
| The kills | `MineEmped` plays the mine explosion by liveness, `BombEmped` the bomb's; `MINEEXPL`/`BOMBEXPL` with them |
| Captures | `--detonator-emp-charge N` (`oag-game`) starts the race at `N` charges; the shots script's `--emp-charge`, `--emp-poll` |

Not wired: the travel cue `~MAGPULSETRAVEL`. The launch-time fork is wired below ("The launch-time fork").

### The fork, wired (2026-11-25)

The fork is reachable on HD: a census of every HD circuit's `WO Track` (`oag_vex::track::from_vex`, 2026-11-24) finds one alternate successor on `zone_3`, `05_ubermall` (both directions) and `15_anulpha_pass` (forward), none on the other 24 files; `Course::branches` agrees (zone_3 one branch of 556 entries leaving at ring entry 9 and rejoining at 0; 05_ubermall one of 210, 0 to 210, both directions; 15_anulpha_pass forward one of 291, 0 to 295; every other file none).

- **Law.** `EMP_SweepSections` is `Repulser_AdvanceWave` cloned: a front that steps onto a branch's split entry spawns a second front (the `REP2` branch), one at a time (`emp+0x188`), which walks the branch `speed` entries a tick and shares the wave's age and lifetime. It is born as the ring front clears the split entry and clears branch entry 0 that tick (the Repulser's shape, by analogy, 60); a wave launched on the split entry itself does not fork on the step (the Repulser's `gap == 0`), the launch-time fork covers it. It does **not** walk the ring on from the merge: any front stepping off a merge releases it ("The merge releases the second front"). `emp::Fork`, `emp::fork_entry`.
- **Survey.** `Survey::branches` carries each `Course::branches` entry, measured like the ring (`oag_raceplay::detonator::survey`, the branch path's control points, every fourth sample); split and merge are ring entries (`sample / 4`). On the three circuits the branch's first entry sits 0.0 units from the ring's split entry and its last 7.9 to 8.0 from the merge entry. ~~**Nothing is laid on a branch**: the placement's own fork walk is unread~~ (2026-11-25: read and wired, [detonator-placement.md](detonator-placement.md#the-fork-walk); `place_stage` and the `--detonator-mines` snap now read `Survey::lap`, which holds the branch's clear sections). A mine put on a branch (a test does) is addressed by `Survey::branch_entry`'s index, one space with the ring's after it, and the fork clears it.
- **Hash.** `Wave` gains `forked` and `fork`; `write_emp` writes the fork only while forked, so an unforked wave hashes as it did.
- **Look.** A second `WO_QUAKE_DETONATOR` and a second `(0, 5, 10)` light ride the fork's front (`RaceView::emp_fork_effect`); the original's `EMP_DrawLights` adds the fork's light at `emp+0xe0` when `emp+0x60` is set. Both go with the wave. Frames: ours on `05_ubermall` from above at 90 and 105 ticks after the launch, one blue front on each arm of the split, the score up from the kills.
- **Tests.** `emp::tests::a_wave_that_crosses_a_split_clears_both_branches` (fails without the fork: the branch's mines stay), `without_the_fork_the_branch_is_left_alone`, `a_wave_that_never_steps_onto_the_split_does_not_fork`, `the_fork_walks_its_branch_and_is_released_at_the_merge`; disc: `tests::emp_fork::an_hd_emp_wave_forks_at_the_split_and_clears_both_routes` (the four forked files, mines on the branch and on the ring past the split die, two effects while forked) and `an_hd_circuit_without_a_split_never_forks_its_wave`.
- **2048 and Omega.** Both **ported**: 2048's `REP2` at `0x811e804c` is in the same sweep; Omega's `eboot.bin` carries one recursive sweep function with the `REP2` fourcc (`0x0133c600`, immediate at `0x0133ce36`, unnamed; the EMP's, "The launch-time fork"). Both run the shared module, and the effect and light ride the fork's front ("2048 and Omega draw it").

Tests: `emp::tests::the_wave_steps_one_entry_a_tick_for_extent_seconds` (15 entries at gauge `0`,
120 at `1`), `tests::detonator_fx::an_hd_emp_wave_carries_the_discs_effect_on_its_front_until_it_ends`
(the disc's effect rides the front, lights it, and is released with the wave),
`tests::detonator_fx::each_event_plays_the_effect_the_original_spawns_for_it` (the kill rows),
`weapon_light::tests::the_detonator_lights_are_the_values_the_render_queue_loads`.
Regression pairs, re-rendered: `pad` 8.17 (8.17 before), `mines-ahead` 14.65 (14.65).

## The launch-time fork (2026-11-25)

Static, `llvm-objdump` over `0x0013b5e8`-`0x0013b930` with every TOC load resolved by the
function's own OPD TOC (`0x008ad4d8`); the same body read on 2048 and Omega. Not live: no capture
fired an EMP at a junction.

**When it runs.** The launch call only (`EMP_Launch`'s tail call passes `init = 1`;
`EMP_SweepSections` runs `EMP_ForkAtJunction` at `0x0013c2c8` while `emp+0x188` is clear), from
the firer's own AI-track cursor (`ship+0x7820`, copied to `emp+0x168`).

```text
sibling = Track_FindSiblingSection(cursor)   ; the other successor of this path's entry junction,
                                             ; else the other predecessor of its exit junction,
                                             ; relocated to the nearest point there (500.0)
if no sibling: return
if |cursor point - sibling point|^2 >= 100.0 (TOC -10940 -> 0x00784c80): return
if not Track_SpansAreNeighbours(own path, sibling path): return
spawn WO_QUAKE_DETONATOR 'EMP2' on emp+0xb0 (identity, template TOC -10988 -> 0x00769d70)
emp+0x178 = sibling cursor
emp+0x18c = sibling.point > sibling.path.count / 2        ; 1: travels backward
emp+0x188 = 1                                             ; a second front is out
```

**Which entry each front starts from.** The main front walks the firer's own cursor, on a branch
or on the ring, forward (`EMP_UpdateWave` and `EMP_Launch` both pass direction `0`, `li r7,0` at
`0x0013c390` and `0x0013c568`). The second front starts on the sibling's nearest point.

**A far-half fork never moves.** The sweep recurses into the fork only when `emp+0x18c` equals the
call's direction (`0x0013bb44`-`0x0013bb5c`). Every EMP call is forward, so a fork born on the
sibling's far half (the merge side) is never stepped, until the merge releases it.

What a player gets from it in the original:

| Question | Answer | Confidence |
| --- | --- | ---: |
| Draws its effect | yes: `WO_QUAKE_DETONATOR` `EMP2` attached by pointer to `emp+0xb0`, which holds the identity matrix, so at the world origin; its extent co-factor is never set | 80 read, not seen |
| Draws its light | yes: `EMP_DrawLights` adds a light at `emp+0xe0` (that matrix's translation, the origin) while `emp+0x60` holds the handle | 75 read, not seen |
| Clears anything | no: the launch call's init branch kills nothing and the sweep never recurses into it | 80 read |
| Visible to the player | maybe, as a distant puff: the world origin is 116 to 250 units from the nearest road section of the three forked circuits, so its 50-unit light cannot reach the road, but the particles can be seen if the view points that way; it lasts until the main front's merge, under 25 or so frames | light 75, particles 40, inferred from the geometry |

Ours draws neither effect nor light for it, a departure: a stationary second front at an
arbitrary point is a rendering defect of the original.

### The merge releases the second front

The sweep's stepper returns `2` for a front on the last point of a path whose exit junction has
two predecessors. The arm at `0x0013c014`-`0x0013c040` (release at `0x0013c300`) then, **for
whichever front is being walked**, releases the second front's effect (`emp+0x60`) if it holds one,
clears `emp+0x188`, sets the stepper mode to `1` and walks on. 2048 (`FUN_811e7dd6`) and Omega
(`0x0133c600`) do the same. So:

- the main front stepping off a merge removes the second front, whether it was born at a split or
  at launch, moving or not, before that front moves that frame;
- the second front stepping off its own merge removes itself, after clearing the first point past
  it (the same call finishes its step);
- the fork flag is clear again, so a later split forks anew: it is one second front at a time,
  not one per wave.

Confidence **80** (HD instructions read; two further builds agree). Wired in
`emp::step_wave` (`ends_at_merge`, `advance_fork`). Tests:
`emp::tests::the_fork_walks_its_branch_and_is_released_at_the_merge`,
`the_main_fronts_merge_releases_the_fork_and_a_later_split_forks_again`,
`a_launch_beside_the_far_half_walks_nothing_until_the_merge`.

**On the disc.** Where a launch forks, on this engine's survey (section positions, 10 units), as
ring entries; branch offsets are the same stretches seen from the branch:

| Circuit | Forward fork (after the split) | Motionless fork (before the merge) |
| --- | --- | --- |
| `zone_3` | ring 9 to 21 (branch 0 to 12) | ring 522 to 564 (branch 513 to 555) |
| `05_ubermall` | ring 0 to 54 (branch 0 to 54) | ring 165 to 209 (branch 165 to 209) |
| `05_ubermall` reversed | ring 0 to 44 (branch 0 to 44) | ring 155 to 209 (branch 155 to 209) |
| `15_anulpha_pass` | ring 0 to 31 (branch 0 to 31) | ring 270 to 294 (branch 266 to 290) |

Where the two routes run within about a unit of each other (`05_ubermall`), `firer_entry` may put
a craft standing on the ring onto the branch's path; either way both fronts exist and the same
objects are cleared.

**Ring launches are unchanged except within those stretches.** There, a ring launch beside the
split now has a second front on the branch at once, and one beside the merge has a motionless
second front until the main front reaches the merge. The record lookup ("The other route's record") changes a few more on `05_ubermall`. On `zone_3`, whose merge is nine ring entries
before its split, such a wave still forks at the split afterwards. Pinned by
`tests::emp_fork::an_hd_emp_fired_beside_the_other_route_forks_at_launch` and
`an_hd_emp_fired_beside_the_merge_stands_still_until_it`.

Confidence **80** (every instruction of the function read on HD; 2048 `FUN_811e7c68`, called from
its sweep's init branch `FUN_811e7dd6`, and Omega `0x0133c600`'s init branch carry the same
`100.0` compare, neighbour-list test, `count / 2` direction, `EMP2` fourcc and identity matrix).
Held under 90: never seen to run, and the disc routes' neighbour listing was not read (taken as
satisfied; the structure is `track+0x0c+4i` counts and `track+0x4c+0x10i` entries, built at load).

**Wired.** `oag_weapons::detonator::emp::launch_fork` (with `SIBLING_REACH = 10.0`) at `fire`;
`oag_raceplay::detonator::firer_entry` starts the wave on the craft's own path (read off its
nearest spline sample; the original's is its tracked cursor, ours near an overlap picks the nearer
route, which the fork covers); the survey holds each section's partner within reach
(`SurveyBranch::{to_primary, from_primary}`, measured section position to section position, as the
placer's route comparison does). A launch on a branch walks the branch, then the ring from the
merge. A launch away from any split's two stretches is unchanged: the state hashes of two stand-fired ring EMPs
(`detonator_ground_truth`'s `emp_from_a_stand`) are identical to the build without any of this section's changes, record lookup included.

Ours chosen: the fork born at launch clears the entry it stands on on the first tick, as the main
front does (the original's launch call kills nothing).

The sweep also reaches the other route per point: "The other route's record".

**Tests.** `emp::tests::a_launch_on_a_branch_walks_the_branch_then_the_ring`,
`a_launch_beside_the_other_route_forks_onto_it`,
`a_launch_beside_the_far_half_walks_nothing_until_the_merge`,
`a_launch_away_from_the_other_route_does_not_fork_at_once`; disc (`tests::emp_fork`):
`an_hd_emp_fired_on_a_branch_starts_there_and_clears_its_mines` (fails with a ring-only start),
`an_hd_emp_fired_beside_the_other_route_forks_at_launch`,
`an_hd_emp_fired_beside_the_merge_stands_still_until_it`,
`an_hd_ring_launch_away_from_a_split_is_unchanged`.

**2048 and Omega.** Both **ported**: the fork, the merge arm and the record lookup below are the
shared `emp` module, and both builds carry the same code (2048 `FUN_811e7c68`, `FUN_811e7dd6`;
Omega `0x0133c600`). Tests: `tests::emp_fork::a_2048_emp_fired_on_or_beside_a_branch_follows_it`
(HD's `zone_3` as 2048's DLC1 pack ships it) and
`an_omega_emp_fired_on_or_beside_a_branch_follows_it` (`05_ubermall`, `15_anulpha_pass`).
These tests set the charge by hand; since 2026-11-25 2048's pads charge it too
([`vita-2048-eu-v104/detonator-pads.md`](../vita-2048-eu-v104/detonator-pads.md)). Omega's `0x0133c600` is the
**EMP's** sweep (it spawns `WO_QUAKE_DETONATOR` with `EMP2`), which closes the 50 below (80).

## The other route's record (2026-11-25)

Static, HD `0x0013ba44`-`0x0013bb2c`, and the same in 2048 `FUN_811e7dd6` and Omega `0x0133c600`.

**The records are the placement entry array** (`*(rm+0x6de50)`, count `rm+0x6de4c`, 48 bytes,
[detonator-placement.md](detonator-placement.md)): one per section its walk stored, `+0x08` usable,
`+0x18` the section. The sweep matches a record only when it is usable **and** its section is the
point's (`lbz r0,8(r29)` at `0x0013ba80`, `lwz r0,0x18(r29)` compare). So a section the walk did
not store (an alternate section not separated from the primary route, the split's first primary
section) or marked unusable (jump gap, start zone, over 20 lanes) has no record.

```text
point = the section the front stands on
if a usable record has point:  destroy every lane of that record
else:
    sibling = Track_FindSiblingSection(point)     ; no distance test (search 500.0)
    if sibling and a usable record has sibling:   destroy every lane of that record
```

An emptied record stays a record: destroying its mines changes lanes, not the entry, so it never
reaches across. Confidence **80** (HD instructions; two builds agree; not seen live).

**On the discs.** Only branch sections reach across. On every forked circuit, no primary-route
section that lacks a record has a partner holding one, so a ring front never uses it; a branch
front does, on `zone_3` over offsets 19 to 28 and 498 to 531, `05_ubermall` 26 to 68 and 151 to
209 (reversed 26 to 58 and 141 to 209), `15_anulpha_pass` 27 to 48, 71 to 72 and 235 to 290.

**Which ring launches change** (every ring entry of the four forked files fired at gauge `1`,
with and without the lookup, comparing what each wave clears): only **`05_ubermall` forward, ring
entries 31 to 44 and 54, and reversed, 21 to 32**, each clearing up to three more ring sections at
the far end of its wave's reach (the branch fork's partners run ahead of the main front); none
clears fewer. `zone_3` and `15_anulpha_pass` ring launches clear the same set.

**Wired.** `emp::sweep_entry` with `Survey::has_record` (`usable` and in `Survey::lap`) and
`Survey::sibling` (the partner the survey holds per section, searched to 500.0). Tests:
`emp::tests::a_point_with_no_record_takes_the_other_routes_record`,
`a_point_with_a_record_leaves_the_other_route_alone`, and disc
`tests::emp_fork::an_hd_front_on_a_section_with_no_record_takes_the_other_routes`,
`an_hd_ring_wave_through_the_fork_reaches_a_few_sections_further`.

### 2048 and Omega draw it (2026-11-26)

Both **ported** for the effect, the light's submission and the four per-effect lists. Static, base
`eboot.bin` of Omega (`/omega/eboot-ps4-omega-eu.bin`) and 2048's v1.04 `eboot.elf`.

**The effect.** Each executable's EMP launch spawns `WO_QUAKE_DETONATOR` with fourcc `EMP1`, attached
by pointer to the wave's matrix, as HD's does: 2048 `0x811e841c` (called from `0x811e8eea`), Omega
`0x0133ec10`. The fork and launch-fork spawns (`EMP2`, `REP2`) are in the sweeps above. Both sweeps
write the edges' distance over `100` into the instance (2048 `0x811e83f4`, Omega
`0x0133c600`'s tail). Confidence **85** (read in both builds; not seen running).

**The file.** `WO_QUAKE_DETONATOR` was read from HD's `DATA00.PSARC`, 2048's `data.psarc` (both
`data/particles` and `data/particles2048`) and Omega's `data00.psarc` and `data05.psarc` (both
directories) and parsed: HD and 2048 give identical emitter records; Omega's differ from them in
the last digit of four floats. Six emitters on all, all colour-mode word `0`. 2048's executable
names `Data/Particles` for a named mode such as Detonator (id below 23,
[particle-paths.md](../vita-2048-eu-v104/particle-paths.md)); this engine reads `Data/Particles2048`
for 2048, which for this file changes nothing.

**The light.** The EMP vtable's slot 5 (2048 `0x811e8650`, vtable `0x81512410`; Omega
`0x0133c150`, vtable `0x01915f50`) submits colour `(0, 5, 10)`, range `50`, at each front's matrix
translation, HD's values (colour: 2048's initialiser at `0x811e8b90`-`0x811e8baa` into
`0x818ac5cc`, Omega's `0x01a132d8`; range: `0x8152006c` = `50.0`, Omega the immediate
`0x42480000`). Confidence **85**. The record carries `1/D` and `D` and no exponent, so the
falloff is HD's, inherited. Wired as `oag_title::Detonator::emp_light`, apart from the cannon's
`cannon_lights` (still HD only).

**What reads the list, and what ours does not draw.** Both titles gather up to six lights from the
32-entry list per drawable, by the drawable's sphere or box and a channel filter (2048
`0x810373d4` and `0x81037524`, called from the draw paths at `0x813b3fb0`-`0x813be044`; Omega
`0x01623270` and `0x016233b0`, called from scene preparation). So the light is not hull-only.
**Which of their shader combines add it on a Zone surface is unread.** In ours the EMP light changes
`0` pixels of a 2048 frame and a few dozen of an Omega frame at ticks 2 to 8 after the launch,
against HD's `31,000`: HD's cyan wash is the Zone `SVC1` add after the fog
([detonator-cannon.md](detonator-cannon.md#the-blue-wash-of-a-volley-read-2026-11-24-det-flash-wash-re)),
which is gated to HD's Zone frame (`ZoneFrame::lit_by_stage_alone`) and Detonator's stage `1`
surfaces are black. **Open:** the Zone shader combine of each title.

**The lists.** Both titles list the effect in `far_bars`, `extents` and `turbulence`, as HD does.
No capture of either original pairs them: the lists are HD's paired choices. `sprites` is not
needed (Vita and PS4 read `.gxt`/`.gnf` for every effect).

**Frames** (ours, `oag-game --race --mode detonator --detonator-emp-charge 12 --press flip`, the
launch at tick 273, shots at 8, 24, 45 and 67 ticks after): on Omega's default circuit and 2048's
`DLC1/environments/zone_1` a blue wall spans the road at 8, then streaks run ahead and the road
behind is clean, as HD's own frames at the same offsets do. No original frame of either title.

**Tests.** `oag-raceplay` `tests::detonator_fx::a_2048_emp_wave_carries_the_discs_effect_and_light_on_its_front`,
`an_omega_emp_wave_carries_the_discs_effect_and_light_on_its_front` (nothing switched on by hand) and
`tests::title_effects::each_trigger_is_answered_by_exactly_the_titles_that_read_it`.

## 2048 and Omega

- **Omega: charge and gauge checked, apply.** `/omega/eboot-ps4-omega-eu-v107.bin`: the Detonator
  update holding `EMPREADY` (`FUN_012b6c70` in this import) tests `ship+0x76f4 & 0x8000000`,
  counts into `rm+0x300c0`, reads min/max at table `+0x234`/`+0x238`, computes
  `min((n - (hi - lo)) / (hi - lo), 1)` into `ship+0x8c0c` and raises `EMPREADY`/`EMPFULL` on the
  same two latches. The speed-pad arm is in the craft update `FUN_0133fff0`: on a pad hit with the
  mode `0xe`, a pad other than `craft+0x2f0` sets `ship+0x76f4 |= 0x8000000` and plays
  `WEAPONPICKUP`. Omega's weapon-pad arm (2026-11-25, base import `/omega/eboot-ps4-omega-eu.bin`
  `0x0130593b`-`0x01305954`): a pad other than `ship+0x8130` is stored and, in mode `0xe`, bit 27
  of the same flag word is set: HD's law, **ported** (wired as measured). Fire and wave: "2048 and
  Omega draw it".
- **2048: charge and gauge checked, apply.** `/2048/eboot-vita-2048-eu-v104.elf`: `FUN_8117e9c2`
  tests `ship+0x60a4 & 0x8000000`, counts into `rm+0x2b1f0`, reads table `+0x234`/`+0x238`, and
  computes the same gauge into `ship+0x7458`, with the divide done as a NEON reciprocal estimate
  and three Newton steps (so not bit-identical to an f32 divide), clamped to `1`, then
  `EMPREADY`/`EMPFULL`. **Its pads: ported** (2026-11-25): `Craft_ApplySpeedPad` (`0x811ccdf4`) and
  `Ship_TestWeaponPad` (`0x811c9c7a`) are HD's two arms without the net guard, and the counts are
  read from `+0x234`/`+0x238` (`4`/`8`)
  ([`vita-2048-eu-v104/detonator-pads.md`](../vita-2048-eu-v104/detonator-pads.md), 75-80). Fire
  and wave: "2048 and Omega draw it"; its HUD widgets differ (`EmpCharge`/`EmpFull`,
  [detonator-hud.md](detonator-hud.md)).

## Names

The 2048 and Omega `EMP_DrawLights` are in their own `names.tsv` (85): `0x811e8650` and
`0x0133c150`.

| Address | Name | Confidence |
| --- | --- | ---: |
| `0x0002bbc8` | `WeaponStats_ParseEmp` | 85 |
| `0x000d4078` | `Ship_TestWeaponPad` | 72 |
| `0x0012a3e8` | `WeaponManager_RollPickup` | 85, [pickup-draw.md](pickup-draw.md) |
| `0x00112730` | `EMPManager_Fire` | 72 |
| `0x0013c430` | `EMP_Launch` | 68 |
| `0x0013b930` | `EMP_SweepSections` | 75 |
| `0x00112e50` | `EMPManager_Update` | 78 |
| `0x0013c320` | `EMP_UpdateWave` | 85 |
| `0x0013afe8` | `EMP_SpawnLaunchEffect` | 82 |
| `0x0013b300` | `EMP_DrawLights` | 82 |
| `0x0013b5e8` | `EMP_ForkAtJunction` | 80 |
| `0x0013b220` | `EMP_Construct` | 80 |
| `0x0013ae68` | `EMP_ResetWave` | 70 |
| `0x0027e118` | `Modifier_NoiseForce` | 72 |

`EMP_SweepSections` rises from 62: its caller is read (`EMP_UpdateWave` and `EMP_Launch`'s tail
call) and live it steps one point a frame. `EMP_ForkAtJunction` rises from 60: its body is read
and 2048 and Omega carry the same test ("The launch-time fork"). `EMP_Construct`:
the `"EMP.cpp"` tag at `+0x30`, the vtable `0x00864348`.

Existing names cited: `Craft_ApplySpeedPad` (`0x000eeb70`, 65, [craft-flags.md](craft-flags.md)),
`Ship_UpdateCannonTrigger` (`0x000e9cd0`, 68), `CannonManager_ArmSlot` (`0x0010f2e8`, 65),
`Detonator_UpdateRace` (`0x00067b40`), `MineManager_DestroyAtLane` (`0x0011a590`),
`BombManager_DestroyAtLane` (`0x0012e260`), `WeaponManager_DispatchFire` (`0x0012b2c0`).
