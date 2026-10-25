# ADR-0075: Dynamic resolution sheds the blur gather before pixels

## Status

**Proposed (2026-10-25)**, for the maintainer to accept or change. Adds a second output
to the controller of ADR-0040, ADR-0042, ADR-0044 and ADR-0073; all four stand.

## Context

`oag_present::drs::Controller` emitted one thing, a render rectangle. The motion blur
already has a cheaper mode, `motion_blur_resolution = "half"`, which gathers a quarter of
the pixels and blends the smear back at full size; the renderer switches it per frame
with no reallocation (`Scene::set_blur_resolution`, the half-size target is allocated on
every resize regardless).

Measured (`docs/rendering/motion-blur-half-rung.md`): the half gather saves 45-52 % of the
blur chain, about 2.4 ms on a Raphael iGPU at 1280x800 and 8.5-9.1 ms on a Steam Deck at
3840x2160, Pulse and HD. The maintainer compared, on one hard-turn frame per title at
3840x2160 with FSR 3 and blur `high`, full gather at 100 % render scale, full gather at
75 % and half gather at 100 %, and judged **75 % scale worse than half blur at 100 %**:
the lower scale softens the hull and its decals, the half gather only coarsens a
background the blur has already smeared. A single still per title, so strong but not
proven in motion.

## Decision

1. **The controller has a second, discrete output**, `Controller::blur_resolution`, `full`
   or `half`, re-derived from the limits every frame like `Controller::extent`.
2. **Shed first, restore last.** An over-budget reading that the extent would have fallen
   on sheds the full gather instead, while the rung can shed (blur on, the player's choice
   `full`, a target running, the blur measured to cost something). The extent loop's own
   cooldown follows. The full gather comes back only with the extent at its ceiling.
3. **A restore is predicted, and must hold.** The restored cost is predicted as the cost
   now, minus the half gather measured now, plus the full gather that half gather stands
   for: the half gather over the saving ratio read just after the shed (half on the first
   reading past its cooldown, over full on the shedding frame). It is taken only when that
   prediction is at or under `DEADBAND.start()` for `RESTORE_SECONDS` (3 s) of the target's
   frames running. Landing at or under 0.80 of the budget cannot shed again, which needs
   more than 0.95: the margin the extent's rise refusal already takes.
4. **A restore that does not hold backs off.** A shed within the relapse window (the
   longest backed-off patience) of a restore doubles the next restore's patience, up to
   three doublings; each window that holds pays one back.
5. **The player's `motion_blur_resolution` is the ceiling**, carried in `drs::Limits` as
   `Option<BlurResolution>` (`None` with the blur off). At `half`, with the blur off or
   with no target, the rung is inert, and going ineligible forgets everything it learned.
   It never writes a setting: the module does not import settings, the invariant
   `drs.rs` states for `render_scale`.
6. **Only the blur gets a rung.** HD's bloom already scales with the extent (ADR-0043) at
   about 0.36 ms; MSAA and the shadow map would recreate targets.

`Cost` gains `blur`, the share of `scalable` that was the blur chain; a further window's
cost sums it like the rest.

### Why the shed has no patience of its own

Patience slower than the extent loop in both directions was asked for when this was
designed. A
shed that waited would either let the extent fall first, which inverts the order this
exists for, or hold the extent over budget and drop frames. The patience lives on the
restore side, where a wrong decision is a dropped frame; a single-frame shed costs a
little blur detail until a spike-free stretch earns it back.

### Why the restore patience is a duration

First wired with 32 frames of patience and a 300-frame relapse window, a Raphael iGPU at
1280x800 aiming at 90 flipped 21 times in 90 seconds: each restore was predicted to fit
for its 32 frames, the scene's next spike (1 to 5 s later) shed it again, and outside the
window the backoff never grew. A run that must predict a fit on every frame is a maximum
over its stretch, so it has to be longer than the gap between spikes. With three seconds
of frames the same run flipped 3 times.

## Consequences

- A machine that cannot hold its target with the full gather keeps more pixels: on the
  iGPU at 3840x2160 aiming at 30, mean pixels per frame went from 1.73 to 2.71 MP at the
  same median frame rate.
- On a large overrun the first decision is now the shed, so the extent falls one
  `COOLDOWN` (four readings) later than it did: about 133 ms at a 30 fps target, 67 ms at
  60. The shed itself buys back half the blur chain in that time.
- `Controller::record`'s return stays the extent; a flip is read off
  `Controller::blur_shed` before and after, and the session logs each one.
- The thresholds are one set for every blur strength. The maintainer reports half is more
  visible at `high` than at `low` in some scenes; a per-strength threshold would be a
  strength in `Limits` and a table in `drs::blur`, not built.
- The capture and headless paths run no controller and draw the profile's own
  resolution, as they draw the allocation's extent.
- A missing blur reading is folded to zero by `session::timing::assemble`, which only
  stops a shed and makes a restore prediction pessimistic; it still makes the blur look
  free to the extent loop, which is open.
