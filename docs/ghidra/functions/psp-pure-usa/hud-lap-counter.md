# Pure's lap counter closes up on lap one

**Binary:** `psp-pure-usa` `BOOT.BIN`, image base `0x08804000`, Ghidra
`/pure/BOOT-psp-pure-usa.BIN`. Read 2026-11-27 (`menu-pairs` lane). Companion to
Pulse's `Hud_UpdateLapCounter` (`0x0881a690`,
[race-progress.md](../psp-pulse-usa/race-progress.md)).

## `Hud_UpdateLapCounter` (`0x0881ed14`, confidence 90)

Called once from the HUD update `0x0881e6bc` (at `0x0881e7b8`). Argument `a0` is the
HUD object; the lap and lap target are `*(hud+0x40)+0x8` and `+0xc`.

- Lap target below 1: writes `Lap`, empties `Lap Outof` and clears `LapOf`'s visible bit.
- Otherwise writes `Lap` and `Lap Outof`, shows `LapOf`, and **if the lap is 1** sets
  both widgets' `x` (`widget+0x1d4`) to the authored `x` less `17.0`
  (`0x0881ee00`-`0x0881ee24`, `lui a0,0x4188`); on any later lap it restores the
  authored `x`.

The authored `x` is saved at bind time by `FUN_08821838`: `hud+0x190` is the widget
looked up by `"Lap Outof"` (string `0x08a465b4`) and `hud+0x194` by `"LapOf"`
(`0x08a465c0`), with their `x` copied to `hud+0x198` and `+0x19c`. `Lap` itself
(`hud+0x18c`) is never moved.

So Pure's layout authors `LapOf` at `x="27"` and `Lap Outof` at `x="40"`
(`Data\XML\TimeTrial_HUD.xml` and `Arcade_HUD.xml`), and the lap-one picture is not
those numbers: it is `10` and `23`. This is the same law as Pulse's, read from
Pure's own code, not borrowed.

## What it explains

On the twelve Pure race pairs and `menu-pause`, all on lap one, ours drew the slash
and the total 17 px right of the original's
([compare-page.md](../../../tools/compare-page.md), "Pure's lap-one close-up").
