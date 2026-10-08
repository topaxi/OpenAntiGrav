# HD handling against RPCS3: one airbrake and late speed still differ

2026-10-08, `hd-handling`; 2026-10-09, `hd-pitch-airbrake`. Results:
`docs/physics/hd-handling-ground-truth.md`. Tool: `scripts/rpcs3-trace.py`
(`docs/reverse-engineering/rpcs3-capture.md`, "A per-frame craft trace"). HD functions:
`docs/ghidra/functions/ps3-hdfury-eu/physics.md`, "The handling update", and
`docs/ghidra/functions/ps3-hdfury-eu/hover-four-point.md`.

Thrust, steering and its ramp, the airbrake ramp and sideshift match Pulse's laws on HD's own
tables. ~~Pitch settles 30% shallower~~: closed 2026-10-09. HD hovers on four probes, `0.15` each,
all cast every frame and pushed along the hit normal (`Craft_HoverFourPoint`, `0x000ede88`); now
`oag_hd::race::HOVER_RIG`, and ours settles `+6.25` / `-2.99` deg against HD's `+6.3` / `-2.8`.

## Open

- **One airbrake turns HD 9-14% less** (`-11.3` deg against `-12.8` at 0.5 s with the new rig).
  The yaw rate runs 9-13% under ours from the second frame at matching speed (drive-scale shape).
  Ruled out on HD: the airbrake force law (`0x000ee730`), steering (`0x000edb50`, same torque
  accumulator), lateral grip (`0x000eff78`), angular damping (`0x000eda30`), the four-point hull.
  Not located: HD's weathervane; whether the yaw inertia is applied in world axes as on Pulse;
  whether ours' traced ramp is one tick off the one its torque used (ours' per-frame yaw rate
  leads `speed * R * 9 * 0.001 / 21.6` by 14-25% on the first frames, HD's trails it by 3-12%).
- **Speed past 2 s is about 1% low on HD** (`119.4`-`120.0` against `120.7` at 3 s). Shape fits
  Pilot Assist's `generalThrustPercentWhenEnabled="99"`, but the flag is cleared and no 0.99
  sits on the craft.
- **Omega**: `Craft_UpdateSurfaceProbes` (`0x0131b510`) marches HD's four hull segments; its
  spring share and offsets are unread, so Omega keeps `hover_rig: None`. 2048 not checked.
- **Not measured**: barrel roll, weapon slowdown, sideshift's gesture timing (as before).
- HD's hover pieces not ported: the trailing 10-unit ray (`0x000ee510`), the escape's class
  gates, `craft+0x358`.

## Next Steps

1. Add the body's angular velocity to `oag-game --trace-out` and compare per-frame yaw-rate
   increments against HD's `body+0x1a0` for the airbrake and steering scenarios. 1 hour.
2. Find HD's weathervane: a `cross(forward, velocity)` into `0x000f5848` among `Craft_Update`'s
   callees (`0x000f1958`). 1-2 hours.
3. Read Omega's spring (the reader of `+0x330..+0x450`) to wire its rig. 1 hour.
