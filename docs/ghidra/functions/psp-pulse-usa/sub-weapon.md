# `SubWeapon`: the line under the pickup's icon

One text widget, `SubWeapon`, is written by the player's status update **every frame**. It
prints the Cannon's rounds left and the Autopilot's seconds left, and is empty otherwise. Read
in all five titles' executables on 2026-10-26 (`cannon-ammo-hud`); the Pulse reading is also
watched live in a frame of the original. Ours draws it as `oag_hud::SubWeapon`
(`Readout::sub_weapon`, `oag_hud::draw::text_for`).

| Address (Pulse USA) | Name | Confidence |
| --- | --- | ---: |
| `0x0883b3b8` | `PlayerStatus_Update` (named, [race-progress.md](race-progress.md)) | 92 |
| `0x0885776c` | `WeaponPickup_ArmCannon` | 85 |

## What `PlayerStatus_Update` writes

For the local player, with a pickup object (`*(entity+0x4c)`) and `g_game_mode != 0xc`
(`AI Race`; or the debug override), it builds one string in a stack buffer and hands it to the
widget. The buffer starts as the empty string (`0x08a7b5a8`, one NUL).

| Step | Instructions | What |
| --- | --- | --- |
| 1 | `0x0883be08..0x0883be48` | `pickup+0x1b8 & 0x800` (the Autopilot's running bit, [autopilot.md](autopilot.md)): `sprintf(buf, "%1.2f", (double)pickup+0x148)`, the format at `0x08a7b5ac` |
| 2 | `0x0883be54..0x0883be98` | the same branch's compare of the timer with `1.0` feeds `Craft_SetAutopilotBlend` (`0x08848664`): **it is the controls' blend, not a restyle of the text** |
| 3 | `0x0883bed0..0x0883bedc` | with the running bit set and the icon id (`+0x90`, from the held id) still `0`, the icon id becomes `6`: **the Autopilot's own icon stays up while it runs** |
| 4 | `0x0883bf60..0x0883bf8c` | `pickup+0x1bc == 3` (the Cannon): `sprintf(buf, "%d", pickup+0x154)`, the format at `0x08a7b5b4`, **over** whatever step 1 wrote |
| 5 | `0x0883bfc4..0x0883c01c` | the widget is `g_hud+0x248` (cached by `Hud_BindWidgets`) or found by name `"HUD->" + "SubWeapon"` when `g_hud+0x169` is clear; `FUN_088ccf6c(widget, buf, 0)` copies the buffer into the widget's text |

Confidence **92** for the Autopilot half: seen live as well as read - `pulse-psp-race/talons-alongside`
shows `5.00` under the Autopilot's hexagon, at the widget's authored place
(`Arcade_HUD.xml`: `x=240 y=45`, `HUD` font, scale `0.6`, centred), and ours draws the same
glyphs in the same place (`2.67` in the same crop). Confidence **88** for the Cannon half: the
same widget, buffer and call, the number read from `pickup+0x154` in the decompile and not
seen on a captured frame.

## The Cannon's counter is armed when the pickup is granted

`WeaponPickup_Grant` (`0x08861d20`, the only caller) calls `FUN_0885776c` for the Cannon:

```c
void WeaponPickup_ArmCannon(void *unused, Pickup *pickup) {   // 0x0885776c
    pickup->held      = 3;                                     // +0x1bc
    pickup->held_copy = 3;                                     // +0x1c0
    pickup->shots     = ActiveCannonStats()->rounds;           // +0x154 <- stats +0x70
    cannons_granted++;                                         // DAT_08ab0ee8
}
```

So the HUD reads the full `<Stats rounds>` (`30`) the moment the pickup is taken, and
`Cannon_UpdateReload` ([cannon-quake-leachbeam.md](cannon-quake-leachbeam.md)) decrements it per
round. Held by id `-1` after the last round (`shots == 0`), the line goes empty. Ours arms its
counter on the first round instead (`Held::cannon_rounds`, chosen), so the readout shows the
authored count while that counter reads `0` - the same number the original has in it.

## The `1000000000.00` on the capture frames is this line

`zone-speed-class.md` recorded a `1000000000.00` in `SubWeapon`'s buffer as "a leftover ... the
writer was not found". The writer is step 1. `scripts/psp-autopilot-rail.py`,
`psp-postrace.py` and `psp-ai-thrust.py` arm the Autopilot pickup's timer at `1.0e9` so a saved
state keeps flying, and the HUD prints that timer: `%1.2f` of `1e9`. The Time Trial grid state
(no Autopilot) reads empty, which is step 0. `psp-pulse-pair.py` now puts the authored `5.0` back
before the screenshot, so a pair shows `5.00`.

## Cross-title

| Title | Where | Reading |
| --- | --- | --- |
| HD/Fury | `0x000cb928` (`PlayerStatus_Update`, [autopilot.md](../ps3-hdfury-eu/autopilot.md)) | **ported**. Timer `%1.2f` at `pickup+0x158` (`0x000cca84`), running bit `pickup+0x200 & 0x800`; Cannon `pickup+0x204 == 3`, `"%d"` of `pickup+0x164` (`0x000cca40..0x000cca50`, strings `0x00781b40`, `0x00781b48`); text written with `FUN_00202d28(widget, buf, 0, 0)` at `0x000cc124`, the widget `hud+0x2a4`. Authored in every pickup layout (`hud_pickups.xml`: `HUDSmall` scale `0.9`, centred, `y=22` under the icon). Wired and drawn |
| Omega | `FUN_012e5470` | **ported**, read in the PS4 executable. Timer `%1.2f` at `pickup+0x158` (`0x012e665f`), Cannon `pickup+0x218 == 3`, `"%d"` (`0x0183cb92`) of `pickup+0x164` (`0x012e66d4..0x012e66f1`), text write `0x014d2ff0(widget=hud+0x418, buf, 0)` at `0x012e67ea`. The `"SubWeapon"` string (`0x01814c84`) has no resolved reference, so the widget's identity is inferred from the neighbouring strings (the same three as Pulse's cluster) - confidence 78 |
| 2048 | `FUN_811b1ee2` (v1.04, `Player_UpdateHudData`) | **checked, applies, not wired**. The same writer: `%1.2f` of `pickup+0x148`, `pickup+0x1f8 == 3` then `"%d"` (`0x814925ec`) of `pickup+0x154`, `FUN_810e33d4(widget, buf, 0, 0)`; the `0x800` bit and `g_game_mode != 0xc` as on Pulse. The layout is the difference: `SubWeapon` sits in `HUD_pickups.xml`'s top-centre cluster (`OffsetX=480 OffsetY=60`) beside the grant announcement (`PickupText`, the big `PickupIcon`, `GiftCannon`), while the icon the player carries is `PickupBgFrame` bottom-left in `HUD_damage_indicator.xml`. Whether that cluster is up outside the announcement is not read (`oag_title::HudArt::sub_weapon_line` is `false`) |
| Pure | around `0x08921a18..0x089224c8` (`PlayerStatus_Update`'s twin, unnamed) | **Autopilot half only**. The running flag (`lbu +0x154` of the status block's pickup object) selects `sprintf(buf, "%1.2f", pickup+0x15c)` (`0x08922340..0x0892236c`) and `SubWeapon` is looked up by name after it (`0x08922484..0x089224c8`). **No Cannon**: no `"%d"` in the function's string cluster (Pulse has `"%1.2f\0%d"`; Pure has `"%1.2f"` then `"SubWeapon"`), and the roster has none ([weapon-stats.md](../../../formats/weapon-stats.md)). Drawn |
| Pulse PS2 | - | **not checkable** here beyond the strings: the PS2 executable carries `SubWeapon` in its layout-name pool, its writer is unread. Ours is shared across the two pressings |

## What ours does not do

- **`g_game_mode == 0xc` (`AI Race`)** prints nothing in the original; ours has no such mode.
- **Pure's pickup icons do not draw in `--race`** (no weapon's, so not the Autopilot's either): a
  pre-existing gap outside this lane, so the timer prints with no hexagon under it.
- **A craft holding the Cannon while an Autopilot runs** shows the Cannon's count (step 4 over step
  1), and ours matches; whether the original lets the two coexist was not read.
