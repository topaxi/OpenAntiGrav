---
categories: [rendering, tooling]
---

# The Deck's paused CPU row: measured at 4.5 ms here, the maintainer's 8 to 10 ms not reproduced

2026-10-21 (`deck-frame-cpu`, then `deck-read`). Findings and every Deck command are in
`docs/rendering/frame-thread-cpu.md` ("On the Steam Deck"). On the Deck, paused, the frame
thread costs 5.3 to 5.5 ms (Pulse and HD), about 4.5 ms of it the meter's `CPU` row; 55 to 60 %
of it is `Queue::submit`, flat from 50 to 150 % render scale, so it is not waiting on the GPU.
Effective user clock was 1.5 GHz on battery.

## Open

- The maintainer's own 8 to 10 ms `CPU` row. This lane read 4.5 to 6.0 on both titles with a copy
  of the maintainer's settings, so the gap is in their session (clock, TDP, a different profile).
- Why turning off any one of SMAA, medium blur or the `psp-3000` filter on Pulse removes the same
  1 ms from `submit` (all three off is no better). A step three chained post passes trip
  together; look in `oag-post` (intermediate targets, wgpu-core init actions).
- Whether a paused scene needs re-recording at all (render bundles or a held scene). Not attempted.

## Next Steps

1. Ask the maintainer to launch from Steam with `--frame-log` in the launch options and send the
   `frame-phases` and `frame-log` lines (paused and racing), plus
   `cat /sys/devices/system/cpu/cpu*/cpufreq/scaling_cur_freq` during the run.
2. If it is the clock: nothing to fix in code; consider a lighter post chain in the Deck's default
   profile (the ablation is 1.3 to 1.9 ms of 5.5).
3. Host command to compare the post chain: `just bench-render pulse --repeat 3` with and without
   `--reconstruction off`, `submit_us` column.
