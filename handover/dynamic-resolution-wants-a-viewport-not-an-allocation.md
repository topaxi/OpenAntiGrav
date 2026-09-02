# Dynamic resolution wants a viewport, not an allocation

**All five phases landed 2026-09-02.** Dynamic resolution works and is off by
default: `[render_profiles.<title>] target_fps` names a target rate,
`minimum_resolution` bounds the fall, `render_scale` is the ceiling - all
three per title, beside each other. The permanent record is
[dynamic-resolution.md](../docs/rendering/dynamic-resolution.md),
[ADR-0037](../docs/architecture/adr/0037-dynamic-resolution-varies-a-viewport-not-an-allocation.md)
and
[ADR-0040](../docs/architecture/adr/0040-the-dynamic-resolution-budget-is-a-share-of-a-frame.md).

**This file survives its own work, which is unusual here.** The phase-by-phase
reasoning it used to carry is on the doc page and in the commit messages; what
is left is the questions the work did not answer, one of which wants hardware
nobody here has. Delete it and its `HANDOVER.md` index line together when those
are closed.

## Traps that still bind

- **Every capture path stays outside the controller.** `race/capture.rs` builds
  its own `Framebuffer` at its own `--render-scale`, never calls `set_extent`
  and never constructs a `drs::Controller`; the front-end capture has no
  `Framebuffer` at all. That is what keeps `--presented` byte-identity usable as
  this project's instrument, and it is a property to preserve rather than a gap
  to wire up. `perf.rs` already makes the same argument for why the overlay is
  window-only.
- **A capture taken with dynamic resolution on is not evidence of anything**,
  for the reason the FSR 1 thread records about `--upscaler`: the setting
  changes the `dev` overlay's own `RENDER` line, so two images differ for an
  unrelated reason.
- **The no-timestamp path cannot be exercised on this machine.** Both adapters
  here have `TIMESTAMP_QUERY`, so a green run is not evidence that a device
  without one degrades correctly - it should simply never move the extent, and
  the menu row cannot grey itself for it.

## Open

- **~~Whether the floor should be per-title.~~ Answered by the merge, not by a
  measurement.** `render_profiles` landed on main in parallel and moved every
  render-cost setting per title, so the floor and the target went in beside the
  ceiling they pair with. What is *still* open is the reading that would say
  what each title's default should be - HD/Fury's scenes and Pulse's are not
  the same cost, and all four titles currently default the same.
- **Whether the ceiling allocation's memory is acceptable on the Steam Deck**,
  which [goals.md](../docs/overview/goals.md) names in the first tier. It is
  the one number ADR-0037 states as unmeasured, and the answer could argue for
  a per-title *ceiling* rather than a per-title floor.
- **`SCENE_SHARE` is 0.45 and is a choice, not a reading.** The budget is the
  `race` pass alone. `bloom`, `hd_bloom` and `motion_blur` draw at scene
  resolution and belong in it, and they take a viewport now - so the reason
  they are out is no longer structural, it is three more timestamp claims a
  frame. Timing them is what shrinks the guess, and is worth more than tuning
  the constant.
- **FSR 3.1 will want a moving jitter phase count.** `jitter::PHASES` is a
  fixed sixteen, deliberately: deriving it from the presentation-to-render
  ratio is the FSR 3.1 port's decision, not this thread's. A moving scale means
  a moving phase count when that lands, and `oag_render::jitter::PHASES` is the
  one place it changes.
- **The scale still steps between two adjacent grid points on a marginal
  load.** `RISE_PATIENCE` took a 4K/144 race from 143 resolution changes a
  minute to 49; the remainder is a closed loop tracking a target the machine
  can only just hold, and the honest answer there is a lower target rather than
  a cleverer policy. Whether 49 a minute is visible is a from-play question
  nobody has answered.

## Next Steps

1. Time `bloom`, `hd_bloom` and `motion_blur` into the budget - the cheapest
   way to make `SCENE_SHARE` less of a guess.
2. Take the Steam Deck memory reading, or say out loud that nobody will.
3. Ask a player whether the stepping is visible at 49 changes a minute, before
   tuning `RISE_PATIENCE` on a hunch.
