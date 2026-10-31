---
status: blocked-hardware
titles: [hd]
updated: 2026-10-31
summary: "Velocity gate landed (AMD 2-7 %); left: Deck shadow/MSAA/blur spans and the cold submit_us"
---

# HD's GPU frame: Deck spans for shadows, MSAA and the blur

2026-10-15, `hd-gpu`. Evidence and numbers: `docs/rendering/hd-gpu-cost.md`,
`docs/tools/render-bench.md` "HD on a notebook". The craft cull landed; what is
left is a measurement only the Deck can make.

## Open

- ~~The velocity gate~~ landed 2026-10-31 (`hd-velocity-gate-amd`): 2 to 7 % of the HD race pass on
  AMD at 1280x800 and 1080p, identical picture; table in `docs/rendering/hd-gpu-cost.md`.
- **The Deck's 28 fps was not a GPU stretch** (2026-10-19, `docs/rendering/hd-gpu-cost.md`, last
  section): it was a stalled frame at race start, now fixed. So the gate is not chasing it.
- **Deck spans for the rest of the frame are not taken.** The gate's Deck run (race pass only,
  1280x800 / 1080p / 4K) exists; the shadow, MSAA and blur spans on the Deck do not. Whether
  the shadows (about 1.4 ms at 1080p), MSAA or the blur is the Deck's worst stretch is unknown.
- **The shadow passes are 1.4 ms of a 6.0 ms frame at 1080p** and the HD shadow map
  is another lane's; the number is recorded, nothing was changed.
- **`submit_us` was 3.8 ms on the Intel driver** in the first, cold run (CPU, not GPU;
  the repeat read 2.5 ms total CPU). Unexplained: pipeline compilation on first use
  is the guess, not checked.

## Next Steps

1. On the Deck, `OAG_RENDER_GPU_BENCH=150 OAG_RENDER_PERF=1` spans with and without
   `--msaa 4x --motion-blur high` and `--shadows off` (recipe: "Measure on the Deck" in
   `docs/rendering/hd-gpu-cost.md`; a headless `--screenshot` run with a `--features perf-probe`
   binary works on the Deck, as the gate's run did).
2. If shadows or MSAA are the Deck's limit, hand that to the queued shadow-map and
   split-bloom lanes.
