---
status: measure
titles: [hd]
updated: 2026-11-26
summary: "Deck 4K: DRS settles Zone at 0.80, time trial 0.93; pin the GPU clock to test why. Also Zone specular, a program split"
---

# An HD Zone race costs more than a time trial on the same circuit

Measurements, the cause and the landed part:
[hd-zone-recolour.md](../../docs/rendering/hd-zone-recolour.md), "What a Zone race costs".

## Open

- **Ranked first, a hypothesis (confidence 50): dynamic resolution settles low because the
  Deck's clock falls with it.** At 3840x2160 with the reporter's settings Zone holds about 0.80
  (time trial 0.93), while with DRS off Zone runs at 58.8 fps at full size. The GPU clock is
  1104 MHz in the first case and 1574 in the second; the controller would be reading the slower
  clock's milliseconds as no room to rise. Evidence for and against:
  [dynamic-resolution.md](../../docs/rendering/dynamic-resolution.md), "On the Steam Deck". The
  race pass's +1.60 ms at 4K is the answered part, and it is faithful.
- **Only if the pin confirms it:** probe one step up after a long hold at the target rate and
  keep it if no frame is late, or judge a rise from late frames as well as GPU milliseconds.
  Either changes the chosen resolution, so the picture changes. TARGET FPS `original` (HD's
  width throttle) also steps on a GPU duration: unmeasured on the Deck.
- **The reporter's setup is not known.** At 1280x800 and 1920x1080 both modes hold 100 %, so the
  75 % seen was a larger output or a lower clock than here. A time trial "at 100 %" was not
  reproduced at 4K (0.93).
- **Fidelity: Zone specular is gated by the lightmap's alpha.** Our Zone chunks still sample the
  lightmap and their specular reads its alpha; no Zone program on Chenghou declares the lightmap
  sampler. Skipping the fetch changed the picture at ticks 1900 and 3100 of an autopilot run
  (ImageMagick `AE` 6,117 and 1,098) and not at tick 700, which fits a term that needs a non-zero
  stage sun (not isolated). Whether the original's Zone program has a specular lobe at all is
  unread.
- **A Zone program of its own for the circuit.** The race pass would be 716 us with the recolour
  compiled out and the throwaway Zone-only ceiling was about 590 us at 4K against 890; reaching
  it means proving every term the original's Zone variant computes.

## Next Steps

1. Test the hypothesis: pin the GPU clock with DRS on. Steam's quick-access manual GPU clock
   (the maintainer's hands) or, with the lead's OK, root writing
   `/sys/class/drm/card0/device/power_dpm_force_performance_level`. Run the headless gamescope
   line on the page above at 3840x2160, `-r 60`, both modes, 70 s, pinned high and unpinned.
   Confirmed if Zone pinned settles near 0.95; refuted if it stays near 0.80.
2. Only if confirmed: design the rise rule above and measure it the same way.
3. Read one Chenghou Zone fragment program end to end for a specular term
   (`scripts/ps3-microcode.py fp-file`), then decide the lightmap alpha.
