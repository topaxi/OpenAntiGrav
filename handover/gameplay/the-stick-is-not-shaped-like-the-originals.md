# The stick is not shaped like the original's

2026-10-07, found by the novice-airbrake lane. The original shapes the steering
stick before anything reads it, and the port does not:

- **Original** (`PlayerInput_Update`, `0x0883c870`): `0.2` deadzone
  (`g_stick_deadzone`), gain `125` (`g_stick_gain`), then an S-curve
  (`v < 50 -> v/2`, else `1.5 v - 50`) onto the `+/-100` record. Measured live:
  see [input-bindings.md](../../docs/ghidra/functions/psp-pulse-usa/input-bindings.md#novice-r-picks-its-airbrake-off-the-steering).
  The same shaping applies to the pitch axis (record `+0x10`), deadzone and
  gain only, no curve.
- **Port** (`oag_input::pad`): `0.15` deadzone, rescaled, linear.

So a half-deflected stick (`0.5` as the game reads it) steers `18.75` on the original and about `41` here. It
affects every player's steering feel, and novice's airbrake threshold sits on
the shaped value (`0.36` raw on the original, `0.1` of the port's shaped stick).

2026-10-07, stick-curve lane: the modern titles' law is recovered (static).
Omega, HD and 2048 all shape the stick per axis as zero below 0.1, then
`(|v| - 0.1) * 100 / 0.9` linearly onto +/-100, no S-curve, pitch identical,
and there is no stick sensitivity setting (only thrust, airbrake and motion).
Evidence and the 0..1 table against Pulse and ours:
[player-input.md](../../docs/ghidra/functions/ps4-omega-eu/player-input.md).
Ours (0.15, rescaled, linear) already has that shape.

## Open

- Maintainer's call: keep 0.15 linear, move to Omega's 0.1 linear (one constant,
  `STICK_DEADZONE`), or offer Pulse's curve as an `original` value on Pulse.
- HD live check not done: the byte-to-float stage upstream of `PlayerInput`
  (does HD zero a byte window like Omega's `Pad` layer, `/128` or `/127.5`)
  needs the RPCS3 sweep written in
  [the HD page](../../docs/ghidra/functions/ps3-hdfury-eu/player-input.md).
- The Omega `Pad_Open` dead-zone bytes (`+0x10fc`) read as the controller's own
  reported dead zones, confidence 60; the DS4 runtime value is unmeasured.
- Pulse's pitch shaping is read, not measured.

## Next Steps

1. Ask the maintainer which of the three options above.
2. If a change: apply it in `oag_input::pad` (the device layer), not in
   `oag_gameplay::ship_controls`, so keyboards and the touch stick choose for
   themselves; test with the table's rows.
3. Optional: the RPCS3 HD sweep, to lift the Omega/HD reading past 84.
