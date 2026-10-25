# The blur rung is built; its cost floor and its thresholds are next

Landed 2026-10-25 as [ADR-0075](../../docs/architecture/adr/0075-dynamic-resolution-sheds-the-blur-gather-before-pixels.md):
the dynamic-resolution controller sheds the motion blur's full gather before the extent
falls and restores it last (`oag_present::drs::blur`). Evidence, rigs and the running-race
check: [the half-gather blur rung](../../docs/rendering/motion-blur-half-rung.md).

## Open

- **The blur's cost floor.** About half the chain does not move with the gather: 2.2 ms on
  a Raphael iGPU at 1280x800, about 10 ms on the Deck at 3840x2160. One 4K tick on the
  iGPU puts it in `tile-max (x)` (5.8 ms, 49,680 fragments each reading `0.08 * height`
  pixels), the full-size `composite` the half gather adds (4.8 ms), `prepare` (1.6 ms) and
  `copy` (1.1 ms). Whole-race numbers per pass and a Deck split are not taken.
- **Per-strength thresholds.** The maintainer reports half is more visible at `high` than
  at `low` in some scenes. Not built; the seam is `drs::Limits` (it would carry the
  strength) and `drs::blur`'s constants (one set today). Blur strength moves the frame
  cost only a little (iGPU 1280x800, `low` 7.0 ms against `high` 8.7 ms GPU), so the case
  for it is the picture, not the budget.
- **A missing blur reading reads as free.** `session::timing::assemble` folds an absent
  blur reading to zero (`blur.unwrap_or(0.0)`). For the rung that is safe (no shed, a
  pessimistic restore). For the extent loop it under-counts `Cost::scalable` by the whole
  chain on such a frame. The fix is a "blur ran" flag beside `has_hd_bloom`, threaded
  through window 1 and `Rings::cost`: not a one-line change, so left.
- **Not run on the Deck.** The rung's motivation is the Deck at 4K, but the running-race
  check was on the workstation's iGPU. Also unchecked: HD in a running session, and a
  player's eye on a flip in motion (the maintainer's comparison was one still per title).
- **The restore constants are chosen.** `RESTORE_SECONDS = 3.0` and `MAX_BACKOFF = 3`
  fit one machine's spike spacing (1 to 5 s on Pulse). A machine with rarer, larger
  spikes would relapse up to three times before the backoff holds it.

## Next Steps

1. Time the blur chain per pass over a whole race on the Deck at 3840x2160
   (`OAG_RENDER_PERF=1` spans) and decide whether `tile-max (x)` can run at the tile grid's
   own resolution or share a pass with `prepare`; the composite and copy are full-size
   bandwidth passes worth one look each.
2. Run the rung on the Deck at 4K with a target it cannot hold (the gamescope command in
   the rendering page, Deck build, `deck.lock`), Pulse and HD, and record the flips and
   pixels beside the iGPU's.
3. Thread a "blur ran" flag into `assemble` so an unmeasured chain skips the frame the way
   an unmeasured HD bloom does.
