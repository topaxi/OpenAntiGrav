# ADR-0084: The resolution target is held to what the display shows

## Status

**Accepted** (2026-11-04). Replaces the premise "this build cannot ask a surface what
its refresh is" in [ADR-0044](0044-the-residual-is-a-learned-upper-bound.md) (its
`RESIDUAL_SHARE` consequence) and in `docs/rendering/dynamic-resolution.md`, "The target
is held to the frame limiter". ADR-0044 is not edited.

## Context

`drs::Target::at_most` held the controller's target to our own frame limiter and
stopped at `Vsync::On`, because nothing could say what the output refreshes at and a
wrong clamp (60 on a 144 Hz panel) is worse than none. A Steam Deck docked to a 60 Hz TV
under gamescope runs `target_fps = 90` with no limiter: gamescope holds the output to 60,
the budget is a third too tight, and the controller sheds resolution for a rate the
output forbids. The CPU governor's budget (`Sample::period`) had the same blind spot.

## Decision

1. The monitor's report (`winit` `MonitorHandle::refresh_rate_millihertz`, asked again
   about once a second so a moved window follows) is believed everywhere except under
   gamescope, whose monitor is a virtual one.
2. A measured rate (`oag_present::refresh`) outranks the report in both directions. An
   interval counts as evidence only when the frame spent at least half of it in the
   swapchain **and** the GPU's timed cost was at most half of it; a window of 90 such
   intervals within 4 % spread gives the rate. A slow GPU therefore never reads as a slow
   display, jitter (variable refresh) and no evidence both give no answer, and no answer
   means no clamp.
3. The target is clamped with `at_most(limiter).at_most(display)`; the governor's period
   is the lower of the two. `Session::presentation_hz` takes the display rate before the
   simulation-rate fallback.

## Consequences

- A 144 Hz panel keeps 144; a 60 Hz output holds a 90 target to 60.
- The measurement needs a GPU timestamp reading; an adapter without one measures
  nothing and relies on the report alone.
- The settings menu's `warn_when` reads settings only and cannot see a runtime refresh,
  so the row does not warn about the display pairing; the clamp is logged at `debug`.
