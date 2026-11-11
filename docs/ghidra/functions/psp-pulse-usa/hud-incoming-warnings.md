# The incoming-weapon warnings: `ForwardWarningIcons` and `RearWarningIcons`

**Binary:** `pulse-psp` `BOOT.BIN` (USA, `UCUS98712`), image base `0x08804000`.

**Status (2026-11-11):** read end to end and measured live. Which weapons raise a
warning, on which side, inside which along-track window, for how long and how it
blinks are all recovered, each checked live in PPSSPP through the game's own fire
path. One value is unread: what makes `FUN_0883dbf4`'s "ok" flag zero (see
[Open](#open)).

| Address | Name | Kind | Confidence |
| --- | --- | --- | --- |
| `0x0881a364` | `Hud_RaiseIncomingWarning` | function | 95 |
| `0x0881aa8c` | `Hud_UpdateIncomingWarnings` | function | 95 |
| `0x08ab0994` | `g_hud_incoming_warning_widgets` | data | 92 |

The caller is `Weapon_AnnounceIncoming` (`0x08871ddc`,
[mine.md](mine.md#weapon_requestfires-jump-table-read-as-a-table)): the same function
that plays the "incoming!" voice line raises the HUD warning, just before the voice.

## The law in one table

A weapon fired by **any craft other than the local player**, at the moment it is
fired, raises a warning when the along-track gap `g = s(player) - s(firer)` (the
wrapped difference of `entity+0x91c`, `FUN_0883dbf4`) lies strictly inside the
weapon's window. Positive `g` means the firer is behind the player.

| Held id | Weapon | Announce idx | Window (strict) | Slot | Widget |
| --- | --- | --- | --- | --- | --- |
| 0 | Rocket | 0 | `0 < g < 400` | rear | `RearWarningRocketIcon` |
| 1 | Missile | 1 | `0 < g < 400` | rear | `RearWarningMissileIcon` |
| 2 | Quake | 2 | `0 < g < 800` | rear | `RearWarningQuakeIcon` |
| 3 | Cannon | 5 | `0 < g < 500` | rear | `RearWarningCannonIcon` |
| 7 | Plasma | 7 | `0 < g < 500` | rear | `RearWarningPlasmaIcon` |
| 8 | Mine | 9 | `-500 < g < 0` | forward | `ForwardWarningMineIcon` |
| 9 | Bomb | 8 | `-500 < g < 0` | forward | `ForwardWarningBombIcon` |
| 10 | LeachBeam | 10 | `0 < g < 500` | rear | `RearWarningLeachBeamIcon` |
| 11 | Repulser | 11 | `0 < g < 500` | rear | `RearWarningRepulserIcon` |
| 12 | Shuriken | 12 | `0 < g < 500` | rear | `RearWarningShurikenIcon` |
| 4, 5, 6 | Turbo, Shield, Autopilot | none | never | - | - |

Before the window, `Weapon_AnnounceIncoming` returns early when `g_game_mode >= 14`
(`slti a0,a2,0xe` at `0x08871df4` on `*(0x08b30f90+0xb8)`), when the firer is the
local player (`*(g_race_manager+0x2c0)`, `0x08871e2c`), when `FUN_0883dbf4`'s flag
byte is zero (`0x08871e44`), or when `|g| > 800` (`0x08871e54..0x08871e70`). The
window test is `lo < g < hi` (`c.le.s` then `c.lt.s`, `0x08871f98..0x08871fb0`;
`lo` starts `0.0` in `f13`, `hi` `0.0` in `f12`, each case loads one of them). Every
case's constant is read off the disassembly: `0x43c8` (400) for Rocket and Missile,
`f14` (800) for Quake, `0x43fa` (500) for Cannon, Plasma, LeachBeam, Repulser and
Shuriken, `0xc3fa` (-500) as the lower bound for Bomb and Mine.

**What the warning does not look at:** the projectile's target, the lock-on, the
projectile's flight or its distance after launch. It is a one-shot raise at fire
time, from the firer's position alone. A Missile locked onto another craft still
warns the player if it was fired from 0-400 behind; a Rocket fired from 450 behind
never warns, however close it later flies. **Confidence 92**: `Weapon_RequestFire`
(`0x08862d9c`) calls `Weapon_AnnounceIncoming(idx, record+0xf0)` with no target
argument in any of its ten cases, and the announce reads only the two crafts'
along-track positions.

## `Hud_RaiseIncomingWarning` (`0x0881a364`)

`(Hud *hud, int idx)`, called once, at `0x08871fc4`, with `hud = *(0x08ab0838)`.
`idx < 13` jumps through the table at `0x08a7a130`:

| idx | target | effect |
| --- | --- | --- |
| 8, 9 | `0x0881a388` | `hud+0x188 = idx; hud+0x180 = 2.0f` (forward) |
| 0, 1, 2, 5, 7, 10, 11, 12 | `0x0881a39c` | `hud+0x18c = idx; hud+0x184 = 2.0f` (rear) |
| 3, 4, 6 | `0x0881a3b0` | nothing |

`lui a2,0x4000` is the 2.0. A second raise in the same slot overwrites the index and
restarts the timer at 2.0; the two slots are independent, so a rear and a forward
warning can show together. `Hud_Construct` stores the same four fields at
`0x08819db0..0x08819dc4` (read live as `0.0`/`13` on an idle HUD).

## `Hud_UpdateIncomingWarnings` (`0x0881aa8c`)

`(Hud *hud, float dt)` (`dt` in `f12`, seconds). `Hud_Update` (`0x0881bf50`) calls it
at `0x0881c50c` when `hud+0x40 & 0x2000`, with its own `dt` (live: `0.0160..0.0173`,
the frame time, not a fixed 1/60). Per call:

1. `pf = trunc(hud+0x180 * 8) & 1`, `pr = trunc(hud+0x184 * 8) & 1`, computed from
   the timers **before** this call's decrement.
2. Forward background (`table[0]`): if `t_f > 0`, `t_f = max(t_f - dt, 0)` and its
   visible bit (`widget+0x2c & 4`) follows `pf`; else `hud+0x188 = 13`.
3. Rear background (`table[1]`): if `t_r > 0`, `t_r = max(t_r - dt, 0)` and visible
   follows `pr`; else visible is cleared and `hud+0x18c = 13`.
4. Each icon `table[2..12]`: visible iff (`icon.idx == hud+0x188` and `pf`) or
   (`icon.idx == hud+0x18c` and `pr`).

So the warning blinks at **4 Hz**: 0.125 s on, 0.125 s off, on while
`floor(8 t)` is odd. From 2.0 the first eighth of a second is off, then eight on
phases; it ends off (`t < 0.125` is phase 0) and the slot index resets to 13 on the
first call after `t` reaches 0. The forward branch's missing clear (step 2) is never
visible, because the last phase before zero is an off phase; `pf` would only be odd
at zero if `dt` jumped by more than 0.125 s.

`widget+0x2c & 4` is the visible bit: `Hud_BindWidgets` clears it on every warning
widget when it binds them (`0x088200c8`), and the screenshots below show the icons
exactly on the ticks the bit is set.

## `g_hud_incoming_warning_widgets` (`0x08ab0994`)

Twelve `{const char *name; Widget *widget; int idx}` records, stride 12, count at
`0x08ab0a24` (`12`). `Hud_BindWidgets` (`0x0882009c`) fills `widget` by looking up
`"HUD > " + name` only when `hud+0x40 & 0x2000`. Read from the file and from memory
live (identical):

| # | name | idx |
| --- | --- | --- |
| 0 | `ForwardWarningBackground` | 13 |
| 1 | `RearWarningBackground` | 13 |
| 2 | `ForwardWarningBombIcon` | 8 |
| 3 | `ForwardWarningMineIcon` | 9 |
| 4 | `RearWarningCannonIcon` | 5 |
| 5 | `RearWarningRocketIcon` | 0 |
| 6 | `RearWarningQuakeIcon` | 2 |
| 7 | `RearWarningPlasmaIcon` | 7 |
| 8 | `RearWarningMissileIcon` | 1 |
| 9 | `RearWarningRepulserIcon` | 11 |
| 10 | `RearWarningShurikenIcon` | 12 |
| 11 | `RearWarningLeachBeamIcon` | 10 |

The `idx` column is the **announce index** (file order, Bomb 8 and Mine 9), not the
held-weapon id (Mine 8, Bomb 9); the remap is [mine.md](mine.md)'s.

## Which modes show them

`hud+0x40 & 0x2000` is set by `ArcadeRace_Construct` (`0x0882c2b0`),
`Elimination_Construct` (`0x0882cb30`), `Tournament_Construct` (`0x0882e6f8`),
`MPRace_Construct` (`0x0882213c`), `MPElimination_Construct` (`0x088227ec`) and
`MPTournament_Construct` (`0x08823794`), from a full `ori ..., 0x2000` scan of the
binary. `TimeTrial`, `FreePlay`, `Zone`, `MPTimeTrial` and `AiBatchTest` do not set
it, so their HUD never binds or updates the warnings. Measured `hud+0x40`: `0x20df`
in a single race (this page's runs), `0x229f` in Eliminator
([hud-kill-column.md](hud-kill-column.md)). A race with **weapons off**
(`*(0x08b30f90+0x18) == 0`) never reaches `Ship_UpdateWeapons`
([autopilot.md](autopilot.md)), so nothing is fired and nothing warns.
**Confidence 88** for the mode list (static, one mode word per mode live).

## Live evidence, 2026-11-11

Own PPSSPP, `pulse-psp-usa.chd`, the `race-talons-venom-alongside` state (single
race, Talon's Junction, Venom, `hud+0x40 = 0x20df`), debugger breakpoints only.

**Blink and clear (planted timer).** At the first `Hud_UpdateIncomingWarnings`
entry, `hud+0x184 = 2.0, hud+0x18c = 1` was written, then 140 consecutive calls
logged. `RearWarningBackground` and `RearWarningMissileIcon` were off for calls
0-1, then alternated on 8 / off 7 or 8 calls (on at 2-8, 16-23, 31-38, 46-53, 61-68,
76-83, 91-98, 106-113; off between), eight on phases in all. `t_r` reached `0.0`
at call 120 and `hud+0x18c` read `13` from call 121. No other warning widget was
ever visible. The same run with `hud+0x180 = 2.0, hud+0x188 = 9` gave the identical
schedule on `ForwardWarningBackground` and `ForwardWarningMineIcon` and ended off.
Screenshots on an on call show a red hexagon right of the time-gap line (rear,
missile glyph) and left of it (forward, mine glyph); the off call shows neither.

**Windows and slots (the real fire path).** With the weapons byte set, for each
case one AI craft got held id `k` (`record+0x1bc`) and its fire byte
(`*(craft+0x94)+0x78`, `+0x15`) inside a `Ship_FireHeldWeapon` breakpoint, and its
`+0x91c` was set to `s(player) - g`. The game's own gap was read in `f0` at
`0x08871e40` (it equalled the planted `g` every time, flag byte `1`), and the HUD
fields on the next update:

| Held id | `g` | Result |
| --- | --- | --- |
| 0 Rocket | 100, 399 | rear, idx 0, `t = 2.0` |
| 0 Rocket | 401, -100 | none |
| 1 Missile | 300 / 420 | rear idx 1 / none |
| 2 Quake | 700, 799 | rear idx 2 |
| 3 Cannon | 450 / 520 | rear idx 5 / none |
| 7 Plasma | 480 / 510 | rear idx 7 / none |
| 8 Mine | -100, -499 | forward idx 9 |
| 8 Mine | -501, 100 | none |
| 9 Bomb | -200 / 100 | forward idx 8 / none |
| 10 LeachBeam | 250 | rear idx 10 |
| 11 Repulser | 250 | rear idx 11 |
| 12 Shuriken | 300 / 0 | rear idx 12 / none |
| 4, 5, 6 | 100 | announce never reached the gap call; none |

Twenty-five cases, all as the table predicts. Natural play did not produce one: the
saved state is a **weapons-off** race (byte `0`), so a full three-lap run with a
breakpoint on `Weapon_AnnounceIncoming` never stopped.

## Lineage

- **Pulse PS2:** inherited from PSP, not measured (the PS2 `*_HUD.xml` carries the
  same widget names, [hud.md](../../../ui/hud.md)).
- **Pure:** checked, differs. Pure's `BOOT.BIN` has none of these widget names;
  it carries `incomingFront` and `incomingRear` (`0x08a7aaa8`, `0x08a7aab8`), so its
  warning is a different widget model. Its law is not read.
- **HD/Fury:** checked, differs. The same twelve names and the same record table
  shape (`0x00920838`, bound in HD's `Hud_BindWidgets` at `0x0009c7f4`); the update
  is `FUN_0008f300` (`0x0008f300`, `(double dt, Hud *hud)`, timers at
  `hud+0x174/+0x178`, indices `+0x17c/+0x180`, visible bit `widget+0x34 & 4`). Two
  differences, static (75): the **backgrounds stay on** for the whole time a timer
  is above zero instead of blinking, and **the icons blink** on the same parity rule
  with the multiplier `DAT_008a7704` (unread; 8.0 if it matches Pulse). It also
  hides every warning widget while `RaceManager+0xad` is set (unread), and walks only
  ten icon records. HD's raise and window are not read.
- **2048:** checked, applies, not wired: `eboot.elf` (v1.04) carries the same twelve
  names plus `WarningIcons`; its update is not located.
- **Omega:** checked, differs: `eboot.bin` (v1.07) carries `ForwardWarningBackground`,
  `RearWarningBackground` and the three group names, but none of the ten icon names,
  so its icons are not looked up by these names. Not followed further.

## Open

- `FUN_0883dbf4`'s flag byte (`*sp` after the call at `0x08871e38`): read `1` in all
  22 live cases that reached it. What sets it to `0` (a craft not yet on the course, or a finished
  one) is unread; start at `0x0883dbf4`.
- HD's `DAT_008a7704` and its raise path: `Hud_BindWidgets` (`0x0009c7f4`) is the
  way in.

## How to reproduce

`scripts/ppsspp-start.sh <lane> :9N <port> data/images/pulse-psp-usa.chd`, load
`race-talons-venom-alongside.ppst` with `scripts/psp-state.py`, `scripts/psp-drive.py
restart`, then with `scripts/ppsspp_debugger.py`: write `1` to `0x08b30f90+0x18`
(weapons on), break at `0x08844ae8` (`Ship_FireHeldWeapon`, `a0` = craft), and for a
non-player craft whose `*(craft+0x4c)+0x1bc` is `-1` write the held id there, `1` to
`*(*(craft+0x94)+0x78)+0x15`, and `s(player) - g` to `craft+0x91c`
(`player = *(*(0x08b317b4)+0x2c0)`); break at `0x08871e40` for the gap (`f0`), then at
`0x0881aa8c` and read `hud+0x180..0x18c` (`hud = *(0x08ab0838)`). Only the most
recently added breakpoint fires on PPSSPP 1.20.4, so arm them one at a time.
