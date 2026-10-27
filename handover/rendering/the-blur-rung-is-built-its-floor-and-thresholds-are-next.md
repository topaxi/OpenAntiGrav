---
status: ready-to-wire
titles: [all]
updated: 2026-10-25
summary: "Drop a full-size pass from the half-resolution blur path and measure on the Deck at 4K; DRS thresholds next"
difficulty: hard (30)
---

# The blur rung is built; its cost floor and its thresholds are next

Landed 2026-10-25 as [ADR-0075](../../docs/architecture/adr/0075-dynamic-resolution-sheds-the-blur-gather-before-pixels.md):
the dynamic-resolution controller sheds the motion blur's full gather before the extent
falls and restores it last (`oag_present::drs::blur`). Evidence, rigs and the running-race
check: [the half-gather blur rung](../../docs/rendering/motion-blur-half-rung.md).

## Open

- **What is left of the blur's cost floor.** `tile-max (x)` is unrolled (2026-10-25,
  about 1.4 ms off the Deck's 4K race median, frames identical; rendering page,
  "`tile-max (x)` unrolled"). On the Deck's 4K tick at half the floor is now `composite`
  2.2 ms, `prepare` 1.1 ms, `tile-max (x)` 0.7 ms, `copy` 0.5 ms. The composite and copy
  are full-size bandwidth passes; folding the copy into the composite (or the reduction
  into `prepare`) is the next lever, each worth one measurement before any code.
- **Per-strength thresholds.** The maintainer reports half is more visible at `high` than
  at `low` in some scenes. Not built; the seam is `drs::Limits` (it would carry the
  strength) and `drs::blur`'s constants (one set today). Blur strength moves the frame
  cost only a little (iGPU 1280x800, `low` 7.0 ms against `high` 8.7 ms GPU), so the case
  for it is the picture, not the budget.
- **A flip in motion, and HD's p99.** Pulse and HD both ran with the rung on the Deck at
  4K (rendering page, "On the Steam Deck"). On HD the rung buys 35 % more pixels for 0.7 fps
  and about 1.5 ms of p99. Nobody has looked at a flip in motion yet.
- **The restore constants are chosen.** `RESTORE_SECONDS = 3.0` and `MAX_BACKOFF = 3`
  fit one machine's spike spacing (1 to 5 s on Pulse). A machine with rarer, larger
  spikes would relapse up to three times before the backoff holds it.

## Next Steps

1. Find a way to drop a full-size pass from the half path (the composite reads the scene
   through `tile_tex`, so it cannot simply write the scene in place), then measure it
   against the current numbers on the Deck at 3840x2160 (`OAG_RENDER_GPU_BENCH=40 OAG_RENDER_PERF=1 oag-game <pulse image> --race --mode
   single_race --seed 1 --autopilot --no-audio --size 3840x2160 --ticks 1500 --motion-blur
   high --motion-blur-resolution half --screenshot out.png`, `deck.lock`).
2. Watch a gather flip in motion on the Deck (Pulse or HD at 4K, target 30), and decide
   whether HD's worse p99 under the rung needs a guard.
