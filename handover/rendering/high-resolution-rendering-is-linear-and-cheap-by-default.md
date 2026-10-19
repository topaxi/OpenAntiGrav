# 4K and 8K rendering is linear and cheap by default; the opt-in chains and two gates are left

2026-10-19, `hires-perf`. Evidence and numbers: `docs/rendering/high-resolution-cost.md`.
Omega's circuit now goes through the depth prepass (3.7 fragments a pixel down to
1.0, byte-identical, pipelined 8K 9.4 to 4.9 ms). Nothing else at 4K or 8K is a defect
on the RX 7800 XT.

## Open

- **Size-gated prepass for Pulse and Pure.** Forced on, Pulse's race pass falls 24 %
  at 4K and its 8K pipelined frame 3.4 to 2.6 ms, but at 1080p it is a wash on a
  CPU-bound frame. Needs a gate (view area, or the adapter being GPU bound) and a
  picture comparison; the PSP titles' fragment shader is not HD's.
- **Per-view gate for Omega's prepass.** A four-view split at 1280x800 (640x400 views)
  costs +20 % (1.7 to 2.05 ms) on this GPU because the second pass of draws is CPU.
  A view smaller than some area could draw plain; unmeasured on a GPU-bound adapter,
  where the prepass is the larger term.
- **The blur gather (5.6 ms at 8K) and FSR 3's render-extent dispatches (7.1 ms at 8K,
  native scale)** are the biggest opt-in costs, both arithmetic bound; skipping
  zero-weight tap fetches measured as nothing. `motion_blur_resolution = half` is the
  existing lever.
- **No windowed 4K or 8K surface measured**: this host's RADV cannot present on Xvfb,
  so the present, HUD overlay and swapchain are only covered by `--presented`'s
  resolve (454 us at 8K).
- **Steam Deck measured 2026-10-19** (`docs/rendering/high-resolution-cost.md`, "Steam Deck,
  4K"): native 4K fails 60 Hz on every title but Pure; FSR 1 adds 7.0 ms (about 4.5 at a steady
  clock, estimated), FSR 3 17.4 to 20.0 ms at a 4K output. Open there: the Omega prepass before
  and after on the Deck, the resolve at a steady 1,600 MHz, GPU memory, a windowed run on a real
  4K display (none attached), Omega/Pure/2048 at scale 67.

## Next Steps

1. Run the Deck's Omega whole race (`bench-render.py run omega --size 3840x2160`) with a binary
   built before the Omega prepass commit, for the before/after (the Deck's after is in the doc).
2. If Pulse/Pure's prepass is wanted, gate it on view pixels and compare 12 stills
   byte for byte before enabling it.
