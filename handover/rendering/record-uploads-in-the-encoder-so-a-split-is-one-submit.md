---
status: measure
titles: [all]
updated: 2026-11-04
summary: "A split submits per view; one submit with the finish overlapped has a ceiling of 1.6 to 2.4 ms a frame on the Deck (Pure)"
categories: [rendering]
size: L
difficulty: hard
---

# Record uploads in the encoder so a split frame can be one submission

2026-11-04, from comparing our frame against wgpu's own guidance (few submits a frame, few
resources, recording that does not depend on submission order). Nothing here is built or
measured beyond the numbers cited.

## Why a split submits once per view

`queue.write_buffer` lands before the commands of the submission that follows it, not at its
place in an encoder. A view's matrices, scene block and animation tables are written with it, so
two views in one submission would both draw with the last view's writes
(`crates/raceplay/src/scene/viewports.rs`, header, "One encoder and one submit per view"). The
batched recordings keep a later view's writes in a `oag_gpu::writes::Log` and replay them just
before that view's submit (`split_record.rs`). The post work of a split (glow, exposure, blur)
submits on top of that.

## Open

- **What the per-view submit costs on the Deck**: the `submit` phase of a two-player Pure split is
  0.45 ms of a 7.4 to 7.65 ms frame thread (`docs/rendering/split-screen.md`, last section), and
  dropping finished encoders in `maintain` is 0.8 ms in the Anulpha profile
  (`docs/rendering/frame-thread-cpu.md`). So the submit count itself looks like a small share; the
  cost sits in encoders and passes. Not isolated.
- **The idea**: stage each view's uploads in one buffer and record `copy_buffer_to_buffer` into the
  view's own encoder, so a view's data is ordered by the encoder and not by the submit. Then a
  split is one submission, views record independently (the threaded and overlapped modes lose the
  write log), and the split finish stops waiting per view.
- **Costs to weigh**: the copies are encoder commands (`finish` counts commands), the staging buffer
  is more memory per view, and the 51 `write_buffer` calls a frame already cost 0.63 ms on the
  frame thread, so the copy path has to beat wgpu's own staging, not just match it.
- `Recording::Threaded` measured 0.6 to 0.7 ms below the interleaved default on the Deck at 800p and
  4K (`split-screen.md`), +10 to 100% on the hybrid notebook: the part of that gap that is the write
  log is unknown.

## Next Steps

1. **The ceiling is already measured, by accident.** The build before the split-finish fix (commit
   `ca91c9c5b`'s parent chain, `0fb333c97`) overlapped every view's finish with recording the next
   view, which is what one submission with the finish on the worker would do, at the price of a
   wrong picture (the post work ran ahead of its scene pass; GPU time is not comparable, CPU is).
   Deck, two-player Pure, frame-thread `total` ms, 800p / 1080p / 4K: that build 6.06 / 5.69 / 7.71,
   the fixed interleaved path 7.65 / 7.47 / 10.10, `OAG_SPLIT_FINISH=0` 7.40 / 7.20 / 10.10
   (`docs/rendering/split-screen.md`). So the ceiling is 1.3 to 1.6 ms at 800p, 1.5 to 1.8 at 1080p
   and 2.4 at 4K, about a fifth of the frame thread. HD is not measured: the same A/B on HD
   Anulpha needs that build again.
2. The ceiling is worth it:  one view's uploads as one staged buffer plus copies, byte-identical pictures
   against the interleaved path, the batched modes on top. Perf work goes to an Opus lane.
