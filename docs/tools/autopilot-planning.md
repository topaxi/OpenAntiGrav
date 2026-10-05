# Planning an input script instead of guessing one

Driving the original to a chosen spot on a track - over a speed pad, into a wall
at a chosen angle, through a corner on a chosen line - needs an
[input script](oag-trace.md#input-scripts). Hand-authoring one costs an emulator
round trip per guess: load the save state, replay, read where the craft ended up,
move a hold by five ticks, repeat. Ten guesses is an afternoon.

This page describes two ways not to do that, and when each is the right one.

| Route | Where the loop closes | Cost per attempt | Command |
| --- | --- | --- | --- |
| **Plan offline, replay once** | our simulation | ~0.7 s, no emulator | `oag-trace plan` |
| **Drive live, record what worked** | the emulator itself | one emulator session | `scripts/psp-autopilot.py --gate` |

They are not rivals. The offline planner is cheap enough to sweep a parameter
grid over, and the live driver is the only one that is actually steering the
thing being aimed. The honest workflow is to plan offline and then either replay
the plan or, if the replay misses, drive the same gate live and keep *that*
recording - which is a committed script in the same format either way.

## Why an offline plan is worth anything

Our simulation is not the original, and a plan made in it is a hypothesis about
the original, not a result. What makes it a useful hypothesis is that the two
track each other closely early and drift later - see
[`oag-trace`'s divergence report](oag-trace.md) and [`HANDOVER.md`](../../HANDOVER.md)
for what "closely" currently measures. So:

- **A short plan is likelier to transfer than a long one.** The pad-0 plan below
  is 221 ticks, under four seconds.
- **A plan whose target is wide is likelier to transfer than a tight one.**
  Speed pad 0 on Talon's Junction is about 9.5 units across, so a plan that
  crosses on the centre line has ~4.8 units of margin either side.
- **Nothing here is verified against the original.** See
  [What remains untested](#what-remains-untested).

## The offline planner: `oag-trace plan`

```sh
cargo run -q -p oag-trace -- plan \
    --source data/images/pulse-psp-usa.chd \
    --pad 0 --start 6.07,-50.07,-196.10 \
    --look-min 26 --look-speed 0.55 --deadband 0.045 \
    --out verification/scenarios/talons-junction-pad-0.inputs
```

It is a controller, not a search. A pure-pursuit steerer with a bang-bang d-pad -
the same shape as the one [`scripts/psp-autopilot.py`](#the-live-driver)
flies the original with - is run against our own physics through
`oag_trace::replay::drive_with`, and **what it pressed, tick by tick, is the
output**. There is nothing to optimise over because the controller closes the
loop itself; a run that reaches the gate has already produced the script.

`drive_with` is `oag-trace drive`'s own stepping loop with the script replaced by
a closure, and `drive` is now implemented in terms of it, so a planned run and a
scripted run are stepped by one piece of code rather than two that have to be
kept agreeing.

### What it aims at

`--pad N` takes everything off the disc: the pad's box centre, its push axis
(row 2 of its transform - see [`oag_vex::pads`](../formats/track.md)) and
half its box width. The pad's own `Pad_ContainsPoint` box is tested as well as
the plane, so the report says whether the pad would actually have fired rather
than only whether the craft passed the right plane. `--gate x,y,z` aims at an
arbitrary point instead, crossing along the spline's tangent there unless
`--gate-dir` says otherwise.

### The line it chases

The spine is the track's **authored racing line** - the same
`lift + racing_line * lateral` `oag-trace track` writes as its `line_*` columns -
so the planner chases a line that came off the disc. Onto that is added a ramp:
the gate's own offset from the racing line, faded in linearly from nothing at the
start to all of it at the gate. A gate sitting on the line leaves the racing line
untouched; pad 0 sits 8 units off it and is converged onto over the whole
approach rather than swerved at in the last few metres. Past the gate the line
runs on 60 units along the gate's own direction, so pure pursuit still has
something to aim at while it is crossing.

### The start pose, which is the trap

**`oag-trace drive`'s default start is not where a time trial begins.** It uses
the track's authored `Start Position` node, the same one `oag_raceplay` uses,
and on Talon's Junction (`16_Track`) that slot sits about 138 units behind the
emulator's actual start line - `(-131.8, -50.9, -173.6)` against `(6.07, -50.07,
-196.10)`. Already documented on
[`Pose::from_start_position`](../formats/track.md); it is repeated here because a
plan made from the wrong pose is a plan for a different run and looks perfectly
healthy until it is replayed.

So `--start x,y,z` places the craft anywhere, taking its attitude from the
nearest spline sample (`--start-yaw` turns it off that tangent, degrees, positive
to the craft's right). Read the position out of the emulator - `psp-trace.py`
writes `pos_x/y/z` every tick - and pass it. `oag-trace drive` takes the same two
flags, so a planned script can be replayed on our side from the pose it was
planned from.

At `(6.07, -50.07, -196.10)` the nearest spline sample has tangent
`(1.000, 0.003, -0.003)` and lateral `(0.003, 0.019, 1.000)`. That is the
"facing `+x` with a slight `-z` veer" the standing start reads as, and it also
settles the reported input quirk: the driver's **right** at that pose *is* `+z`,
so `right` steering toward `+z` is the axis behaving, not an emulator oddity.

### The two-tick lead is not cosmetic

`scripts/psp-trace.py --script-lead 2` sends the script's tick `k + 2` at the
breakpoint for tick `k`, which cancels the emulator's own input latency in steady
state but means **the first two states never reach the emulator at all** - see
[`Script::with_capture_lead`](oag-trace.md#input-scripts). `plan --lead 2` (the
default) therefore plans those two ticks as released, so the run validated here
is the run replayed there. Match the two numbers or they are different runs.

### Sweeping, which is the actual payoff

One plan costs ~0.7 s and no emulator. Forty-eight of them cost half a minute:

```sh
for lm in 12 18 26 34; do for ls in 0.30 0.45 0.55 0.70; do for db in 0.02 0.045 0.08; do
  cargo run -q -p oag-trace -- plan --source data/images/pulse-psp-usa.chd --pad 0 \
      --start 6.07,-50.07,-196.10 --look-min $lm --look-speed $ls --deadband $db \
      --out /tmp/sweep.inputs 2>/dev/null | grep -E 'crossed|never'
done; done; done
```

Across that grid the crossing point moved from **12.3 units right** of the pad's
centre to **5.8 units left** of it, and 21 of the 48 never entered the pad's own
trigger box at all.
The best cell - `--look-min 26 --look-speed 0.55 --deadband 0.045` - crosses
**0.09 units right of centre**. Nothing about that is a measurement of the
original; it is a measurement of how sensitive a bang-bang controller is, and the
reason to read it is the next section.

## The measured result, in our simulation

Target: Talon's Junction speed pad 0, centre `(168.24, -48.53, -179.12)`, push
axis `(0.9995, 0.0238, -0.0194)`, half width 4.8. Start: the emulator's Time
Trial line, `(6.07, -50.07, -196.10)`. Assegai, Venom, 60 Hz fixed step.

```
crossed the gate on tick 161 at (168.2, -48.0, -179.0), 0.09 units right of
centre - inside its half width - doing 110.8
inside the pad's own trigger box from tick 159
221 tick(s) of input planned
```

The script is
[`verification/scenarios/talons-junction-pad-0.inputs`](../../verification/scenarios/talons-junction-pad-0.inputs)
and it is twelve lines. Replaying it through the ordinary scenario runner
reproduces the planned run frame for frame, which is the check that the committed
artefact and the planner agree:

```sh
cargo run -q -p oag-trace -- drive verification/scenarios/talons-junction-pad-0.inputs \
    --source data/images/pulse-psp-usa.chd --start 6.07,-50.07,-196.10 --every 20
```

Both parsers read the file identically, as ever:

```sh
diff <(cargo run -q -p oag-trace -- script verification/scenarios/talons-junction-pad-0.inputs --expand) \
     <(uv run scripts/input_script.py verification/scenarios/talons-junction-pad-0.inputs --expand)
```

### How to replay it into the emulator

```sh
uv run --with websocket-client scripts/psp-trace.py \
    --script verification/scenarios/talons-junction-pad-0.inputs --script-lead 2 \
    --out data/traces/talons-junction-pad-0.csv
```

with the emulator sitting on a Time Trial start line on Talon's Junction, in an
Assegai, Venom class. Then compare where it actually went:

```sh
cargo run -q -p oag-trace -- compare data/traces/talons-junction-pad-0.csv /tmp/planned.csv
```

(`plan --trace-out /tmp/planned.csv` writes the planned run in a capture's own
columns for exactly this.)

## The live driver

The other route closes the loop where the answer actually lives. `psp-autopilot.py`
already breaks in `Ship_UpdateCraft` every tick, reads the craft's real position
and basis out of the emulator, steers toward the track's own spline and
**records what it pressed** - so it never needs a plan to transfer, because it is
correcting against the thing it is aiming. What it could not do was aim at a
*point*; it drove laps. Now it can:

```sh
cargo run -q -p oag-trace -- track --source data/images/pulse-psp-usa.chd > /tmp/spline.csv
cargo run -q -p oag-trace -- pads  --source data/images/pulse-psp-usa.chd    # pad centres and axes

uv run --with websocket-client scripts/psp-autopilot.py --spline /tmp/spline.csv \
    --gate 168.2369,-48.5302,-179.1155 --gate-dir 0.99953,0.02378,-0.01945 \
    --script-out verification/scenarios/talons-junction-pad-0-live.inputs \
    --trace-out data/traces/talons-junction-pad-0-live.csv
```

`--gate` bends the spline ring onto the gate with the same linear ramp the
offline planner uses (`Line.converge_to`, mirroring
`oag_trace::plan::Path::to_gate`), and the run stops `--gate-after` ticks past the
crossing. Because it is breakpoint-driven it writes the trace and the script from
the same run, which is the only way to have both without hoping two runs agree.

**Prefer this route when the plan has to be right rather than cheap**: a target
that is narrow, far from the start, or past a corner where our physics has
already drifted. Prefer the offline planner when the target is wide and near, or
when the question is "which of these forty controller settings even works" -
which is not a question worth an emulator session.

Nothing in either controller is a claim about the original's AI. Both were
written to get a craft to a point, and no page in `docs/` should cite either as
evidence of anything. What *is* evidence is the recording: the per-tick button
state the live driver produces is exactly what the emulator was sent.

## What remains untested

Everything on the emulator side. Specifically:

1. **The pad-0 script has never been replayed into PPSSPP.** Whether it crosses
   pad 0 in the original - or crosses it at all - is unknown. The in-sim result
   above is a statement about our physics only.
2. **`psp-autopilot.py --gate` has not been run.** The ramp, the crossing test
   and the stop condition are covered only by their offline twin's unit tests in
   `crates/trace/src/plan.rs`; the Python side has been parsed and its `--help`
   exercised, nothing more.
3. **The start pose `(6.07, -50.07, -196.10)` is second-hand.** It was reported
   from an emulator session, not re-read here, and the craft's real starting
   *attitude* is taken from the spline tangent rather than from the emulator's
   own basis. If a replay misses the pad early rather than late, this is the
   first thing to re-measure - `psp-trace.py` writes the basis columns.
4. **The controller tuning is fitted to our physics.** The sweep above shows a
   12-unit spread in crossing position across a modest parameter grid, so the
   0.09-unit result is a best cell in *our* simulation and should not be read as
   precision that survives the transfer.
5. **Only one gate, one track, one team and one speed class have been planned.**
   Nothing says the ramp behaves on a banked corner, and `lateral_miss` uses
   world up rather than the track's, which would be wrong on a pad mounted on a
   wall. No shipped PSP track has one, and the value is a report rather than
   simulation state, but it is an assumption rather than a measurement.

## See also

- [`oag-trace`](oag-trace.md) - the input-script format, `drive`, `track`, `pads`
- [PPSSPP debugger](../reverse-engineering/ppsspp-debugger.md) - how the capture
  side breaks in `Ship_UpdateCraft`, and `--script-lead`
- [Verification protocol](../reverse-engineering/verification-protocol.md) - why
  an authored script is the third input mode and the only reusable one
