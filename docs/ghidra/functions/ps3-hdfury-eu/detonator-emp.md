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
| Wave extent | `minDistance + (maxDistance - minDistance) * gauge` = `0.25 .. 2.0` at gauge `0 .. 1`; unit unread | 70 |
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

Not read. [detonator-hull.md](detonator-hull.md) reads that `WorldManager_Construct` adds a
`Detonator_Pad` model on every speed and weapon pad. Live, four frames taken on the tick a speed
pad was entered (Vineta K, stages 1 and 2) show the white `Detonator_Pad` blocks on plain track:
no chevron or glow of Vineta K's own speed pad is visible round or between them. Confidence 60
that the circuit's pad is not drawn (a flat decal under the blocks' footprint would not show).

## Live

Run 1 (the setup on [detonator-speed.md](detonator-speed.md#live-rpcs3-2026-11-22)): every rise
of `rm+0x2dd8` was attributed to the pad that caused it, from `craft+0x274` (speed pad) and
`ship+0x7960` (weapon pad) in the samples before it.

| Read | Value |
| --- | --- |
| speed pads | four per lap on Vineta K (`0x30624060`, `0x306a5960`, `0x306237c0`, `0x30625ee0`), each entry `+1`, 38 entries, 38 charges; `craft+0x274` back to `0` after each |
| weapon pad | one charge under player input (`ship+0x7960` `0 -> pad`), cleared to `0` on the next crossing; none under the Autopilot (its running bit `slot+0x200 & 0x800` is the test's own exclusion) |
| `craft+0x2e8` | `0.0` throughout: no boost timer on any pad |
| slot `+0x204` | `3` (the Cannon) for the whole race |
| gauge `ship+0x7da0` against `n` | `0..4` -> `0.0`, `5` -> `0.25`, `6` -> `0.5`, `7` -> `0.75`, `8..39` -> `1.0` |
| the flag | `ship+0x6a64 & 0x8000000` never seen set (set and cleared inside one frame) |

Run 2, L1 pressed for 0.5 s with `n = 12` (gauge `1.0`), the Autopilot bit cleared: `n` read `0`
and the gauge `0.0` on the next sample; `rm+0x6de3c` went `3 -> 15` and the score `150 -> 450`
(`+25` per mine, `<Emp> old_mine_score`, the dark-mine value; no live-mine `+50` was seen)
over the next 1.8 s; the frame after the press shows a blue wave ahead of the craft; the clip
read `09` before and after. Triangle, R1, R2 and L2 changed nothing.

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

## Open

- The wave's reach and speed (above), and whether it hurts the player.
- Whether the circuit's own pad geometry is drawn at all (four live frames show none, below).
- Weapon pads under player input over several laps (one live charge only).

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
| `0x0013b930` | `EMP_SweepSections` | 62 |

Existing names cited: `Craft_ApplySpeedPad` (`0x000eeb70`, 65, [craft-flags.md](craft-flags.md)),
`Ship_UpdateCannonTrigger` (`0x000e9cd0`, 68), `CannonManager_ArmSlot` (`0x0010f2e8`, 65),
`Detonator_UpdateRace` (`0x00067b40`), `MineManager_DestroyAtLane` (`0x0011a590`),
`BombManager_DestroyAtLane` (`0x0012e260`), `WeaponManager_DispatchFire` (`0x0012b2c0`).
