# HD's GPU frame: the velocity gate waits for a Deck number

2026-10-15, `hd-gpu`. Evidence and numbers: `docs/rendering/hd-gpu-cost.md`,
`docs/tools/render-bench.md` "HD on a notebook". The craft cull landed; what is
left is a measurement only the Deck can make.

## Open

- **The velocity gate exists and is not merged.** Branch `hd-gpu-velocity-gate`
  (commit `e58a38d0e`) builds the track's and hulls' opaque, prepass and cutout
  pipelines a second time with the velocity write masked, and `Scene::render` picks
  them on a frame that has no reader (`scene/velocity_gate.rs` is the one list of
  readers: the motion blur and a temporal upscaler). Masked pipelines are
  colour-identical (a render test). Repeated, it saves nothing measurable on the
  Intel iGPU and 2 to 8 % of the race pass on the NVIDIA card, so it stays out until
  a Deck run shows a bandwidth saving. A split view with blur on (`split-bloom`'s
  work) must pass its blur strength to `velocity_readers`.
- **No Deck number exists.** Every figure is the Intel iGPU or the NVIDIA card, and
  neither predicts the Deck's 8-CU RDNA2 APU (this host's iGPU is CPU-bound at
  1280x800, the Deck is GPU-bound at 4K under the 1080p budget). Whether the shadows
  (about 1.4 ms at 1080p), MSAA or the blur is the Deck's worst stretch is unknown.
- **The shadow passes are 1.4 ms of a 6.0 ms frame at 1080p** and the HD shadow map
  is another lane's; the number is recorded, nothing was changed.
- **`submit_us` was 3.8 ms on the Intel driver** in the first, cold run (CPU, not GPU;
  the repeat read 2.5 ms total CPU). Unexplained: pipeline compilation on first use
  is the guess, not checked.

## Next Steps

1. On the Deck, run the commands under "Measure on the Deck" in
   `docs/rendering/hd-gpu-cost.md` (whole-race bench at 1920x1080 and 1280x800, and
   the `OAG_RENDER_GPU_BENCH` spans with and without `--msaa 4x --motion-blur high`),
   once on `main` and once on `hd-gpu-velocity-gate`.
2. If the gate wins on the Deck, merge it; if not, delete the branch.
3. If shadows or MSAA are the Deck's limit, hand that to the queued shadow-map and
   split-bloom lanes.
