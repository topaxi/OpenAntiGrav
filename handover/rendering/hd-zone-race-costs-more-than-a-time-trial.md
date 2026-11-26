---
status: measure
titles: [hd]
updated: 2026-11-26
summary: "HD Zone race pass +25-30% over a time trial, mostly faithful; open: Deck check, lightmap-gated Zone specular, a Zone program split"
---

# An HD Zone race costs more than a time trial on the same circuit

Measurements, the cause and the landed part:
[hd-zone-recolour.md](../../docs/rendering/hd-zone-recolour.md), "What a Zone race costs".

## Open

- **The Deck is unmeasured.** The report (Chenghou Zone settles near 75 %, time trial at 100 %)
  is more than the desk iGPU reproduces (0.96 against 1.00). Unknown: the Deck's own settings
  (motion blur, reflection, output size) and whether sound changes it.
- **Fidelity: Zone specular is gated by the lightmap's alpha.** Our Zone chunks still sample the
  lightmap and their specular reads its alpha; no Zone program on Chenghou declares the lightmap
  sampler. Visible only at stages with a non-zero sun colour (skipping the fetch moved 164,239
  pixel-units at tick 1900 of an autopilot run). Whether the original's Zone program has a
  specular lobe at all is unread.
- **A Zone program of its own for the circuit.** The race pass would be 716 us with the recolour
  compiled out and the throwaway Zone-only ceiling was about 590 us at 4K against 890; reaching
  it means proving every term the original's Zone variant computes.

## Next Steps

1. Deck, when reachable (vsync off, no frame limit; 1280x800 and 3840x2160):
   `just bench-render` does not take a mode, so run
   `OAG_RENDER_BENCH_RACE=<stem> oag-game <hd iso> --race --mode zone --track
   /data/environments/04_chenghou_project/track.vex --seed 1 --autopilot --no-audio --size
   1280x800 --ticks 5400 --screenshot out.png` and again with `--mode time_trial`; then a
   windowed race of each with `--frame-log`, the maintainer's own settings and sound.
2. Read one Chenghou Zone fragment program end to end for a specular term
   (`scripts/ps3-microcode.py fp-file`), then decide the lightmap alpha.
