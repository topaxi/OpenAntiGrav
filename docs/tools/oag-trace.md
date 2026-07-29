# `oag-trace`

Runs our simulation over a trace captured from the original and reports **the
first tick at which the two diverge, and by how much**. The reading half of M3's
verification harness; the writing half is
[`scripts/psp-trace.py`](../reverse-engineering/ppsspp-debugger.md), and what the
two are for is [the verification protocol](../reverse-engineering/verification-protocol.md).

```
oag-trace show    <trace.csv>                      what is in a capture
oag-trace run     <trace.csv> [--source <image>]   replay it and compare
oag-trace compare <recorded.csv> <simulated.csv>   diff two traces
oag-trace script  <script.inputs> [--expand]       validate an input script
oag-trace drive   <script.inputs> --source <image>  run a scenario, no capture
oag-trace track   --source <image> [--track <e>]   dump a track's spline as CSV
```

`drive` is `run` without the recording. `run` takes a scenario's length, its
per-tick delta and its initial condition from a capture; `drive` takes only the
script and a disc, puts the ship on the track's own start line the way a race
does, and prints [a run report](#the-run-report). So a scenario can be exercised
against our side before, or without, anybody capturing it - which is most of the
time, since a capture costs an emulator session and a script costs a text editor.

`track` is the odd one out: it simulates nothing and compares nothing. It reads
the `WO Track` node out of a track's `.vex` and writes the resampled spline -
frame, half-widths, AI corridor, authored racing line, and the lifted and
racing-line points already computed - one row per sample. It is here because
[the whole-lap scenario](#a-whole-lap-and-where-it-came-from) is steered along it
and a comparison tool is where the track already gets loaded.

Traces are **derived game data**. They live under `data/traces/`, which
`.gitignore` covers, and are never committed - so nothing here runs in CI, and
every test in the crate works from a hand-authored fixture instead.

## Two commands, if you do not want the plumbing

```sh
just scripted-sim                    # a scenario through OUR physics, with a report
just scripted-emu                    # the same scenario through the ORIGINAL, captured
```

Both default to [the whole-lap scenario](#a-whole-lap-and-where-it-came-from) and
both take one as an argument:

```sh
just scripted-sim verification/scenarios/steer-left.inputs --every 20 --out /tmp/ours.csv
just scripted-emu verification/scenarios/steer-left.inputs data/traces/left.csv
```

`scripted-sim` needs a disc image and nothing else - no emulator, no capture, no
GPU - so it is the half that always works, and it is deterministic: the same
scenario gives the same run every time.

`scripted-emu` is **autonomous from a booted emulator**. It checks it has a
debugger to talk to and prints how to start one if not; walks the front end into
a Time Trial from whatever screen the game is on; puts the craft back on the
start line; and sends the scenario with the measured `--script-lead 2`
correction. The one thing it cannot pin is *which track* - see
[the menu walk](../reverse-engineering/ppsspp-debugger.md#walking-the-menus-without-a-human)
for why, and note that it checks afterwards rather than hoping.

Override the disc with `OAG_IMAGE`, the ISO PPSSPP wants with `OAG_ISO`, and the
emulator binary with `PPSSPP_BIN`.

## The whole loop

```sh
# 1. capture, out of a race already running in PPSSPP
just trace --script verification/scenarios/steer-both-ways.inputs \
    --warmup 25 --out data/traces/venom-steer.csv

# 2. replay the same script and compare
cargo run -p oag-trace -- run data/traces/venom-steer.csv \
    --source data/images/pulse-psp-usa.chd \
    --script verification/scenarios/steer-both-ways.inputs \
    --out data/traces/ours.csv
```

`--hold cross` on both sides is the older, simpler form and still works; it is
the right thing for a straight line and cannot express anything else.

A **whole lap** is the same two commands with a longer file, and it exists now -
see [the lap scenario](#a-whole-lap-and-where-it-came-from):

```sh
cargo run -p oag-trace -- run data/traces/talons-junction-time-trial-lap.csv \
    --source data/images/pulse-psp-usa.chd \
    --script verification/scenarios/talons-junction-time-trial-lap.inputs \
    --out /tmp/ours-lap.csv
```

The capture side's traps - a breakpoint added while the CPU runs never fires, a
memory read costs 520 ms running and 0.3 ms stepping, and holding thrust through
the countdown gets you a false-start penalty that looks exactly like a broken
input path - are all on
[the debugger page](../reverse-engineering/ppsspp-debugger.md). Read it before
capturing.

## The run report

`oag-trace drive` has no recording to diverge from, so what it prints is what the
ship did:

```
verification/scenarios/steer-left.inputs: 200 tick(s) at 60 Hz, Assegai on ..., Venom class
start: (56.487, -49.648, -187.694), spawn height 3.978 above the surface line

tick        position                  speed  grounded  off-spline
    0  (     56.5,   -49.6,   -187.7)     0.0     0.00         4.0
  100  (     89.6,   -49.3,   -188.7)    16.1     1.00         3.9
  199  (     58.0,   -49.7,   -176.0)    64.1     1.00        12.3

200 tick(s)
...
grounded on 199/200 tick(s); worst distance from the spline 12.4
still in contact with the track on the last tick
```

`off-spline` is the column to read: distance to the nearest point of the track's
own spline, which is the one number that says whether the run is still on the
circuit. `--every N` sets the row spacing. The last two lines answer the question
a `grounded` *count* cannot - a ship that scrapes over every crest and one that
left the track at tick 313 and never came back have similar counts and are not
the same failure.

## What the output says

```
first divergence at tick 1 (row 1), field velocity
  recorded (0.000000, 0.000000, 22.000000), simulated (0.000000, 0.000000, 21.926260)
  error 0.073740 units/s (3.352e-3 relative), tolerance 1e-3 relative
  and on the same tick: speed by 0.073740 units/s, grounded by 1.000000

field                   max error   at tick      max rel  exceeded  trend
position                  1.507e1       199     3.406e-1         5  growing (slope 7.630e-2/tick, 3.344e-1 -> 1.177e1)
velocity                  8.733e0       199     3.970e-1         1  growing (slope 4.365e-2/tick, 1.209e0 -> 7.757e0)
...
```

Not a pass or a fail. A subsystem that tracks the original for 400 ticks and then
drifts has a different bug from one that is wrong at tick 1, so the tick is the
headline; and because an error that grows every tick is systematic even while it
is inside tolerance, every field also carries a **trend** - `exact`, `bounded`,
`growing` or `shrinking`, from a least-squares slope and a comparison of the
first quarter's mean error against the last quarter's.

Fields that go on the same tick as the leader are listed with it. One term wrong
usually shows up in one field; a wrong initial condition shows up in all of them.

## Input scripts

A capture and a replay have to be driven by the *same* input or the comparison is
between two different experiments. Until this existed there were only two ways to
arrange that, and both are limited:

- `--hold cross`, one constant input for the whole run. Fine for a straight line,
  and unable to express anything else.
- `FromTrace`, the fallback, which rebuilds each tick's input from that tick's
  recorded control states. Those are the original's *already-ramped output*, so
  feeding them back in ramps them twice, and it is not reusable: it describes one
  particular capture rather than an experiment.

An **input script** is the third mode, and the one the protocol's Components table
always called for. It is a plain-text file, committed, read identically by
`scripts/psp-trace.py --script` on the emulator side and `oag-trace run --script`
on ours. Committed scripts live in `verification/scenarios/`.

```
# verification/scenarios/steer-both-ways.inputs
60 cross
30 cross left
30 cross
30 cross right
50 cross
```

The first field on a line is how many consecutive ticks the state lasts; the rest
are tokens in any order. Run-length encoded rather than one line per tick because
what is interesting about a script is where the state *changes* - and `1 cross` is
a one-tick press, so nothing is lost.

| Token | Meaning |
| --- | --- |
| `cross`, `circle`, `square`, `triangle` | Face buttons |
| `up`, `down`, `left`, `right` | D-pad |
| `l`, `r` | Shoulders |
| `start`, `select` | For menu work |
| `stick_x=`, `stick_y=` | Analog stick, `-1..=1` |
| `airbrake_left=`, `airbrake_right=` | Airbrakes, `0..=1` |
| `none`, `-` | Nothing held; the only token on its line |
| `#` | Comment, to end of line |

An out-of-range axis is **rejected, not clamped**. A pad gets clamped because a
miscalibration is not the player's fault; a script is a specification, and one
that says `stick_x=2` is wrong rather than saturated.

**A script shorter than the recording holds its last state**, and both sides say
so on stderr when it happens, because that is nearly always an over-short script
rather than an intent.

### Why the axes have defaults rather than being required

A script names buttons because that is what the emulator can be sent: PSP hardware
has a d-pad and two shoulder buttons. Our side consumes an `InputSnapshot`, which
has axes. The translation is not invented for scripts - it is exactly what
`oag_input::Input::snapshot` does for a keyboard, which is what makes a scripted
run the same experiment as a played one: `stick_x` is `right - left`, `stick_y` is
`up - down`, and the shoulders are the airbrakes. An explicit assignment overrides
the derived value, which is how a script asks for something a pad cannot produce -
a half-deflected stick. Those go to the emulator through `input.analog.send`; a
*fractional airbrake* has no hardware equivalent at all, and `psp-trace.py`
refuses the script rather than quietly rounding it.

### Two parsers, and what stops them drifting

The capture side is Python and our side is Rust, so the format is implemented
twice. Both can dump one line per tick, in the same shape, and the dumps must be
byte-identical:

```sh
diff <(cargo run -q -p oag-trace -- script F --expand) \
     <(uv run scripts/input_script.py F --expand)
```

`uv run scripts/input_script.py --self-test` checks the Python half's invariants
without needing a Rust toolchain or an emulator, and `oag-trace`'s own tests parse
every committed scenario.

### The committed scenarios

| File | What it is |
| --- | --- |
| `straight-line.inputs` | 200 ticks of thrust. Protocol scenario 1, and deliberately the same input as the `--hold cross` reference capture, so the scripted path can be checked against a result measured before scripts existed |
| `steer-both-ways.inputs` | Thrust throughout, half a second of full left, a straight, then half a second of full right. A variant of protocol scenario 2 |
| `steer-left.inputs`, `steer-right.inputs` | One constant turn each, for the yaw ground-truth tests |
| `airbrake-asymmetric.inputs` | Thrust and steer with one airbrake, the only scenario that exercises the `1e-5` airbrake drag term |
| `pitch-both-ways.inputs` | **360 ticks of pitch, stationary on the start line, no thrust.** The cleanest capture in `data/traces/` - `speed/|velocity|` reads `1.0000` on 360 of 360 ticks - and the first with any pitch input at all |
| `pitch-hold-thrust.inputs` | Thrust and a held nose-up off the line. Clean to tick 128 at 84 units/s, and dirty after |
| `standing-start.inputs` | **300 ticks of held thrust from the start line.** Two measurements in one: a clean launch to tick 186, then a sustained scrape at speed. The force-balance regression test - see [below](#the-standing-start-and-what-it-regression-tests) |
| `talons-junction-time-trial-lap.inputs` | **3,146 ticks: one completed lap of Talon's Junction White.** Not hand-authored - see below, and note it no longer flies clean open-loop |

`steer-both-ways` turns both ways on purpose. The sign of the yaw response is one
of the things this harness exists to settle - row 0 of the recorded basis being
the ship's *left* is an 84-confidence finding, and `--basis` exists because of it -
and a run that is symmetric about a straight shows a sign error as the two halves
swapping rather than as a plausible-looking curve. A single constant turn cannot
distinguish those. The half-second segments are short enough that at the reference
scenario's ~24 units/s the ship should still be on Talon's Junction's first
straight rather than scraping a wall, which would confound steering with collision
response.

### A whole lap, and where it came from

Every scenario above this one is a few hundred ticks somebody wrote by hand.
`talons-junction-time-trial-lap.inputs` is 3,146 ticks and nobody wrote it: it
was **recorded off a closed-loop autopilot flying the original**, and the file is
the log of what that autopilot pressed.

`scripts/psp-autopilot.py` breaks in `Ship_UpdateCraft` once per tick exactly the
way `psp-trace.py` does, reads the craft, and steers it with a pure-pursuit
controller aimed at the track's own spline - which comes off the disc through the
new `oag-trace track` subcommand, so the line being chased is authored data and
not a guess:

```sh
cargo run -p oag-trace -- track --source data/images/pulse-psp-usa.chd > /tmp/spline.csv
uv run --with websocket-client scripts/psp-drive.py restart
uv run --with websocket-client scripts/psp-autopilot.py --spline /tmp/spline.csv \
    --laps 1.02 --shot-dir /tmp/shots \
    --script-out verification/scenarios/talons-junction-time-trial-lap.inputs \
    --trace-out data/traces/talons-junction-time-trial-lap.csv
```

Three things about it are worth knowing before using the file.

- **The controller is not a claim about anything.** It is bang-bang steering with
  a speed-scaled lookahead; nothing in `docs/` cites it and nothing should. What
  *is* evidence is the recording, which is byte for byte what the emulator was
  sent, and the trace, which is what the original then did.
- **The file is shifted two ticks later than it was sent**, so it reads in this
  tool's convention rather than the breakpoint's. Input set at a breakpoint lands
  three frames later and a replay's row `k` is driven by input `k-1`, so the
  emulator is two ticks behind us. `--emit-lead 2` (the default) undoes that once,
  here; `psp-trace.py --script-lead 2` puts it back when the file is replayed
  into the emulator. Replaying it at lead 0 would be a *different* run.
- **Because it is breakpoint-driven, the capture and the finished lap are the
  same run.** There is no second run to hope agrees with the first, which is the
  usual way a "capture of the run that worked" goes wrong.

**What the game itself said**, which is the point of the whole exercise and the
only thing here that is not this repository marking its own homework: at tick
3,087 the HUD read **`Lap 2 of 3`** with **`best 1.11.08`** - the game's own lap
counter had advanced and the game had timed the lap. A longer run of the same
autopilot took the 3-lap Time Trial to its end and the game started a fresh
attempt behind it (`Lap 1 of 3`, `best` cleared), which is also how a capture
tool sees a finished race: `Ship_UpdateCraft` simply stops being called.

**The lap is 95.2 % wall-free** (`speed/|velocity|` reads `1.0000` on 2,996 of
3,146 ticks) with a longest clean run of **314 consecutive ticks**. That retires
the standing complaint in [`oag-game.md`](oag-game.md) that no capture has more
than 60 consecutive wall-free ticks: the force law now has a clean, *cornering*
straight to be measured on, which no previous capture provided.

### The second lap, taken for one column

`data/traces/talons-junction-time-trial-lap-omega.csv` is the same scenario
flown again on 2026-07-28, by the same command, for one reason: the file above
predates the `omega_*` (`body+0x150`) columns and **a capture cannot be
backfilled** - it is a driven session, not a build step. The old lap is kept;
the two are independent runs of one circuit and the overlap between them is the
provenance check that made the re-capture worth doing.

| | first lap | with `omega_*` |
| --- | ---: | ---: |
| ticks | 3,146 | 3,140 |
| clean intervals | 2,996 (95.2 %) | 2,969 (94.6 %) |
| speed range | 90-164 u/s | 0-157 u/s, median 110 |
| the game's own HUD afterwards | `Lap 2 of 3` | `Lap 2 of 3` |
| `avel = -I * omega(basis)`, pitch/yaw/roll | `0.231` `0.970` `0.647` | `0.231` `0.969` `0.654` |

That last row is the point: two separate laps, flown an unknown number of hours
apart from different craft addresses, agree on a kinematic fit to the third
decimal. The new lap's own finding - the identity breaks between `body+0x150`
and the basis, and does so almost entirely in the track's **inverted section** -
is in
[cornering-ground-truth.md](../physics/cornering-ground-truth.md#the-refutation-is-scoped-the-inverted-section-was-carrying-it).

Its input script was **not** committed. The one in `verification/scenarios/` is
the first lap's, it is what every existing comparison is quoted against, and a
second bang-bang recording of the same circuit adds nothing a scenario file is
for. The trace is the artefact here.

### The lap does not replay into the emulator, and why

The obvious follow-up was run, and it failed, which is worth more than not
running it: feeding the committed file straight back into the original from a
fresh start.

```sh
uv run --with websocket-client scripts/psp-drive.py restart
just trace --script verification/scenarios/talons-junction-time-trial-lap.inputs \
    --script-lead 2 --out data/traces/talons-junction-lap-replay.csv
cargo run -p oag-trace -- compare data/traces/talons-junction-time-trial-lap.csv \
    data/traces/talons-junction-lap-replay.csv
```

The replay travels **860 units in 3,146 ticks** against the recording's 5,169. It
is not a lap; it is a ship that hit a wall at about tick 150 and ground to a halt
(speed 106 -> 77 -> 55 -> 28 -> 14 -> 2.8 over ticks 150 to 600).

The cause is in row 0, before a single input has been sent:

| Tick | Gap | Heading apart | Recorded speed | Replay speed |
| ---: | ---: | ---: | ---: | ---: |
| 0 | 0.03 | **2.51°** | 0.0 | 0.0 |
| 30 | 0.21 | 2.51° | 19.1 | 19.1 |
| 60 | 1.22 | 7.29° | 48.3 | 48.1 |
| 150 | 18.41 | 12.37° | 106.1 | 77.5 |
| 400 | 384.15 | 25.23° | 119.4 | 14.4 |

**`psp-drive.py restart` reproduces the craft's start *position* to every digit
it prints and its *heading* only to about two and a half degrees.** A craft
sitting on its hover at the start line is not at rest; it settles, and where its
nose has drifted to depends on how many frames it has been sitting. Two and a
half degrees is nothing to a player and everything to an open-loop script: it is
a metre of lateral error by the first corner and a wall shortly after.

So, precisely: **the committed file is a faithful record of the inputs of a lap
that happened; it is not a reproducible emulator lap.** For our own side it is
fully deterministic, which is what it is committed for - `oag-trace run --script`
gives the same run every time from the same seed row.

**The pose half of that is now fixed** (2026-07-28). The craft on the start line
was never settling: it yaws at a constant `0.311` deg/s and the 2.51 degrees
were eight seconds of wall-clock jitter in the handover between the restart and
the capture. `psp-trace.py --start-heading 101.0` waits for the pose instead of
for a duration and pins the heading to **`0.0001` degrees across three
restarts**, against `4.3468` for the same three with a wall-clock handover;
method, numbers and what it does not claim are on
[the debugger page](../reverse-engineering/ppsspp-debugger.md#the-start-pose-is-pinnable-and-the-craft-was-never-settling).
The candidate that page used to name - PPSSPP's input-recording API - was tried
and is a dead end: `replay.flush` crashes the emulator on any recording that
spans a screen transition.

**The replay half is now measured, and it is a negative.** Driven open-loop from
the pinned start, the committed lap script reproduces neither the recorded lap
nor a second run of itself: two runs whose starts agree to `0.0000` degrees are
`1` unit apart by tick 123 and `100` apart by tick 495. The cause is the
original's variable timestep - `dt` is identical on `1.0 %` of ticks between two
runs, because the game integrates a measured frame duration and the emulator's
frame durations depend on host load. Numbers and consequences on
[the debugger page](../reverse-engineering/ppsspp-debugger.md#measured-a-pinned-pose-is-necessary-and-nowhere-near-sufficient).
So the sentence above stands permanently rather than pending a fix: **the
committed file is a faithful record of the inputs of a lap that happened, and it
is not a reproducible emulator lap.** Short scenarios are unaffected, and our own
side stays deterministic.

### What a real scripted run found

**The whole loop has been run end to end**, on the reference scenario - Time
Trial, Venom, Talon's Junction White, Assegai - with `steer-both-ways.inputs`
driving both the capture and the replay.

Two things had to be fixed to get there, and both were found by the run rather
than by reading:

- **PPSSPP calls the shoulders `ltrigger` and `rtrigger`.** The first capture died
  on `input.buttons.send: Unsupported 'buttons' object key 'l'`. The debugger
  page's API table below had taken the PSP's own naming on trust. `l`, `r`, `L`,
  `R`, `trigger.left`, `lt`, `rt` and `shoulder.left` are all refused; only
  `ltrigger` and `rtrigger` are accepted. Confidence **95**: probed directly
  against v1.20.4, and the accepted pair works in a real capture.
- **The input phase is three rows, so `--script-lead 2` is right.** At lead 0, the
  recorded `steer` column first moved at row 63 for a script that asked for `left`
  at tick 60, and the same +3 held on all four transitions of that capture -
  release at 90 seen at 93, right at 120 seen at 123, release at 150 seen at 153.
  Our own replay shows a change at row `T+1`, because input `T` drives the step
  from row `T` to row `T+1`, so the emulator is two ticks behind us and `--script-lead 2`
  closes it. Re-captured at lead 2, `steer` moves at row 61 for the script's tick
  60 and at 109 for its tick 108: aligned. Confidence **90**: four transitions in
  one capture, then confirmed by the corrected capture, but one binary and one
  emulator version.

The comparison that came out of the aligned capture, 200 ticks:

| Field | First tick outside tolerance | Max error | Trend |
| --- | ---: | ---: | --- |
| `speed` | 0 | 30.7 units/s | growing |
| `velocity` | 1 | 41.7 units/s | growing |
| `position` | 1 | 38.4 units | growing |
| `orientation.forward` | 1 | 3.11 rad | growing |
| `grounded` | - | 1.0, 37 of 200 ticks | growing |
| **`steer`** | 121 | **2.62e-1 (1.06e-2 relative)** | **bounded** |
| `throttle` | 2 | 100 | shrinking |
| `brake`, airbrakes | - | 0 | exact |

**`steer` is the row that only exists because scripts do.** Every previous
comparison ran with the stick centred on both sides, so `steer` read "exact" while
saying nothing. Here it is a real measurement: our steer ramp tracks the
original's to about 1 % and the error is **bounded**, not growing, over 200 ticks
that include two full-deflection transitions in each direction. That is the first
independent evidence that the recovered steering ramp is right, and it is
unaffected by the over-acceleration that dominates every other row - see
[`oag-game`'s "no speed equilibrium"](oag-game.md#what-still-does-not-match-the-speed-and-the-trace-comparison).

**The `throttle` divergence at tick 2 is a capture artefact, not a physics one.**
`--warmup-hold cross` releases the warmup's thrust just before the loop starts,
and the script's own first `cross` takes the same three frames to arrive, so the
recording has two ticks of zero throttle at rows 1 and 2 that the script never
asked for. It is `shrinking` and gone by row 3. Warming up with the same button
the script's first line holds is what makes it harmless.

### What is still not confirmed

- ~~**PPSSPP's analog sign.**~~ Settled, as a side effect of measuring the pitch
  axis: `input.analog.send`'s y **is** positive-up, matching
  `InputSnapshot::stick_y`. Probed against the control block the game reads,
  `stick_y=+1` writes the same `-100` to `*(craft+0x78) + 0x10` that `up` on the
  d-pad does. Confidence 90, one emulator version.
- **The lead on anything but this build.** `--script-lead` is a switch rather than
  a constant precisely because 2 is a measurement of one emulator version, and
  re-measuring it is one capture of `steer-both-ways` and one look at the `steer`
  column.

### One thing a capture found about the original

At lead 0 the first version of this scenario held each turn for **30** ticks, and
the capture showed `speed_cached` falling from 22.2 to 4.06 - the ship hit a wall
rather than turning. The arithmetic that said 43 degrees would fit Talon's
Junction's first straight was simply wrong, and the turns are now 12 ticks. Worth
recording because it is the sort of thing a scenario file's comment will otherwise
keep asserting: **a scenario's claim about where the ship ends up is a hypothesis
until a capture checks it.**

## Tolerances

Straight from the protocol's table, and provisional there and here. Each is
overridable (`--position-absolute`, `--orientation`, `--velocity-relative`, ...)
so a measurement can be argued about without editing code - not so a comparison
can be made to pass.

| Quantity | Default | Fields |
| --- | --- | --- |
| Position | 0.01 units absolute **or** 1e-4 relative | `position` |
| Orientation | 1e-4 rad, as the angle between the axes | `orientation.row0`, `.up`, `.forward` |
| Velocity | 1e-3 relative | `velocity`, `speed`, `speed_cached` |
| Control states | 1e-3 relative | `throttle`, `brake`, `steer`, `airbrake_l`, `airbrake_r` |
| Angular velocity | 1e-4 rad/s absolute **and** 1e-3 relative | `angular_velocity` |
| Timers | 1e-3 s absolute | `stun_timer` |
| Discrete | **exact** | `grounded` |

Control states, angular velocity and timers are not in the protocol's table,
which predates the capture format; each row above records the choice rather than
hiding it in a constant. Angular velocity is relative for the same reason linear
velocity is - it integrates into the attitude. Timers are compared absolutely
because they are floats decremented by a variable `dt`, so the protocol's
"timers, counters: exact" row, which is about integers, cannot apply; `1e-3` s is
about a sixteenth of a frame, so a timer running a whole frame long is a
divergence.

Two absolute floors exist and are **off by default**: `--velocity-absolute` and
`--control-absolute`. A purely relative test has no meaning when both sides are
near zero - on a standing start a simulated `1e-9` against a recorded `0` is a
relative error of 1 - so raising them is a deliberate widening of the protocol
and should be said out loud wherever the result is quoted.

**`--angular-velocity-absolute` is the exception and defaults to `1e-4` rad/s**,
because that degenerate case is not an edge for this field but its normal state:
a straight-line capture records an angular velocity of *exactly* zero on every
tick, and a purely relative test would report tick 0 of every straight capture as
the divergence and bury the real one. At 60 Hz, `1e-4` rad/s integrates to
`1.7e-6` rad of attitude per tick - a sixtieth of the orientation tolerance - so
the floor cannot hide an orientation divergence behind an angular-velocity one.

### A field that is not in the trace is *not compared*

Captures cannot be re-taken: each one is a hand-driven PPSSPP session, so a
column added today can never be backfilled into a file recorded last week. The
columns below are therefore read when present and left **absent** when not, and a
field that either side is missing is reported as `not compared` rather than
compared against a zero:

```
angular_velocity                -         -            -         -  not compared (no column in one or both traces)
```

Absent is not zero. A zero angular velocity is a claim that the ship was not
rotating, and a comparison that passes because it never looked is the one thing
this tool must never do.

| Column | Read from | Present since |
| --- | --- | --- |
| `avel_x`, `avel_y`, `avel_z` | `body+0x160` | the angular-velocity pass |
| `omega_x`, `omega_y`, `omega_z` | `body+0x150` | the pitch-capture pass |
| `stun_timer` | `craft+0x290` | the angular-velocity pass |
| `timer_2e0` | `craft+0x2e0` | the same; captured, never compared - nothing on our side models it |

**The two angular columns are not the same quantity, and only one of them is
directly comparable.** `body+0x160` is angular **momentum** - `I * omega` in body
coordinates - so comparing it against our own angular velocity carries the
inertia tensor and cannot separate a wrong rotation from a wrong tensor.
`body+0x150` is the rotation rate by definition: `Body_Integrate` advances the
basis by it. Measured on the two pitch captures, `body+0x160 = I * body+0x150`
holds at `100.0 %` explained on the pitch axis with the code-literal tensor, and
`body+0x150` is the negated body-local rate at `-1.0011 / -0.9999 / -0.9993` on
the three axes. So a run compares `angular_rate` in rad/s against rad/s, and
`angular_velocity` in the recording's own units against a simulated `I * omega`
built to match. See
[angular-velocity-column.md](../physics/angular-velocity-column.md).

## The angular velocity, and the four readings of it

`body+0x160` rests on two legs, one per binary: PSP `Body_ClearVelocity`
(`0x0884da5c`) zeroes `body+0x140` and `body+0x160` and nothing else, and
`+0x140` is the linear velocity the capture already verifies against the position
delta; PS2 `Ship_ApplyAngularDamping` (`0x0015c1b0`) reads `body+0x160` as the
angular velocity it damps.

**Its sign and its frame are both open**, and the raw value is what is recorded:

- *Sign.* [`craft-update.md`](../ghidra/functions/ps2-pulse/craft-update.md)
  derives `w_game = -w_physics` on three independent legs. That is a result about
  the accumulators; that the stored velocity carries the same convention is the
  obvious reading and not a measured one.
- *Frame.* The same page names the field `angularVelocityLocal` and then lists
  which frame it is expressed in as unresolved;
  [`engine.md`](../ghidra/functions/psp-pulse/engine.md) caps the local/world
  split of the angular accumulators at confidence 74.

Two binary questions, so `--angular` takes four values: `negated-local` (the
default), `local`, `negated-world`, `world`.

**`oag-trace show` settles it, without a simulation.** An orthonormal basis
recorded on two consecutive ticks determines the rotation between them - for a
rotation of angle `t` about `n`, `sum cross(row_i, row_i')` is exactly
`2 n sin(t)` - so the column can be checked against the rows sitting beside it in
the same file. `show` scores all four readings against that derivative and prints
them best first:

```
angular velocity: 200/200 tick(s), rms 1.5103 rad/s
  against the basis derivative, best first: negated-local 0.0021, local 3.0204, ...
```

One reading an order of magnitude below the other three has answered both
questions at once. A level ship separates only the sign - a rotation about the
world `+y` axis with the body's up along `+y` reads the same local or world - so
the reading is best taken from a capture with some roll or pitch in it.

## The stun timer, and why it is in the capture

`craft+0x290` above zero means the frame produced **no thrust and no lateral
grip**: `Ship_UpdateEngine`'s prologue returns having zeroed the throttle state
(`0x0884c634`, confidence 88) and `Ship_ApplyLateralGrip` returns early
(`0x08848b78`). `Ship_ApplyCollisionImpulse` (`0x0883f274`) arms it with `+= 0.5`
on a hit, so it is also the cheapest wall-contact indicator the documented craft
fields offer.

That matters for the force balance: a straight-line capture with a nonzero count
here is a *stunned* ship coasting, and its speed is not an equilibrium of the
engine force law at all. `show` reports the count:

```
stun timer: armed on 34/200 tick(s) - the original produced no thrust and no lateral grip there
```

`craft+0x2e0` is the second gate on the same early return and is captured
alongside it, because the arithmetic can say the return was taken and not which
arm fired. What arms it has never been read, so it keeps its offset for a name.

## The two conventions that have to be reconciled

**The recorded basis.** The capture's rows are named `right`, `up` and `fwd`, and
row 0 was measured to be the ship's **left** (confidence 84; see the
[roadmap](../overview/roadmap.md)). The recorded basis is positively oriented -
`cross(row0, row1) = row2`, 200/200 - while `oag_physics::Body`'s is not, since
its forward is `-Z`; so mapping one onto the other takes an *odd* number of sign
flips and a reading that flipped only row 0 would be a reflection rather than a
rotation. Hence two coherent readings, and `--basis` picks between them:

- `left-up-forward` (default): the measured reading.
- `right-up-back`: the column names taken at face value, which then forces row 2
  to be the tail.

Running a comparison both ways and seeing which keeps the orientation inside
`1e-4` rad settles the question, which is why it is a switch and not a constant.

**Sampling phase.** The capture breaks on `Ship_UpdateCraft`'s *entry*, so a
recorded row is the craft as the update found it. A simulated row is therefore
emitted **before** its step. Getting this backwards shifts every field by one
tick and reads as a small systematic lag in the physics.

## What a run is seeded with, and what it cannot be

A run starts from the recording's first row: position, basis, velocity and the
five control states. Four things are not in the capture and are therefore not
replayed, rather than being invented:

- **Angular velocity**, *for a capture taken before the angular columns
  existed*. Those runs start with the rotation at rest, so capture on a straight,
  not in a corner. A capture that has the columns seeds the body's rotation from
  its first row like every other initial condition, under whichever `--angular`
  reading is chosen - and nothing is filled in for the older files. **`omega_*`
  seeds it directly and `avel_*` is divided by the inertia tensor first**; a run
  prefers the former when both are there. Seeding a body's angular velocity
  straight out of the momentum column, which this did until the tensor was
  recovered, starts every run spinning between `15.6x` and `21.6x` too fast.
- **Pitch.** The craft's control block holds `steer` and no second axis, so
  `steer_y` is always zero.
- **`time_since_landing` and the leap timer.** A run starts outside the landing
  window with no leap in progress.
- **Raw stick positions.** `steer`, `brake` and both airbrakes are recorded
  *after* the original's own ramps, so replaying them as inputs ramps them twice.
  Use `--script` with the file the capture was taken with, or `--hold cross` (and
  `--steer`, `--airbrake-left`, `--airbrake-right`) for a constant input; without
  any of those the run falls back to deriving input from the recorded states,
  which is general but approximate.

Without `--source` there are no handling parameters and no track: the run is a
coast, useful for checking the harness and useless for checking the force law.
With one, the handling stats and the track's collision geometry are read off the
disc exactly as `oag-game` reads them - through
[`oag_assets::pulse::Archives`](../architecture/frontend-boot.md), which finds
the bulk archive **by name** rather than assuming the PSP's path. So a PS2
pressing works as a `--source` too: both entry names are spelled the same on
both releases, and a run off `pulse-ps2-eu.chd` loads the same 196 colliders and
the same Assegai Venom handling as one off `pulse-psp-usa.chd`.

## Reseeding, and why a long comparison needs it

A run seeded once, at tick 0, answers **"how long do we track the original?"**
That is the right question for a two-hundred-tick scenario and the wrong one for
a lap.

The original integrates the frame duration it actually measured
([ADR-0007](../architecture/adr/0007-fixed-timestep-vs-original.md)), and those
durations follow host load. Driving the *original itself* twice with the same
script and the same pinned start pose, `dt` agrees on about **1 %** of ticks and
the two runs of the original are **100 units apart by tick 495**. So over three
thousand ticks a single-seeded comparison measures the divergence of two chaotic
trajectories, and **no implementation can pass it - not even a byte-exact one.**
Chasing that number is chasing the emulator's scheduler.

`--reseed N` puts the ship back on the recording's own state every `N` ticks,
using the same seeding path tick 0 uses. The recording becomes a sequence of
independent `N`-tick comparisons, each asking the question the force law can
actually answer: *given exactly where the original was, where do we put the ship
over the next `N` ticks?* Error stops compounding, so a maximum is a statement
about the worst window rather than about how long the run happened to be.

### What a re-seed restores, and what it silently resets

A window boundary runs the *same* `initial_state` tick 0 runs, which is the point
- but that also means it inherits tick 0's limits, and
[what a run is seeded with](#what-a-run-is-seeded-with-and-what-it-cannot-be)
now applies once per window rather than once per run. Concretely, every seeded
tick rebuilds the ship from `ShipState::default()` plus the recorded columns, so
these are **reset rather than carried**, because the capture does not have them:

| Field | What the reset asserts |
| --- | --- |
| `time_since_landing` | `1.0` - outside the landing window, so `landing_rebound` does not act |
| `leap_timer` | no leap in progress |
| `sideshift_timers` | no sideshift in flight |
| `stun_timer` | not stunned - true on every capture so far, and checked |
| `mag_lock_blend`, `mag_contact` | no magstrip hold |

So a window that begins one tick after the original landed starts as though it
had not, and `--reseed 12` would assert that five times a second. Choose `N`
large enough that a window is mostly *simulation* rather than mostly
initial-condition artefact - `60` is a second, and is the value the whole-lap
readings below use.

It is deliberately not the default, and the report says so on every reseeded run
(the written CSV carries it as a header comment too, because a CSV outlives the
terminal it came from). The two numbers answer different questions and neither
substitutes for the other:

```sh
# how long we track the original
oag-trace run data/traces/lap.csv --source ... --script ...
# how wrong the physics is, per second, across the whole lap
oag-trace run data/traces/lap.csv --source ... --script ... --reseed 60
```

## Options

| Option | Meaning |
| --- | --- |
| `--source <image>` | Disc image or an `oag-unpack` extraction, for handling and collision |
| `--track`, `--team`, `--class` | Which track, team and speed class the capture was taken in |
| `--script <file>` | Drive the run from the same input script the capture was taken with. Excludes the four held options below |
| `--hold <button>` | Hold a button for the whole run, as `psp-trace.py --hold` did. Repeatable |
| `--steer`, `--airbrake-left`, `--airbrake-right` | Hold an axis for the whole run |
| `--fixed-dt` | Step at our own 60 Hz instead of the recording's own frame times |
| `--reseed <N>` | Put the ship back on the recording every `N` ticks. See [below](#reseeding-and-why-a-long-comparison-needs-it) |
| `--basis` | `left-up-forward` (default) or `right-up-back` |
| `--angular` | What the recorded angular velocity means: `negated-local` (default), `local`, `negated-world`, `world` |
| `--no-collision` | Ignore the track's geometry: a ship with nothing to hover on |
| `--out <path>` | Write the simulated trace, in the same columns |

## What the first real comparison found

Run against a real capture - the [reference
scenario](../reverse-engineering/ppsspp-debugger.md#the-reference-scenario), 200
ticks of Talon's Junction White in Venom on an Assegai - against our own
simulation seeded from the same first row and given the same held thrust.

**The first run of this comparison was against the wrong track, and its headline
finding was an artefact of that.** It reported `grounded` at `1.0` on all 200
recorded ticks and `0.0` on all but our seeded tick 0, and read that as our hover
probes finding nothing on the original's own starting position - "the ship falls
through the floor at the first step", a force-law failure. It was not. The
scenario's Talon's Junction lives in `Data\Environments\16_Track`, not the
`01_Track` this tool and `oag-game` both defaulted to, so the run was seeding a
Talon's Junction ship into a different track's collision soup. See
[`oag-game`'s note](oag-game.md#the-suspension-was-never-the-problem-the-default-track-was)
for the measurement that settled it - 200 of 200 recorded positions find geometry on
`16_Track` at a mean height of 4.002, against the 3.978 our own spring predicts.

**Read this as a lesson about the tool, not only about the track.** `compare` is
built to say *where* two runs diverge and it did that correctly; what it cannot do
is notice that its two inputs describe different worlds. A `--track` that is merely
wrong produces a clean, confident, entirely misleading physics report, and three
separate force-law explanations were built on top of this one before anybody cast
the recording's own positions at the geometry.

With `16_Track`, on the same capture and the same held thrust:

| Field | First tick outside tolerance | Max error | Trend |
| --- | ---: | ---: | --- |
| `grounded` | 30 | 1.0 (exact field), 30 of 200 ticks | growing |
| `speed` | 0 | 152 units/s | growing |
| `velocity` | 1 | 172 units/s | growing |
| `orientation.forward` | 1 | 1.72 rad | growing |
| `position` | 3 | 259 units | growing |
| `throttle`, `brake`, `steer`, airbrakes | - | 0 | exact |

`speed` leads the report and should still be read past: it is out at tick 0, which
is the *seeded* row, so it cannot be a physics result. It is the `+0x398` mismatch
described below.

**`grounded` now agrees exactly for the first 29 ticks**, and what breaks it is not
the suspension: our ship over-accelerates. The recording holds 23.6 to 25.1 units/s
at throttle 100 while ours passes 55 by tick 30, 81 by tick 100 and 170 by tick 190,
and it starts shedding probe contact as soon as it is going fast enough that the
surface curves away from under it. The control columns match exactly, so this is a
force *magnitude* question - see
[`oag-game`'s "no speed equilibrium"](oag-game.md#what-still-does-not-match-the-speed-and-the-trace-comparison)
for the arithmetic and the four candidates.

**The handedness question is settled, and not by simulation.** The capture alone
answers it: the ship travels along **+row 2** (`dot(velocity_hat, fwd) = +0.997`
on every tick), so row 2 is the nose, and `cross(row0, up) = forward` holds
200/200. `oag_physics::Body` has `right x up = -forward`, so row 0 cannot be the
right axis under that convention - it is the **left**. That is an independent
confirmation of the 84-confidence finding in the roadmap, which came from the
sign of the yaw response to a held steer rather than from the direction of
travel. Running the comparison the other way (`--basis right-up-back`) is worse
on every field - 2.80 rad of orientation error against 1.52, 512 units of
position against 423 - which is the switch doing its job.

**Neither speed column is the velocity's length.** `speed_cached` on the craft is
**the previous tick's `dot(velocity, forward)`** - the forward-projected speed -
which is what
[`engine.md`](../ghidra/functions/psp-pulse/engine.md#the-cached-speed-and-its-staleness-measured)
already had at confidence 95 and what a shorter pass here wrongly recorded as a
stale magnitude. This capture corroborates the page and tightens it: over 200
ticks the residual is a mean of 3.3e-6 and a max of 9.7e-6, which at a speed of
24 is the capture's own `%.7g` print rounding, so it is indistinguishable from
exact rather than merely inside a 0.01 tolerance. The stale magnitude is out by a
mean of 0.077.

It also pins *which* vectors are stale, which the tolerance test could not:

| Candidate | Mean residual |
| --- | ---: |
| `dot(velocity(t-1), forward(t-1))` | 3.3e-6 |
| `dot(velocity(t-1), forward(t))` | 1.3e-3 |
| `dot(velocity(t), forward(t-1))` | 1.6e-2 |

Both vectors are the previous tick's, by a factor of 400 over the mixed reading -
so the cache is written from a coherent pre-integration state, not part-way
through the update.

Confidence **90**: one binary, one capture, and a residual four orders of
magnitude below the competing reading - but a *straight* run, where the
projection and the magnitude differ by only the 0.3 % that a `+0.997` heading
cosine allows. The two readings separate properly only under slip, and nothing
real has been captured there yet; the crate's own across-the-nose test is
synthetic. That is what holds this below `engine.md`'s 95 rather than above it.

`speed` at body `+0x398` is neither: it runs 3.67 % above the length (ratio
1.0367, sd 0.0026) and 4.02 % above the forward projection (1.0402, sd 0.0028),
and those spreads overlap, so this capture cannot say which it is a scaling of,
nor separate a constant factor from a constant offset across a 3 % speed range.

A run writes the projection for `speed_cached`, which is what was measured, and
the length for `speed`, which is the closer of two readings that this capture
cannot separate. So a comparison carries a permanent ~3.5 % divergence on `speed`
from tick 0 onwards. That is the recording disagreeing with our model of it
rather than a physics error, and it is left visible rather than scaled away -
but it does mean `speed` heads the first-divergence line while saying nothing
about the force law.

## What the whole-lap comparison found

> **Superseded, and worth knowing why.** The run below was measured 2026-07-28
> against a capture that no longer exists - `data/traces/` is gitignored and
> those files were lost. More importantly it **predates `919526d`**, which gave
> `oag_physics::wall` the original's ten box sample points and the angular half
> of the contact response, and three further contact-generation commits after
> that. Its diagnosis - "points at hover-probe contact generation: eight probes
> taking only the deepest hit, and no `cross(r, impulse)` angular response" -
> describes a crate that has not existed for some time. It is kept because the
> *shape* of the reading is still the right way to read one of these, and
> because a superseded measurement left in place is cheaper to correct than a
> deleted one is to rediscover.

### 2026-07-29, on a fresh capture of the same scenario

Both sides driven by `talons-junction-time-trial-lap.inputs`, 3,146 ticks,
`16_Track`, Assegai in Venom. Nothing here was tuned to.

| Field | single-seeded | `--reseed 60` |
| --- | ---: | ---: |
| Position, max error | 148.7 at tick 435 | **10.3** at tick 299 |
| Position, mean first -> last quarter | 100.8 -> 22.9 | 1.89 -> 1.65 |
| Orientation, worst axis | 0.66 rad | **0.114** rad |
| Velocity, max | 97.8 | 54.5 |
| `grounded` | exact on 3,146 of 3,146 | exact |
| Trend | shrinking or bounded on every field; **nothing growing** | same |

The two columns are the two questions this page's
[reseeding section](#reseeding-and-why-a-long-comparison-needs-it) describes, and
the gap between them is what compounding costs: most of the single-seeded 148 is
one early divergence carried forward, not a force law that is 148 units wrong.

Three readings from the same run, in descending order of how much they are
worth:

- **The ship travels 618 units of path where the original travels 1,045**, on
  identical inputs. That is the open question, and it is the one the run report
  calls a wedge: ours stops dead at one place on the circuit for roughly a
  thousand ticks and then reverses.
- **`grounded` says less than it looks like it says.** The original's column is
  `1.0` on every tick of this capture, so agreement means our ship also never
  leaves the ground - real progress against the 2026-07-28 run below, and not a
  test of how the hover model quantises contact. A constant column cannot be one.
- **The angular-velocity hypothesis is dead.** Our run averages 21.2 rad/s over
  the lap, which reads like a craft spinning three and a half times a second and
  was the obvious suspect. The original reads **22.96**. Both sides do the same
  thing with that column.

**Caveat on the capture itself, which changes what a clean number would take:**
it fails the `speed / |velocity| == 1.0000` cleanliness test on **67.3 %** of its
ticks, where the 2026-07-28 capture of the same script was 95.2 % wall-free. The
script is a **closed-loop autopilot recording**, and replaying it open-loop
drifts into the walls - the same reproducibility negative, seen from the
authoring side rather than the measuring side. A clean lap scenario has to be
re-derived from a fresh `just autopilot` run, not replayed from the committed
file. It also means this capture **cannot** serve as a force-balance regression:
for much of it the craft is wedged at under one unit per second, where the ratio
is dominated by the normal impulse and `|velocity|` can exceed `speed` outright.
[The standing start](#the-standing-start-and-what-it-regression-tests) is the
capture for that.

### 2026-07-28, for comparison

Both sides driven by `talons-junction-time-trial-lap.inputs`, 3,146 ticks, the
recording seeding tick 0. Measured 2026-07-28; nothing here was tuned to.

| Ticks | What ours does |
| --- | --- |
| 0-75 | **Tracks the original.** Gap under 1 unit at tick 75; speeds 60.2 against 65.4 |
| 75-123 | Gap opens to 20 units - one narrow half-width. Ours reads `grounded = 0.5` where the original holds `1.0`, and its speed falls behind (74 against 83 at tick 100) |
| 123-313 | Gap grows to ~100 units. Ours is airborne for part of every crest |
| 313 | **Leaves the surface for good** at `y = -44.7`, and never regains it |
| 313-3145 | Free fall. `y` reaches `-10,794` against the original's `-48.9`; speed saturates at 234 units/s, which is nothing but terminal velocity |

So the divergence is not the force law running out of accuracy: for the first
75 ticks the two agree to a unit, and what fails afterwards is **staying on the
track**. `grounded` dropping to `0.5` and then to `0` while the original never
leaves `1.0` dates it precisely, and points at hover-probe contact generation -
the gap [HANDOVER](../../HANDOVER.md) already names: eight probes taking only the
deepest hit, and no `cross(r, impulse)` angular response. This run is the first
measurement that says *when* that gap bites, rather than that it exists.

Two smaller readings from the same run:

- The recorded lap's angular column scores `negated-local` best (31.98 rms
  against the basis derivative, ahead of `negated-world`'s 32.24), the same order
  every earlier capture gives, now over a whole lap with real cornering rather
  than over a straight.
- The original's stun timer never armed across the entire lap, and `craft+0x2e0`
  never rose above zero - on a lap that is 95.2 % wall-free, which is what makes
  that a statement about the lap rather than about the gate.

## The standing start, and what it regression-tests

`verification/scenarios/standing-start.inputs` is 300 ticks of held thrust from
the start line with no steering, and it is deliberately two measurements joined
in one capture: a clean launch, then a sustained scrape **at speed**. Captured
2026-07-29 to replace a lost predecessor.

| | |
| --- | --- |
| Clean ticks (`speed / \|velocity\| == 1.0000`) | **186** of 300 |
| First contact tick | 186 |
| Speed range | 0.015 to 119.39 units/s |
| Launch acceleration, ticks 2-5 | 32.69, 32.50, 32.97, 33.61 |
| Per-frame loss over the 114 contact ticks | 3.835 % falling to 3.633 % |
| Minimum loss anywhere in the contact stretch | **3.534 %**, at 104 units/s |
| Contact ticks above 5 units/s below the `0.035` floor | **0** |

Both halves reproduce a documented number from an independent capture:

- **The launch.** `force-balance-ground-truth.md` records 32.52 against a
  predicted 31.8 for the same measurement; this capture reads 32.69 on its third
  tick. The engine force law is confirmed, not re-fitted.
- **The scrape.** `contact-response.md` derives `contact->friction = (0.05 +
  0.02) / 2 = 0.035` from two literals and argues it is a hard *floor* on the
  per-frame loss, approached from above and never crossed. **114 fresh contact
  ticks, minimum 3.534 %, floor never crossed**, decaying toward it exactly as
  that page predicts. Note the contact starts at tick 186 here rather than the
  predecessor's 66, so this is not the same recording read twice.

**Why the speed condition is not optional.** Below a few units per second the
ratio stops measuring friction: the normal impulse dominates, and restitution can
make `|velocity|` exceed `speed` outright - the whole-lap capture has ticks
reading a *negative* loss for that reason. Any future use of this test must
restrict itself to contact ticks at speed, which is what the last row above does.

## Not yet

- **No save state.** The starting point is still the documented menu walk from a
  cold boot: reproducible, and slow enough that redoing it is the main cost of
  taking a capture. Input scripts now exist (above), so a scenario is a named,
  committed thing rather than "whatever was captured"; the save state is the other
  half of the protocol's fixed starting point and is not built.
- ~~**No cornering capture.**~~ Retired by the whole-lap capture above, which is
  3,146 ticks of a real circuit at 90-164 units/s. The speed columns can now be
  separated at real slip angles, which the reference straight could not do; that
  analysis has **not been run**, and is the obvious next use of the file.
- **The lap does not replay open-loop into the emulator, and that is measured
  rather than assumed.** See
  [below](#the-lap-does-not-replay-into-the-emulator-and-why).
- Nothing compares a PSP capture against a PS2 one.
- Shield, weapons, lap and race-position state are in the protocol's list of what
  gets traced and in neither the capture nor this tool: nothing downstream of the
  craft has been decoded yet.
