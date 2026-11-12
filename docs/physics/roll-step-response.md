# The roll step response: Pulse's roll oscillator is not 13 % stiff

2026-11-11, lane `roll-stiffness`. Pulse PSP USA (`UCUS98712`) in PPSSPP v1.20.4, Time Trial,
Talon's Junction, VENOM, the profile's default craft, standing on the grid after the countdown.

**Result.** The original's roll oscillator and this engine's run at the same frequency: a half
period of `0.6249 s` measured against `0.6250 s` here (`0.02 %`). The "13 % stiff" gap
(`5.06 rad/s` here against a measured `4.49`) was an error in how the first capture was reduced,
not a property of the game or of the recovered tensor. Confidence **92**: four captures, two
signs, two amplitudes, eight zero crossings each, and the old capture refitted gives the same
answer.

## What was measured

`scripts/psp-roll-step.py` breaks in `Ship_UpdateCraft` (`0x08849618`) once per frame. After a
settle it rotates the rigid body's rows 0 and 1 about row 2 (the nose) by the step angle, writes
the transposed copy at `body+0xc0`, zeroes the rate columns `body+0x150` and `body+0x160`, and
then records the basis, both columns, `dt` (`craft+0x1c8`) and `grounded` (`craft+0x2b0`) on every
frame. Roll is the signed angle of row 1 from its pre-step value about the nose.

```sh
scripts/ppsspp-start.sh <lane> <display> <port> data/cache/pulse-psp-usa.iso
uv run --with websocket-client python3 scripts/psp-drive.py --port <port> menu
uv run --with websocket-client python3 scripts/psp-roll-step.py --port <port> \
    --roll 0.25 --settle 60 --frames 300 --out data/traces/roll-step.csv
```

`grounded` read `1.0` on every frame of every run; `dt` averaged `0.016683 s`.

| Step (rad) | Zero crossings (frames after the write) | Half period | Same-side peak ratio over one period |
| ---: | --- | ---: | ---: |
| `+0.25` | 20.79, 58.31, 95.79, 133.26, 170.73, 208.21 | 37.484 frames, `0.62535 s` | `0.3346` |
| `+0.25` | 20.79, 58.31, 95.79, 133.26, 170.72, 208.20 | 37.483 frames, `0.62532 s` | `0.3345` |
| `-0.25` | 20.80, 58.29, 95.81, 133.22, 170.78, 208.07 | 37.455 frames, `0.62490 s` | `0.3340` |
| `+0.10` | 20.76, 58.18, 95.73, 133.06, 170.75, 207.83 | 37.414 frames, `0.62419 s` | `0.3337` |

Mean half period `0.6249 s`, spread `0.0005 s`: a damped frequency of `5.027 rad/s`. The peak
ratio is taken between peaks on the same side (third over first), which cancels a constant
offset in the level reference; the two smaller-signal rows alternate by a few percent peak to
peak without it.

This engine under the same step (`oag-physics`'s
`a_rolled_craft_swings_through_level_at_the_originals_period`, flat floor, `1/60 s` frames):
crossings at frames 21.05, 58.60, 96.11, 133.61, 171.11, 208.60, a half period of `37.50` frames
or `0.6250 s`, and the frame map `theta'' = -k * theta - c * theta'` fitted to its roll series
returns `k = 25.643`, `c = 2.000` with a residual of `2e-6 rad`, which is `400 / 15.6` and the
`-2.0` roll damping exactly.

So `ALIGNMENT_GAIN = 400` (`0x0884ac5c`), the box tensor's roll entry `15.6`
(`Body_SetBoxInertia`, [rigid-body.md](../ghidra/functions/psp-pulse-usa/rigid-body.md)) and the
roll damping `-2.0` stand as recovered, now with a live measurement of their combined effect on
the roll axis (`k / c` and `k` separately, to `0.1 %` on the period).

## Where 4.49 came from

The first capture ([README.md](README.md), "Alignment gain: the measurements behind the
numbers") read the first zero crossing, frame 21, as a quarter of the period:
`2 * pi / (4 * 21 / 60) = 4.49 rad/s`. A damped oscillator released from rest does not cross zero
after a quarter period. With `theta(0) = theta0` and `theta'(0) = 0`,

```text
theta(t) = A * exp(-c*t/2) * cos(omega_d * t - phi),   phi = atan(c / (2 * omega_d))
first zero at t = (pi/2 + phi) / omega_d
```

At `c = 2` and `omega_d = 5.027` the phase is `0.196 rad` and the first crossing falls at
`0.3516 s`, frame 21.1. Reading that as a quarter period gives `4.47 rad/s`. The whole "gap"
is `phi`, and `5.064 / 4.49 = 1.128` is the 13 %.

The old capture itself agrees once fitted with the frame map in place of the quarter-period
rule: its 28 recorded values (four decimals) fit `k = 25.35`, `c = 2.15`, a natural frequency of
`5.03 rad/s`, with a residual of `0.0005 rad`; `k = 20.2`, `c = 1.6` leaves `0.020 rad`.

## The damping: the angular body damping is flown by the original

The decay is the one place the two differ, by `0.3 %` of amplitude a swing, and the unflown body
damping accounts for it. Same-side peak ratio over one period:

| | Ratio |
| --- | ---: |
| original, four captures | `0.3337` to `0.3346` |
| this engine, `Laws::PULSE` (no body damping) | `0.3366` |
| this engine, `body_damping.angular = 0.01` | `0.3345` |

First five peaks of the `+0.25` step as fractions of the step: original `0.5778, 0.3340, 0.1933,
0.1118, 0.0648`; this engine with the angular term `0.5769, 0.3334, 0.1929, 0.1116, 0.0646`,
without it `0.5788, 0.3356, 0.1948, 0.1131, 0.0657`. So Pulse's `Body_Integrate` angular damping
(`0x0884e374`, `body+0x380 = 0.01`,
[rigid-body.md](../ghidra/functions/psp-pulse-usa/rigid-body.md)) is live in the original at
about `0.01` (confidence **85**: the difference is five times the spread of the two clean runs,
and the two noisier runs sit on the same side). It lowers the damped frequency by `0.01 %`, so
it is no part of the old frequency gap. Pulse still flies `BodyDamping::NONE`, for the reason
[body-damping.md](body-damping.md) gives ("Why Pulse does not fly it yet").

## Open

- **The original leads by about a quarter of a frame from the release on.** Its first frame
  moves `0.00075 rad` where the frame map gives `0.00059` (`1.26x`; `1.22x` on the `0.1 rad`
  step, so it scales with the step), and from the second frame the per-frame differences are a
  constant `0.0004 rad`: an extra `0.026 rad/s`, about a quarter of one frame's alignment
  acceleration, delivered once. Every crossing is then `0.27` frames early while the spacing
  matches. It may be the write itself (the capture rewrites the basis mid-frame and the hull's
  cached state is not rewritten with it) or a real first-frame term. Next read: repeat with the
  write placed at the call to `Body_Integrate` in place of `Ship_UpdateCraft`'s entry, and see
  whether the lead moves.

## Other titles

- **Pure: checked, applies, not measured live.** Its constructor passes the same box
  (`0.9`, `(12, 8, 12)`) to `Body_SetBoxInertia` (`0x08934530`,
  [craft-physics.md](../ghidra/functions/psp-pure-usa/craft-physics.md)) and it runs Pulse's
  physics here ([pure-physics.md](pure-physics.md)), so the same test covers what Pure flies. No
  step capture was taken on Pure's disc.
- **HD: checked, differs, not measured live.** The same `-400` alignment term
  ([hover-four-point.md](../ghidra/functions/ps3-hdfury-eu/hover-four-point.md)) acts on a box
  built at mass `1.0`, roll entry `17.333`
  ([craft-inertia.md](../ghidra/functions/ps3-hdfury-eu/craft-inertia.md)), so the predicted
  natural frequency is `sqrt(400 / 17.333) = 4.80 rad/s`. There was never a "13 %" claim against
  HD: the figure was Pulse's capture alone. Not measured: an HD step needs the RPCS3 write path
  on the body's basis, which this lane did not run.
