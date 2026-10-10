# Pilot Assist

HD/Fury and 2048 offer Pilot Assist, a guidance system for new players: a spring on the
track's own spline that yaws the craft back from a wall it is about to meet, paid for with a
few percent of thrust. This page is the behaviour; the instruction-level evidence for HD is
[pilot-assist.md](../ghidra/functions/ps3-hdfury-eu/pilot-assist.md).

## HD/Fury's law (confidence 75, static)

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
