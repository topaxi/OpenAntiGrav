---
status: measure
titles: [hd]
updated: 2026-11-26
summary: "Deck 4K: DRS settles Zone at 0.80, time trial 0.93; lead: the falling GPU clock holds the controller down. Also Zone specular, a program split"
---

# An HD Zone race costs more than a time trial on the same circuit

Measurements, the cause and the landed part:
[hd-zone-recolour.md](../../docs/rendering/hd-zone-recolour.md), "What a Zone race costs".

## Open

- **Ranked first: dynamic resolution settles low because the Deck's clock falls with it**
  (confidence 70). At 3840x2160 with the reporter's settings Zone holds about 0.80 (time trial
  0.93), while with DRS off Zone runs at 58.8 fps at full size. The GPU clock is 1104 MHz in the
  first case and 1574 in the second, and the controller reads the slower clock's milliseconds as
  no room to rise. Measurements and the weak part of the evidence:
  [dynamic-resolution.md](../../docs/rendering/dynamic-resolution.md), "On the Steam Deck".
  Ideas, none built or measured: probe one step up after a long hold at the target rate and keep
  it if no frame is late; or judge a rise from late frames as well as GPU milliseconds. Either
  changes the chosen resolution, so the picture changes. TARGET FPS `original` (HD's width
  throttle) steps on a GPU duration too and may or may not do better: unmeasured on the Deck.
  To measure a change, run the headless gamescope line on that page at 3840x2160, `-r 60`,
  both modes, 70 s, and compare the settled scale, frame rate and `sclk`
  (`/sys/class/drm/card0/device/pp_dpm_sclk`).
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

1. Decide whether the controller should probe up (above), then measure it on the Deck.
2. Read one Chenghou Zone fragment program end to end for a specular term
   (`scripts/ps3-microcode.py fp-file`), then decide the lightmap alpha.
