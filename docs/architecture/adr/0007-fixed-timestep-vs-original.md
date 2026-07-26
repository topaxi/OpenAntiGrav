# ADR-0007: Keep a fixed timestep, and diverge from the original

## Status

Accepted. Refines [ADR-0002](0002-determinism-model.md), which it does not
supersede: the determinism model is unchanged.

## Context

[ADR-0002](0002-determinism-model.md) chose a fixed timestep on general
grounds, before anything was known about the original's timing.

The original has now been read. It does **not** use a fixed timestep. See
[frame pacing](../../psp/frame-pacing.md) for the evidence. In summary:

- Delta time is measured per frame from `sceKernelGetSystemTimeWide` and passed
  straight into the update, clamped to `[0, 0.067]` seconds.
- Presentation waits one vblank while racing (~59.94 Hz) and two in menus
  (~29.97 Hz), with no catch-up: an overrunning frame simply produces a larger
  next delta.
- Fixed-step code paths for 1/30 and 1/60 exist but are behind globals that are
  never written. They are a video-capture mode.
- Fourteen sites nevertheless sub-step at exactly 1/60 with
  `n = (int)(dt / 0.016666668f)`, discarding the remainder. Every
  exponential-smoothing coefficient in the engine is tuned for a 1/60 s step.

So the original's behaviour depends on its frame rate, and a faithful
reimplementation would have to reproduce that dependence.

## Decision

**Keep the fixed timestep at 60 Hz.** Deliberately diverge from the original on
this point, and document the divergence rather than hiding it.

Three specific rules follow:

1. **Reproduce the 1/60 sub-stepping idiom where the original uses it**,
   including the discarded remainder. That is observable behaviour, and with a
   fixed 1/60 step it degenerates to exactly one sub-step per tick, which is the
   original's own behaviour at its target frame rate.
2. **Do not reproduce raw-`dt` integration.** Where the original integrates
   measured delta directly, we integrate a fixed 1/60.
3. **The verification harness must account for this.** Comparing against a trace
   is only valid over intervals where the original held its target frame rate.
   Traces must record the original's per-frame delta so intervals that drifted
   can be excluded from comparison, or replayed through our sim as a variable
   delta when a like-for-like check is wanted.

## Alternatives considered

**Reproduce the variable timestep exactly.** Maximum fidelity, and the only way
to match the original frame for frame under load. Rejected: it forfeits
determinism, and with it replays-as-inputs, golden tests and the entire
verification approach. It would also make our behaviour depend on *our* frame
rate, which has nothing to do with the PSP's, so the "fidelity" is illusory
except on hardware we are not running on.

**Fixed timestep at 59.94 Hz**, matching the PSP's actual LCD. Tempting for
accuracy: over a five-minute race, 60 versus 59.94 is about 18 frames of drift.
Rejected because the *engine* is authored against 1/60 exactly, as the fourteen
sub-stepping sites show, and a non-integer rate complicates the tick clock for
an error smaller than the one we are already accepting by fixing the step at
all. Revisit if lap-time comparison shows systematic drift.

**Fixed timestep with an accumulator that carries the remainder.** The textbook
approach, and what `TickClock` already does at the outer level. Rejected *inside*
the sub-stepping sites specifically, because the original discards the
remainder there and carrying it would be a real behavioural difference in the
smoothing.

**Offer both, switchable.** Rejected as the worst option: two code paths, one of
which is untested at any given time, and every bug report ambiguous about which
was in use.

## Consequences

**Good.** Determinism survives, so replays, golden tests and cross-platform
comparison all still work. Our simulation is frame-rate independent, which is
strictly better behaviour for a modern port and is what unlocked frame rates
require anyway. The 60 Hz choice is now evidence-backed rather than assumed.

**Bad.** We will not match the original frame-for-frame when the original drops
frames, and the divergence is cumulative rather than self-correcting: once the
positions differ, they keep differing. Any trace captured from real hardware or
an emulator under load contains intervals we simply cannot match, and telling
those apart from genuine bugs needs the per-frame delta recorded alongside.

There is also a subtler cost. Some of the original's *feel* may come from its
frame-rate dependence, for instance handling that changes character when the
frame rate dips in a crowded scene. If so, we will not reproduce it, and players
who know the original well may notice. That is a trade we are making knowingly.

**Open.** Whether the craft physics integrator sub-steps at 1/60 or integrates
raw delta is not yet known, and it decides how large this divergence actually
is. If it sub-steps, the divergence is confined to frame-rate dips. If it
integrates raw delta, ship handling in the original is frame-rate dependent
throughout, and this ADR's consequences are correspondingly larger. Resolving it
is an M4 prerequisite; see [frame pacing](../../psp/frame-pacing.md).
