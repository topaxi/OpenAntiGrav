---
status: measure
titles: [hd]
updated: 2026-11-26
summary: "Undocked Deck parks at 1040 MHz in 4K Zone, cause unnamed (docked it does not); probe causes TT stutter: no; Zone specular"
---

# An HD Zone race costs more than a time trial on the same circuit

Measurements, the cause and the landed part:
[hd-zone-recolour.md](../../docs/rendering/hd-zone-recolour.md), "What a Zone race costs".

## Open

- **Ranked first: dynamic resolution settles low because the Deck's GPU clock falls with it**
  (confidence 90). At 3840x2160 unpinned, Zone holds about 0.80 and a time trial 0.93; with the
  clock pinned at 1600 MHz, 0.96 and 0.99. The cause is measured, the fix is not designed: a rise
  that GPU milliseconds cannot veto (a probe kept only if no frame is late), which changes the
  chosen resolution, so the picture changes. It costs power: unpinned DRS drew 3 to 6 W less.
  Needs and acceptance: [dynamic-resolution.md](../../docs/rendering/dynamic-resolution.md),
  "What a rise rule that survives this needs". TARGET FPS `original` (HD's width throttle) also
  steps on a GPU duration: unmeasured on the Deck under a falling clock. The race pass's
  +1.60 ms at 4K is the answered part, and it is faithful.
- **What parks the undocked Deck at 1040 MHz is still not named.** Not power (about 8 W of 15) and
  not CPU; the demand governor is a hypothesis (45). Docked, the park does not happen: the clock sits at
  or near its ceiling and the 15 W package limit trims the dips
  ([dynamic-resolution.md](../../docs/rendering/dynamic-resolution.md), "On the Steam Deck: what
  holds the clock, and the time trial's late seconds").
- **probe causes TT stutter: no** (confidence 85, 4K, docked, same page). The
  probe rise is on the unmerged branch `drs-probe-parked`; its own time trials were docked and its
  Zone runs undocked, which its notes do not say.
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

0. Deck undocked, on AC: Zone at 3840x2160 four times (before, after) with the sampler of that
   section, refusing to run if `DP-3` reads connected. It settles the profile, throttle bits and
   thermal state during the park.
1. Maintainer's call first: is a higher resolution worth 3 to 6 W on a Deck? If yes, measure how
   long the Deck's governor takes to raise the clock after a step up (a DRS-off run alternating a
   light and a heavy size, sampling `freq1_input`), then design the probe as an ADR and measure it
   with the reproduction on the page above.
2. Read one Chenghou Zone fragment program end to end for a specular term
   (`scripts/ps3-microcode.py fp-file`), then decide the lightmap alpha.
