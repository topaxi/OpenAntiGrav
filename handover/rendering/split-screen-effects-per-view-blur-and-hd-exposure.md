---
categories: [rendering]
---

# Split screen draws Pulse's glow per view; blur, the temporal upscaler and HD's exposure are open

2026-10-15, `split-bloom`. Evidence and numbers: `docs/rendering/split-screen.md` rules 3, 4 and 6
and "Per-view glow"; `docs/tools/render-bench.md`.

Landed: `oag_post::bloom` and `ps2_bloom` read their rectangle from `Frame::origin` (they read from
the target's top-left; a letterboxed `--screenshot` capture glowed from rows shifted by its bar),
a scratch lane per view, `Scene::prepare_split_bloom` after the last view and a per-view
composite in `Scene::composite_bloom`, lane sizes that keep the glow's size against the view
(chosen, not measured), and `Chain::set_reach` so HD's whole-frame chain reaches as far across a
view as it does across one picture. HD's bloom was never skipped in a split. About 100 us of CPU
a frame for two views on the NVIDIA card; one view byte-identical.

## Open

- **Motion blur per view.** The blur reads the velocity and depth attachments, which every
  view's scene pass clears whole, and its output cannot be written as a view is drawn because the
  next view's multisample resolve rewrites the target. Needs a region of velocity and depth per
  view, the blur run per view after the last (as the glow is), and `Scene::history` extended from
  windows to view slots. `hd-gpu`'s "is velocity read this frame" rule must count split views once
  this lands. Not started.
- **The temporal upscaler and camera jitter** hold one camera's frame record; not touched.
- **HD's exposure is one for the whole picture** and the glow bleeds a few pixels across the gap.
  A per-view adaptation needs the chain's scratch targets sized per view.
- **The Steam Deck**: run the commands in "Per-view glow" (`split-screen.md`) and send the
  pipelined and CPU medians of one view, 2h and 2v, and look at the stills; the Arrow Lake iGPU
  reading on the notebook was too noisy to quote.
- **PS2 and Pulse stills were judged by eye on two circuits**; Pure has no glow mask, 2048 and
  Omega draw through other chains and were not looked at under the new reach.

## Next Steps

1. Deck numbers (above), 15 minutes on the Deck.
2. If blur per view is wanted: start with the velocity and depth regions, then the history slots.
