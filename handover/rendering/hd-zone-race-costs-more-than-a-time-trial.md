---
status: measure
titles: [hd]
updated: 2026-11-26
summary: "Probe rise landed (undocked Deck 4K Zone 0.84 -> 0.90); the undocked 1040 MHz park is unnamed; TT stutter is not the probe; Zone specular"
---

# An HD Zone race costs more than a time trial on the same circuit

Measurements, the cause and the landed part:
[hd-zone-recolour.md](../../docs/rendering/hd-zone-recolour.md), "What a Zone race costs".

## Open

- **Ranked first: the probe rise reaches 0.90, not 0.96.** ADR-0096's probe takes unpinned
  Zone at 3840x2160 from 0.84 to 0.90 with no more size changes or late seconds, at about 3 W more
  ([dynamic-resolution.md](../../docs/rendering/dynamic-resolution.md), "The probe rise"). The
  rest of the gap is the governor: in 38 of 71 steps up it kept the clock at 1040 MHz with the GPU
  86 to 100 % busy, so a probe from there reads a steady clock and reverts. Unknown why the
  governor parks there (power profile, the CPU's share of the APU budget, gamescope's frame pacing
  are all unread). TARGET FPS `original` (HD's width throttle) also steps on a GPU duration: still
  unmeasured on the Deck under a falling clock. ADR-0096 is accepted: the maintainer took the
  power trade (8 to 11 W) once the time trial's stutter was shown not to be the probe.
- **The time trial at 4K loses frames on the Deck** (58 to 191 a minute on both binaries, 2026-11-26),
  GPU bound at the top clock with the scale at 0.98. Not seen on the earlier day (0.93 then).
  The probe is not the cause (next item).
- **What parks the undocked Deck at 1040 MHz is still not named** (answered docked only). Not power (about 8 W of 15) and
  not CPU; the demand governor is a hypothesis (45). Docked, the park does not happen: the clock sits at
  or near its ceiling and the 15 W package limit trims the dips
  ([dynamic-resolution.md](../../docs/rendering/dynamic-resolution.md), "On the Steam Deck: what
  holds the clock, and the time trial's late seconds").
- **probe causes TT stutter: no** (confidence 85, 4K, docked, 2+2 in reversed order, same page).
  The probe's own measurements mixed docked time trials with undocked Zone runs.
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

0. Open, not scheduled (the maintainer keeps the Deck docked): the undocked clock park. About
   8 minutes, Deck undocked and on AC. Build "no probe" from main and "probe" from
   `drs-probe-parked` with main merged in (touch the differing files if the two share a target
   directory). Run each under `~/oag-perf/deck-run.sh`, order probe, no probe, no probe, probe,
   on a copy of the player's settings with `pause_on_focus_loss = false`:

   ```sh
   [ "$(cat /sys/class/drm/card0-DP-3/status)" = disconnected ] || exit 3
   [ "$(cat /sys/class/drm/card0/device/power_dpm_force_performance_level)" = auto ] || exit 3
   RUST_LOG=info,oag_game::app::session::timing=trace \
   gamescope --backend headless -W 3840 -H 2160 -w 3840 -h 2160 -r 60 -- \
     oag-game <hd iso> --race --no-intro --autopilot --frame-log --mode zone \
     --track /data/environments/04_chenghou_project/track.vex --script 70:quit
   ```

   Stamp each stderr line with `$EPOCHREALTIME`, and beside it sample at 20 Hz what
   [dynamic-resolution.md](../../docs/rendering/dynamic-resolution.md) lists under "Method"; what to
   read from the parked samples is under "Open: the undocked clock park" there.
1. To get past 0.90 (undocked only): sample `pp_power_profile_mode` and the CPU clock beside the GPU clock in one
   unpinned 4K Zone run (the commands in "The probe rise") and find what holds 1040 MHz.
2. Read one Chenghou Zone fragment program end to end for a specular term
   (`scripts/ps3-microcode.py fp-file`), then decide the lightmap alpha.
