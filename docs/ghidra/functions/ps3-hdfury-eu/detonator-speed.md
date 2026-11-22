# Detonator: the auto-speed, keyed on the stage

**Binary:** `EBOOT.elf` (`/hdfury/EBOOT-ps3-hdfury-eu.elf`). Every function below is under
`0x32d5e0`, so its TOC is `r2 = 0x008ad4d8` ([memory.md](memory.md)); constants resolved with
`scripts/ps3-toc.py resolve`, bodies read off `llvm-objdump -d --mattr=+altivec,+vsx` where
Ghidra truncates. Static reading 2026-11-22, then checked live on RPCS3 the same day ("Live" below). Read first:
[craft-flags.md](craft-flags.md) ("`Craft_UpdateThrottle`, read whole"), [detonator-mode.md](detonator-mode.md)
(the stage counter), [hover-four-point.md](hover-four-point.md) (the Zone-modes mask).

## The answer

| Quantity | Value | Confidence |
| --- | --- | ---: |
| Engine target in Detonator | `T = ramp * (start + increment * n)`, the Zone-modes branch of `Craft_UpdateThrottle`; the player's throttle never reaches it | 80 |
| `start`, `increment` | `34.0`, `4.4`: `<Handling><Global><Zone start increment>` in `/data/xml/handlingstats.xml` (`DATA02`), the same attributes and values Pulse reads | 85 |
| `n` | `craft+0x2d0`, written by `Detonator_UpdateHud` with the HUD stage on every start-line crossing after the first: `2, 3, ..., 14` | 90 (live: `2..11` read on each crossing) |
| `n` on lap 1 | `0` (the craft reset, `li r10,0` at `0x000ef104`, stored at `0x000ef2c8`): live `0` from the grid to the first completed lap | 90 |
| Gate | `T = 0` unless `craft+0x264 & 1` (grounded) and not `& 2` (grid state) | 85 |
| Ramp | `craft+0x2d4 += dt / 1.3` while under `1.0`; `ramp = max(r - 0.1, 0) / 0.9`, `1.0` once `r >= 1`; zeroed by a rescue under the mask | 75 (live: `1.0` through the whole grid state) |
| Tail | the shared tail: Autopilot speed-up (`flags & 4`), turbo add, launch multiplier `craft+0x2dc`, `+100` while `craft+0x2e0 > 0`, the doubling | 85 |
| Brakes | none (the mask's, [hover-four-point.md](hover-four-point.md)) | 85 |
| Speed pads | **no boost** in Detonator: the pad charges the EMP instead ([detonator-emp.md](detonator-emp.md#pads-charge-they-do-not-boost)) | 88 (live) |

So **Detonator's auto-speed is Zone's law, not its own**: the same branch, the same two
numbers, but `n` is the Detonator stage (fourteen steps over the whole run) where Zone's `n` is the
zone count the Zone manager writes. At the doubled force that is `2 * 34 = 68` on lap 1 (`n = 0`)
and `2 * (34 + 4.4 * 14) = 191.2` on stage 14, before the launch multiplier and the tail.

## `Craft_UpdateThrottle`'s Zone-modes branch (`0x000efeb8`-`0x000eff74`)

`craft-flags.md` has the whole function; this is the arm it summarised as "zone target", read
instruction by instruction. `f29` is `dt` (`fmr f29,f1` at `0x000efb68`), `f30` is `0.0` on both
paths that reach the branch (`lfs f30,-0x438c(r2)` at `0x000efc2c` and `0x000efe3c`).

```text
0x000efeb8  lis r0,0x20; li r9,1; ori r0,r0,0x6040     ; mask 0x206040 (modes 6, 13, 14, 21)
0x000efec4  sld r9,r9,r11; and r11,r9,r0; cmpdi       ; r11 = game mode (`< 0x16` tested before)
0x000efed0  beq 0x000efc98                            ; not a Zone mode: the throttle path
0x000efed4  lfs f10,-0x4394(r2)                       ; 1.0
0x000efed8  lfs f0,0x2d4(r31)                         ; r = craft+0x2d4
0x000efedc  fmr f11,f10                               ; ramp = 1.0
0x000efee0  fcmpu cr7,f0,f10; blt 0x000eff44          ; r < 1: advance the ramp
0x000efee8  lwz r10,0x264(r31)                        ; flags
0x000efeec  lfs f31,-0x438c(r2)                       ; T = 0.0
0x000efef0  rldicl r0,r10,63,63; xori r0,r0,1; and r9,r10,r0
0x000eff00  beq 0x000efcfc                            ; !(flags & 1) || (flags & 2): T = 0, to the tail
0x000eff04  lwz r0,0x2d0(r31)                         ; n
0x000eff08  lwz r9,-0x42e0(r2)  -> 0x0098d634         ; &increment
0x000eff18  lwz r11,-0x42dc(r2) -> 0x0098d630         ; &start
0x000eff14  fcfid; frsp                               ; float(n)
0x000eff28  fmadds f0,f0,f12,f13                      ; n * increment + start
0x000eff2c  fmuls f31,f11,f0                          ; T = ramp * (...)
0x000eff30  b 0x000efcfc                              ; the shared tail (flags & 4, turbo, launch, doubling)

0x000eff44  lfs f13,-0x42e8(r2)                       ; 0.76923 (0x3f44ec4f = 1/1.3)
0x000eff48  fmadds f13,f29,f13,f0                     ; r' = r + dt / 1.3
0x000eff4c  lfs f12,-0x4310(r2); lfs f11,-0x42e4(r2)  ; 0.1, 1.1111 (0x3f8e38e4 = 1/0.9)
0x000eff58  stfs f13,0x2d4(r31)
0x000eff5c  fsubs f12,f13,f12; fsubs f0,f30,f12; fsel f0,f0,f30,f12   ; max(r' - 0.1, 0)
0x000eff68  fmuls f11,f0,f11                          ; ramp = max(r' - 0.1, 0) / 0.9
0x000eff6c  ble 0x000efee8                            ; r' <= 1
0x000eff70  stfs f10,0x2d4(r31); b 0x000efee8         ; r' > 1: store 1.0 (this tick's ramp is a hair over 1)
```

The branch replaces the throttle path, so the cross button does nothing to `T` in Detonator
(the throttle at `craft+0x30c` is still written above it, `0x000efc50`, and still drives the
engine sound). The brakes are off under the same mask. Confidence **80**: one function, every
instruction of the arm read, and the shared tail is live-measured for the throttle path
([craft-flags.md](craft-flags.md)).

### `start` and `increment`: `Handling_ReadGlobalClasses`

`0x0098d630` and `0x0098d634` are BSS. A census of every word in the image that points into
`0x0098d5c0`-`0x0098d660` and of every load of those TOC slots finds one writer, through the block
base `0x0098d600` (TOC slot `0x008a8674`): `Handling_ReadGlobalClasses` (`0x000c6af0`,
[pilot-assist.md](pilot-assist.md)) loads the base at `0x000c6d80` and, in its `<Zone>` loop,
stores `start` at `0x000c6e24` (`stfs f1,0x30(r30)`) and `increment` at `0x000c6e50`
(`stfs f1,0x34(r30)`); the attribute names are TOC `-0x4d88` `start`, `-0x4d84` `increment`,
`-0x4d80` `recharge`, under the element `-0x4d8c` `Zone`. HD's
`/data/xml/handlingstats.xml` authors `<Zone increment="4.4" recharge="10" start="34"/>`
(`just psarc cat data/images/hdfury-ps3-eu-dec.iso /data/xml/handlingstats.xml`). Confidence 85.
`oag_tables::handling` already parses the element (`Zone`, `crates/tables/src/handling.rs`).

### `n`: the Detonator stage, written on the crossing

`Detonator_UpdateHud` (`0x00067120`, [detonator-cannon.md](detonator-cannon.md)) runs the stage
step on a forward crossing (`craft+0x7001`) with the crossing count `craft+0x7810 > 1`:

```text
rm+0x2dee += 1                            ; the HUD stage, a u16, constructed at 1
if rm+0x2dee >= 15: rm+0x2dee = 14        ; 0x000671f8, sth at 0x000671fc
... announcer cues from the stage script at rm+0x2e20 ...
*(ship+0x5fac)+0x2d0 = rm+0x2dee          ; 0x00067230-0x00067240: stw r0,0x2d0(r11)
```

`*(ship+0x5fac)` is the craft ([craft-flags.md](craft-flags.md)), the same `r31` `Craft_UpdateThrottle`
reads `+0x2d0` from. So after lap 1 `n` is `2`, after lap 13 it is `14`, and it stays `14`
(the race ends when `rm+0x2e10` reaches `15`, [detonator-mode.md](detonator-mode.md)). The first
crossing (count `1`) takes the other arm and writes nothing. Confidence **75** for the write and
its value.

The other `+0x2d0` writers through `*(ship+0x5fac)`: `0x0003f01c`, `0x0004013c` (`FUN_0003eea0`
and the function after it) and `0x0006d364` (`FUN_0006d0d0`, beside `Zone_UpdateCraftClass`'s
loop) store a converted float, the Zone manager's zone count; `0x00076138` (`FUN_00075a70`) is the
same stage-write shape as `Detonator_UpdateHud` for another mode manager. None of them is on
Detonator's update path as read ([detonator-mode.md](detonator-mode.md)'s `Detonator_UpdateRace`
calls only `Detonator_UpdateHud` for this). The craft resets (`0x000ef2c8`, `0x000f04a8`) store
the same register as `+0x264`, which is `0` there (`li r10,0`, `0x000ef104`).

### The ramp and the rescue

`craft+0x2d4` ramps only inside the mask's branch, before the grounded gate, so it climbs through
the grid countdown and is `1.0` well before GO. Its one Zone-mode reset is in `FUN_000e8b38` (the
rescue's re-placement: the eight-entry track-history ring at `ship+0x62f0`): under the mask it
calls `0x000d6eb0` with a zero speed and stores `0` to `*(ship+0x5fac)+0x2d4` (`0x000e8d84`), where
every other mode re-places the craft with a speed of `ship+0x68fc * k + c`. So **after a rescue a
Detonator craft starts from rest and its engine comes back over 1.3 s**, the first 0.13 s at
nothing. Confidence 75 (writers by displacement census, `0x000ef33c`/`0x000f051c` are the craft
constructors).

## Live (RPCS3, 2026-11-22)

Racebox, race type DETONATOR (seven `right` presses on the race-type row from the boot default),
Vineta K (`01_Vineta_K`), weapons on, the default team; game mode `*(0x00936fe8+0xe0) = 14`.
Memory polled about every 30 ms through `/proc/<pid>/mem` (the race manager `**0x008a6814`,
the ship `rm+0x13e8`, the craft `*(ship+0x5fac)`). Phases: grid and countdown, 10 s after GO
with nothing held (player input), 8 s with cross held, then the Autopilot's running bit for
about 300 s (to get round the laps only; no speed below is read as a law off it), energy
topped up so the mines did not end the run.

| Read | Value | What it settles |
| --- | --- | --- |
| `craft+0x2d4` | `1.0` from the first grid sample on | the ramp is complete by GO |
| `craft+0x264` | `0x3` through the grid, `0x1` from GO | the gate |
| `craft+0x2d0` with `rm+0x2dee`/`rm+0x2e10` | `0` with `1/1` from the grid through lap 1, then `2/2`, `3/3` ... `11/11`, each on the sample the stage stepped | `n` is the HUD stage, `0` on lap 1 |
| speed `*(ship+0x6944)+0x4c4` | `3.3` on the grid, `107.6` within 4 s of GO with **cross never pressed** (`craft+0x30c = 0.0`) | the engine runs with no throttle |
| `craft+0x30c` | `100.0` while cross is held, `0.0` otherwise | the throttle is written and does not reach `T` (the held segment scraped a wall and cannot compare speeds) |
| `craft+0x2e8` | `0.0` on every sample, across 38 speed-pad entries | no pad boost ([detonator-emp.md](detonator-emp.md)) |

Under the Autopilot the lap's top speed climbed stage by stage (`~110` on lap 1 to `~190` on lap
11), the direction the law predicts; not used as a measurement (Autopilot, mines, x1.2).
Reproduce: the lane's scratch poller, a `drive.Session` walk with
`{"Main Menu": ["right"], "Single Player": ["right"] * 7 + ["cross"]}`, then the fields above;
the start needs `cross` pressed until `craft+0x264 & 2` clears (the walk returns on the
team-launch transition, before the race takes a press). Confidence of the target law with this:
**85**.

## Open

- A player-input speed curve on a clean lap, to compare against our engine once wired (the run above coasted into a wall 4 s after GO).
- `0x00076138`'s mode (which manager owns `FUN_00075a70`).
- **HD's own Zone**: this branch is HD's Zone engine too, and it is not what
  [zone-start.md](../psp-pulse-usa/zone-start.md) says of it ("its Zone mask only ramps
  `craft+0x2d4`, and it reads no `+0x2f0`"): the arm computes `start + increment * n` with the
  disc's `34`/`4.4` and falls into the tail that holds the `flags & 4` speed-up
  (`0x000efcfc`-`0x000efd0c`, which multiplies by `craft+0x2f0`). What HD Zone does not share with
  Pulse is the ramp. Correction recorded here, 2026-11-22; that page and the weapon-lineage
  Autopilot row are not edited by this lane.

## 2048 and Omega

- **Omega: checked, applies, not wired.** `/omega/eboot-ps4-omega-eu-v107.bin`: the `<Zone>`
  loader `FUN_012f1c50` stores `start` at `0x01a84960` and `increment` at `0x01a8495c`; their one
  engine reader, in `FUN_0133fff0` (the craft update), is the same branch under the same modes
  (`mode - 6 < 0x10 && (0x8181 >> (mode - 6)) & 1`, modes 6, 13, 14, 21): `craft+0x5f8 += dt *
  0.7692308` while under `1.0`, `ramp = max(r - 0.1, 0) * 1.1111112`, and `T = (float(craft+0x5f4)
  * increment + start) * ramp` when `flags & 1 && !(flags & 2)`. Which field Omega's Detonator
  update writes the stage into is on [detonator-mode.md](detonator-mode.md) (`craft+0x5e4`, read
  structurally); that it is the `+0x5f4` this engine reads was not traced.
- **2048: checked, applies, not wired.** `/2048/eboot-vita-2048-eu-v104.elf`:
  `Handling_ReadGlobalSettings` stores `increment` at `0x818a6390` (`start` beside it at
  `0x818a638c`); the reader `FUN_811cf532` has the same ramp (`craft+0x540`, `0.7692308`, `- 0.1`,
  `* 1.1111112`) and `T = (start + increment * float(craft+0x53c)) * ramp` behind the same gate.
  Its mode predicate and the stage write into `+0x53c` were not traced.

## Names

| Address | Name | Confidence | Note |
| --- | --- | ---: | --- |
| `0x000efb50` | `Craft_UpdateThrottle` | 75 | existing ([pilot-assist.md](pilot-assist.md)) |
| `0x000c6af0` | `Handling_ReadGlobalClasses` | 80 | existing |
| `0x00067120` | `Detonator_UpdateHud` | 65 | existing ([detonator-cannon.md](detonator-cannon.md)) |
