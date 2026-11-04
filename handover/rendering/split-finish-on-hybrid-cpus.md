---
status: measure
titles: [all]
updated: 2026-11-04
summary: "Split finish now only on uniform cores (S24 flyby -1 ms); probe reads Linux sysfs only, hybrid Windows/macOS unprobed"
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
   right (`submit_with_closed`) put the finish back on the critical path. Candidates, each an
   Opus perf lane: no split frame when more than one view is drawn (recovers 0.25 ms at 800p and
   1080p), or `OAG_SPLIT_RECORDING=threaded` as the default on uniform cores (800p 6.94 against
   7.65, 4K 9.52 against 10.10, 1080p 6.81 and 7.86 in two rounds, so noisy); the notebook measured
   it slower (`split_record.rs`). Details: `docs/rendering/split-screen.md`.
4. The parked indirect depth prepass (`park/indirect-depth-prepass`) slightly regressed together
   with the split (maintainer); re-measure per platform if either changes.

## Next Steps

1. On a hybrid Windows or macOS machine: `--frame-log` with `OAG_SPLIT_FINISH=0` and `=1`, compare
   `scene` and `submit`. The S24 needs nothing more (maintainer, 2026-10-27).
