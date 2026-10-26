---
status: blocked-hardware
titles: [all]
updated: 2026-10-16
summary: "Split-screen CPU: run the Deck commands for encoder.finish and uniform uploads on main and the encoder-finish branch"
categories: [rendering, tooling]
---

# Split-screen CPU: `encoder.finish` is the texture bind-group switches

2026-10-15, `split-cpu` then `encoder-finish`, on an Intel Core Ultra 7 265H notebook (NVIDIA RTX
PRO 2000 Blackwell Laptop GPU and the Arrow Lake iGPU). Evidence and numbers:
`docs/tools/render-bench.md` "Split CPU on the notebook" and "Encoder finish and the uniform uploads",
`docs/rendering/split-screen.md` "Cost".

Landed: the circuit shine pass's normals are kept, the craft shine maps are written by the first view
of a frame, HD's depth prepass binds one texture slot (`split-cpu`); one shared buffer of scene blocks
(a block per view and per kind, bound by dynamic offset: 33 fewer `queue.write_buffer` calls a view on
Pulse; Pulse's CPU -4 to 5%), every drawable's matrices on one buffer with a region per view (Pulse
56 to 36 writes a view in one view, 93 to 45 in two), the craft animation tables written by the
first view only, every still byte-identical and every state hash unchanged
(`encoder-finish`). Tried and reverted (`5b646661f`, still in the history): merging contiguous
same-texture batches into one `draw_indexed` (HD 1,447 to 969 calls a view) took 19 us out of HD's
`finish` and nothing out of Pulse's or Pure's, and costs 3 to 8 us to queue: draw calls are not what
`finish` pays for, the texture bind-group switches are.

## Open

- ~~**The texture bind-group switches are 55 to 65% of `encoder.finish`**~~ **Taken out on native
  (2026-10-16, `bindless`)**: binding arrays bound once per drawable, the slot's indices pushed as
  immediates; NVIDIA 2h, the second view's `finish` Pulse 208 to 124 us, HD 283 to 160, Pure 138 to
  88, the iGPU's frame unchanged. Open: the Deck, in
  [bindless-textures-could-take-a-fifth-of-a-views-cpu.md](bindless-textures-could-take-a-fifth-of-a-views-cpu.md).
- **What is left of the uploads** is the per-view effect pipelines' own buffers (exhaust, sparks,
  clouds, shadow geometry, beams, which the probe does not count: `exhaust-upload` alone is about 35 us a view on Pulse) and the advert cards; the matrices and the
  scene blocks are done.
- ~~**Views recorded on separate threads**~~ **Built, measured slower, opt-in (2026-10-15,
  `threaded-views`)**: every view's frame is a prepare (all writes, a `ViewPass` value) and a
  record that reads no shared state (`Recorder: Sync`, compile-time), and `OAG_SPLIT_RECORDING=
  threaded` records the views on scoped threads; on the notebook that cost 10 to 100% more CPU than one after another (a cold second core for a 0.2 to 0.4 ms burst), so `interleaved` is the
  default. Evidence: `docs/rendering/split-screen.md` "Prepare and record",
  `docs/tools/render-bench.md` "Threaded views". **Open: the Deck A/B** (commands there) decides
  whether any platform defaults to threads; a persistent worker pool (would need `unsafe` lifetime
  erasure, a spinning core gains, a parked one does not) is not built. ~~**The lever the numbers
  point at instead**: record view k on a worker while view k+1 is prepared~~ **Built, measured
  slower, opt-in (2026-10-16, `split-followups`)**: `OAG_SPLIT_RECORDING=overlapped`, the effect
  pipelines' draw handles are a cloneable `Drawer` each view keeps; 50 to 130 % more CPU than
  interleaved on NVIDIA, 25 to 150 % on the iGPU, within 7 to 11 % pinned to one core (a worker
  woken per view). `docs/tools/render-bench.md`, "Overlapped recording". Open: the Deck A/B; a
  spinning worker is not built.
- **Render bundles are not cheap**: a bundle's commands are fixed at creation, and every drawable's
  draw list changes each view with the culling and the level of detail, with per-view dynamic offsets.
- **The Arrow Lake iGPU's driver (Mesa ANV) costs about four times NVIDIA's CPU on Pulse** and is
  CPU-bound there; not profiled below `perf-probe` spans (`samply` could not sample,
  `kernel.perf_event_paranoid` is 2).
- A bench on this machine must be pinned (`taskset -c 0-5`) and read from an interleaved A/B;
  `scripts/bench-render.py` does neither itself, and **a whole-race run at load 3 to 14 read +-40% on
  identical binaries**, so compare with the `OAG_RENDER_BENCH` line, minimum of rounds.
- Not measured: the Steam Deck (the maintainer runs it; commands in `docs/tools/render-bench.md`).

## Next Steps

1. Run the Deck commands in `docs/tools/render-bench.md` ("Encoder finish and the uniform uploads")
   on `main` and `encoder-finish`.
2. ~~Decide whether the group-1 switches are worth a native-only bindless path~~ built
   (`bindless`, 2026-10-16); its Deck run is in its own thread.
3. ~~List and split what is left shared at record time~~ done (`threaded-views`); run the
   threaded A/B on the Deck (`docs/tools/render-bench.md`, "Threaded views").
