# ADR-0040: The dynamic-resolution budget is a share of a frame, not a measured frame

## Status

Accepted. Completes the thread
[ADR-0036](0036-ui-composites-at-presentation-resolution.md),
[ADR-0037](0037-dynamic-resolution-varies-a-viewport-not-an-allocation.md) and
[ADR-0038](0038-a-stage-with-no-scene-draws-at-presentation-resolution.md)
opened: those three made a moving render extent *possible*, and this is the one
open question about what should move it.

## Context

Dynamic resolution is a closed loop. The measurement is
`oag_render::timing::PassTimer`, which times the `race` pass on the GPU every
frame and hands back a reading that names the frame it was taken on. What the
loop needs beside that is a **budget**: the number the measurement is divided
by to say whether this frame had room.

There are two obvious places to get one, and this project already has both
numbers in hand.

**The frame interval.** `perf::Meter` is fed the wall-clock duration between
loop iterations. A controller could take `interval - scene` as headroom and aim
to keep it positive, which reads as the more honest of the two: it is a
measurement of the actual machine rather than a constant somebody chose.

**A share of the target period.** The player names a target rate; a fixed
fraction of `1 / rate` is what the scene pass may take; everything else in the
frame gets the rest.

The measurement itself is also an under-measurement, recorded rather than
hidden: the budget covers the `race` pass alone. `bloom`, `hd_bloom` and
`motion_blur` draw at scene resolution and belong in it, and the resolve, the
UI composite and the performance overlay permanently do not, because folding a
fixed cost into a budget that exists to be divided by a moving one is exactly
the error the allocation/extent split guards against.

## Decision

**The budget is `drs::SCENE_SHARE` of the target frame period.** The frame
interval is not an input to the controller, and there is nowhere in
`drs::Controller::record` to put one.

`SCENE_SHARE` is a named constant, documented as a choice rather than a
reading, and it rises when the other scene-resolution passes join the budget.

## Why the frame interval is not merely worse but degenerate

**Under `Vsync::On` the loop sleeps to the refresh, and under any `FrameLimit`
it sleeps to the limit.** `perf::Meter`'s own documentation already says so:
the interval "tracks real work only with vsync off *and* no limit - a
diagnostic configuration rather than a shipping one".

So in any paced configuration, `interval` is the target and `interval - scene`
is whatever the scene did not use. Headroom becomes identically equal to
`target - scene`, the ratio `scene / headroom` is a fixed function of `scene`
that no longer references the machine at all, and - this is the part that
matters - **the slack absorbs every change the controller makes.** Halve the
resolution and the scene costs less; the loop sleeps longer; the interval does
not move. The controller has no gradient to descend and sits inert at whatever
scale it started on.

The configuration this fails in is not an edge case. It is vsync on, or a frame
limit set - which is the default, `FrameLimit::DEFAULT` being 240 - and it is
the configuration a player who wants dynamic resolution is most likely to be
in.

There is a second, structural reason. `drs.rs`'s whole claim is that it is a
pure function of fed measurements, the way `perf::Meter` is: it never reads a
clock, which is what makes every constant in it testable against a sequence
somebody chose rather than against whatever the machine happened to do. A frame
interval is a clock reading. Taking one would move the module from "testable
headlessly" to "testable only against a real frame loop", and
`docs/architecture/determinism.md`'s argument for why this module is not a
determinism problem is built on the first of those.

## Consequences

- **`SCENE_SHARE` is a number somebody has to own.** It is 0.45, and 0.45 is
  not measured: 1.556 ms of a 16.67 ms frame at 100 % on the development
  machine is 9 %, but that is one adapter on one track, and a controller aiming
  at 9 % would chase a resolution the rest of the frame cannot afford. It is
  the cost of not measuring the rest of the frame, and the way to shrink that
  cost is to time more passes rather than to tune the constant.
- **The controller cannot be wrong about the machine, only about the share.**
  It holds a scene-pass cost, exactly, at whatever share it was given. If a
  player's frames still drop, the diagnosis is that the untimed part of the
  frame is larger than 55 % - which is a measurable claim with a known next
  step, unlike "the headroom calculation is off".
- **A target above the refresh is meaningful and is offered.** The simulation
  is fixed 60 Hz with unlocked presentation, so aiming at 144 on a 144 Hz panel
  is a real request. It would have been meaningless under the interval
  formulation, where the interval is pinned to the panel.
- **The test that pins this is
  `a_loop_paced_at_the_target_still_reaches_the_ceiling`**, and it is named for
  the failure rather than the behaviour. It feeds a constant under-budget scene
  cost and nothing else at all, and asserts the scale climbs - which is exactly
  what the rejected formulation could not do.
- **This says nothing about how the correction is computed.** The
  `sqrt(budget / measured)` shape, the deadband, the grid and the patience are
  policy inside `drs::policy` and may be tuned without reopening this; see
  [dynamic-resolution.md](../../rendering/dynamic-resolution.md).
