---
status: measure
titles: [hd]
updated: 2026-11-25
summary: "S21 HD is GPU-bound, not serialised; half its GPU frame is untimed; advert cards cost about 3 ms at render scale 25: pick the levers"
categories: [tooling]
difficulty: medium (40)
---

# HD on the S21 is GPU-bound and the advert cards cost it about 3 ms

Evidence and method: `docs/tools/android.md`, "What the frame thread waits for". In short: the frame
thread's 14 to 20 ms in `Queue::present` is Mali's `vkQueueSignalReleaseImageANDROID` waiting for
the **previous** frame's GPU fence, so the CPU already runs one frame ahead; the frame is the GPU's
time, about twice what the timestamps add up to. No improvement was implemented; this thread lists
the leads for whoever picks them.

## Open

- **Where the other half of the GPU frame goes.** The fence period (35 ms in the traced run) is
  about twice the timed passes plus `bloom_ms` (about 15 to 18 ms). Candidates not separated:
  untimed passes (the composite, the resolve), Mali's vertex and tiling work, which its timestamps
  miss (`android.md`, "Where an HD frame goes on the S21"), and SurfaceFlinger composing this
  landscape surface on the same GPU (`composition=CLIENT`). Measure with the skip toggle per pass
  (`OAG_SKIP_PASSES=resolve,composite,hud` one at a time, `OAG_SKIP_PASSES_TOGGLE=10`,
  `scripts/android-skip-windows.py`), from AP 33 C or less.
- **The advert cards** (about 2.6 ms of frame and 4 ms of the phases on the S21, 1.6 ms on a
  Radeon 610M, 0.16 ms on an RX 7800 XT): eight 512x256 targets redrawn every frame whatever the
  render scale, eight times the scene's pixels at render scale 25. Levers to weigh, none measured:
  draw only the cards whose quads pass the PVS and the frustum this frame; redraw a card only when
  its animation moved (some cards may be still); size the targets with the render scale (a setting
  of ours, as REFLECTION RESOLUTION is). How often the original redraws a card is not read
  (`docs/ghidra/functions/ps3-hdfury-eu/billboards.md` has the size, not the rate). Check the
  picture against the same frame on the desktop (`docs/tools/frame-compare.md`).
- **The maintainer's desktop reading** of `ADVERT CARDS` 1.4 ms: on this workstation that figure
  is the integrated Radeon 610M's, not the 7800 XT's (0.16 ms). Check which adapter the overlay's
  renderer line names on the maintainer's session.
- **The CPU side**, 11.5 to 12.9 ms a frame: wgpu-core's encoding at `CommandEncoder::finish`
  (14 % of the frame thread's wall time in a profiled run) and `Queue::submit` (7 %), the draw
  lists (8 %); `memset` inside the Mali driver per draw is the top single function. It only matters
  once the GPU frame falls under it.

## Next Steps

1. Skip-toggle the resolve, the composite and the HUD on the S21 (one run each, about 4 minutes
   with cooling) to account for the untimed half.
2. Decide among the advert levers with the measured cost above; the PVS/frustum one keeps the
   picture identical by construction.
