# 2048's `PlayerInput_Update`: the same stick law

2026-10-07, stick-curve lane. Static reading. Full law:
[`../ps4-omega-eu/player-input.md`](../ps4-omega-eu/player-input.md).

Constructor `FUN_811b2f72` (`"player input %d"`, `Backend/Ships/PlayerInput.cpp`
at `0x81492af0`) sets vtable `0x81511c88`. Slot 3 is `0x811b3073` (Thumb), code
at `0x811b3072`: `PlayerInput_Update`, confidence 75.

- `fVar154 = 0.1` is the deadzone when the scheme word
  (`DAT_818a5094[player*0x51]`) is 0; for other schemes it is read from the pad
  record (`pad + 0x5e5c`, a configurable value).
- `DAT_818a6bec = (1.0 / (1.0 - dz)) * 100.0`, computed once behind a static
  guard (`DAT_818a6be8`): the same latch-on-first-use as HD.
- Per axis: `|v| <= dz -> 0`, else `(v -+ dz) * DAT_818a6bec` into `param_5+0x48`
  (steer) and `+0x5c` (pitch). The d-pad writes `0xc2c80000` / `0x42c80000`.

So 2048 has HD's code (including the latch); Omega's folded constant is the
later form. Status against Omega: **checked, applies** (same law, plain stick).
