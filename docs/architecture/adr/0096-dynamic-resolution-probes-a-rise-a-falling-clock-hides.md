# ADR-0096: Dynamic resolution probes a rise a falling clock hides

## Status

**Accepted (2026-11-26)** by the maintainer, once the time trial's extra stutter was shown not to be the probe (`dynamic-resolution.md`, "On the Steam Deck: what holds the clock, and the time trial's late seconds"). Extends the budget loop of
[ADR-0042](0042-the-dynamic-resolution-budget-subtracts-what-it-can-measure.md) with a second
way to rise; ADR-0042's budget, ADR-0044's residual, ADR-0073's floor and ADR-0075's blur rung
stand, and so do the loop's two rise guards (`RISE_PATIENCE` and the overshoot guard).

## Context

On a Steam Deck at 3840x2160, HD's Zone race on Chenghou Project settles at 0.80 under dynamic
resolution, the time trial at 0.93. With the GPU clock pinned at its top, the same races settle
at 0.96 and 0.99 (`docs/rendering/dynamic-resolution.md`, "Why the 4K settle is so deep").

The cause is the governor, not the budget. The controller lowers the scale, the load falls, the
Deck lowers its clock, and the GPU's timestamps (milliseconds, not cycles) grow back: unpinned,
the race pass took 1.29 to 1.38 times its full-size cost times the scale squared in Zone, pinned
0.99 to 1.00. A governor that holds the GPU about as busy as it was makes every scale across its
clock range read about the same milliseconds, inside `DEADBAND` (0.80 to 0.95 of the budget). A
rise needs `RISE_PATIENCE` (8) readings under 0.80 and a prediction that the step lands under
0.80 too, so the loop never rises: it is stuck wherever the race's opening pushed it.

Three constraints shape the answer:

- **The controller reads no clock** (`drs.rs`, "The controller"), and a GPU clock source is Linux
  and driver specific. A rule that read one would be a Deck rule; the project defaults the same on
  every OS (maintainer, 2026-11-25).
- **GPU milliseconds below the deadband's top say nothing about room** on such a GPU, so the
  evidence has to come from trying.
- **A GPU that holds its clock must not change**: its loop's guards stopped a size-change loop
  (143 changes a minute before `RISE_PATIENCE`, 176 in 41 s before the overshoot guard).

## Decision

1. **A probe rise.** When the scale has sat below the ceiling for `HOLD_SECONDS` (4 s) of the
   target's frames, every one of them quiet (a reading at or under the deadband's top, a
   wall-clock frame time that was not late, no step and no rung action), the controller steps up
   one grid point on trial. The normal rise and both of its guards are unchanged; the probe is a
   second path, taken only when the normal loop decided nothing.
2. **The trial outlasts the governor's ramp.** For `RAMP_SECONDS` (1.5 s, the Deck's measured
   ramp) after the step nothing but a
   late frame moves the scale: the readings are taken at the old clock and a fall would undo the
   step at once. Then the trial samples the scalable cost for `SAMPLE_SECONDS`.
3. **Kept only if the clock paid for it.** At a steady clock the cost after a step from `here` to
   `next` is the pre-step cost (the mean of the hold's last `SAMPLE_SECONDS`) times
   `(next / here)^2`, or less where part of it does not scale with pixels. The step is kept only
   if the sampled mean is at or under `KEEP_SHARE` (0.92) of that prediction **and** at or under
   the deadband's top. A governor that raises its clock to carry the step reads about
   `1 / (next / here)^2` of the prediction (0.886 to 0.905 over the top quarter of the grid); a
   steady clock reads 1.0 on a cost that is all per pixel.
4. **A late frame ends a trial at once**, except inside the step's own `COOLDOWN`, where the
   resize itself costs one (29 to 36 ms between frames straight after a step, pinned). Late is
   a wall-clock frame longer than `LATE_PERIODS` (1.25) target periods. The loop's deadline grid
   makes a frame that long one that pushed the next off its slot.
5. **A reverted probe backs off.** It doubles the next hold, up to `MAX_BACKOFF` (5) doublings
   (4 s becomes 128 s); a kept probe clears them. A stall (a load, a rebuilt surface) ends a trial
   where it stands without a verdict and keeps the backoff.
6. **The same rule everywhere.** No clock read, no platform test, no setting. A frame with no
   wall-clock reading breaks the hold, so a caller that feeds none gets the loop as it was.

## Measured

### The governor's ramp

On the Deck at 3840x2160, over 71 steps up of the scale in six unpinned Zone runs: in 38 the clock
did not rise at all (mostly parked at 1040 MHz with the GPU 86 to 100 % busy); where it rose, 90 %
of the rise took 0.01 to 1.6 s (median 0.67 s). `RAMP_SECONDS` is 1.5 s; at 0.5 s most trials were
judged half way up the ramp.

### The Deck, before and after

Unpinned, 3840x2160, 60 Hz, three interleaved runs each (`docs/rendering/dynamic-resolution.md`,
"The probe rise"): Zone settles at 0.90 (0.895 to 0.906) against 0.84 (0.831 to 0.863), with no
more size changes a minute (8 to 17 against 15 to 19) and no second whose p99 passed 20.8 ms in
either. The GPU clock's median rises from about 1040 to about 1400 MHz, and its power from about
8 to about 11 W. The time trial's scale does not move (0.98 both); it had more seconds with a p99 over 20.8 ms after (15 to 22) than before (9 to 14), unresolved: the worst run fired no probe. 1920x1080 and 1280x800 stay at 1.00
with no probe. With the clock pinned at 1600 MHz (a steady clock), Zone settles at 0.96 with and
without the probe; every probe there reverted.

The claim this was built to, a settle **near 0.96** unpinned, is not met: 0.90. Kept probes read
0.82 to 0.92 of the steady-clock prediction, reverted ones 0.93 to 0.97, so the line is where the
model put it; what stops the climb is a governor that often does not raise the clock for a step.

### Modelled

`crates/present/src/drs/tests/probe.rs` runs the controller against two modelled GPUs at 60 Hz
for minutes of frames, a race opening half again as heavy for two seconds: one whose clock (0.6
to 1.0 of its top) moves a fifth of the way a frame toward holding the GPU 70 % busy, one at a
steady clock. Governed, with a race of 12.5 ms at full scale and top clock, the loop without a
probe holds 0.85, the probe takes it to 0.95, and a steady clock settles at 0.90; no frame late,
no size change in the last minute. The model's governor always raises its clock for a step; the
Deck's often does not, which is why the Deck lands at 0.90 and not where the model does. At a steady clock with races of 14 to 26 ms, the settle with
and without the probe is the same grid point and every probe reverts.

## Consequences

- **A GPU at a steady clock pays for the probes it reverts.** Each is a pair of size changes, one
  grid step up for under a second and back. The backoff makes that 2 changes in 4 s at first and
  about one pair every two minutes after about seven minutes at one scale. A real cost the old
  loop did not have, bounded and rare; the claim it is checked against is that the settled scale
  does not move.
- **A cost with a large part that does not scale with pixels can fool the keep test.** At 0.95 to
  1.00 a scene that is 60 % flat reads 0.94 of the quadratic prediction, which is why `KEEP_SHARE`
  is 0.92 and not halfway at 0.95; a scene flatter still keeps a step a steady clock pays for. Such
  a step lands inside the deadband by the second keep condition, so it costs margin, not a frame.
- **The probe can rise past where a held clock settles.** Where the governor still has clock in
  hand, a step that lands inside the deadband is kept, where the normal rise wants a landing under
  0.80. The scale then carries less margin against a spike, and a spike is answered by a fall as
  before.
- **The settle under a governor is higher, so it costs power**: on the Deck's 4K Zone the GPU
  drew about 11 W with the probe against about 8 W without (pinned, 15 W). On battery the deeper settle may be what a player prefers;
  that trade is a setting the maintainer may want, and is not part of this decision.
- **TARGET FPS `original`** (HD's width throttle) steps on a GPU duration too and does not get a
  probe: it reproduces the original's law, which has none.
