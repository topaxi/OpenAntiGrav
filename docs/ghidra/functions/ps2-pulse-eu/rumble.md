# Vibration: PS2 Pulse's own, a strength and a time

Functions in `SCES_547.48` (Wipeout Pulse, PS2, SCES-54748), image base
`0x00100000`. Read 2026-11-04, no live run, so capped at 84. There are no pattern
files: the PS2 build drives the DualShock 2 from code, with **two gameplay triggers
and one menu preview**. The whole picture is
[gameplay/rumble.md](../../../gameplay/rumble.md).

**The PSP has none of this.** Pulse PSP and Pure hold neither the `VibOnOff` key
nor the `1800.0` scale (no byte or `lui` match in `BOOT.BIN`), and the PSP has no
motor.

| Address | Name | Conf |
| --- | --- | ---: |
| `0x00150cc0` | `Rumble_TriggerForPlayer` | 80 |
| `0x00202850` | `Input_SetVibration` | 84 |
| `0x00202928` | `Input_UpdateVibration` | 84 |
| `0x002028e8` | `Input_StopVibration` | 84 |
| `0x002020e0` | `Input_DisableVibration` | 84 |
| `0x002020c0` | `Input_EnableVibration` | 84 |
| `0x0010af40` | `Pad_SetMotors` | 84 |
| `0x00212c08` | `Pad_SetSmallMotor` | 80 |
| `0x00212c30` | `Pad_SetLargeMotor` | 80 |
| `0x00242270` | `Pad2_SendActuator_q` | 60 |
| `0x00156ec8` | `Ship_Destroy_q` | 65 |
| `0x001545c8` | `Ship_Damage` | 80 |
| `0x0018a080` | `Options_ReadVibration_q` | 68 |

## The law

`Rumble_TriggerForPlayer(duration, player, strength)` returns when the race state
is 2 (`DAT_002dabd0`, unless `DAT_0027a7a8`), or when `g_settings+0x2c4` is set,
then calls `Input_SetVibration`:

1. Skip if the player's enable flag `0x002849a0[player]` is 0 (both start 1).
2. Skip unless `+0x170`, the time since the last vibration ended, is at least
   **0.3 s** (`0x002849ac`).
3. Clamp strength to 0..255.
4. If strength >= **240** (`0x002849a8`), multiply duration by **4**.
5. Store duration at `+0x16c` and strength at `+0x168`, in the player's `0x138`
   input block.

`Input_UpdateVibration`, from `Input_BuildState`, per update with `dt`: adds `dt`
to `+0x170`. If strength or time is 0: stop. Otherwise
`Pad_SetMotors(player, strength, strength >= 240)`, takes `dt` off the time, and
stops at 0. While it runs, `+0x170` is held at 0, so a request during a vibration
is dropped, it does not replace it.

So one vibration runs at a time, a flat level, with no envelope. A strength of 240
or more also starts the small motor and runs four times as long.

`Pad_SetMotors` caches into the pad: `Pad_SetSmallMotor` sets bit 0 of `pad+0x20`,
`Pad_SetLargeMotor` puts the byte at bit 1 up. `FUN_00212950` sends the two bytes
through `Pad2_SendActuator_q`, RPC `0x0103400b` of libpad2 (`PsIIlibpad2 3100`).
The link to the motor is the reading, not a captured one: confidence 60 on that
RPC, 84 on the rest.

## The triggers

| Site | Event | Duration | Strength | Who |
| --- | --- | ---: | ---: | --- |
| `0x00154688` | `Ship_Damage`, after its state checks | 0.1 s (`0x0027e8d8`) | `damage * 1800` (`0x0027e8d4`) | human craft (`+0x3f8 == 0`) |
| `0x0015718c` | `Ship_Destroy_q`, beside `Camera_ArmShake(0.3, 0.4, 3)` | 0.5 s, so 2.0 s | 255 | human craft |
| `0x0018a14c` | Options, on a change of `VibOnOff` | 0.1 s, so 0.4 s | 255 | front end |

`Ship_Damage` here matches the PSP's (`0x088439ac`,
[shield.md](../psp-pulse-usa/shield.md)) by its five arguments, the amount first, its
state returns and the difficulty halving. It returns first for craft states 2, 4,
5, 6 and flag `0x1000`. The
damage argument's unit is unread at this call: with energy on a 0..100 scale nearly
every hit would clamp to 255, and on a 0..1 scale only a hit over 0.133 reaches
240. **Open**, and the first thing to measure on PCSX2. The call sits before
`Ship_Damage`'s difficulty halving, so strength takes the unscaled damage.

## The master setting

`Options_ReadVibration_q` runs on the `Controls`, `InGame Settings` and
`InGame Settings TT` screens, reads the menu value `VibOnOff`, and stores
`g_settings+0x2c4 = (value == "FE_OFF")`. Changing it fires the preview above.
`DAT_0027f0a0` holds the previous value. The trigger returns when the option is off,
so only switching it on is felt.

`Input_DisableVibration` and `Input_EnableVibration` set the per-player flags, from
the front-end functions `FUN_001c2390`, `FUN_001c2618` and `FUN_001c2698`; when
they fire is unread.
