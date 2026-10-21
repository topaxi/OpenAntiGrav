---
categories: [rendering, tooling]
---

# The Deck's paused CPU row is 8 to 10 ms; this host's is 1.2 ms - what to read on the Deck

2026-10-21 (`deck-frame-cpu`). The findings are in `docs/rendering/frame-thread-cpu.md`. On the
desktop host a paused race costs the frame thread about 1.2 ms (Pulse) and 1.5 ms (HD), 47 to 58 %
of it in `Queue::submit`, and a 9 to 10 ms Deck figure is 6 to 8 times that. Either the Deck is
that much slower per draw, or the CPU row includes time blocked on the GPU in a way this host
cannot show.

## Open

- Which phase carries the Deck's 8 to 10 ms. Not measured; the host split cannot tell.
- Whether `submit` or a `write_buffer` blocks on the GPU on the Deck's shared memory.
- Whether a paused scene needs re-recording at all: a paused race draws identical pixels, so
  re-encoding the scene (321 draws on Pulse, 617 on HD) every frame is the bulk of the CPU. A
  "hold the last scene, redo only the composite" path would remove it, but the pause menu and
  HUD move, the exhaust and sky are wall-clock driven, and the result must stay pixel-identical:
  not attempted.
- wgpu-core's per-texture init actions (`register_init_action` plus a `retain`) with bindless
  texture arrays bound on every pass.

## Next Steps

1. On the Deck, run Pulse and HD with `--frame-log --race --autopilot --no-audio`, paused and
   racing (the race starts paused; a `3:start` script tap releases it), and read the
   `frame-phases` line: `scene`, `submit`, `readback`, `present` and `total` in ms.
2. If `submit` or `readback` dominate and scale with the GPU (compare `--render-scale 50` and
   `150`), the CPU row is waiting on the GPU: look at `Queue::submit` and the timestamp map.
3. If `scene` plus `submit` dominate and do not move with the render scale, the cost is per draw:
   count draws with `just bench-render <title>` (`draws` column) and cut passes or draws.
4. Measuring command (host): `gamescope --backend headless -W 1280 -H 800 -w 1280 -h 800 -r 60
   -- oag-game <image> --race --autopilot --no-audio --frame-log --script "12:quit"`.
