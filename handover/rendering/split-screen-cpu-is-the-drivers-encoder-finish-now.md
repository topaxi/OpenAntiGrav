---
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
Pulse; Pulse's CPU -4 to 5%), every still byte-identical and every state hash unchanged
(`encoder-finish`). Tried and reverted (`5b646661f`, still in the history): merging contiguous
same-texture batches into one `draw_indexed` (HD 1,447 to 969 calls a view) took 19 us out of HD's
`finish` and nothing out of Pulse's or Pure's, and costs 3 to 8 us to queue: draw calls are not what
`finish` pays for, the texture bind-group switches are.

## Open

- **The texture bind-group switches are 55 to 65% of `encoder.finish` and 20 to 25% of a view's CPU**
  (a hack that never re-binds group 1: Pulse's second view 216 to 96 us of finish, HD 273 to 98, Pure
  133 to 39; 2h CPU Pulse 1,605 to 1,297, HD 2,283 to 1,814, Pure 680 to 420 us). That is the ceiling
  of any scheme that removes them. Sorting the draws by texture moves pixels (the 2026-10-03 note in
  `scene/shine.rs`), `texture_2d_array` needs one size and format, an atlas breaks repeat wrapping,
  and a `binding_array` indexed by the instance index is native only (the web and phone builds keep
  the per-draw path) and touches group 1's five bindings in every title's mesh shader. Its own lane,
  decided by the maintainer.
- **Group 0 is still one `queue.write_buffer` per drawable per view** (matrices; Pulse 56 writes a view
  after the scene blocks). Packing them into one write a view, a dynamic offset per drawable inside the
  view's region, is the next upload win (about 1 us a write removed, an estimate from the scene
  blocks' 33 writes and 4 to 5%) and is also what a threaded view needs.
- **Views recorded on separate threads**: the scene block region is per view, but the group-0
  buffers, the animation tables, `Drawable`'s `RefCell`s (`shown`, `ripple`, `pad_glow`, `hidden`),
  `Scene`'s `layer`, `history`, `scratch`, `opaque_order` cells, the effect pipelines' buffers and the
  race's viewpoint cell are shared at record time (list in `docs/rendering/split-screen.md`, "Cost").
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
2. Decide whether the group-1 switches are worth a native-only bindless path, and who owns it.
3. Pack the group-0 matrices into one write a view with per-view regions, then list what is left
   shared at record time for the threaded-views lane.
