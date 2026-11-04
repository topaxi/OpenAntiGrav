---
status: blocked-hardware
titles: [all]
updated: 2026-11-04
summary: "ADR-0084's refresh clamp: Deck undocked panel and QAM limit, a real dock and undock, Windows/macOS poll cost, Wayland mode change, main width baseline"
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
- A render width against a `main` baseline on the docked Deck, same scene and settings
  (the earlier before/after compared two branch builds, not `main`).
- The once-a-second poll on Windows (`GetMonitorInfo`, `EnumDisplaySettingsExW`) and macOS
  (`CGDisplayCopyDisplayMode`) runs on the frame thread and was not timed. The Linux X11 read is
  off it.
- A mode change under a Wayland compositor: winit reads the `wl_output` current mode, live by
  reading its source, but no run changed a mode.
- With no report (Android) and an external limit lifted mid-race, the measured rate stays until
  the race ends.

## Next Steps

1. Ask for the Deck and run the dock and undock sequence above with `RUST_LOG=oag_game=debug`,
   watching `display refresh in force`.
2. Time the Windows and macOS poll in `Host::refresh_millihertz`; move it to a thread like the
   X11 one if it shows up in a frame.
3. Run `main` and the branch at `target_fps = 90` docked and compare the render width.
