# Physics

**Status: model recovered from static analysis, and now partly verified against
captures of the original.** The structure below is implemented in
[`oag-physics`](../../crates/physics/src/lib.rs).

The shape of the ship dynamics is known, and every constant it needs lives in
[handling stats](../formats/handling-stats.md). **"None of it is
runtime-verified" stood here until captures existed and is no longer true**: the
engine's thrust law, the two passive resistances, the lateral grip law, the
airbrake terms and the yaw accumulator have all been measured against the
original and are marked in place below. The rest of this page is still a
specification to test against rather than truth, and what exists in code is a
transcription of it, deliberately including the parts that look wrong. See [what is implemented](#what-is-implemented) for the split,
[the cross-product signs](#the-cross-product-signs-a-contradiction-here-resolved)
for the contradiction on this page that arithmetic could settle, and
[still open](#still-open) for the readings the evidence leaves undetermined.

**One part of this model is measured against the original rather than read.**
[The along-track force balance](force-balance-ground-truth.md) reconstructs, from
captured traces, what force the original actually applies along a craft's forward
axis. **Its conclusion has since inverted, and this paragraph used to carry the
retracted version** ("the crate applies about 53 units of net force too much, in
a term linear in speed"): the force law was right all along, and the two
reference captures were recorded scraping a wall, losing `3.5 %` of their
tangential velocity per frame in a post-integrate contact pass no force
accumulator can see. That page is still the authority on the longitudinal
balance and on which candidate terms are eliminated; read its resolution
section, not its investigation, for the current state.

**A wall-free cornering capture now measures the rest of the law.**
[Cornering, measured against the original](cornering-ground-truth.md) projects a
completed Time Trial lap onto the ship's forward *and* right axes: it reconfirms
the forward law at slip angles up to 25 degrees, settles `Ship_ApplyLateralGrip`
at `0.9985` of the disc's `grip_ground`, closes the yaw accumulator term by term
(`steer * Turning.amount` at `0.9998`, damping at `-5.04`), and **refutes**
`avel = -I * omega` on the pitch and roll axes while confirming it on yaw. It is
the authority on the lateral axis and on the airbrake terms' magnitudes.

**A second part is measured rather than read**, and it is the rotational half of
the same story. [What `body+0x160` holds](angular-velocity-column.md) settles the
sign and the frame of the recorded angular-velocity column against five captures'
own basis derivatives, and finds the yaw axis carrying a factor of `21.2`,
arrived at with no simulation and no fit against our own physics. The tensor's
writer has since been read - a hard-coded solid box, see
[rigid-body.md](../ghidra/functions/psp-pulse-usa/rigid-body.md) - and it gives
`21.6`, so `oag_physics::forces::YAW_INVERSE_INERTIA` is now a recovered value
rather than the fitted `YAW_DRIVE_CALIBRATION` that stood there.

**This page no longer covers the whole force law.**
[Engine, brakes, steering and pitch](../ghidra/functions/psp-pulse-usa/engine.md) was read
out of `BOOT.BIN` afterwards, and it is the authority wherever the two disagree: it was
read from the XML loader and the craft update path directly, with addresses and a
per-component enumeration of every accumulator write. It **corrected five things on this
page**, all of them now marked in place: `ride_height` does reach the force law, the
`slidegrip` pseudocode was right once its load-time scaling is known, gravity has no
track-relative component, groundedness is one frame stale for every control term rather
than only for the hover spring's load factor, and the roll negative needed narrowing from
"no Z component is ever written" to "no *control input* writes one". It also closed this
page's "no angular damping" gap, and it introduced a second instance of the
[cross-product sign problem](#the-cross-product-signs-a-contradiction-here-resolved).

## The two answers that were blocking M4

### It is floating point, not fixed point

Pure IEEE single precision throughout, scalar FPU plus VFPU. The only integer
operations in the craft path are pointer arithmetic, loop counters and flag
masks. Confidence **92**: an exhaustive negative over the whole craft path,
every call site consistent, which is exactly the shape of claim decompilation
is good at settling. Not checked against shipped data and never run under an
emulator, so per the [rubric](../reverse-engineering/confidence-rubric.md) it
stays below 95 like the rest of this page; the earlier 97 predated the rubric
saying explicitly that nothing here has earned the top band.

So [ADR-0002](../architecture/adr/0002-determinism-model.md)'s fallback of
mirroring a fixed-point format exactly does not apply. Bit-exactness is not
available for free, and the tolerance-based approach stands.

### The integrator does not sub-step at 1/60

This was the open question from [frame pacing](../psp/frame-pacing.md), and the
answer is the less convenient one:

```
dt = clamp(measured_dt, 0, 0.06666)
evaluate all forces once at full dt
integrate 3 explicit Euler sub-steps of dt/3
```

The sub-step *count* is fixed at three; the sub-step *size* is variable. There
is no 1/60 anywhere in the craft path, despite fourteen other systems using it.

**So the original's ship handling is frame-rate dependent.** A longer frame
produces larger sub-steps and a different trajectory, and explicit Euler diverges
more at larger steps rather than degrading gracefully.

That makes [ADR-0007](../architecture/adr/0007-fixed-timestep-vs-original.md)'s
stated cost real rather than hypothetical: our fixed timestep will not track the
original through a frame-rate dip, and the divergence is cumulative. The
verification harness must record the original's per-frame delta so drifted
intervals can be told apart from genuine bugs.

Reproducing the three-sub-step structure at our fixed `dt` is still right: at a
steady 60 Hz it is exactly what the original does.

## The air cushion

Two raycast probes, each a spring-damper. Rays are cast along the **ship's own
up axis**, not world gravity, which is what lets magstrips and inversions work.

Per probe:

```
h    = dot(probeWorld - rayHit, up)
v    = bodyLinearVel + M * cross(probeLocal, angularVel)
vn   = dot(v, normal)

spring = mass * 0.3 * (targetHeight - h) * K
       * (0.75 * grounded_prev + 0.25)
       * (normal_gravity + track_gravity)

d    = clamp(-0.1 * vn, -1.0, 2.0)
reb  = timeSinceLanding >= 0.2
         ? rebound
         : 0.5 * (rebound * t + landing_rebound * (1 - 5*t))

F    = up * spring * (1 + reb * d) * (1 - magLockBlend)
AddForceAtPoint(F, probeWorld)
```

Several details are counter-intuitive and worth calling out:

- **Stiffness carries no handling parameter.** It is
  `mass * 0.3 * (normal_gravity + track_gravity)`, calibrated against gravity so
  weight balances at a height error of `1 / 0.3 ≈ 3.33` units. Ship-to-ship
  differences come from the gravity values, not from a spring constant.
- **`rebound` and `landing_rebound` scale only the damping**, never the spring.
  `landing_rebound` replaces `rebound` for the first 0.2 s after touchdown.
- ~~**`ride_height` never appears in the force law.**~~ **Corrected.** It is the
  raycast length *and* the primary term of the hover spring's target height. The offset
  chain from the parser to the spring is traced in
  [engine.md](../ghidra/functions/psp-pulse-usa/engine.md) at confidence **88**:
  `target = (ride_height + craft+0x74 - min(leapTimer, 4)) * (1 + 0.2 * magLockBlend) * K2`.
  What writes the additive `craft+0x74` was not found, so the composition is still open;
  `antigrav_height_adjust` is the obvious suspect and nothing was found that copies it
  there. The height the ship settles at is still emergent, being wherever the spring
  balances the load, but it is emergent *around a target that the data sets*.
- **Nothing explicitly pitches the ship to the track.** Pitch and roll response
  is entirely emergent from applying two forces at two points.
- Contact is accepted only for surface types 1 (Floor) and 3 (Mag Floor).
- Groundedness is quantised to `{0, 0.5, 1.0}`: contact count over two.

`K` is a global that the static image holds as `1.0` but which ship construction
overwrites with `1.3333` before any race runs. Confidence **90**, and worth a
runtime check since it scales hover stiffness directly.

A hard constraint exists but is not the hover mechanism: when `h < 1.0` on a
floor surface, position is teleported by `up * (1 - h)` with no velocity change.
That is penetration escape for the last unit only.

Grounded-only terms from the same function: a surface-alignment torque of
magnitude `400` along `cross(up, avgNormal)` with the right-axis component
projected out, which levels roll and yaw but deliberately **not** pitch; a
bank-to-yaw coupling of `+30 * right.y`; and a downforce along the ground normal
opposing the hover spring, whose magnitude is
`track_gravity * mass * grounded * (1 - magLockBlend)` - read at instruction
level in
[engine.md](../ghidra/functions/psp-pulse-usa/engine.md#the-grounded-downforce-read-instruction-by-instruction),
where the coefficient is `1` and the term turns out to be what the spring's
`normal_gravity + track_gravity` calibration is calibrated *for*.

Confidence **91** for the force law.

### The contact test itself is exact against a capture, 2026-09-07

The force law's confidence is one thing; **whether our probes decide *contact*
where the original's do is a separate question, and it is now measured rather
than inferred.** `crates/trace/tests/hover_contact_ground_truth.rs` walks the
recorded poses of `data/traces/talons-junction-clean-lap.csv` - the original's
own position and basis, tick by tick - and asks `oag_physics::hover::evaluate`
what it finds there. No integration, no accumulated error, so nothing about the
trajectory can enter the answer.

**It reproduces the recording's entire `grounded` column: 0 disagreements over
all 2,976 comparable ticks**, including the 23 ticks of `0.5` and both airborne
events. The comparison is our verdict at pose `t` against the recording's column
at `t + 1`, which is the same frame ordering the replay harness already emits
under - `oag_trace::replay::replay` pushes its row *before* stepping, so a replay
row at `t` also carries the contact evaluated at pose `t - 1`, and `speed_cached`
is documented as the previous frame's value at 199/199 for the same reason.

Two things follow, and the second is the useful one:

- The probe reach (`target_height`, which is also the ray length), the surface
  classes accepted, the probe offsets and the fast-path branch are all right on
  this circuit. **A residual that looks like the cushion holding on too long is
  not the cushion.** The seven-tick liftoff lag the hover thread measured at
  `--reseed 60` (measured: ours airborne 1603-1615 against the original's
  1595-1608, liftoff eight ticks late) survives only until the pose is made
  exact. `oag-trace run --reseed 2` on the same capture reports `grounded max
  error 0.000e0 ... exact` with the integrator and the sweep in the loop, which a
  pose walk does not exercise - though read that as half a column: at reseed 2 the
  even rows are the reseed echo, and only the odd rows carry a stepped verdict.
  They cover every transition on the crest. What is left is trajectory phase -
  where the craft is on the crest by the time it gets there - which is the
  force-law drift [oag-trace.md](../tools/oag-trace.md) already documents.
- **The pose walk is the technique to reach for whenever a per-tick boolean
  disagrees**, and it is cheap. It is the same method
  `crates/trace/tests/wall_contact_ground_truth.rs` uses for the hull, and it
  answers a question a whole-lap replay structurally cannot: a reseed restores
  position, not the phase of the feature the craft is crossing.

### The crest-phase drift traces to a wall event, not a force term, 2026-09-10

The pose walk above generalises past the contact flag: `crates/trace/tests/
force_term_pose_walk_ground_truth.rs` walks the same capture's recorded poses
over ticks 1560-1600 - the reseed boundary through the crest's own liftoff -
and asks `oag_physics::forces::evaluate` what force it computes there, once
per tick, with no integration in the loop. `evaluate` does not integrate (only
`oag_physics::integrate::integrate` does), so `state.body.force` afterwards is
exactly that tick's accumulated total, seeded from zero; dividing by mass gives
a **predicted acceleration** with no accumulated trajectory error in it by
construction. The recording's own `v(t + 1) - v(t)` over `dt` gives a
**measured acceleration** from quantities the original actually held. The
difference is what the force law fails to explain about the recording's own
next velocity, fresh at every tick - getting tick 1594 right never depends on
getting tick 1560 right.

Every one of `forces::evaluate`'s eleven force terms (engine, lateral grip,
brakes, the airbrake path, gravity, drag, the hover downforce, the two probes'
own spring forces, rolling resistance, vertical damping, the speed-pad boost)
is read directly off `Evaluated` and summed; the sum is asserted equal to
`state.body.force` itself before anything is attributed to any of them, so a
forgotten term cannot hide in the residual and get credited to whichever real
term happens to be a similar size.

**One tick in the window, 1583, has a residual over 100x any other**: the
recording's own velocity goes from `(-65.6, -8.5, -86.2)` to `(-83.4, 4.0,
-53.4)` in one tick while the recorded speed barely moves (108.6 to 108.8
units/s) - a near-elastic redirect, not a smooth integration. That is the
signature of a projection-like event outside the force law entirely - most
plausibly `oag_physics::wall::resolve`, which runs *after* the integrator on
the position it produced and so is invisible to a pose walk of
`forces::evaluate` by construction, but **this is the strongest reading of the
evidence, not a confirmed one**: `wall::resolve` was never run at this pose in
this pass, and the collision geometry at tick 1583 was never queried. What is
measured, not inferred, is near-constant speed, a large direction change,
`escape == 0`, `stun_timer == 0` throughout the window (a track wall never
arms it in this engine, only a weapon or rival contact does -
`ShipState::stun_timer`'s own doc), and no force term predicting it. Tick
1579, the landing tick of the smaller 1571-1579 airborne event, fires a
penetration-escape teleport for a related but distinct reason - a discrete
hover-geometry correction, not a force. Both are excluded from the analysis
below and reported on their own rather than averaged in.

**On the 39 remaining ticks, no single term explains the residual.** The
best-fit scalar `s` minimising `sum |residual - s * term|^2` over the clean
ticks never accounts for more than a few percent of the residual's own energy
for any term - rolling resistance is the largest at 3.1%, drag next at 1.6%,
every other term under 1%. Scaling any one term up or down, by any amount,
does not close this gap. **This part of the result is solid regardless of
what follows**: it holds on raw per-tick residuals with no weighting or
projection in it.

**Bridged to a position-error yardstick, the tick-1583 event does not by
itself close the gap either - and this part of the measurement is the
weakest link in the pass.** A velocity residual injected at tick `t` has the
remaining time to the crest to become a position error, so its weight is the
*sum of `dt`* over the ticks between `t` and liftoff, not one tick's worth;
because the yardstick is a scalar along-track distance, the weighted vector is
then projected onto the recorded velocity direction one tick before the crest
(1594) rather than compared by raw magnitude, which would credit a lateral or
vertical component of the residual for a timing shift it cannot cause. An
eight-tick liftoff shift at the crest's own recorded speed (100.8 units/s) is
worth about **13.5 units** of along-track position error. Weighted and
projected the same way, the residual **over every tick, including 1583,
comes to -1.6 units along-track** (raw magnitude 7.1) - about 12% of the
yardstick, and the *wrong* sign for the direction a late liftoff needs.
Restricted to the 39 clean ticks alone it is **-0.25 units** (raw magnitude
0.25) - under 2%, same sign. So even crediting the tick-1583 event its full
weight, this window's residual does not close the eight-tick gap by this
arithmetic - which means either the along-track approximation is too crude
for a 35-tick window where the craft is actively cornering (one fixed axis
held over the whole window, not the craft's own tick-by-tick heading), or
part of the lag's cause sits outside ticks 1560-1600 altogether, or both.
This is the part of the pass to distrust most; re-deriving it with a
per-tick heading instead of one fixed axis is an obvious next refinement
before leaning on these two numbers for anything further.

So the crest-phase drift this thread opened with is not a force-law
calibration problem: every term in `forces::evaluate`, individually rescaled,
leaves the clean-tick residual almost untouched. What the measurement does
not establish is that the tick-1583 event is *sufficient* on its own to
produce the full observed lag - only that it is the dominant single
disagreement in the window, by a wide and unambiguous margin, and the one
event worth chasing before scaling any force term. The reading that this
generalises past this one circuit and this one event is out of scope for this
pass and unmeasured - the technique (pose walk the recorded poses, sum every
named term, assert the sum against the whole, fit each term's own scalar
against the residual) is reusable on any capture, but the finding itself is
this capture's own. The next step it points at -
`oag_physics::wall::resolve` on the recording's own poses around tick 1583,
the same pose-walk generalisation one step further, confirming or refuting
the wall reading against the actual collision geometry - is unmeasured.

### The cross-product signs: a contradiction here, resolved

This page originally recorded that term as `-400 * cross(up, avgNormal)` *and*, in
the same sentence, as one "which levels roll and yaw". **Those two statements
cannot both be true**, and the disagreement is settled by arithmetic rather than by
re-reading the prose, which is the lesson [`HANDOVER.md`](../../HANDOVER.md) draws
from every other disagreement this project has had.

Let `omega = up x n`. A torque along `omega` grows angular velocity along `omega`,
and the up axis then moves as

```text
d(up)/dt = omega x up = (up x n) x up
         = n * (up . up) - up * (n . up)      [ (a x b) x c = b(a.c) - a(b.c) ]
         = n - up * (n . up)                  [ up is a unit vector ]
```

which is the component of `n` perpendicular to `up`: it points **from `up` toward
`n`**. So `+k * cross(up, n)` aligns and `-k * cross(up, n)` diverges, and no
magnitude can change that - not the gain, not the inertia tensor, not the probe
offsets, not the target height. The prose's claim is about *behaviour*, which is
what a trace would show and is the stronger claim; a leading sign or an operand
order is a detail of how a decompiled expression was written down. Two readings of
the original produce the aligning term and both fit the prose: transposed operands,
`cross(avgNormal, up)`, or a transposed sign.

Simulation of the transcription corroborates it: with the literal `-400`, a ship
placed on a flat floor with a 0.02 rad roll grows to a full tumble in under a
second of simulated time at 60 Hz, which is the signature of anti-alignment rather
than of a mistuned gain.

**A second term has since turned out to have the same shape, and that changes the best
explanation.** [engine.md](../ghidra/functions/psp-pulse-usa/engine.md)'s weathervane torque
is `cross(forward, velocity) * (grounded ? -0.1 : -0.3)`, described two lines later as
"what turns the nose toward the direction of travel" - the identical pattern, a negative
coefficient on a cross product that the same identity says must be positive for the
stated behaviour. An anti-weathervaning term would spin every ship out of every corner.

Two independent transcription errors of one shape is a poor explanation where a
**systematic handedness difference** is a good one, and handedness is listed as not
determined on both pages. If that is what this is, both expressions are correct as
written *in the original's own frame*, nothing is a typo, and a reimplementation in a
right-handed frame is simply obliged to flip every cross product it inherits from this
path. That is a prediction: the next cross-product term recovered from the craft path
should need the same flip. Cheap to check, and it would settle the handedness question
as a side effect.

**Confidence 84** on the direction, which is the top of the *Probable* band and the
[rubric](../reverse-engineering/confidence-rubric.md)'s stated ceiling for
decompilation-only evidence. Deliberately not 85, even though the arithmetic admits
no third reading: neither leg of the 85-94 band applies. The decompilation as
recorded here is not unambiguous - it contradicts itself, which is the whole
problem - no call site was re-read, and there is no multi-file data invariant. What
puts it at the top of its band rather than lower is that the derivation is
compulsory and that simulation agrees. What would raise it: re-reading
`Ship_HoverTwoPoint` at `0x0884a658` to see which of the two candidate errors this
was, and then a trace.

**Only the signs are settled: the magnitudes `400`, `0.1` and `0.3` are unverified and
stay M3 work**, like everything else on this page. `oag-physics` implements the aligning
direction for both terms, carries the derivation at each constant, and pins each
direction with a test asserting it on the torque itself rather than through a simulation,
so that neither the integrator nor an invented inertia tensor can mask a regression.

### A numeric prerequisite for M3, found while implementing this - and since resolved

This section used to derive the stability limit wrong, in a way that made the problem
look nearly twice as mild as it is, and then asked M3 to settle three open questions with
a trace. A trace has since answered two of them, and the third turned out to need a
different kind of evidence than a trace at all. What follows is corrected; see
[oag_physics::hover::ALIGNMENT_GAIN](../../crates/physics/src/hover.rs) and
[docs/ghidra/functions/ps2-pulse-eu/craft-update.md](../ghidra/functions/ps2-pulse-eu/craft-update.md)
for the full evidence trail.

First, this **closes an earlier observation**, made while implementing from this page
alone, that there is no angular damping anywhere in the model. That was true of what this
page recorded and false of the game: `pitch_damping` is consumed by
`Ship_ApplyAngularDamping` rather than by the pitch term, and that function damps all
three axes at `(-pitch_damping, -5.0, -2.0)` times the body-local angular velocity. Only
the pitch axis is per ship; yaw and roll damping are the same for every craft in the game.
**Both binaries now confirm this at instruction level**: the PS2 build's
`Ship_ApplyAngularDamping` uses the identical triple, with roll damping at `-5.0` rather
than `-2.0` in what the PS2 code calls "mode 0" - so `c = 2` is specifically the
non-zero-mode value.

`Ship_ApplyAngularDamping` damps roll at a hard `-2.0` outside mode 0, so roll is a damped
oscillator with stiffness `k = 400` and damping `c = 2.0` - both now confirmed as literal
immediates in the PS2 binary too (`0xC3C80000` and the damping triple), not a PSP
transcription error. **The derivation this page previously gave, `h <= c/k` against the
sub-step `h = 1/180`, is wrong**, because it assumes the acceleration is recomputed every
sub-step. The integrator does not do that - `oag_physics::integrate` computes the
acceleration once from the frame's starting state and holds it across all three sub-steps,
and the PS2's own `Body_Integrate` does the identical thing at instruction level. That
changes the governing step from the sub-step to the **frame**, and the correct condition,
accounting for the real position/velocity update across `n` sub-steps of `h = H/n`, is

```text
c >= k * H * (n + 1) / (2n)
```

which at `k = 400`, `H = 1/60` and `n = 3` needs `c >= 4.44` against the actual `c = 2` -
a **2.22x** shortfall, not the 11 % this page previously computed. Raising the sub-step
count cannot fix it: the requirement floors at `k*H/2 = 3.33` as `n -> infinity`, still
above 2. A ship sitting still on flat ground with a small roll grows its oscillation and
tumbles within a few seconds under this reading, which `oag-physics` used to reproduce.

Two properties still make this a useful measurement:

- It is **independent of whether the angular accumulators hold torque or angular
  acceleration**, since `k` and `c` scale together - which mattered, because that question
  was open.
- It is now **confirmed in a second binary at instruction level**, closing the
  "is a magnitude just wrong" question definitively: it isn't. `-400` and `-2.0` are both
  exactly what the PS2 build uses too.

**What this page asked M3 to settle with a trace has been settled, and the answer is a
third thing neither "a transcription error" nor "the sub-step count is wrong" covers.** A
step response on the running PSP game - roll the ship `0.25 rad` via the debugger, trace
the recovery - measured the *closed-loop* stiffness at about **20.2**, not 400, while the
damping came out near the transcribed `2.0`. Both readings are correct at once because
something divides the `400` torque by about **20** before it reaches the body as angular
acceleration - most likely a roll moment of inertia this crate's accumulators bypassed
(`oag_physics::forces::drain` multiplied by the inertia tensor that
`oag_physics::integrate` then divided back out, so the term never actually saw it).

**That guess was right, and the fitted `ALIGNMENT_INERTIA = 19.8` it stood on is now
retired.** The accumulators hold torque, the round trip is gone, and the divisor is the
recovered tensor's own entries - `15.6` on roll and `21.6` on yaw, the two axes the term
acts on once its pitch component is projected out. The fit's `19.8` sits between them,
which is what a term spread across two axes should look like and is a check the fit could
not have arranged. On roll alone the recovered value is `400 / 15.6`, a natural frequency
of `5.06` rad/s against the original's measured `4.49`: **13 % stiff, left standing rather
than tuned away**, and pinned by
`hover::tests::the_roll_oscillator_is_stable_by_the_margin_that_was_measured`. See
`ALIGNMENT_GAIN` in `crates/physics/src/hover.rs` and
[angular-velocity-column.md](angular-velocity-column.md).

The remaining open question is not this stability arithmetic - it's the separate,
still-unresolved torque-sign anomaly (`Body_AddForceAtPoint` computes `F x r`, not the
physically-correct `r x F`, confirmed in both binaries' raw assembly and ruled out as an
operand-convention misreading against the vendor VU manual). Whatever compensates that is
not this section's concern, since the measured behaviour above is convergent (aligning),
not divergent - see `craft-update.md` for where that question currently stands.

### Alignment gain: the measurements behind the numbers

Moved here from the `ALIGNMENT_GAIN` doc comment in `crates/physics/src/hover.rs`.

**Stability arithmetic.** One frame of the scheme `integrate` runs (`a` fixed from the
frame's starting state, three sub-steps of `h = H/3`):

```text
theta'  = theta + H*theta_dot + (H^2/3)*a
dtheta' = theta_dot + H*a                       a = -k*theta - c*theta_dot
det = (1 - k*H^2/3)(1 - c*H) + k*H^2 - c*k*H^3/3
```

Stable when `c >= (2/3) * k * H`, i.e. `H <= 3c / (2k) = 0.0075 s`. At `k = 400`,
`c = 2`, `H = 1/60` the needed `c` is 4.444, so the frame is 2.22x too large,
`det = 1.040741`, and roll grows by `sqrt(det) = 1.020167` per tick. Measured in this
crate: a ship at rest on a flat floor with 0.01 rad of roll grew its peaks 1.018 to
1.022 per tick over 200 ticks, period 19 ticks against the predicted
`sqrt(400) = 20 rad/s`. The growth matches the undivided `k = 400` (a roll inertia of
about 3.1 would have given `k_eff = 129`), which confirms the torque reaches the body as
an angular acceleration.

**The original at rest** (Pulse in PPSSPP, Time Trial, 150 ticks, no input, via
`scripts/psp-trace.py`): `grounded` 1.0 on every tick, distance travelled 0.0087 units,
`pos_y` range 0.00019, up-axis deviation from its own mean at most 0.000476 rad
(0.027 deg), no oscillation. So the crate's growth is an artefact of the reading, not a
property of the game. The hover constant pool around `0x08ab0e00` holds
`TARGET_GLOBAL_SCALE`, `HOVER_K` and `BANK_TO_YAW_GAIN` (`0x08ab1098` reads `30.0`) but
no `400.0` and no `-2.0`/`-5.0` pair.

**Step response** (Pulse, Time Trial, breakpoint in `Ship_UpdateCraft`, basis rows
rewritten through `memory.write_u32` to roll the ship 0.25 rad about its forward axis;
the breakpoint fires about eight times a frame on a full grid, so distinct values are the
frames). Roll in rad, one value per frame:

```text
0.2500 0.2493 0.2466 0.2424 0.2367 0.2295 0.2210 0.2113 0.2004 0.1886
0.1758 0.1621 0.1476 0.1329 0.1175 0.1014 0.0855 0.0693 0.0531 0.0369
0.0210 0.0055 -0.0097 -0.0244 -0.0384 -0.0518 -0.0646 -0.0762 ...
```

A clean damped cosine crossing zero at frame 21: quarter period 21 frames, `T = 1.4 s`,
`omega = 4.49 rad/s`, stiffness `omega^2 ~= 20.2`, `zeta ~= 0.18` over 0.583 s,
damping `2*zeta*omega ~= 1.6` (against the transcribed 2.0, itself confirmed at `-2.0`
in the PS2 `Ship_ApplyAngularDamping`, `0x0015c1b0`). At `20.2` the oscillator runs at
4.49 rad/s with `det = 0.9704` and roll decays about 1.5 % a tick. Confidence **80**:
one ship, one track, hand-fitted from a quarter period and one overshoot.

**Sign.** With `omega = up x n`, `d(up)/dt = omega x up = n - up*(n . up)`, the component
of `n` perpendicular to `up`, so `+k * cross(up, n)` aligns and the page's literal
`-400` diverges. A simulation with the literal `-400` and a 0.02 rad roll tumbled within
a second. The magnitude is confirmed in the PS2 build, which materialises `0xC3C80000`
(`-400.0f`) in both `Ship_HoverFourCorner` (`0x0015a940`) and `Ship_HoverTwoPoint`
(`0x0015b978`) with the same sign and operand order. Confidence **84** on the direction.

## Airbrakes

Each side ramps toward its analog input at `gain` upward and `falloff` downward,
so `falloff` is a per-second decay rate rather than a drag coefficient. The states run
`0..=100`, not `0..=1`; see the [control range](#still-open) note.

```
slide  = |L - R| * drag * |steerX| * 0.01
world += forward * speed * slide * 0.001
world += right   * speed * amount * (R - L)
angAccelLocal.y += speed * turn * (R - L) * 0.001
```

- `amount` is a **lateral force gain**, not a drag.
- `turn` feeds body-local angular acceleration directly.
- **Both `(R - L)` terms are written in the original's frame and change sign on
  the way into this crate.** `right` above is row 0, which points *left*, and a
  positive `angAccelLocal.y` there turns the nose *right*. In a right-handed
  `(right, up, forward)` frame the pair becomes `(L - R)`, so braking one side
  turns the nose toward that side and pushes the body the other way. Transcribed
  literally, both terms run backwards; see
  [engine.md](../ghidra/functions/psp-pulse-usa/engine.md#ship_updateairbrakes-force-block-read-end-to-end)
  for the measurement (`airbrake-left-only`, 33/33 wall-free ticks) and
  `crates/physics/src/airbrake.rs` for where the negation lives.
- **Airbrakes produce no direct roll torque.** Confidence **85** on this negative,
  established by checking every write to the angular accumulators. Visible roll must come
  from the surface-alignment and bank-coupling terms, or from graphics-only state.
- **Narrowed.** This page used to add "no Z component is ever written", which
  [engine.md](../ghidra/functions/psp-pulse-usa/engine.md) showed was too strong. The precise
  claim is that **no control input writes an angular Z**: not the engine, the brakes, the
  steering, the airbrakes or the pitch axis, so roll is never *commanded*. Angular Z is
  written in exactly two places and both are passive - `Ship_ApplyAngularDamping` and the
  surface-alignment torque. Confidence **80** on the narrowed version, which is lower
  because `Ship_UpdateMagLock` was not scanned and a Z write there would have been missed.
- `sideshift` is a **world-space force** applied straight to the body, bypassing
  the craft accumulator, for the `0.2 s` its per-side timer runs and **only while
  the craft is in contact**. Read at instruction level in
  [engine.md](../ghidra/functions/psp-pulse-usa/engine.md#the-sideshift-is-a-force-and-its-direction-is-read-rather-than-guessed),
  which also settles the direction: a craft shifts toward the side it was
  flicked. **What arms the timer** is one of two gestures - a stick flick on the
  novice scheme, a double-tapped airbrake on the veteran one - with a `1.0 s`
  lockout common to both; see
  [input-bindings.md](../ghidra/functions/psp-pulse-usa/input-bindings.md).

Lateral grip:

```
k = max(L, R) * (0.01 - slidegrip) - 1.0
lateral += grip_ground * dot(vel, right) * k * grounded
lateral += grip_air    * dot(vel, right) * k * (1 - grounded)
```

With the load scaling, `slidegrip` reads as **percent of grip retained at full
airbrake**, 0 to 100: at 0 grip vanishes and the ship drifts freely, at 100 it is
unchanged. Confidence **86**, and now **90**, because
[engine.md](../ghidra/functions/psp-pulse-usa/engine.md) found the load-time factor that
makes the pseudocode above and this reading the same statement: the parser stores
`slidegrip * 1e-4`, so an XML 0..100 is 0..0.01 in memory, `(0.01 - slidegrip)` runs from
`0.01` down to exactly `0`, and the control states run 0..100 rather than 0..1.

**Now runtime-confirmed, and raised to 92.** Projecting a wall-free cornering
capture onto the ship's right axis fits the whole lateral law to `99.95 %` with
an rms of `3.28` units against a grip force reaching `375`, and returns
`grip_ground` at `0.9985 +/- 0.0005` of the disc's own value. The `k` factor is
confirmed by the data rather than only by arithmetic: dropping it triples the
residual. The law is **linear in `dot(v, right)` and independent of speed** -
alternatives scaling with `|v|` or with `|v_lat|` are 3.5x and 6x worse - and it
does not saturate out to a lateral speed of `37.5`. `grip_air` remains untested;
no capture has a usable airborne stretch. Evidence in
[cornering-ground-truth.md](cornering-ground-truth.md#the-lateral-axis-ship_applylateralgrip-is-exactly-right).

**Worth recording as a method note.** This crate first implemented the prose and
rejected the pseudocode as garbled, on the grounds that taken at face value it multiplied
grip by about a hundred. The pseudocode was correct and the *units* were missing. A
formula that cannot be right on the units in front of you is evidence about the units.

**"There is no dedicated airbrake drag term" - a fourth copy of a claim that is
false, and this is its correction.** `Ship_UpdateAirbrakes` applies
`forward * fs * |L - R| * Airbrake.drag * |steerX| * 1e-5` (conf 88, read from
both literals), `crates/physics/src/airbrake.rs` has implemented it since the
crate was written, and a wall-free cornering capture measures it at `1.03` to
`1.07` of the read value on 1,997 intervals where it reaches `31.5` units of
force. Three other copies of this sentence were corrected in an earlier pass and
this one was missed.

What the sentence should say is that the `drag` parameter **does not slow the
ship down**: every factor is non-negative and the term points along `+forward`,
so for a ship moving forwards it accelerates. Speed loss under braking is
indirect: lateral grip converting sideways motion into body-frame force, plus the
always-on quadratic drag and a constant `-2.0 * unit(v)` rolling resistance.

## Magstrips

Magstrip surfaces are **ordinary floor geometry with a tag** (surface type 3
rather than 1), not a distinct mechanism. See
[collision](../ghidra/functions/psp-pulse-usa/collision.md).

When a dedicated probe detects one, a 0-to-1 blend fades out the ordinary
suspension: spring, damping and lift are all scaled by `(1 - blend)` while a
separate magnetic-hold block takes over. Confidence **82**; the hold block's own
physics was not decoded.

## What is implemented

[`oag-physics`](../../crates/physics/src/lib.rs) transcribes this page and
[engine.md](../ghidra/functions/psp-pulse-usa/engine.md). It is structure only: no constant
was invented, nothing was tuned, and every test in that crate asserts a structural
invariant rather than a speed, a height or a turn rate. M3's
[verification protocol](../reverse-engineering/verification-protocol.md) comes before any
tuning, per the [roadmap](../overview/roadmap.md).

**Implemented, transcribed from the evidence:**

| Term | Where | From |
| --- | --- | --- |
| Clamped delta, forces evaluated once, three explicit Euler sub-steps of `dt/3` | [`integrate.rs`](../../crates/physics/src/integrate.rs) | this page |
| Two-probe air cushion: spring, damping multiplier, rebound and landing blend, magstrip blend consumption, target height from `ride_height` | [`hover.rs`](../../crates/physics/src/hover.rs) | both |
| Penetration escape at `h < 1.0`, surface alignment, bank-to-yaw, the downforce **and its magnitude**, the code-literal probe offsets and the target-length reach | [`hover.rs`](../../crates/physics/src/hover.rs) | this page, engine.md |
| Engine thrust and its cap, the brake and its ramp, steering and its asymmetric ramp, the pitch axis | [`engine.rs`](../../crates/physics/src/engine.rs) | engine.md |
| The five control states and their ramps, on the original's `0..=100` scale | [`controls.rs`](../../crates/physics/src/controls.rs) | engine.md |
| Quadratic drag with all four coefficients, rolling resistance, weathervane, angular damping, vertical damping, gravity | [`passive.rs`](../../crates/physics/src/passive.rs) | engine.md |
| Airbrake ramps, slide, lateral force, yaw, sideshift impulse, lateral grip | [`airbrake.rs`](../../crates/physics/src/airbrake.rs) | both |
| Four accumulators, the fifteen-step term order, and the stale/fresh groundedness split | [`forces.rs`](../../crates/physics/src/forces.rs) | engine.md |
| The speed-pad boost, step 15: the re-armed timer, the ramp-then-flat force law and its unscaled per-class tunables | [`engine.rs`](../../crates/physics/src/engine.rs) | engine.md, [pads](../ghidra/functions/psp-pulse-usa/pads.md) |
| The per-class gravity scale, `g_class_gravity_scale`, supplied from the disc rather than defaulted | [`passive.rs`](../../crates/physics/src/passive.rs) | engine.md, [handling stats](../formats/handling-stats.md) |
| Segment-triangle narrowphase as a plane sign change plus three edge half-space tests | [`collide.rs`](../../crates/physics/src/collide.rs) | [collision](../ghidra/functions/psp-pulse-usa/collision.md) |
| The magstrip hold: blend ramp, mag-floor probe, two-sample axis blend, reposition, velocity projection and the **kinematic basis rewrite** | [`maglock.rs`](../../crates/physics/src/maglock.rs) | engine.md |

**Implemented as a shape with the coefficient left at the identity or zero**, because the
evidence records that the term exists and not how large it is. That is deliberate: a
plausible-looking number would be indistinguishable from a recovered one later.

- ~~The engine's `craft+0x294` output multiplier (identity).~~ **Left the list 2026-10-01**: it is the
  launch boost, graded by when thrust first lands - [launch-boost.md](launch-boost.md).

Three names have left this list. `K2`, the hover target's global scale, was read at
`0.75`. The grounded **downforce** was read at `track_gravity * mass * grounded`,
and it was carrying more than its own weight: because the spring's damper is a
multiplier on the spring magnitude, the missing downforce was also the missing
pitch damping - see
[angular-velocity-column.md](angular-velocity-column.md#resolved-the-missing-49-is-the-hover-downforce).
The third is the per-class gravity scale, read out of
`<GlobalClass><GravityMul airborne/>` - and **the shipped table is not the
identity**, which two pages including that one had assumed. See the correction
below.

### Correction: the per-class gravity scale is not `1.0` for every class

This page used to record that "the shipped table reads `1.0` for all four, so the
identity is also the value", and
[angular-velocity-column.md](angular-velocity-column.md) repeats the same
parenthetical inside a derivation. **Both are wrong.** The table is filled from
`<GlobalClass><GravityMul airborne/>` in the engine-wide
`Data\XML\HandlingStats.xml`, the four classes carry four different values, and
the class the reference captures were taken in is **not** one of the ones holding
`1.0`. The values themselves stay off this page per
[ADR-0006](../architecture/adr/0006-no-copyrighted-content.md).

Three consequences, and the first is the interesting one:

- **Re-deriving `angular-velocity-column.md`'s rest compression with the real
  scale makes it agree *better*.** That page computes
  `z = (normal_gravity * classScale + track_gravity) / (...)` and checks it
  against a live probe read of `1.237` taken off the running original:

  | `classScale` | predicted `z` | error |
  | --- | ---: | ---: |
  | `1.0`, as that page assumed | `1.2500` | `1.05 %` |
  | the class's real scale | **`1.2390`** | **`0.16 %`** |

  The error falls by a factor of `6.6`, and the measurement did not move - only
  the prediction did.
- **That also picks the lane, independently of the disassembly.** `z` is a
  *resting* compression, so it contains `normal_gravity` and `track_gravity` and
  not `flight_gravity`. Had the scale gone on the airborne term - which is what
  the attribute's name suggests - `z` would have stayed at `1.2500` and the
  `1.05 %` gap with it. A capture taken long before this table was decoded
  therefore agrees with the `vmul.p` chain about which term the number reaches.
- **The measured effect on a race is small.** A headless `--race --hold cross` run
  on `01_Track` in the class the captures use moves the reported height above the
  spline by `0.01` units and leaves speed unchanged to two decimal places at every
  logged tick. So this was never a regression hiding in the fits; it is a term
  that was being applied at slightly the wrong strength.

**Deliberately not implemented, and why:**

- **Everything flag-gated on the undecoded `craft+0x1c0`**: turbo's boost lift, the
  uncapped-thrust mode, the `craft+0x2a0` engine multiplier, the engine kill switch, and
  the steering bias at `craft+0x2e4`. Implementing them means inventing their triggers.
  The `craft+0x31c` one-shot scale used to be in this list and is not flag-gated at all:
  it is the LeachBeam's `slowShipFactor`, armed by `Ship_ApplyPendingWeaponDamage` and
  spent by `oag_physics::engine` through `Environment::thrust_scale` since 2026-09-16
  (see [cannon-quake-leachbeam.md](../ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md)).
- **The `craft+0x2a4` mode enum**, which selects a `-0.9` drag coefficient, a `-5.0` roll
  damping and a disabled `rebound` in mode 0, and skips the control block entirely in 4,
  5, 6 and 8. Naming it is a guess at confidence 40, so the racing values are used.
- **The four-corner hover variant.** The selector is known (`Ship_UpdateHover`, every frame:
  `g_game_mode == 6`, confidence 95) and the epilogue's bank-to-yaw gain `50.0` is ported
  (`ShipState::four_corner`, [zone-rest.md](../ghidra/functions/psp-pulse-usa/zone-rest.md),
  confidence 90). Still unported: the four-probe layout, and the downforce without a
  groundedness or magstrip factor.
- **One branch of the speed-pad boost** (`Ship_ApplySpeedupPad`, `0x08848f9c`), down from
  two. `if (craft+0x2cc < 1.0) f *= craft+0x2cc` fades the boost in over a second after
  flag `0x200` clears, and `0x200` is one of the eleven undecoded bits of `craft+0x1c0`,
  so it is left out for the same reason as the entries above. **`craft+0x2cc` is not the
  contact ratio**, which an earlier plan assumed: `Ship_UpdateEngine`'s prologue
  accumulates `dt` into it, so it is unbounded and cannot be a `0..1` groundedness.
  The other arm, `if (controls->0x24 & 1) dir += craft+0x160 * speedpad_jump`, **is now
  implemented**: both of its unknowns were closed live - `craft+0x160` is the hull up axis
  and the gate is d-pad Up - so it is no longer a guess.
- **The per-team in-air pitch bias** at `stats_base + 0x90`, which is outside every class
  block and whose XML element is unknown, so there is no field to read it from.
- **The gate on pitch input** (`FUN_088492bc`), not decoded, so pitch applies
  unconditionally.
- **The sweep-and-prune broadphase.** A linear scan over triangles behind an AABB reject
  is correct and is not the bottleneck yet. Worth recording before anyone implements it:
  the original's packing quantises coordinates to 1 unit over roughly +/-1024 and caps ids
  at 1024 per list, a hard limit inherited from the data rather than a design choice.
- **The in-air roll-levelling branch**, which is [dead code](#a-dead-branch-not-to-port).
- **The engine throttle ramp**, which is dead in the original: `<Engine gain/>` and
  `<Engine falloff/>` are parsed and overwritten by the raw input before anything reads
  them, verified in disassembly at confidence 85. Both parameters are carried and unused.
  Reintroducing the ramp is the obvious mistake, and the throttle lag it adds is exactly
  the kind of difference that reads as "feels close enough".

## Still open

- **Nothing is runtime-verified.** All of the above is static analysis.
- **The parameter scaling boundary.** Four fields are pre-scaled at load
  (`Engine.amount` x0.001, `Brakes.amount` x-0.01, `Airbrake.amount` x1e-4,
  `Airbrake.slidegrip` x1e-4). `oag_tables::handling` returns the document's raw values,
  `oag_physics::Handling` holds the scaled in-memory form, and `oag-gameplay` applies the
  factors between them. **Applying them twice is the most likely integration bug here**
  and it would be silent: the ship would simply be sluggish.
- **The control range - settled for the steering axis.** Three separate pieces of
  arithmetic only come out right if the control states run `0..=100` rather than `0..=1`:
  the brake ramp's `100.0` clamp, the engine's `throttle > 100.0` test, and the grip
  coefficient reaching exactly zero at full airbrake. That the *analog axes* share that
  scale was an inference; for `stick_x` it is now **measured**. Both captures in
  `data/traces/` show `craft+0x2c0` ramping and falling at rates that reproduce the
  shipped `Turning.gain`/`Turning.falloff` to under 1 %, which only works on the
  `0..=100` reading. `stick_y` stays an inference - the craft control block holds no
  second axis to capture.

  **The old warning attached to this entry - that an unscaled `Turning.amount` would put
  yaw out by whatever the axis scale was wrong by - is falsified.** The axis scale is
  right and `Turning.amount` really is stored verbatim, yet yaw still comes out about
  **22x** too strong. Dividing by the control range "fixes" it to 4.7x too weak. See
  [engine.md](../ghidra/functions/psp-pulse-usa/engine.md#steering) for the instruction-level
  reading and `oag_physics::forces::YAW_INVERSE_INERTIA` for what closes it -
  **the yaw entry of the inverse inertia tensor**, recovered from
  `Body_SetBoxInertia` (a diagonal fixed in *world* axes, corrected 2026-09-10 -
  on a level craft the world and body readings of a `diag(a, b, a)` tensor are
  identical, see
  [cornering-ground-truth.md](cornering-ground-truth.md#the-tensor-is-a-world-axis-diagonal-and-that-answers-a-not-c)),
  applied to the whole yaw axis because the
  bank-to-yaw coupling recorded above shares the same accumulator and therefore
  the same factor.
- **Whether the two angular accumulators hold torque or angular acceleration.** Nothing in
  any term visibly divides by an inertia, and the angular damping term's shape is
  dimensionally an acceleration, so `oag-physics` treats both as acceleration. Under that
  reading the inertia tensor has no effect on any angular term at all.

  **For magnitude this no longer matters, and that is now proven rather than assumed.**
  `Ship_UpdateSteering` and `Ship_ApplyAngularDamping` write to the *same* accumulator
  (`craft+0x340`), which `Ship_UpdateCraft` zeroes every frame and `Body_AddTorqueLocal`
  forwards unscaled, so any common factor - the inertia tensor included - cancels out of
  the resulting equilibrium whichever quantity the accumulator turns out to hold.
- **The alignment gain and the roll damping are marginally unstable together**, by 11 % at
  the specified sub-step; see
  [the arithmetic above](#a-numeric-prerequisite-for-m3-found-while-implementing-this---and-since-resolved).
- **Nothing in the *force law* can hold an inverted ship, and nothing was ever going to.**
  Gravity writes world `.y` only, `track_gravity` reaches the force law solely through the
  hover spring's *magnitude*, and the hover spring always pushes the ship *away* from the
  surface it found. This used to end "so the magnetic hold must be what holds a ship to a
  ceiling, and it is exactly the block that was not decoded". That block is read now, and
  the reason no force could be found is that **there is no force**: `Ship_UpdateMagLock`
  writes the body's position, velocity and basis directly and never touches an
  accumulator. See [`maglock.rs`](../../crates/physics/src/maglock.rs) and
  [engine.md](../ghidra/functions/psp-pulse-usa/engine.md#ship_updatemaglock-rewrites-the-basis-directly-and-that-is-the-missing-mechanism).
- **The hover target's additive offset**, `craft+0x74`. Nothing was found that writes it,
  and `antigrav_height_adjust` is only a suspect (a search for readers of that field found
  none, a weak negative at confidence 50). It matters more than it looks: with the offset
  at zero the target equals the raycast length, so a ship sits only its own spring sag
  below the top of its probe range and a small pitch loses contact. A negative offset
  would give the probes headroom.
- **Whether the airbrake `drag` term accelerates or decelerates.** Every factor in
  `world += forward * speed * slide * 0.001` is non-negative, so the literal reading
  accelerates while the parameter's name argues the other way. Implemented literally,
  flagged in code as a guess.
- **The quadratic drag's direction while reversing.** `velocity * forwardSpeed * k`
  delivers power `|v|^2 * forwardSpeed * k`; with `k` always negative that is dissipative
  exactly while `forwardSpeed > 0` and *adds energy* while reversing. Unlike the
  cross-product signs, no prose on either page contradicts the expression - the "20 to 50
  times more drag" note is about the coefficient's magnitude - so it is transcribed
  literally, and no test in `oag-physics` lets a ship reverse.
- **The rebound blend inside the landing window.**
  `0.5 * (rebound * t + landing_rebound * (1 - 5*t))` is discontinuous at `t = 0.2`, where
  it evaluates to `0.1 * rebound` against the other arm's `rebound`, and it multiplies a
  coefficient by a time. `1 - 5*t` reaching zero exactly at the window's end is the one
  part that reads as intended, so a factor was probably lost when this was written down.
  Transcribed as written.
- **The probe point velocity.** This page writes
  `bodyLinearVel + M * cross(probeLocal, angularVel)`, the negation of the conventional
  rigid-body point velocity. The crate uses the conventional `omega x r`, since that is
  what damps a descending nose rather than pumping it.
- ~~**Where the two probes sit.**~~ - **closed**: a code literal,
  `(0, -1.125, +/-4.5)` after the `0.75` global, identical for every craft, and
  landed. `<Misc>` reaches the collider, not the probes.
- ~~**Whether `sideshift` is an impulse or a velocity**~~ - **closed**: it is
  neither, it is a force for `0.2 s`, so mass does divide it. ~~What is still
  open is the *trigger*~~ - **also closed, 2026-08-04**: the original has two
  trigger gestures, one per control scheme, and both are ported. The
  tap-history path above them was never a sideshift trigger at all; it is the
  barrel roll. See
  [input-bindings.md](../ghidra/functions/psp-pulse-usa/input-bindings.md). What is
  still open here is only a *measurement*: no capture of either gesture off the
  original exists, so the timings are static analysis with no runtime leg.
- **Which speed the airbrake terms use.** This page writes only "speed"; the craft caches
  `|dot(velocity, forward)|` and the engine reads it from there, so that is what the crate
  uses, rather than `|velocity|` as the brake does.
- **Whether the brake force is horizontal-only.** A VFPU target prefix reads, under the
  standard selector, as zeroing the world `y` lane. Confidence 55, the encoding was not
  confirmed, and it is low-stakes because braking only runs while grounded. Not
  implemented.
- **The pitch axis's sign convention**, and coordinate conventions generally: handedness,
  units and angle representation are open on both pages. Triangle winding in the collision
  data is unknown too, and the crate assumes a normal that points out of the surface.
- **The XML `mass` at `+0xf4` versus the body mass at `body+0x374`.** Every force term in
  engine.md reads the latter; the hover spring on this page reads "mass". The crate reads
  the parameter set for the spring and the body for gravity, and keeping the two equal is
  the integration layer's job.
- The magnetic-hold force law.
- What the `craft+0x2a4` mode enum is, and the eleven flag bits at `craft+0x1c0` other than
  bit 0.
- **What arms the `craft+0x2e0` timer**, which lowers the hover target and suppresses
  lateral grip while it runs. A leap, a respawn and a race start are all plausible.

- **A craft already under a floor face falls through in ours and is recovered by the
  original.** Read 2026-09-29: the hull narrowphase makes contacts against floors,
  `Collision_AddContact` keeps one only if the sample projects into the crossed
  triangle, and `Body_StepWorld`'s pass 1 clips the body back along its velocity.
  All three are read and measured and **none is merged**: the first alone strands a
  craft, the player included, on its flank below the holes in `01_Track`'s and
  `06_Track`'s racing lines. See
  [leaving the track](../gameplay/leaving-the-track.md#the-one-divergence-the-original-recovers-a-craft-that-has-sunk-into-the-floor-and-ours-does-not).

## A dead branch not to port

An in-air roll-levelling term is computed into a VFPU register and **never
stored**; the next function reloads that register immediately. It is dead code,
most likely a compiler artefact.

Recorded here because it is exactly the kind of thing a careful reimplementation
would faithfully copy by mistake.

## Approach

Physics is where "it feels right" is most tempting and least trustworthy. Every
part of this system gets verified against a trace from the original before it is
considered done. See the
[verification protocol](../reverse-engineering/verification-protocol.md), whose
early scenarios exist specifically to isolate the systems on this page.
