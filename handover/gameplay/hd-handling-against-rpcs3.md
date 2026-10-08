# HD handling against RPCS3: pitch and airbrake response differ, the laws do not

2026-10-08, `hd-handling`. Results: `docs/physics/hd-handling-ground-truth.md`. Tool:
`scripts/rpcs3-trace.py` (`docs/reverse-engineering/rpcs3-capture.md`, "A per-frame craft
trace"). HD functions: `docs/ghidra/functions/ps3-hdfury-eu/physics.md`, "The handling update".

Thrust from rest, steering and its ramp, the airbrake ramp and sideshift match Pulse's laws on
HD's own tables (Feisar concept1, Venom, Talon's Junction). HD's load-time scales are Pulse's,
read off a live class-block dump. Nothing in this engine was changed: the three differences
below are not in any table, and their laws are not read yet.

## Open

- **Pitch settles about 30% shallower on HD** (`-2.8` deg against `-4.1` nose down at 86
  units/s; `+6.3` against `+8.3` nose up). `Craft_UpdatePitch` (`0x000f0980`) is Pulse's
  torque law. HD hovers on four hull points at `(+/-1.5, -1.125, +/-4.5)` where ours runs
  Pulse's two at `(0, -1.125, +/-4.5)`; the restoring side is unread.
- **One airbrake turns HD 9-14% less** (`-11.3` deg against `-12.6` at 0.5 s). `Craft_UpdateAirbrakes`
  (`0x000ee730`) is Pulse's force law term for term, and steering matches, so the cause is in
  what the airbrake alone exercises: lateral grip under `slidegrip`, the weathervane torque,
  or the four-point hull.
- **Speed past 2 s is about 1% low on HD** (`119.4`-`120.0` against `121.0` at 3 s). Shape fits
  Pilot Assist's `generalThrustPercentWhenEnabled="99"`, but the flag is cleared and no 0.99
  sits on the craft.
- **Not measured**: barrel roll (needs a pose before a jump on Talon's Junction or another
  circuit), weapon slowdown (Time Trial has no weapons; a Racebox single race with weapons
  on and `rpcs3-hd-weapon.py`'s slot write would do), sideshift's gesture timing.
- HD integrates the craft with each frame's own delta (`craft+0x308`), while
  `Physics_TickWorld` reads as fixed `1/60` steps; how the two fit is not read.

## Next Steps

1. Read `Craft_IntegrateHull` (`0x000ef450`) and the spring law behind its four
   `Collision_MarchSegment` calls with capstone (`data/scratch/hd-weapon-blasts/ppcdis.py`);
   compare the restoring torque per degree of pitch with ours. 2-3 hours.
2. Read HD's lateral grip and weathervane terms (the body appliers `0x000f5860`/`0x000f5dd8`
   and their other callers near `0x000ee000`-`0x000f2000`). 2 hours.
3. Only then make HD's law a Title field (ADR-0058) and re-run
   `verification/scenarios/hd-pitch-*.inputs` and `hd-airbrake-*.inputs` on both sides.
