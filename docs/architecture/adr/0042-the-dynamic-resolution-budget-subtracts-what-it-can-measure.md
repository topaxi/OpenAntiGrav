# ADR-0042: The dynamic-resolution budget subtracts what it can measure

## Status

Accepted. Supersedes [ADR-0040](0040-the-dynamic-resolution-budget-is-a-share-of-a-frame.md)'s
**budget formula** - its argument against deriving headroom from the wall-clock
frame interval stands untouched; only the constant-share half is replaced.

## Context

ADR-0040 chose `budget = SCENE_SHARE * period`, `SCENE_SHARE = 0.45`, timed
against the `race` pass alone. It was a real number from a real machine - 1.556
ms of a 16.67 ms frame at 100 % render scale, on one adapter, on one track - and
it was explicit that this was "a choice, not a reading" that would need
revisiting once more of the frame joined the budget.

**Reported from play, 2026-09-03**: on Wipeout HD/Fury with FSR 3.1, MSAA 4x,
motion blur `high`, `target_fps = 120`, the frame ran at 80-90 FPS with the
render scale sitting at 100 %, sometimes 95. Read off the `dev` overlay during a
heavy race: `GPU SCENE` and `FSR3` were both **2.4 ms**, at a frame of roughly
11-12 ms. Two derivations settle what was wrong, both from that one measurement
and neither needing a reproduction:

- **The controller was correctly satisfied and the budget was wrong.**
  `2.4 / 3.75` (the `SCENE_SHARE` budget at 120 Hz) is 0.64, comfortably under
  the deadband floor of 0.80. The controller asked to climb; it was already at
  the ceiling.
- **The scene pass was 20 % of the frame, not 45 %.** At 85 FPS (11.76 ms), 2.4
  ms of scene against 2.4 ms of FSR 3.1 chain and roughly 7 ms of everything
  else is a scene share of 20 %, not the 45 % `SCENE_SHARE` asserted.

**No single constant can fix this**, and that is the finding this ADR is built
on rather than a restatement of ADR-0040's own caveat. On the same machine and
circuit, holding render scale and target fixed and varying only `msaa` and
`motion_blur` (measured in the `.claude/worktrees/drs-budget` worktree, once
the `--race` profile bug below was fixed):

| `msaa` | `motion_blur` | scene | fsr3 chain | frame |
| --- | --- | --- | --- | --- |
| off | off | 2.5 ms | 5.3 ms | 9.3 ms |
| 4x | off | 4.5 ms | 5.5 ms | 11.5 ms |
| off | high | 2.5 ms | 5.1 ms | 10.8 ms |
| 4x | high | 4.5 ms | 5.4 ms | 13.6 ms |

The scene pass's own share of the frame ranges from 23 % to 39 % across four
rows of one settings page. `SCENE_SHARE` would have to be a different constant
for each of them, which is not a constant at all.

### What was actually missing: the FSR 3.1 chain was timed and thrown away

`Session::upscale_cost` already measures the FSR 3.1 chain on the GPU, every
frame it runs, and reaches the `dev` overlay as its own row. Nothing between
that measurement and `drs::Controller::record` ever read it. The chain is not a
small fixed overhead: in the table above it is comparable to or larger than the
scene pass itself, and unlike the scene pass it does not fall when the render
extent does - the temporal accumulate and RCAS passes both run at *presentation*
resolution.

A controller budgeting against a share of the frame period, blind to a
same-order-of-magnitude cost sitting right next to the one it can see, cannot be
made correct by choosing a better constant. It needs the second number.

### A blocking measurement bug, found and fixed on the way

**`--race` silently ignored the render profile.** `Session::render_profile`
resolved `settings.render_profiles` off `self.shell.title.name`, and the
`--race` route - the only route that can be driven headlessly, which is what
any of the measurements above needed - has no shell. It fell to
`RenderProfile::default()`: `render_scale` 100, `reconstruction` off, `msaa`
off. Every `--render-scale`, `--reconstruction`, `--msaa` and `--motion-blur`
CLI flag was silently discarded, and so was the player's own settings file.
Measured before the fix: `--render-scale 100` and `--render-scale 200` both
gave a 0.80 ms scene pass and zero FSR 3.1 chain readings - two different
requests producing identical measurements.

Fixed by carrying the title `race::load` already resolves (to pick a default
track and team) out as `race::Loaded::title`, and reading it in
`Session::render_profile` and in `App::open`'s framebuffer sizing, which had the
same hole. Verified against the report: after the fix, `--render-scale 100`
gives 0.63 ms and `--render-scale 200` gives 2.93 ms, and the FSR 3.1 chain
reports thousands of readings where it reported none. This is what made every
table in this ADR a measurement rather than a repeat of the same default.

### Xvfb cannot be used to measure any of this

Tried first, because it is the sandbox's usual headless path. Measured
2026-09-03 at 2560x1440: the scene pass reads 7.0 ms against 2.4 ms on the real
display, the loop manages roughly 6 FPS, and toggling `msaa` or `motion_blur`
moves the reading by nothing at all - four legs within noise of each other.
Xvfb presents in software, and a 2560x1440 copy every frame dominates whatever
it is standing beside. It remains fine for driving the game and for anything
about pixels; it cannot size a GPU pass. Every number in this ADR is from the
real display, via `env -u WAYLAND_DISPLAY -u XDG_SESSION_TYPE` - winit prefers
Wayland whenever `WAYLAND_DISPLAY` is set, so a plain `DISPLAY=:99` from inside
a Wayland session does not reach Xvfb either and silently opens a window on the
real desktop instead.

## Decision

**The budget is `period - fixed - RESIDUAL_SHARE * period`**, where `fixed` is
a measured `drs::Cost::fixed` and `RESIDUAL_SHARE` is a small constant for what
is left over. `drs::Cost` splits one frame's GPU cost into:

- **`scalable`**: the `race` pass plus the motion-blur chain, both drawn
  through the render extent and both now timed. Motion blur needed a new
  timestamp pair of its own - see "Timing motion blur" below.
- **`fixed`**: the FSR 3.1 chain's reading, in full. Conservative rather than
  exact - the chain's render-resolution passes (depth/velocity prepare, the
  luma and shading-change pyramids) do shrink with the extent, and only
  `accumulate` and `rcas` are truly presentation-bound - but splitting those out
  needs a second timestamp pair inside `Fsr3::render` and is not done here.
  Counting the whole chain as fixed makes the budget smaller than the true
  figure, which biases the controller toward falling rather than rising - the
  safe direction for the bug this ADR fixes.

`RESIDUAL_SHARE = 0.15` covers what neither term measures: the HUD, the UI
composite, the blit, the performance overlay, the MSAA resolve, and the
driver's own overhead. Measured across the four rows in the table above, this
residual sat at **1.45 ms of a 9.3 ms frame and did not move between rows** -
0.156, rounded down to 0.15 so the reservation is slightly generous rather than
slightly tight, given the known gap below.

**This is a constant, and it is defensible as one for a reason `SCENE_SHARE`
never had**: everything folded into it is either fixed presentation-resolution
work (HUD, composite, blit, overlay) that does not vary with the render
profile, or - the MSAA resolve - work over the *allocation* rather than the
extent, so a controller cannot shrink it by changing the extent regardless of
whether it is timed. `SCENE_SHARE` stood for the FSR 3.1 chain, which is
neither of those things: it is comparable in size to the scalable cost and it
changes with the render profile (`msaa`, `motion_blur`, `reconstruction`,
`upscale_sharpness`) in ways `RESIDUAL_SHARE` provably does not, per the table.

**Known gap, not fixed here**: `hd_bloom` draws through the extent and belongs
in `scalable`, and is not timed. The measured circuit authors `blur steps
1/1`, the cheapest ladder on the disc, so its cost hides inside the 1.45 ms
residual. A circuit with a longer ladder would push the residual estimate up,
and the controller would under-shrink there. This is recorded as the next term
to add, not a reason to inflate `RESIDUAL_SHARE` speculatively for a case that
was not measured.

### Timing motion blur: a chain, not a pass

The motion-blur pass is six render passes (prepare, two tile-max reductions,
neighbour-max, reconstruct, copy), and `PassTimer::writes` brackets exactly one
pass. `PassTimer` gained `half_writes(Half::Begin | Half::End)`: wgpu's
`beginning_of_pass_write_index` and `end_of_pass_write_index` are independently
optional, so the opening timestamp can ride the chain's first pass and the
closing one its last, covering all six with one claimed slot and no
`TIMESTAMP_QUERY_INSIDE_ENCODERS`.

This needed one addition to `PassTimer` beyond the two indices:
**`PassTimer::abandon`**, to give a claimed slot back when the chain turns out
to encode nothing. `MotionBlur::render` returns early - at `strength <= 0.0`, or
before its scratch targets are built - and a slot that only ever received an
opening write resolves its closing timestamp to an unspecified value, not a
zero. `PassTimer::begin`'s own contract is that an unresolved claim never comes
back and four of them end measurement for the run, so the caller has to be able
to say "there will be no pass" after having already claimed.

### Reading three rings without crediting a partial frame

The frame loop now claims and resolves three `PassTimer` rings a frame: scene,
motion blur, FSR 3.1. They are read together before any reading is acted on,
and a `drs::Cost` is assembled only from readings that **name the same frame**.
The three claims happen on the same frames and resolve into the same encoder,
so in a running race they arrive together; a mismatch means one ring skipped a
slot (all four full), and a cost built by pairing one ring's reading against a
different frame's from another is not a real frame's cost - it would credit a
cheap frame with an expensive frame's fixed cost, or the reverse, and step the
controller in a direction nothing observed.

An absent FSR 3.1 reading is read one of two ways, and they are not
interchangeable: if `reconstruction` is not temporal, nothing was ever going to
run, and absent means the fixed cost was genuinely zero. If `reconstruction`
*is* temporal and the reading is still absent, the ring was simply full, and
treating that as zero would silently drop the single largest term in the
budget on exactly the frames a loaded machine is most likely to skip a slot.
The second case is a skipped frame, not a free one.

### The unreachable target: earned by a run, not declared on one frame

`Controller::record` gained an `unreachable` signal for the case ADR-0040's
`Target::at_most` exists to prevent from the other direction: a target no
render scale can deliver, because the measured fixed cost alone fills more of
the frame period than `RESIDUAL_SHARE` leaves room for. The controller must not
walk toward the floor chasing a rate that will never arrive - every pixel given
up would buy nothing, which is the exact failure `at_most` was written to stop
at the clamping stage. Here the same failure is possible with the target
correctly clamped and still out of reach, because a measured fixed cost is not
a static ceiling.

**Earned by `RISE_PATIENCE` consecutive frames of "nothing left to decide"
rather than declared on one frame**, for the same reason a rise needs earning: a
target the machine can only just hold makes the condition flicker between
frames, and a caller that logged every flicker would print continuously about a
situation that had not changed. Measured on the reported HD/Fury profile before
this fix: the raw per-frame condition fired 68 times in 41 seconds; after
requiring a run, 7.

### A rise has to survive its own overshoot, not just clear the deadband

Building and testing the budget surfaced a second defect the report's own
numbers did not: with the scalable budget now visible in full, the controller
oscillated **176 times in 41 seconds** on the reported profile - worse than the
143-a-minute figure `RISE_PATIENCE` was originally added to fix, and for a
related but distinct reason.

One grid step is a fixed fraction of the *ceiling* (5 % at `GRID = 20`), so it
is a growing fraction of *cost* as the scale falls - cost goes as pixel count,
so a step from scale `s` to `s + STEP` multiplies cost by
`((s + STEP) / s)^2`. At the top of the range that is about 10 %; at half
scale it is about 21 %. [`DEADBAND`](../../../crates/present/src/drs/policy.rs) is
15 % wide. Below roughly 70 % scale, a rise the deadband would have permitted
lands *outside* the band on the other side and is corrected back down next
frame - the two-point ping-pong `RISE_PATIENCE` already exists to prevent,
reappearing once the true scalable cost pushed the controller low enough on the
grid for the geometry to bite.

**Fixed by predicting the landing ratio before taking a rise**: a candidate
step to `next` is refused unless `ratio * (next / here)^2` still clears
`DEADBAND.start()` - conservative on purpose, checked against the *start* of
the band rather than its midpoint, because the quadratic cost model is itself
optimistic (a scene pass carries per-frame cost that does not scale with
pixels; measured at half scale the real multiplier was 1.30 against the model's
1.21). Measured after the fix, same profile: 83 changes in 41 seconds.

## Alternatives considered

**Split the FSR 3.1 chain into its own render-resolution and
presentation-resolution readings**, folding the first into `scalable` and only
the second into `fixed`. More accurate, and declined for now: it needs a second
timestamp pair inside `Fsr3::render`, a fourth `PassTimer` ring, and a
`fsr3::Dispatch`-level split between "renders at extent" and "renders at
output" that does not exist yet. Counting the whole chain as fixed is
pessimistic in the safe direction in the meantime - see the Decision section.

**Time `hd_bloom` now, to close the known gap immediately.** Declined for the
same reason motion blur's own multi-pass chain was worth doing carefully:
`hd_bloom`'s ladder length is a runtime decision (`docs/rendering/hd-bloom.md`),
so bracketing it correctly needs the same first/last split `half_writes` gives
motion blur, on a chain whose length can be zero, one, or several levels. This
ADR's own residual measurement was on the cheapest ladder the disc authors;
doing this properly wants a circuit with a longer one to measure against; and
the correctness of the fix depended on the `--race` profile bug being fixed
first, which this ADR needed to do regardless.

**Derive the fixed cost from a formula (upstream FSR 3.1's own documented
per-dispatch costs, or a fitted curve) rather than measuring it.** Declined
because it reproduces exactly the failure mode this ADR replaces: a formula
tuned on one adapter is wrong on another the moment the adapter's own timestamp
period differs, which `docs/rendering/dynamic-resolution.md` already measured
at 52x between two adapters in one laptop. `Session::upscale_cost` already
measures the real number on the real device; using anything else would be
choosing to be wrong again.

## Consequences

**The controller now depends on two more timers being available.** On an
adapter with `TIMESTAMP_QUERY` where the FSR 3.1 ring never claims a slot -
every menu frame, every non-`fsr3` reconstruction - `fixed` is legitimately
zero and the controller behaves as it always did. Where the ring exists but is
momentarily full, the frame is skipped rather than mis-costed; see "Reading
three rings" above. Where the *device* has no `TIMESTAMP_QUERY` at all, all
three timers are `None` from `PassTimer::new` and the controller runs exactly
as before this ADR: it never receives a `Cost` and never moves the extent,
which is the existing `pass_timer: Option<PassTimer>` behaviour and not a new
failure mode.

**`SCENE_SHARE` is gone**; every reader of it - the two constants' doc comments,
`docs/rendering/dynamic-resolution.md`'s budget table and its "budget is a
share of a frame" section, `docs/overview/roadmap.md` - now names
`RESIDUAL_SHARE` and says what it covers.

**A player on the reported profile gets a working `target_fps = 120`.**
Verified end to end, same disc and circuit, before and after: before, the
render scale sat at 100 % (dipping to 95) and the frame ran 80-90 FPS; after,
the render scale settles between 50 and 60 % and the loop holds 120 FPS with 83
scale changes in 41 seconds rather than 176.

**The measured `RESIDUAL_SHARE = 0.15` is one circuit, one adapter, and the
disc's cheapest bloom ladder.** It is a considered constant with an argued
reason to expect it to travel (everything it covers is presentation-bound or
allocation-bound, neither of which the render profile moves) but it has not
been checked against a circuit with a heavier `hd_bloom` chain, a different
title, or a second adapter. The same caution ADR-0040 stated about
`SCENE_SHARE` being "a constant to revisit" applies here, narrower in scope: not
"this is the wrong shape of number" but "this is the right shape of number,
measured once."

**The rise-refusal guard trades some responsiveness for stability.** A rise
that the guard blocks is a rise the old deadband-only check would have allowed
and then immediately undone; refusing it outright means the controller can sit
one grid step lower than the deadband alone would tolerate, for as long as the
predicted-landing test keeps failing. That is the same trade `RISE_PATIENCE`
already made once, at a different point in the loop, and the same argument
applies: a picture that holds a slightly lower resolution steadily reads better
than one that visibly resizes twice a second.
