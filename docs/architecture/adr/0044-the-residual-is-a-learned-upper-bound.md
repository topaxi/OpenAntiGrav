# ADR-0044: The residual is a learned upper bound, not a constant

## Status

Accepted. Extends [ADR-0042](0042-the-dynamic-resolution-budget-subtracts-what-it-can-measure.md)
and [ADR-0043](0043-hd-bloom-joins-the-scalable-budget.md) rather than
superseding either: the budget is still `period - fixed - residual`, and the
only thing that changes is where the last of those three numbers comes from.
It also answers, in the narrow form the evidence supports, the alternative
ADR-0043 recorded and declined - "time the CPU-side per-frame cost directly".

## Context

**Reported from play, 2026-09-03, on a Steam Deck (AMD, RADV) at a 90 Hz
target.** The machine holds 90 FPS at the full render scale with no temporal
reconstruction running, and the controller shrinks it to 60-65 % of the
allocation and settles there.

**That report has two candidate explanations and this ADR is only one of
them**, which is worth saying before any of the argument below. A separate fix
landed the same day for a controller that could *freeze* at whatever scale it
happened to hold - a failed FSR 3.1 shader build left a timestamp slot
unwritten, and the read-back asked the settings row rather than the running
state, so `Cost::fixed` read as "not reported yet" forever. As relayed into
this work the scale "wobbles a little" around its settling point, which is a
live loop settling too low rather than a frozen one; the thread that found the
freeze reads the same report as never moving again. The two faults are
independent, both fixes stand whichever reading is right, and what is not in
dispute either way is where the constant came from.

`RESIDUAL_SHARE` is 0.20 - 2.2 ms of an 11.1 ms frame - and ADR-0043 says in
as many words where that number came from: one circuit (`talons_junction`),
one adapter, one grid size, on the development machine. The Deck is a second
adapter, and it does not spend 2.2 ms of every frame outside its timers. Both
ADRs that set this constant recorded the same caveat about it, and neither had
any way for the number to be wrong *in the direction of reserving too much*
and be noticed: over-reserving does not drop a frame, it takes pixels away
quietly.

The raw material for a better answer already exists and is already on screen.
`perf::GpuCost::residual_ms` computes `frame - (scene + blur + bloom + fsr3)`
for the `dev` overlay's `OTHER` row, which ADR-0043 added. That is the real
per-machine number. It is not fed back into anything.

**It cannot be fed back raw, and ADR-0040 says why.** A paced loop sleeps
whenever it has headroom - to the refresh under vsync, to the limiter's period
otherwise - so a comfortable frame's wall clock reads as the target period
however little work it did. `frame - timed` is then mostly sleep, and folding
that into the budget would shrink it, which lowers the scale, which creates
more sleep. The failure ADR-0040 rejected the frame interval for is exactly
this one, one term further along.

The algebra says how bad it gets, and it is worth writing out because it also
rules out the obvious gate. Let `E` be the reserve in force, `B = period -
fixed - E` the budget, and `r = scalable / B` the ratio the controller
divides by. Under pacing, a comfortable frame's reading is

```text
reading = period - fixed - scalable = (1 - r)(period - fixed) + rE
```

Accepting readings from frames at the deadband's lower edge - `r` in
0.80..=0.95, which is where the controller *settles* - folds in a value above
`E` for every `E` below `period - fixed`. Iterated, its fixed point is
`E = period - fixed`: a budget of zero, a scale at the floor, and a loop that
sleeps longer the harder it shrinks. A learning rule that runs away in the
direction the controller already errs in would be worse than the constant.

## Decision

**The reserve is a `drs::Residual`: an estimate in seconds that starts at
`RESIDUAL_SHARE * period` and is tightened by frames that can prove a smaller
number.** `Target::scalable_budget` takes one instead of reading the constant,
`Controller` owns one, and `Controller::record` gains a `frame_seconds:
Option<f32>` the composition root fills from the same `elapsed` it feeds
`perf::Meter`. The controller still never reads a clock; it is handed one more
fed measurement, which is the shape every other input to this module already
has.

Two facts carry the whole design, and both are properties rather than choices.

**Every reading is an upper bound on the truth.** Sleep is never negative, so
`reading = frame - timed = residual + sleep >= residual` on every frame, paced
or not. An observation can never claim the machine spends *less* outside its
timers than it does - only less than the last guess did.

**The gate is the no-slack condition, solved for the reading.** Under pacing,

```text
reading <= E  <=>  period - fixed - scalable <= E  <=>  scalable >= B
```

so a reading at or below the reserve in force is exactly a frame whose
scalable cost had already filled its budget: a ratio of 1.0 or more, with no
slack for the loop to have slept away. That is not a threshold somebody
picked. It is ADR-0040's own condition rearranged, and it is why the rule is
"a reading below what is held is always safe to fold in".

From those two, the rule:

- **Down** needs nothing further. The reading is a tighter upper bound than
  what is held and is bounded below by the truth, so the estimate converges on
  the machine's real residual from above and cannot pass it.
- **Up** needs evidence that the loop was not sleeping, because a reading
  above the reserve is either real untimed work - the AI/physics cost for a
  full grid ADR-0043 measured at ~2.4 ms of a ~8.7 ms frame and could not time
  - or it is sleep. `OVERRUN = 1.05` is the test: a frame that ran 5 % past
  the target period is a frame the loop did not sleep through. Five per cent
  is jitter's width, since a timer wakes at or after its deadline and never
  before, which is why `Session::schedule_next_frame` exists.
- **`RESIDUAL_SHARE` caps the result.** The rise test has one hole - a display
  refreshing *below* the target, where every frame overruns the period while
  still sleeping, and this build cannot ask a surface what its refresh is (see
  `Target::at_most`, which clamps to a frame limiter for that reason and
  cannot clamp to a panel). Capped at the constant, that mistake costs exactly
  the budget this build already shipped and never more.

**`RESIDUAL_SHARE` therefore keeps two roles and loses one.** It is the prior
a session opens on and the ceiling on what any amount of learning may reserve
- **a floor under the budget, not a floor under the reserve.** A floor under
the reserve is the one thing it cannot be: the machine this ADR was written
for needs to reserve *less* than 0.20, and a constant that could only be
raised would have no way to say so.

**`LEARN_RATE = 0.25`, and the number is empirical.** The window a sample can
arrive in is short and the controller is what closes it: a machine over budget
falls in a handful of steps, and every step buys back slack the next frame's
reading hides in. Against the modelled machine below, a twentieth of the gap
per frame reaches 2.10 ms of a 2.22 ms reserve before the evidence dries up,
and one grid step of scale; a quarter reaches 1.61 ms and two steps. The
reverse risk is bounded by the same two gates: an estimate that tightened too
far leaves the budget claiming room the machine does not have, which shows up
as frames past the period, which is what `OVERRUN` admits.

**The estimate is in memory, for the session, and structurally cannot be
anything else.** `Residual` carries no `Serialize`/`Deserialize` - the reason
`Limits` is passed by value and `drs` does not import `crate::settings`. It is
not a preference; it is what this machine is doing this run, and a boot is
exactly the granularity at which the answer can change: a different adapter, a
different driver, a different display. `Controller::reset` deliberately keeps
it across a track load, for the same reason it keeps the scale.

**The caller passes the instantaneous frame time, not `Stats::mean_ms`.** The
overlay's `OTHER` row subtracts from the mean, and that is right for a
diagnostic. It is wrong here: a mean over the last hundred-odd frames against
a single frame's timed passes is two signals a hundred frames out of phase,
`residual_ms`'s own doc already records that the subtraction can go negative
for that reason, and a rule that only ever tightens keeps every dip the phase
error invents. One frame against its own passes, and the estimate's own
smoothing is the only smoothing.

## Alternatives considered

**Gate on the deadband's lower edge** - "the ratio was already at or over
0.80, so there was no slack". This was the first shape proposed, and the
algebra in the Context section is why it is not the one implemented: at the
settle point that condition folds in readings *above* the reserve, and its
fixed point is a zero budget. The gate that survives is the same idea taken to
where it is actually true rather than nearly true.

**Persist the learned value to `settings.toml`.** Rejected outright, and the
type is shaped so it cannot happen. A residual is hardware- and
session-transient; writing one into a player's settings file is the same
mistake as writing `render_scale` there, which `drs`'s module docs have
forbidden since the controller landed. A machine whose driver changed would
carry the old number forward with nothing to notice it.

**Feed the rolling mean.** Simpler at the call site - `self.meter.stats()` is
already there and is what the `OTHER` row uses - and rejected for the one-way
leak described above.

**Accept every reading when nothing is pacing the loop** (vsync off *and*
`FrameLimit::UNLIMITED`), where `frame` is the work and no gate is needed at
all. It would be correct, and it is a third branch for the one configuration
`perf::Meter`'s own docs call "a diagnostic configuration rather than a
shipping one". The existing two branches already accept every frame in that
configuration anyway - with no sleep, an over-budget frame's reading is below
the reserve and an under-budget one's is the truth with the loop running past
its period only when it genuinely is - so the branch would buy nothing but a
third thing to keep consistent.

**Time the untimed work directly**, the fifth ring ADR-0043 declined. Still
the right eventual answer and still not this change: the largest term in the
residual is CPU-side (AI, physics and draw-call submission for a full grid),
`PassTimer` reaches GPU work only, and there is no existing mechanism to
reuse. This ADR is what can be inferred from the clock the loop already reads;
it does not replace measuring the thing.

**Measure `RESIDUAL_SHARE` again on a second adapter and pick a better
constant.** That is the option ADR-0042 and ADR-0043 both took, and the report
that opened this thread is what a third round of it looks like. No constant
can be right on two machines whose untimed work differs by a factor of four,
and the tell is that both previous ADRs wrote the caveat down and neither
could act on it.

## Consequences

**A modelled machine keeps two grid steps more scale, and the model is in the
test suite rather than in this paragraph.** `drs::tests::machines` simulates a
machine frame by frame - `flat + per_pixel * scale^2` for the scene pass,
`max(pace, work)` for the wall clock - and feeds the real controller. The
split inside the scene pass is chosen to put the constant's settling point
where the report puts it; everything else is what the report states. On the
reported shape (11.1 ms period, 0.5 ms genuinely outside every timer):

| Pacing | Constant | Learned | Reserve reached |
| --- | --- | --- | --- |
| Vsync at the target rate | 0.80 scale | **0.90** | 1.61 ms of 2.22 |
| Frame limiter at 240, target 90 | 0.80 scale | **0.95** | 0.50 ms - the truth |

**The vsync row is the honest limit of the whole mechanism.** A loop sleeping
to the very rate the controller aims at destroys the evidence before this
module sees it: once the controller has bought back enough slack to be
comfortable, every reading is inflated by the sleep it just created, the gate
refuses all of them, and the estimate stops where the last tight frame left
it. Two grid steps back, not the four the truth would allow. Under a limiter
above the target - the shipped default is 240 - the loop's own pace is shorter
than the budget's period, the work is what is left on the clock, and the
estimate converges exactly. Closing the vsync gap needs the untimed work
timed, not a cleverer inference from this signal.

**Nobody has run this on the Steam Deck.** No machine in the session that
wrote it has that hardware, so every number above is a model of a report and
is labelled as one in the test that produces it. The handover thread carries
the check forward - and it should be taken with the freeze fix already in,
since that is the first competing explanation. It carries a second: a settle at
60-65 % implies a scalable cost around 19 ms at full scale under the quadratic
model, which is *not* consistent with the same machine holding 90 FPS at full
scale unless a large part of that pass does not scale with pixels at all. A
mis-scaled `TIMESTAMP_QUERY` period would produce the same picture - and
`docs/rendering/dynamic-resolution.md` already records a 52x difference in
timestamp period between two adapters on one laptop. This ADR makes the budget
right; it does not prove the budget was the only thing wrong.

**The budget is no longer constant within a session, so the log had to say
so.** Two `dynamic resolution:` trace lines a minute apart can divide the same
scalable cost by two different budgets. Both that line and the once-per-spell
`out of reach` warning now print the reserve in force and whether it is
learned or assumed; without it there is nothing in a log that would let a
reader reconstruct the arithmetic.

**No budget shrinks, ever.** `a_learned_residual_never_reserves_more_than_the_constant`
asserts over a grid of targets, fixed costs and learned values that the budget
with a learned reserve is never smaller than the budget with the constant, and
that no target becomes unreachable that was not already. The worst case this
change can produce is the behaviour that shipped before it.

**It does not make the picture hunt**, which was the risk of a budget that
moves under a controller: `a_moving_budget_does_not_make_the_scale_hunt` runs
all three modelled machines and asserts zero scale changes over their last
thousand frames, with the constant's own answer as the bar. That property is
worth more than the extra sharpness - `RISE_PATIENCE` exists because a scale
that settles reads better than a scale that is right on average.

**`Controller::record` grew an argument**, and every existing test names it.
That is deliberate: the alternative was a second entry point, and a second
entry point is where a caller forgets to feed the learning. `None` is the
honest answer for a frame that carried a load, which is the same guard
`meter.clear()` sits behind.

**A player on a display refreshing below their target gets the old
behaviour**, not a worse one: every frame overruns, every reading is trusted,
and the cap holds the result at `RESIDUAL_SHARE`. Getting that case *right*
rather than merely bounded needs a refresh rate off the monitor, which
`Target::at_most` has wanted since it was written.
