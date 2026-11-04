---
status: measure
titles: [all]
updated: 2026-11-04
summary: "A split submits per view; one submit with the finish overlapped has a CPU ceiling of 1.3 to 4.0 ms a frame on the Deck (Pure, HD)"
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
- `Recording::Threaded` is the base this builds on: on equal cores it is the default once
  `THREADS_ON_EQUAL_CORES` is released (`split_record.rs`; Deck, 2.3 to 5.5 ms off the frame thread at
  three and four views, 0.4 to 1.5 at two; `split-screen.md`, "Threaded recording on equal cores"),
  +10 to 100% on the hybrid notebook, which keeps interleaved. It removes the per-view wait on the
  split finish but still prepares every view serially, replays a write log per later view and
  submits per view; the part of its gap that is the write log is unknown.

## Next Steps

1. **The ceiling is measured, by accident, on Pure and HD.** The build before the split-finish fix
   (`0fb333c97`) overlapped every view's finish with recording the next view, which is what one
   submission with the finish on the worker would do, at the price of a wrong picture (CPU is
   comparable, GPU order was not). Deck, two-player, `--render-scale 100`, frame-thread `total` ms:
   Pure 800p 5.33 against the fixed 6.62 (-1.29), HD Anulpha 800p 10.40 against 12.83 (-2.43, 91
   against 76.5 fps), 1080p 11.93 against 14.91 (-2.98), 4K 14.60 against 18.63 (-4.03): about a
   fifth of the frame thread. `docs/rendering/split-screen.md`, "Rerun at a true render size".
   Threaded recording (a lane is making it the default on uniform cores) gets 40 to 50% of that gap
   with the write log and per-view submits left in; this thread is the remaining half.
2. The ceiling is worth it:  one view's uploads as one staged buffer plus copies, byte-identical pictures
   against the interleaved path, the batched modes on top. Perf work goes to an Opus lane.
