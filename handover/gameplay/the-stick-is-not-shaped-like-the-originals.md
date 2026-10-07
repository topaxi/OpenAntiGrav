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

So half deflection steers about `11` on the original and about `41` here. It
affects every player's steering feel, and novice's airbrake threshold sits on
the shaped value (`0.36` raw on the original, `0.1` of the port's shaped stick).

## Open

- Whether to adopt the original's curve for the stick, as a default or an
  `original` setting value (maintainer's call; it changes how every pad feels).
- The pitch axis shaping, read but not measured.

## Next Steps

1. Ask the maintainer whether the curve is wanted, and as what setting.
2. If yes: apply it in `oag_input::pad` (the device layer), not in
   `oag_gameplay::ship_controls`, so keyboards and the touch stick choose for
   themselves; test with the measured rows on the page above.
