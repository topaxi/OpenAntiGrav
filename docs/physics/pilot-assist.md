# Pilot Assist

HD/Fury and 2048 offer Pilot Assist, a guidance system for new players: a spring on the
track's own spline that yaws the craft back from a wall it is about to meet, paid for with a
few percent of thrust. This page is the behaviour; the instruction-level evidence for HD is
[pilot-assist.md](../ghidra/functions/ps3-hdfury-eu/pilot-assist.md).

## HD/Fury's law (confidence 85, read live on RPCS3)

Each tick, for a local player's craft with the save option on, racing (not on the grid), in
any mode but Zone and Detonator:

1. **Look ahead** `la = min(laDistConst + speed * laDistVelMul, laDistMax)` along the craft's
   forward axis.
2. **Probe the corridor** at that point (radius `2`) and at the craft (radius `5`): locate the
   nearest track-spline record, take the lateral offset `d` across it, and measure how far
   inside `radius` of either half-width edge the point is: `L = max(0, r - left - d)`,
   `R = min(0, right - r - d)`. Facing more than ~127 degrees back down the track reads as no
   push.
3. **Blend** toward `1` while any hull point is grounded and the craft is upright on the track
   (`dot(down, up) <= -0.5`), toward `0` otherwise: up at `2` per second, down at `20`.
4. **Yaw** `clamp((L + R)_ahead * torqueMul * blend, +/-maxTorque)` as a local torque, zero while
   the craft already spins faster than `maxAngVel`; **push** sideways along the craft's own right
   axis by `(L + R)_here * springMul * blend`.
5. **Charge for it**: a yaw over `25` starts a `penaltyDuration` timer, and the throttle is
   scaled by `thrustPercentOnUse` while it runs, `generalThrustPercentWhenEnabled` otherwise.

It never reads the steering input. All numbers are per speed class from
`/data/xml/handlingstats.xml`'s `<GlobalClass>` rungs.

The penalty timer decays even while the assist is off, and the throttle keeps the
`thrustPercentOnUse` scale until it reaches zero; switching it off mid-race does not refund
the last three seconds.

**Measured on RPCS3** (the table is on the evidence page): aimed at the right-hand wall at
100 units/s and let go, the craft hits it and drops to 16 units/s with the option off, and
is yawed back clear of it and holds 100-120 units/s with it on.

## 2048: Normal and Extreme (confidence 65-72, static)

2048's three settings are Off, **Normal (its default)** and Extreme ("Super"). Extreme is HD's
law above, read from 2048's own global `<PilotAssist>` rungs, which are softer than HD's.
Normal is the same law run on a second object, read from the ship's own `<Assist>` block,
which is far gentler (a torque cap of 50 against 600-1000). Its strength ramps in with speed
between `SteerAssist.min_speed` and `min_speed + ramp_up_range`. The two never run together.
Evidence: [vita-2048-eu-v104/pilot-assist.md](../ghidra/functions/vita-2048-eu-v104/pilot-assist.md).

## The HUD indicator

Both HD and 2048 author four widgets: a background shown while the assist is enabled, a main
icon shown for 3 s after it acts, and a left and a right arrow that blink at 4 Hz for 1.5 s
on the side of the wall it steered away from.

## Other titles

- **Omega**: checked, applies, not wired: its executable carries 2048's whole set of names.
- **Pulse, Pure**: no Pilot Assist in either executable.
