---
status: measure
titles: [all]
updated: 2026-11-04
summary: "Split finish only on uniform cores; threaded split recording on equal cores built but off until the Deck two-view and Pulse runs"
categories: [rendering, tooling]
size: S (40)
difficulty: easy (40)
---

# The split encoder finish on hybrid CPUs

The S24's reported CPU row rise (about 9 to 12-13 ms, HD campaign event 1 flyby) was measured
on 2026-10-27: the threaded encoder finish adds about 1 ms of `scene` on the phone and saves no
`submit`; the rest did not reproduce. The split is now on only where every core reports the same
`cpu_capacity` (`oag_gpu::split_finish::wanted`). Numbers: `docs/rendering/frame-thread-cpu.md`,
"The maintainer's scene: the split finish costs the phone about 1 ms".

## Open

1. The probe reads Linux/Android sysfs only. Windows on an Intel hybrid part and macOS on Apple
   silicon keep the split unprobed; measure one of each (`OAG_SPLIT_FINISH=0` against `=1`, the
   `frame-phases` `scene` and `submit` phases) before deciding their default.
2. An Intel hybrid part on Linux reports `cpu_capacity` only on kernels with asymmetric capacity
   support; unreadable means the split stays on.
3. **Under an interleaved split the split finish gains nothing on the Deck** (measured 2026-11-04,
   two-player Pure, frame-thread `total` ms, split finish on / off: "800p" (a 1280x800 window drawn at
   1920x1200, so not comparable with the other sizes) 7.65 / 7.40, 1080p 7.47 /
   7.20, 4K 10.10 / 10.10; one view it helps, 4.27 / 4.86 at 800p). The fix that made the picture
   right (`submit_with_closed`) put the finish back on the critical path. **Threaded recording on
   equal cores is built and off** (`THREADS_ON_EQUAL_CORES` in `split_record.rs`, 2026-11-04): the
   rule picks `threaded` only on native Linux with every `cpu_capacity` readable and equal, so this
   notebook, the S24, unreadable lists and other OSes keep interleaved. Deck, production default with
   the rule on against `interleaved`: 2.3 to 5.5 ms off `total` at three and four views in HD and four
   in Pure, nothing at Pure's three views, whose mean p99 at 1080p is 0.8 ms worse (ranges touching).
   Open: the two-view production default (Pure, HD) and Pulse two views were not run (the Deck was
   taken back); commands and the table in `docs/rendering/split-screen.md`, "Threaded recording on
   equal cores". Release by setting the constant to `true` after them. The other candidate, no split
   frame when more than one view is drawn (recovers 0.25 ms at 800p and 1080p), is still open.
4. The parked indirect depth prepass (`park/indirect-depth-prepass`) slightly regressed together
   with the split (maintainer); re-measure per platform if either changes.

## Next Steps

1. On the Deck: the two-view and Pulse runs of `docs/rendering/split-screen.md`, "Threaded
   recording on equal cores", with a binary built with `THREADS_ON_EQUAL_CORES = true`; flip it if
   they hold, and look at Pure three views at 1080p (p99) again.
2. On a hybrid Windows or macOS machine: `--frame-log` with `OAG_SPLIT_FINISH=0` and `=1`, compare
   `scene` and `submit`. The S24 needs nothing more (maintainer, 2026-10-27).
