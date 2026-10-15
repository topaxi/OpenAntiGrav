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

## Next Steps

1. Run `samply` on `oag-game --race --players 2` once the paranoid level allows it, on both
   adapters, and read `encoder.finish` and the ANV frames by symbol.
2. Try the shared per-view scene uniform: it is the smaller change and it removes 40 writes
   a view.
3. Judge the texture-array change by what it removes from `finish`, on HD first (its circuit
   draws about 800 batches).
