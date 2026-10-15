---
categories: [rendering, tooling]
---

# Split-screen CPU: the shine passes are cheap now, `encoder.finish` is what is left

2026-10-15, `split-cpu`, on an Intel Core Ultra 7 265H notebook (NVIDIA RTX PRO 2000 Blackwell
Laptop GPU and the Arrow Lake iGPU). Evidence and numbers:
`docs/tools/render-bench.md` "Split CPU on the notebook", `docs/rendering/split-screen.md` "Cost".

Landed: the circuit shine pass's normals are kept (`TrackNormals`), the craft shine passes'
environment maps are written by the first view of a frame, and HD's depth prepass binds one
texture slot. A Pulse view's extra CPU fell from about 774 to 595 us, HD's two-view CPU by 10 to
16%, every still byte-identical.

## Open

- **`encoder.finish` is the biggest item left**: about 230 us of a Pulse view's 590 and about
  500 us of an HD view's at about 800 draws. It is wgpu validating the recorded commands, so it
  scales with their count (the probe's `scene-pass` recording is 40 us). Ideas, each a change to
  how a drawable is recorded, none tried: one texture array or atlas per drawable instead of a
  bind group per texture; render bundles for the draws a camera does not change (a bundle takes
  no per-view uniform, so the scene uniform would have to move to a shared dynamic-offset
  buffer first); fewer, merged draws for a craft.
- **The scene uniform is written once per drawable per view** (`Scene` is about a kilobyte and
  there are 40 to 60 drawables with their own `fog` buffer): around 40 `queue.write_buffer`
  calls a view at 1.5 to 4 us each on NVIDIA. One shared buffer per view, bound by every
  drawable, would be one write.
- **The Arrow Lake iGPU's driver (Mesa ANV) costs about four times NVIDIA's CPU on Pulse**
  (5.4 ms for one view, 9.2 ms for two), so Pulse's split is CPU-bound there at about 9 ms. Not
  profiled below `perf-probe` spans: `samply` could not sample (`kernel.perf_event_paranoid` is
  2, no root). A maintainer run of `echo 1 | sudo tee /proc/sys/kernel/perf_event_paranoid`
  lets `cargo install samply` and `samply record` see inside the driver.
- A bench on this machine must be pinned (`taskset -c 0-5`) and read from an interleaved A/B;
  `scripts/bench-render.py` does neither itself.
- Not measured: the Steam Deck (the maintainer runs it).

## Concurrency share of a two-view frame's CPU (2026-10-15)

NVIDIA RTX PRO 2000 Blackwell Laptop GPU, pinned P-cores, `perf-probe` spans of the **second
view** of a 2h frame (it leaves out the first view's once-per-frame work, which stays serial;
the first view was not re-profiled after the fixes, and HD's `finish` is from before the
prepass fix). Probe spans, not a flame graph (`samply` could not sample).

| Per view | Pulse (~590 us) | HD (~1,000 us) | Pure (~310 us) |
| --- | --- | --- | --- |
| `encoder.finish` | 234 (40%) | ~500 (50%) | 142 (46%) |
| scene-pass recording | 40 (7%) | ~96 (10%) | 22 (7%) |
| culling and gathers | 20-25 (4%) | ~100 (10%) | small |
| `queue.write_buffer` uploads | ~250 (42%) | ~230 (23%) | ~120 (38%) |
| submit | 44 (7%) | 31-47 (4%) | 23-30 (9%) |

- **Could run on its own thread now** (finish, recording, gathers): about 50% of Pulse's view,
  70% of HD's, 55% of Pure's. That is the ceiling for a concurrent-encoder lane.
- **Parallel only after a redesign** (the uploads, 25-40%): `queue.write_buffer` lands before the
  next submission's commands, not at its place in the encoder, so views recorded concurrently
  into separate encoders still need per-view uniform buffers, or one shared per-view scene
  buffer with dynamic offsets (the "scene uniform is written once per drawable per view" item).
- **Serial regardless:** submit (4-9%), any device or queue locking wgpu takes inside `finish`
  and `write_buffer` (not measured), and the first view's shared work (shadow maps, gather,
  scenery clocks, adverts). The real ceiling is lower than the shares above.

## Next Steps

1. Run `samply` on `oag-game --race --players 2` once the paranoid level allows it, on both
   adapters, and read `encoder.finish` and the ANV frames by symbol.
2. Try the shared per-view scene uniform: it is the smaller change and it removes 40 writes
   a view.
3. Judge the texture-array change by what it removes from `finish`, on HD first (its circuit
   draws about 800 batches).
