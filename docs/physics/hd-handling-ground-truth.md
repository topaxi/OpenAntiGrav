# HD/Fury handling against the original (RPCS3), 2026-10-08

**What this page settles:** whether the handling this engine flies on Wipeout HD/Fury's
own data (Pulse's physics, fed HD's handling tables) moves the craft the way the
original does. Measured on RPCS3 with [`scripts/rpcs3-trace.py`](../../scripts/rpcs3-trace.py)
against `oag-game --race --trace-out` from the same scripts, the same pose, the same
team and class. The method and its traps are on
[rpcs3-capture.md](../reverse-engineering/rpcs3-capture.md#a-per-frame-craft-trace-2026-10-08-hd-handling);
the HD functions read here are on
[ps3-hdfury-eu/physics.md](../ghidra/functions/ps3-hdfury-eu/physics.md#the-handling-update-read-against-a-live-trace-2026-10-08-hd-handling).

| Item | Verdict | Numbers (HD against ours) | Confidence |
| --- | --- | --- | ---: |
| Thrust from rest | **matches** Pulse's law on HD's table | 50 runs, 3 boots: within 0.7% through 1.4 s (`59.41`-`59.86` against `59.77` units/s at 1.0 s) | 90 |
| Speed past 2 s | **differs by about 1%**, cause not found | `103.30`-`104.06` against `104.66` at 2.0 s, `119.36`-`119.96` against `120.95` at 3.0 s | 75 that the gap is real |
| Steering (full lock) | **matches** | heading at 0.8 s `-46.20`..`-46.25` deg (4 runs, 2 boots) against `-47.37`; late turn rate within 0.5% | 88 |
| Steering ramp | **matches**: HD's `craft+0x314` climbs `8.3` a frame to 100, the `<Turning gain="500">` per second ours uses | read live, every steer run | 90 |
| Airbrakes (one side) | **differs**: HD turns 9-14% less, same law read in the binary | right: `-11.22`..`-11.36` deg at 0.5 s against `-12.58`; `-32.73`..`-33.07` at 1.0 s against `-36.34` | 85 that the gap is real; cause open |
| Airbrake ramp | **matches**: `+13.3` a frame, `<Airbrake gain="800">` | read live | 90 |
| Pitch | **matches since 2026-10-09** (HD's four-point hover, below) | at 86 units/s, nose-down hold settles at `-2.75`..`-2.79` deg, ours `-2.99` (was `-4.00`); nose-up `+6.28`..`+6.42`, ours `+6.25` (was `+8.24`), at 1.45 s after onset (4 runs, 2 boots) | 88 |
| Sideshift | **present, same gesture, same size** | double-tapped left airbrake: `-8.00`/`-7.91` units sideways at 0.4 s against `-7.89`; right `+7.88`/`+8.05` against `+9.65`, converging by 0.8 s; HD's shift starts about one frame later | 75 |
| Stall rescue | **consistent, not discriminating** | nose-on into either side wall, thrust held 8.7 s: HD never resets the craft; speed bounces between 0.2 and 60, so this engine's under-1-for-2-s rule would not fire either | 60 |
| Barrel roll, weapon slowdown | **not measured** | needs a pose before a jump; Time Trial carries no weapons | - |

## Falsification criteria, written before the captures

- **Thrust**: Pulse's law fails on HD if the speed at matching game time differs by more
  than 5% anywhere in the first two seconds from rest. It does not (0.7%).
- **Steering**: fails if the heading after 0.8 s of full lock, or the late turn rate,
  differs by more than 10%. It does not (2.4% and 0.5%).
- **Airbrakes and pitch**: the same 10% threshold. The airbrake sits at the edge (9-14%)
  on four runs that agree with each other to 0.4 deg, so it is reported as a difference.
  Pitch is clearly over it.

## What was fixed in the capture before anything could be compared

Four things made the first captures disagree with ours for reasons that were not
handling. Each is now handled in the tool, and each would have produced a wrong
finding:

1. **Pilot Assist was on in the save** (`options+0x473`). HD authors a per-class
   `<PilotAssist>` steering spring and a `<PilotAssistPenalty thrustPercentOnUse="92">`
   in `/data/xml/handlingstats.xml`; the first probe's throttle read exactly 92, and
   the craft followed the first curve at 120 units/s with no steering input while ours
   hit the wall. The tool clears the flag at the Main Menu.
2. **The team.** The default campaign walk races `feisar_c1` (concept1): `accelcap` 21
   against `feisar`'s 18.5, which made HD look 14% quicker off the line until ours ran
   the same team (`21 / 18.5 = 1.135`).
3. **The step is variable.** The craft's game clock `craft+0x308` advances by each frame's
   own delta, 0.6 to 1.5+ sixtieths on a normal boot and two sixtieths at a time when
   RPCS3 drops to 30 fps. Distance travelled over speed times that delta is `0.996`-`0.999`
   across twenty runs, so the craft integrates with it. A frame-keyed script held a state
   for a different game time on every run, which made one boot's steering look 20-29%
   quicker; keying the script on the game clock collapsed every repeat to within 0.4 deg.
4. **The airbrakes are L2/R2, and analog.** RPCS3's stock evdev profile reads the
   triggers from `ABS_Z`/`ABS_RZ`, so a button press alone never arrived; L1/R1 move
   nothing in HD.

Two smaller alignments: ours logs the *ramped* steering and airbrake state, so HD is
aligned on its own ramps (`craft+0x314`, `+0x318`/`+0x31c`), which start one frame after
`PlayerInput` changes; aligning on the raw input instead manufactures a one-frame lag.

## The tables: this engine reads HD's own values, with HD's own scales

The class block the original holds at `*(craft+0x7c)` (dumped live, Feisar concept1
Venom) carries every `<Class>` value at the load-time scale this engine applies:
`Engine amount 418` as `0.418` (x0.001), `Airbrake amount 8` as `0.0008` and
`slidegrip 60` as `0.006` (x0.0001), `Brakes amount 100` as `-1` (x-0.01), `<Pitch>`
unscaled (`0.2`, `1.0`, `5`, `1.0`). So the scales in `oag_gameplay::handling` hold on HD,
and the per-team, per-class values this engine loads are HD's own: the thrust match
above only appeared once ours flew the same team's `accelcap`. Confidence 90 (one dump,
every value accounted for).

## The differences, and what they are not

- **The airbrake gap is not the force law.** HD's `Craft_UpdateAirbrakes` (`0x000ee730`)
  computes slide `|L-R| * drag * |steer| * 0.01`, the lateral `amount` term and yaw
  `speed * turn * (R-L) * 0.001` from the 800-per-second ramps, with `speed` the previous
  frame's forward speed (`craft+0x340`, equal to the previous row's forward speed to the
  printed precision): Pulse's law term for term. It is not the steering response either,
  which matches. 2026-10-09 ruled out four more: HD's lateral grip (`0x000eff78`), steering
  (`0x000edb50`, the same torque accumulator), angular damping (`0x000eda30`) and the four-point
  hull (flying it moves the airbrake under 2%) are all Pulse's law. HD's yaw rate under one
  airbrake runs 9-13% under ours from the second frame at matching speed, a drive-scale shape,
  not a slip-driven one. Still unread: HD's weathervane, and the inertia frame. See
  [hover-four-point.md](../ghidra/functions/ps3-hdfury-eu/hover-four-point.md#the-terms-the-one-airbrake-gap-is-not-in).
- **The pitch gap was the hover rig, now flown (2026-10-09, hd-pitch-airbrake).** HD hangs on
  four probes at `0.15` each and casts every one every frame; Pulse's two-point law derives the
  rear hit above 50 units/s with a slope gain of `6` where the probes are `9` apart, which leaves
  two thirds of the geometric pitch stiffness. Read on
  [hover-four-point.md](../ghidra/functions/ps3-hdfury-eu/hover-four-point.md); HD's
  `RaceDefaults::hover_rig` now carries it. Per scenario at 1.45 s after onset (HD four runs /
  ours before / ours after): nose up `+6.28`..`+6.42` / `+8.24` / `+6.25`; nose down
  `-2.75`..`-2.79` / `-4.00` / `-2.99`. Side effects on the other scenarios, all inside their
  10% criteria: steering heading at 0.8 s `-48.00` (was `-47.37`, HD `-46.2`); speed at 3 s
  `120.72` (was `120.95`, HD `119.4`-`120.0`); one airbrake unchanged within 2%.
- **The pitch torque law was never the gap.** HD's `Craft_UpdatePitch` (`0x000f0980`) applies
  `pitch * pitch_ground` grounded and `pitch * pitch_air + weight_distribution` airborne,
  the same as Pulse, behind a gate (`dot(up, -ship+0x7850) > 0.9`) that stayed open in
  every run. HD hovers on **four** hull points at body-local `(+/-1.5, -1.125, +/-4.5)`,
  each marched down to `-5.25` (`Craft_IntegrateHull`, `0x000ef450`), where this engine
  runs Pulse's two probes at `(0, -1.125, +/-4.5)`. Same fore-aft arm and drop, twice the
  springs: the restoring side of the pitch balance is the first place to read.
- **The 1% late-speed gap** is unexplained. Pilot Assist's `generalThrustPercentWhenEnabled
  ="99"` would produce exactly this shape (no effect while `accelcap` limits thrust,
  1% once `amount` does), but no 0.99 or 99 sits on the craft, and the throttle reads 100.

None of these is a table the original authors differently. The pitch law was read and is
flown (above); the airbrake and late-speed gaps are not, so nothing else was changed.

## Captures

All under `data/scratch/hd-handling/` (gitignored): `b4` (20 runs, frame-keyed),
`b5`, `b6` (game-time keyed, two reps of each scenario), `b7` (walls), with `ours/` the
matching `oag-game` traces and `handling-blocks.pkl` the class-block dump. Scenarios:
`verification/scenarios/hd-*.inputs`. Talon's Junction, Racebox Time Trial, Venom,
Feisar concept1 (`--team feisar_c1 --variant concept1`), teleported to grid slot 0
`(6.10, -51.91, -195.92)` facing +x after a 60-frame settle.

**Omega**: not checkable live. No PS4 emulator is in this project's toolchain, so there is no
live craft to trace. Statically, the four-probe march is HD's (checked, applies, not wired; see
[hover-four-point.md](../ghidra/functions/ps3-hdfury-eu/hover-four-point.md#omega-and-2048)).
