---
status: measure
titles: [all]
updated: 2026-11-04
summary: "A split submits per view because queue writes land before the submission; uploads recorded in the encoder would allow one submit"
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

1. Measure the ceiling before building anything: a build that submits a two-player split once per
   frame with the views' uploads going wrong (a throwaway, never committed), Deck, Pure and HD, 800p,
   1080p and 4K, the harness in `docs/rendering/split-screen.md`. If the ceiling is under about 0.3 ms
   a frame, close this thread with the number.
2. If it is worth it: one view's uploads as one staged buffer plus copies, byte-identical pictures
   against the interleaved path, the batched modes on top. Perf work goes to an Opus lane.
