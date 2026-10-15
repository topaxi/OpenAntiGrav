# HD's GPU frame: the velocity write is the biggest cut left, and the Deck has not been measured

2026-10-15, `hd-gpu`. Evidence and numbers: `docs/rendering/hd-gpu-cost.md`,
`docs/tools/render-bench.md` "HD on a notebook". The craft cull landed; what is
left is a decision and a measurement the maintainer owns.

## Open

- **The velocity attachment is written by every opaque pipeline whether or not
  anything reads it** (`Scene::velocity`'s doc: "always written in the game path",
  by design, for FSR 3.1/TAA). Masking it for the opaque pipelines measured 0.3 to
  0.5 ms of a 3.5 to 4.5 ms HD race pass at 1080p on the Intel iGPU (about 10 %,
  MSAA off and 4x). Doing it needs a second variant of the opaque, prepass-shade and
  cutout pipelines, chosen per frame by "blur on, or camera jitter on"; the blur row
  applies live, so one baked mask is not enough. Reverses a documented design, so it
  needs the maintainer's yes first. The member asked the lead and had no answer.
- **No Deck number exists.** Every figure is the Intel iGPU, which is faster than the
  Deck (this host's iGPU is CPU-bound at 1280x800, the Deck is GPU-bound at 4K under
  the 1080p budget). Whether the velocity write, the shadows (about 1.4 ms at 1080p)
  or MSAA is the Deck's worst stretch is unknown.
- **The shadow passes are 1.4 ms of a 6.0 ms frame at 1080p** and the HD shadow map
  is another lane's; the number is recorded, nothing was changed.
- **`submit_us` was 3.8 ms on the Intel driver** in the first run (CPU, not GPU;
  the repeat read 2.5 ms total CPU). Unexplained: pipeline compilation on first use
  is the guess, not checked.

## Next Steps

1. On the Deck, run the commands under "Measure on the Deck" in
   `docs/rendering/hd-gpu-cost.md` (whole-race bench at 1920x1080 and 1280x800, and
   the `OAG_RENDER_GPU_BENCH` spans with and without `--msaa 4x --motion-blur high`).
2. If the Deck's race pass is the limit, decide the velocity write, then build the
   masked pipeline variants and re-run the spans.
3. If shadows or MSAA are, hand that to the queued shadow-map and split-bloom lanes.
