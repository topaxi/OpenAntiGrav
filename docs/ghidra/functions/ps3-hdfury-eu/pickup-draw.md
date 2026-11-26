# HD's pickup draw: two draws, and a per-mode weapon mask

**Binary:** `EBOOT.elf` (`/hdfury/EBOOT-ps3-hdfury-eu.elf`), PowerPC, big endian. Read statically
2026-11-26, lane `hd-weapon-roster`, with the Ghidra bridge, `scripts/ps3-toc.py` for strings and
`capstone` for the mode jump table Ghidra does not recover. No live run.

## The answer

**HD's executable never hands out a Repulser or a Shuriken**, although its Eliminator table weighs
both (`Repulser` 8, `Shuriken` 10, `ai` and `human` alike). Each pickup draw tests a bit of a
16-bit weapon mask on the game state before it can grant a weapon, and no mask a menu can build
holds the Repulser's bit 11 or the Shuriken's bit 12. Confidence **88**.

The effective roster per mode, the mask intersected with the disc's own odds:

| Mode | Pickups | Confidence |
| --- | --- | ---: |
| Single race, tournament, campaign race (odds: `weaponstats_race.xml`) | Rocket, Missile, Quake, Cannon, Plasma, Bomb, Mine, LeachBeam, Turbo, Shield, Autopilot (11) | 88 |
| Eliminator, modes 8 and 20 (odds: `weaponstats_elimination.xml`) | Rocket, Missile, Quake, Cannon, Plasma, Bomb, Mine, LeachBeam (8) | 88 |
| Detonator, mode 14 | none: a weapon pad charges the EMP and the roll is skipped | 75, [detonator-emp.md](detonator-emp.md) |
| Zone Battle, modes 13 and 21 | the roll runs with the default mask; this project has no such mode | 55, read from the mask defaults only |
| Time trial, speed lap, Zone | no weapon pads | [pickups.md](../../../gameplay/pickups.md) |

The odds are the disc's `<Pickupodds>`, equal on all four classes and across `DATA00` and `DATA02`
(and `DATA05`'s race table). The race table zeroes the Repulser and the Shuriken itself; the
Eliminator table zeroes Turbo, Shield and Autopilot.

## The gate

`GameState_IsWeaponEnabled` (`0x00014658`)`(g_GameState, bit)` returns bit `bit` of one of three
halfwords of the game state, picked by the front end's `Main Menu->Mode` string (`FE_RC` campaign
`+0xe6`, `FE_RACEBOX` `+0xe8`, `FE_ONLINE` `+0xea`), and in race by the game mode (above 15 the
online mask, else `+0x90` picking `+0xe6` or `+0xe8`). **Game mode 2 (the demo) returns true for
every bit.**

### Which bit is which weapon

Three independent witnesses agree on all thirteen: the weapons screen's widget stored at
`this + 0xf0 + 4 * bit` (`WeaponsScreen_ShowMask`, `0x0025ed10`), the manager each draw arm calls
at `WeaponManager + offset` with the state it writes to slot `+0x208`, and the odds block
`WeaponStats_ParsePickupOdds` (`0x0002df78`) fills for that `type` string.

| Bit | Weapon | Screen widget | Slot state, manager | `ai` odds block |
| ---: | --- | --- | --- | --- |
| 0 | Rocket | `Rocket` | 0, `+0x6c` | `+0x218` |
| 1 | Missile | `Missile` | 1, `+0x64` | `+0x2d8` |
| 2 | Quake | `Quake` | 2, `+0x74` | `+0x318` |
| 3 | Turbo | `Turbo` | 4, none | `+0x358` |
| 4 | Shield | `Shield` | 5, none | `+0x398` |
| 5 | Cannon | `Cannon` | 3, `+0x70` | `+0x258` |
| 6 | Autopilot | `AutoPilot` | 6, none | `+0x418` |
| 7 | Plasma | `Plasma` | 7, `+0x68` | `+0x3d8` |
| 8 | Bomb | `Bomb` | 9, none | `+0x1d8` |
| 9 | Mine | `Mines` | 8, `+0x60` | `+0x298` |
| 10 | LeachBeam | `LeechBeam` | 10, `+0x78` | `+0x458` |
| 11 | Repulser | **none** | 11, `+0x7c` | `+0x498` |
| 12 | Shuriken | **none** | 12, `+0x80` | `+0x4d8` |

Each odds block holds four columns `0x10` apart (`ai`, `human`, `front`, `back`), four classes
each. `+0x518` onwards holds the columns' sums over **all thirteen weapons**, mask or no mask.

`+0x7c` and `+0x80` are two managers `WeaponManager_Construct` never builds: every path of both its
copies ends by storing `0` to both (`0x0012c924`/`0x0012c928`, `0x0012d3f0`/`0x0012d3f4`, confidence
88, `+0x80` read null live; lane `hd-open-air-arms`'s page, "HD builds no Shuriken"). The mask is
what keeps the draw from calling through them.

### Who writes the masks

A sweep of every `sth` to `+0xe6`, `+0xe8` and `+0xea` off a register finds these and no other:

| Writer | What it writes | Confidence |
| --- | --- | ---: |
| The two game state constructors, `0x000158c8` and `0x00015ac0` | `0x7ff` to all three: bits 0 to 10 | 90 |
| `GameState_ResetWeaponMaskForMode` (`0x00012ae8`), from the front end's mode change (`bl` at `0x00248700`, which every case of the jump table at `0x00248778` reaches, 8 and 20 included) | modes 8 and 20: `0x7a7` (Turbo, Shield, Autopilot, Repulser, Shuriken off); any other mode: `0x7ff` | 80 |
| `GameState_SetWeaponEnabled` (`0x00012938`), from the reset above and from `WeaponsScreen_StoreMask` (`0x0025e5a8`) | one bit; the screen has rows for bits 0 to 10 only, and hides Turbo, Shield and Autopilot in Eliminator | 85 |
| `GameState_SetWeaponMask` (`0x00012868`) | a whole mask; no branch and no descriptor reference reaches it | 70 |
| `0x000168b0` | a whole mask copied from an imported record; **the record's source is unread** | not named |

The one open door is that importer: whatever it reads could carry any bits. Everything a menu does
leaves 11 and 12 off.

## Two draws

`WeaponManager_DispatchFire` spends a pending pickup (slot `+0x20c`) on one of two draws, picked
by the slot's `+0x214` byte (`+0x2a4` from the manager's slot base): non-zero calls
`WeaponManager_RollPickup` (`0x0012a3e8`, a person), zero calls `WeaponManager_RollPickupAi`
(`0x00129c20`). `WeaponManager_AwardPickup` (`0x0012ad90`) makes the same choice. Confidence **85**.

### The AI draw

Flat `ai` column. **A disabled weapon's weight is subtracted from the total and zeroed**, so the
rest are drawn in proportion. It writes the slot without comparing the previous grant: **no repeat
refusal**. Confidence **85**.

### The person's draw

`human + back * t + front * (1 - t)`, the place blend Pulse's grant uses. **A disabled weapon's
weight stays in the total.** The thirteen are walked in a fixed order, each step adding its weight
and taking the pickup only if the roll is below the running sum **and** its bit is set:

Autopilot, Bomb, Cannon, Mine, LeachBeam, Missile, Plasma, Quake, Repulser, Shuriken, Rocket,
Shield, Turbo.

So a roll landing on a disabled weapon passes to the next enabled one. In Eliminator the
Repulser's 8 and the Shuriken's 10 both land on the Rocket: **28 of 90** against the AI's 10 of 72.
A grant equal to the previous one is rolled again. The last arm (Turbo) takes whatever is left
without a sum test, and rolls again if its bit is off. Confidence **85**, instruction level (each
failed gate branches to the next step: `0x0012ac98` to the Shuriken's, `0x0012ace4` to the
Rocket's).

### One-live exclusions

Both draws also drop a weapon from the total, and from the walk, while enough of it is live: the
LeachBeam while the count at `*(TOC-0x30e0)` is non-zero, the Quake while `*(TOC-0x30dc)` is, and
the Cannon while `*(TOC-0x30d8)` is 2 or more. What those three counts are is **not read**
(confidence 55 that they count live instances), and this project does not apply them.

## Names

| Address | Name | Confidence | Evidence |
| --- | --- | ---: | --- |
| `0x00014658` | `GameState_IsWeaponEnabled` | 85 | Decompile and disassembly; called by both draws and by the weapons screen |
| `0x00012938` | `GameState_SetWeaponEnabled` | 85 | Sets or clears one bit of the mask the menu mode picks |
| `0x00012868` | `GameState_SetWeaponMask` | 70 | Whole-mask store; unreached |
| `0x00012ae8` | `GameState_ResetWeaponMaskForMode` | 80 | Thirteen calls to the bit setter, two sets by mode 8/20 |
| `0x00129c20` | `WeaponManager_RollPickupAi` | 85 | The flat-`ai` weighted roll, called when the slot is not a person's |
| `0x0012ad90` | `WeaponManager_AwardPickup` | 75 | The same choice between the two draws, with the mode 14 skip |
| `0x0012a3e8` | `WeaponManager_RollPickup` | 85 | Already named on [detonator-emp.md](detonator-emp.md); read whole here |
| `0x0025ed10` | `WeaponsScreen_ShowMask` | 75 | Binds thirteen widget pointers (11 and 12 null) and shows `FE_ON`/`FE_OFF` per bit |
| `0x0025e5a8` | `WeaponsScreen_StoreMask` | 75 | Writes each widget's `FE_ON` back as its bit |
| `0x0002df78` | `WeaponStats_ParsePickupOdds` | 88 | Matches each `type` string and stores `ai`/`human`/`front`/`back` per class |

## What this project does

- The roster is `Title` data: `oag_title::pickup_roster::PickupRoster`, HD's values in
  `oag_hd::pickups::ROSTER`. A race takes the mode's list into `SimSetup::allowed_weapons`, and the
  person's order into `SimSetup::pickup_order`.
- `oag_weapons::pickup::draw` leaves a disallowed weight out for the AI and passes it on for a
  person, as above.
- **Not applied:** the AI's missing repeat refusal (ours refuses repeats for both), the three
  one-live exclusions, the weapons screen's toggles, and the demo's open gate.
- Disc-backed: `hd_pickup_roster_ground_truth` (raceplay) holds both rosters and the two Rocket
  shares through a loaded race.

## Cross-title

- **2048: checked, applies, not wired.** The 1.04 executable carries the same weapons screen
  strings, eleven `WeaponsScreenParent->` weapon rows with none for the Repulser or the Shuriken,
  and its code references neither `ShurikenManager.cpp` nor `RepulserManager.cpp` (a census,
  confidence 55). Its draw and mask defaults are not read.
- **Omega: checked, applies, not wired.** The 1.07 executable carries the same eleven rows, and its
  HD-named Eliminator table's first four classes equal `DATA00`'s
  ([weapon-stats.md](../../../formats/weapon-stats.md)). Its draw and mask are not read.
- **Pulse and Pure: not examined.** Their draw is untouched.
