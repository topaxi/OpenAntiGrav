---
status: measure
titles: [pulse, pure, hd]
updated: 2026-11-05
summary: "Deck runs judge allocation batch 2 and the GPU folds; threaded split recording flipped on 2026-11-05"
categories: [rendering, tooling]
size: S (70)
difficulty: easy (70)
---

# The Deck runs that release threaded recording and judge the allocation batches

Threaded split recording is released: `docs/rendering/split-screen.md`, "Threaded recording on equal
cores". What stays is the Deck's reading of batch 2 and the GPU folds, running since 2026-11-05.

Rules for the runs: `deck@steamdeck`, every run under `flock -o ~/.cache/oag/deck.lock`, one Deck job
at a time, 800p, 1080p and 4K with `--render-scale 100`. A binary built on this notebook does not
run there (glibc): build in the `oag-appimage-build:bookworm` container with
`--no-default-features --features extensions,dev-cli`. fps paces near 102 (split) and 120 (one view)
and 4K is GPU-bound, so judge CPU by mean `total` from `frame-phases`, and say when fps is pacing.

The harnesses and full commands are on the lead's notebook, gitignored, in its scratch directory `lf`:
`split-threaded-plan.md` ("Ready-to-run Deck block"), `batch2-plan.md` ("Steam Deck"),
`gpu/deck-block.sh` (not the early copy in `gpu-plan.md`). On the Deck: `~/oag-perf/split-threaded/`
(`cell.sh`, `run.sh`, `avg.sh`, `xdg/`) and `~/oag-perf/batch1/` (`all.sh`, `abba.sh`, `freq.sh`).

## Open

2. **Batch 2, Deck effect.** Merged as `bb97e0a5b` on `625d949d9`; pictures byte-identical,
   allocations down, CPU measured on the notebook only. Not run: the base-against-merge matrix, three
   rounds, Pure, Pulse, HD Anulpha, one and two views, three sizes. Also the open Pure 2v 1080p
   +0.25 ms from batch 1 (ABBA with the CPU clock sampled).
3. **GPU folds, Deck effect.** Spans, track shader folds, the plain-draw pipeline and the atlas skip
   are merged; `docs/rendering/hires-gpu-levers.md` holds the Intel iGPU numbers, which do not
   forecast the Deck. Not run: before and after at three sizes, `RADV_DEBUG=shaderstats` for VGPRs,
   and the plain draws' CPU side with `OAG_RENDER_BENCH=300` (+48 % `pipeline_sets`; the notebook
   shows no penalty, HD `render_viewports` 2,047 to 1,884 us). The Deck is submit-bound, so a CPU
   rise there overrules the notebook: fall back to the rich pipeline over short plain runs.

## Next Steps

1. Read the GPU block and the batch 2 matrix when they finish.
2. Fall back on the plain draws if the Deck's CPU rows say so.
3. Write the numbers into `docs/rendering/split-screen.md` and `docs/rendering/frame-thread-cpu.md`,
   then delete this thread.
