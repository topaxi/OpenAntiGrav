# HD handling against RPCS3: late speed still differs

2026-10-08, `hd-handling`; 2026-10-09, `hd-pitch-airbrake`, `hd-airbrake`. Results:
`docs/physics/hd-handling-ground-truth.md`. Tool: `scripts/rpcs3-trace.py`
(`docs/reverse-engineering/rpcs3-capture.md`, "A per-frame craft trace"). HD functions:
`docs/ghidra/functions/ps3-hdfury-eu/physics.md`, "The handling update", and
`docs/ghidra/functions/ps3-hdfury-eu/hover-four-point.md` and `craft-inertia.md`.

Thrust, steering and its ramp, the airbrake ramp and sideshift match Pulse's laws on HD's own
tables. ~~Pitch settles 30% shallower~~: closed 2026-10-09. HD hovers on four probes, `0.15` each,
all cast every frame and pushed along the hit normal (`Craft_HoverFourPoint`, `0x000ede88`); now
`oag_hd::race::HOVER_RIG`, and ours settles `+6.25` / `-2.99` deg against HD's `+6.3` / `-2.8`.

~~One airbrake turns HD 9-14% less~~: closed 2026-10-09 (`hd-airbrake`). HD builds the craft's
box inertia with mass `1.0` (`I_yy` 24, `Ship_Construct` `0x000debc8`) where Pulse passes `0.9`,
and HD's steering ramp stops at its target where Pulse's cycles `83.3/91.7/100`; the old steering
match was the two cancelling. Both are `oag_hd::race::CRAFT_LAWS`; one airbrake `-11.54` against
HD `-11.3` at 0.5 s, steering `-46.28` against `-46.2` at 0.8 s, yaw rate within 1.6% every frame.
`oag-game --trace-out` now writes `omega_*`/`avel_*`.

## Open

- **The AI's yaw ceiling on HD** (`oag_ai::driver::pace::hull_yaw_ceiling`) still divides by
  Pulse's `I_yy` 21.6, so it reads `1.667` rad/s where HD's Feisar reaches `1.5`. An AI-lane
  change: pass the craft's own inertia in.
- **Omega's craft laws** are not located: no `12.0f` immediate near the craft code, no steering
  ramp among the `vminss` users in `0x01310000`-`0x01330000`. Omega keeps Pulse's (`craft_laws:
  None`). 2048 not checked.
- **Speed past 2 s is about 1% low on HD** (`119.4`-`120.0` against `120.7` at 3 s). Shape fits
  Pilot Assist's `generalThrustPercentWhenEnabled="99"`, but the flag is cleared and no 0.99
  sits on the craft.
- **Omega**: `Craft_UpdateSurfaceProbes` (`0x0131b510`) marches HD's four hull segments; its
  spring share and offsets are unread, so Omega keeps `hover_rig: None`. 2048 not checked.
- **Not measured**: barrel roll, weapon slowdown, sideshift's gesture timing (as before).
- HD's hover pieces not ported: the trailing 10-unit ray (`0x000ee510`), the escape's class
  gates, `craft+0x358`.

## Next Steps

1. AI: give `hull_yaw_ceiling` the craft's `Body::inertia` so HD's AI plans on `I_yy` 24. 30 min.
2. Omega: reach the craft update through `Craft_UpdateSurfaceProbes`'s vtable slot, then read its
   steering ramp and its `Body_SetBoxInertia` call's mass. 1-2 hours.
3. Read Omega's spring (the reader of `+0x330..+0x450`) to wire its rig. 1 hour.
4. The 1% late-speed gap: Pilot Assist's `99` is the only shape that fits; find what scales
   `amount` past `accelcap`. 1-2 hours.
