# 2048's Detonator pads charge the EMP

2026-11-25 (`det-2048-pad-emp-charge`). **Binary:** `/2048/eboot-vita-2048-eu-v104.elf`
(image base `0x81000000`), read statically in Ghidra; nothing here ran. This is the 2048
counterpart of HD's two pad arms on [`detonator-emp.md`](../ps3-hdfury-eu/detonator-emp.md)
("Pads charge, they do not boost"). Read that page first. The charge itself (`FUN_8117e9c2`:
it tests `ship+0x60a4 & 0x8000000`, counts into `rm+0x2b1f0` and reads the table's
`+0x234`/`+0x238`) is on that page's "2048 and Omega" section.

| Address | Name | Confidence |
| --- | --- | --- |
| `0x811ccdf4` | `Craft_ApplySpeedPad` | 80 |
| `0x811c9c7a` | `Ship_TestWeaponPad` | 75 |
| `0x8100f7c2` | `WeaponStats_ParseEmp` | 70 |

## The answer

| Quantity | 2048 | HD | Confidence |
| --- | --- | --- | ---: |
| Speed pad, Detonator | charges once per pad entered, plays `WEAPONPICKUP`, **no boost** | same | 80 |
| Weapon pad, Detonator | charges once per pad, again only after another weapon pad or the next lap; no weapon is awarded | same | 75 |
| Lap reset of the weapon-pad memory | `ship+0x6f80 = 0` in `Pilot_UpdateLapProgress` | `ship+0x7960 = 0` on a forward crossing | 65 |
| Ready / full counts | `minNumPadsToCollectEMP` at `+0x234`, `maxNumPadsToCollectEMP` at `+0x238`, from the `Data\XML\weaponstats_Detonator.xml` 2048's archive ships (`4`/`8`) | `+0x1bc`/`+0x1c0`, `4`/`8` | 80 |
| Net guard | **none** on either arm | both arms test the net byte | 75 |

The mode literal on every arm is `0xe`, read from `DAT_8153fd24` (`0x8153fc40 + 0xe4`): the same
global and value that `FUN_8121e862` tests before it puts `Detonator_Pad` on each pad
([`detonator-emp.md`](../ps3-hdfury-eu/detonator-emp.md), "Do the circuit's own pads still
draw?").

## Speed pads: `Craft_ApplySpeedPad` (`0x811ccdf4`)

The pad test is `FUN_8121f66a` against the craft's position (`craft+0x240`). A miss stores
`craft+0x274 = 0` and goes to the force step. On a hit with `DAT_8153fd24 == 0xe`:

```text
if pad == craft+0x274: return                    ; same pad as last tick
craft+0x274 = pad
ship+0x60a4 |= 0x8000000                         ; 0x811cce5e, orrs r3,r3,#0x8000000
FUN_812609a4(1.0, ship+0x5580, ..., "WEAPONPICKUP", ...)   ; string 0x81494654
return
```

`ship` is `*(craft+0x268)`. Every other mode writes the boost timer and strength
(`craft+0x554 = table[class] * handling+0x468`, `craft+0x558 = table[class] * handling+0x464`)
and, on a new pad, counts it (`craft+0x5e0`, the `FeisarProtoSpeedPad` cue). The force step
(`0x811cd0e4`) pushes only while `craft+0x554 > 0`. The Detonator arm returns before any of
that, so **a 2048 speed pad gives no speed in Detonator**, as HD's does. Confidence **80**: every
statement of the arm read, the flag's one reader (`FUN_8117e9c2`) agrees on the offset and the
bit, and HD's twin was confirmed live (37 entries).

## Weapon pads: `Ship_TestWeaponPad` (`0x811c9c7a`)

The weapon slot is `*(ship+0x557c)`. While `ship+0x6642` is clear, if the slot's `+0x1f8` is
`-1` (empty), or the mode is `0xe` and it is `3` (the Cannon), it calls `WeaponPads_TestCraft`
([`weapon-pads.md`](weapon-pads.md)) with the last pad `ship+0x6f80`. A hit counts only with the
slot's `+0x1f4 & 0x800` clear (the Autopilot's running bit; HD's `+0x200 & 0x800`). On a counted
hit it marks a pending pickup (`FUN_8120f7e4`: `slot+0x200 = 1`), and then:

```text
0x811c9ef4  cmp r0, r1                 ; pad against ship+0x6f80
0x811c9ef6  beq 0x811c9d5c             ; the same pad: no charge
0x811c9f0a  str r0, [r4, #0x6f80]      ; remember it
0x811c9f0c  cmp r1, #0xe               ; DAT_8153fd24
0x811c9f12  ...
0x811c9f18  orrs r1, r1, #0x8000000    ; ship+0x60a4: the EMP charge flag
```

**No weapon in mode 14.** The pending pickup is spent in the weapon update at `0x8120e74c`
(code Ghidra holds no function for): with `slot+0x200` set and `slot+0x208` set, mode `0xe`
(`0x8120e97a`) clears `slot+0x200` (`0x8120e996`) and returns to the main path. Before that it may
play `WEAPONPICKUP` (string `0x814a6418`) for a pilot. Every other mode goes to `0x8120ea7e`, the
award. Confidence **75**: the charge arm read instruction by instruction; the award skip read from
the disassembly only, its `slot+0x208` gate unread.

`ship+0x6f80` is written in three places: here, at construction, and in `Pilot_UpdateLapProgress`
(`str r1, [r5, r0]` with `r1 = 0` at `0x811c2f96`, the same block that zeroes
`craft+0x5e0`). That block runs when `r6 != 0` (`0x811c2f64`); `r6` is the byte stored at
`pilot+0x5620` just before. It reads as the line-crossed flag but was not traced to its writer,
hence **65** for "again after the next lap".

The same function also ORs in a debug path (`FUN_8123221e(&DAT_818bbf88)` then
`FUN_81232582`) that can raise the pickup without a pad. It is unread; it never fires in a race
without that input.

## The counts: `WeaponStats_ParseEmp` (`0x8100f7c2`)

This is the only reader of the strings `minNumPadsToCollectEMP` (`0x8141c548`) and
`maxNumPadsToCollectEMP` (`0x8141c560`). Each is read as a float, converted with `vcvt.s32.f32`
and stored: the value before them at `+0x230` (`0x8100f844`), the minimum at `+0x234`
(`0x8100f868`), the maximum at `+0x238` (`0x8100f88c`). Those are the two words `FUN_8117e9c2`
reads. 2048's archive ships `Data\XML\weaponstats_Detonator.xml` (HD's name, no `_2048` suffix;
`oag_2048`'s `weapons.detonator`), and
`emp::Config` reads `4` and `8` from it (`a_2048_detonator_pad_charges_the_emp_and_gives_no_boost_or_pickup`).
Confidence **70** for the name (three stores read, the rest of the function not); **80** for the
two offsets.

## What differs from HD

- **No net guard.** HD's speed arm runs with the net byte clear and HD's weapon arm sets the flag
  only with it clear. 2048's two arms test the mode alone. This changes nothing offline. A
  2048 online Detonator race is not in this engine.
- **Field offsets.** Last speed pad `craft+0x274` (same as HD), last weapon pad `ship+0x6f80`
  (HD `+0x7960`), flag word `ship+0x60a4` (HD `+0x6a64`). The bit is the same.
- Nothing else: same pads, same once-per-entry edge, same once-per-pad-per-lap weapon latch, same
  counts.

## Omega: checked, applies, ported

`/omega/eboot-ps4-omega-eu.bin` (the base import; the addresses on
[`detonator-emp.md`](../ps3-hdfury-eu/detonator-emp.md) "2048 and Omega" are a v1.07 import):

- speed pad: `0x0131a42f` `cmp [0x01f999e4], 0xe`; a pad other than `craft+0x2e0` is stored and
  `ship+0x71f4 |= 0x8000000` (`0x0131a493`), then `WEAPONPICKUP` (`0x01817683`);
- weapon pad: in `WeaponPads_TestCraft`, a pad other than `ship+0x8130` is stored (`0x01305944`)
  and, with `[0x01f999e4] == 0xe`, `or byte [ship+0x71f7], 0x8` (`0x01305954`): bit 27 of the
  same word.

Both are HD's law with no net guard in the lines read. Omega's `weapon_pad_charges` moves from
inherited to **measured** (75); its value does not change.

## Wired

`oag_2048::race::DETONATOR.pads` is `speed_pad_charges` and `weapon_pad_charges` **measured
`true`**: `oag_raceplay::pads` runs HD's arms on 2048 with no code change. Tests in
`crates/raceplay/tests/detonator_ground_truth.rs`:

- `a_2048_detonator_pad_charges_the_emp_and_gives_no_boost_or_pickup` (DLC1 Vineta K): a lap on
  the autopilot charges 4 from the speed pads with no boost, a hand-placed speed pad charges once,
  a weapon pad charges once and hands out nothing, the same pad again charges nothing, another
  pad charges, and the counts are `4`/`8`;
- `a_2048_detonator_race_on_zone_1_takes_the_hd_fury_dlc3_table`: a hand-placed speed pad charges
  with no boost. The autopilot's `zone_1` line enters none of the seven speed pads over a
  2120-tick lap, and the circuit has no weapon pads. A single race on the same circuit still
  boosts on that pad.
