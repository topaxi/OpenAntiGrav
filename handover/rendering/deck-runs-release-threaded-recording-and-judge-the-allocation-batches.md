---
status: blocked-hardware
titles: [pulse, pure, hd]
updated: 2026-11-04
summary: "HIGH PRIORITY: Deck runs release threaded split recording and judge allocation batch 2; blocked while the Deck is offline, 2026-11-04"
categories: [rendering, tooling]
size: S (70)
difficulty: easy (70)
---

# The Deck runs that release threaded recording and judge the allocation batches

**High priority, first thing once the Deck is back.** The thread frontmatter has no priority
field, so the summary says it. Threaded split recording is the largest CPU win measured on the Deck
this round (2.3 to 5.5 ms off the frame thread at three and four views) and it ships off until these
runs say otherwise. The Deck is offline for the rest of 2026-11-04: nobody runs anything on it, and
every Deck result for batch 2 and the GPU work reads "not run" until then.

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

1. **Threaded recording, release evidence.** Rule landed, `THREADS_ON_EQUAL_CORES = false`
   (`crates/raceplay/src/scene/split_record.rs`; `docs/rendering/split-screen.md`, "Threaded
   recording on equal cores"). Not run: the production default at two views (Pure, HD, three sizes),
   Pulse at two views (800p, 1080p), and four more rounds of Pure three views at 1080p, whose mean
   p99 reads 0.8 ms worse. A flip that shows p99 worse anywhere stays off; say which cell.
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

1. Build the three binaries from current `main`: main, `625d949d9`, and main with
   `THREADS_ON_EQUAL_CORES = true` (one line; local branch
   `worktree-agent-aad5c369ca8f2ed85-flip`, commit `49a73f6bd`, is the same change on an older base).
2. Run item 1 first, then item 2, then item 3. Items 1 and 2 are about two hours of Deck time at
   the earlier runs' pace, an estimate; the first builds take about 10 minutes each.
3. If item 1 is clean, set the constant, strike "false for now" in `split-screen.md` and the module
   doc in `split_record.rs`, gate and push.
4. Write the numbers into `docs/rendering/split-screen.md` and `docs/rendering/frame-thread-cpu.md`,
   then delete this thread.
