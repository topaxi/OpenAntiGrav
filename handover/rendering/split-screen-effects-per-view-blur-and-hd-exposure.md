---
status: blocked-hardware
titles: [all]
updated: 2026-10-16
summary: "Deck numbers for split-screen per-view effects and blur are open (15 minutes each on the Deck)"
categories: [rendering]
---

# Split screen draws Pulse's glow, the motion blur, HD's and Omega's exposure and FSR 3 per view; the Deck numbers are open

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

- ~~**Motion blur per view.**~~ **Done, 2026-10-15, `split-blur`** (`docs/rendering/split-screen.md`
  rule 3 and "Per-view blur"): a later view loads velocity and depth, a camera history per seat,
  the blur once per view after the last (one submit each), `hd-gpu`'s velocity-gate rule moot (the
  buffer is always written on `main`; if the gate ever lands, a split's views count as read when the
  blur is on). Open from it: the Deck reading (below).
- ~~**The temporal upscaler and camera jitter**~~ **Done, 2026-10-16, `split-followups`**
  (`split-screen.md`, "Per-view FSR 3"): a jitter sequence and record per seat, a chain per seat
  reading at the view's corner. Open: the overlay's FSR 3 rows idle in a split (no timestamps on
  the per-view chains); how the history settles in motion was not looked at (captures show two
  frames); the Deck numbers.
- ~~**HD's exposure is one for the whole picture**~~ **Done, 2026-10-15, `hd-exposure`**
  (`docs/rendering/split-screen.md` rule 6 and "Per-view exposure"): HD's chain (and 2048's, which
  draws through it) and Omega's tone map run once per view over the view's rectangle with an
  adaptation lane per seat (forgotten by the first single-view frame), one submit each; the glow stops
  at the view's edge. The scratch levels did not need to be sized per view: the views run one after
  another in their own submissions, each over its fraction of the same levels. Open from it: HD's exposure is
  unity above an adapted luma of 0.15 on Talon's Junction, so it only moves a very dark view; what
  an ordinary difference between views moves is the glow fade (Zone 4 in Zone mode, 2h and 2v: the
  dark view's bright-pixel share 0.4 % to 5.2 %, the bright view unchanged); the other circuits'
  `Tone` values were not read. The zoom streak and Omega's distortion are still not drawn in a
  split. ~~**A multi-window session where one window holds a single seat and another a split**~~
  **Done, 2026-10-16, `split-followups`** (`split-screen.md` rules 3 and 6): a seat is forgotten
  alone when it missed a frame (`Scene::keep_split_seat`); a one-seat window no longer wipes the
  split window's histories (which had also cost that window its blur) and lanes.
- **The Steam Deck**: run the commands in "Per-view blur", "Per-view glow" and "Per-view exposure"
  (`split-screen.md`) and send the pipelined and CPU medians of one view, 2h and 2v, and look at
  the stills; the Arrow Lake iGPU reading on the notebook was too noisy to quote.
- **PS2 and Pulse stills were judged by eye on two circuits**; Pure has no glow mask, 2048 and
  Omega draw through other chains and were not looked at under the new reach (2048 and Omega were
  looked at, 2h and 2v, under the per-view chains in `hd-exposure`).

## Next Steps

1. Deck numbers (above), 15 minutes on the Deck.
2. The Deck's blur reading ("Per-view blur", `split-screen.md`), 15 minutes.
