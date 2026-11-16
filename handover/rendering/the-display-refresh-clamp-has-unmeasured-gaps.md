---
status: blocked-hardware
titles: [all]
updated: 2026-11-04
summary: "ADR-0084's refresh clamp: the Deck test (dock, undock, pacing, QAM limit, width baseline), Windows/macOS poll cost, Wayland mode change"
categories: [rendering]
size: S (60)
difficulty: easy (70)
---

# The display refresh clamp: what [ADR-0084](../../docs/architecture/adr/0084-the-target-is-held-to-what-the-display-shows.md) did not measure

The clamp reads the output's refresh (`oag_present::refresh`; on X11 through RandR in
`x11_refresh`) and holds the resolution target and the CPU governor's period to it.

## Open

- A real dock and undock on the Deck: launch undocked then dock (expect 60 within about two
  seconds), launch docked then undock (expect back to 90). Headless Xvfb mode changes follow;
  gamescope headless could not change its rate, so the rate change under gamescope is unproven.
- The Deck's own panel undocked: Steam's 45-90 Hz slider and a Quick Access frame limit.
- A render width against a baseline from before the clamp on the docked Deck, same scene and settings
  (the earlier before/after compared two branch builds, not `main`).
- The once-a-second poll on Windows (`GetMonitorInfo`, `EnumDisplaySettingsExW`) and macOS
  (`CGDisplayCopyDisplayMode`) runs on the frame thread and was not timed. The Linux X11 read is
  off it.
- A mode change under a Wayland compositor: winit reads the `wl_output` current mode, live by
  reading its source, but no run changed a mode.
- With no report (Android) and an external limit lifted mid-race, the measured rate stays until
  the race ends.

## Next Steps

1. Run "Deck test" below. Everything in it needs the Deck on hand and someone to dock it.
2. Time the Windows and macOS poll in `Host::refresh_millihertz`; move it to a thread like the
   X11 one if it shows up in a frame.
3. Run a mode change under a Wayland compositor and watch `display refresh in force`.

## Deck test

Setup, about 5 minutes: `just appimage-deck`, copy it over with `just deploy deck`, and run it in
Big Picture with scratch XDG dirs, so the Deck's saves and settings stay untouched. Audio is
allowed there. Use `RUST_LOG=oag_game=debug` and `--frame-log`, and set `[graphics] target_fps =
"90"` with no frame limit and vsync off, which is the default. Every step below reads the
`display refresh in force` line. Kill by PID and leave nothing running.

The dock itself takes a person, so have someone on hand for steps 1 to 3.

| Step | Do | Pass |
| --- | --- | --- |
| 1 | Launch undocked, in a race, then dock to the 60 Hz TV | 60 Hz within about 2 s, one log line |
| 2 | Launch docked, in a race, then undock | back to 90 within about 2 s |
| 3 | Redock | 60 again, no flapping |
| 4 | Docked at 3840x2160: the commit before the clamp merged, then current `main`, same scene, 90 s autopilot | current width at or above the old one, overlay GPU row under 16.7 ms when it holds |
| 5 | Docked: watch the pacing in the overlay | the frame graph is flat at 16.7 ms; the original report was uneven pacing and a falling render scale behind gamescope's limit |
| 6 | Undocked: Steam's 45-90 Hz slider, then a Quick Access frame limit | the target follows, or the gap is named here |

Step 5 is the one nothing has covered: no lane changed pacing, a nested gamescope held 60.0 fps,
and the maintainer's complaint came from a real TV.

Known values: the TV runs 59.98 Hz on `card0-DP-3`, and the panel is `eDP-1`. Under gamescope a
1920x1080 run is the same 3840x2160 surface, so it adds nothing. A rate stuck at 60 on the panel
after step 2 is the failure to look for: nothing corrects a wrong low report.

If everything passes, strike those items from "Open", and delete this file once Next Steps 2 and 3
are done too.
