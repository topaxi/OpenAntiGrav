# ADR-0084: The resolution target is held to what the display shows

## Status

**Accepted** (2026-11-04). Replaces the premise "this build cannot ask a surface what
its refresh is" in [ADR-0044](0044-the-residual-is-a-learned-upper-bound.md) (its
`RESIDUAL_SHARE` consequence) and in `docs/rendering/dynamic-resolution.md`, "The target
is held to the frame limiter". ADR-0044 is not edited.

## Context

`drs::Target::at_most` held the controller's target to our own frame limiter and
stopped at `Vsync::On`, because nothing could say what the output refreshes at and a
wrong clamp (60 on a 144 Hz panel) is worse than none. A Steam Deck docked to a 60 Hz TV
under gamescope runs `target_fps = 90` with no limiter: gamescope holds the output to 60,
the budget is a third too tight, and the controller sheds resolution for a rate the
output forbids. The CPU governor's budget (`Sample::period`) had the same blind spot.

## Decision

1. **The monitor's report is the rate, gamescope's included.** Asked again once a second
   (a clock check in `Session::feed_display`) through `Host::refresh_millihertz`. **On X11
   (a Deck in Game Mode included) it is the X server's own answer**, read through RandR by a
   thread of its own (`x11_refresh`: its own connection, the CRTC under most of the window,
   `dot_clock / (htotal * vtotal)`, once a second, published in an atomic), because winit's
   report freezes after window creation there: `current_monitor()` changes only on a
   synthetic `ConfigureNotify`, gamescope sends none, and no RandR event reached winit in any
   run. Under headless gamescope (1280x800 to 1920x1080) winit stayed at 89887 mHz while the
   thread read 89887 then 89926; on Xvfb, with modes of 59975 and 90023 mHz switched by
   RandR, the log changed 60, 90, 60 Hz within 0.97, 0.99 and 1.04 s of each switch (thread
   poll plus session poll, at most about 2 s). Everywhere else, and before the first answer, it is winit's
   `MonitorHandle::refresh_rate_millihertz` (Wayland: read from the compositor's `wl_output`
   current mode, live by reading winit's source; a mode change under a Wayland compositor was
   not run). Measured on a Steam Deck docked to a
   4K LG TV in Game Mode: DRM `card0-DP-3` 3840x2160 (EDID preferred timing 594 MHz pixel clock,
   60.000 Hz), gamescope's mode save file `3840x2160@60`, its log `Updating mode for xwayland
   server #1: 3840x2160@60` and the WSI layer's `Swapchain received new refresh cycle:
   16.67ms`, `xrandr` 59.98 Hz on both Xwaylands, and winit 59980 mHz. The report agrees
   with the output. A report below 20 Hz after rounding (a driver's 0 or 1 for "hardware
   default") is no report. Not measured: the Deck's own panel undocked (Steam's 45-90 Hz
   slider; RandR carries the mode Xwayland is given, which a change of the slider is
   expected to change; a change of report resets the measurement), and a Quick Access frame limit (gamescope limits below the report;
   the clamp stays at the output rate, as a wrong high clamp only keeps today's behaviour).
2. **A measurement may raise the report but never lower it** (`hz = max(report,
   measured)`). A measurement below a report is a loop that missed vblanks or a limiter outside
   the game, and a wrong low clamp costs frames for the session (a low target loosens the resolution
   budget, so the scale rises until the loop fills it; a target that is too high is what costs
   pixels). The old measurement did exactly
   that on the docked Deck: with `vsync = off` (the default; `Immediate`, which gamescope's
   WSI still blocks in for about 15-20 ms a frame) it read 37-52 Hz on the 60 Hz TV.
3. **Measurement is evidence only where the mode can prove it.** A frame counts only under
   `Fifo` (the one present mode that holds a frame to the refresh; under `Mailbox` and
   `Immediate` an interval is the loop's own pacing, a 240 fps reading on a 60 Hz panel), and
   only when its busy work (`interval - wait + gpu`) is at most 40 % of the interval: work that
   fits in half the interval would have made the earlier vblank, so the interval is not a
   doubled one. 90 such intervals within 8 % spread (`(p90 - p10) / median`, i.e. plus or
   minus 4 %) give the rate. Jitter (variable refresh) and no evidence both give no answer,
   and no answer means no clamp. With no report (Xvfb, Android, web) this is the only source.
4. **The measurement is dropped when its state changes**: a change of the report, every surface
   configure (the output counts them: a resize, an outdated or lost surface), a vsync change, a change of present mode and the start of every race (and any
   frame outside a race). A rate measured under a clamp cannot recover on its own (the clamp
   fills the GPU and removes its own evidence), so a lifted external limit with no report to
   floor it stays measured until the race ends: bounded by the race, accepted.
5. **Design choice for default settings.** Under default `vsync = off` no frame is evidence
   (point 3), so a docked Deck is clamped only because the report is accepted. Gamescope's own
   atoms are no alternative: `GAMESCOPE_DISPLAY_REFRESH_RATE_FEEDBACK` (60 here) sits on
   Steam's X server `:0`, a game on `:1` sees none of the refresh atoms, and the report is the
   same number. Pacing under gamescope is not changed. Where the report is missing and the mode
   is not `Fifo`, no clamp: nothing proves the rate, and `vsync = on` is what would.
6. The target is clamped with `refresh::held_target` and the governor's period is
   `refresh::held_hz`, the lower of the limiter and the display. `Session::presentation_hz`
   takes the display rate before the simulation-rate fallback.

## Consequences

- A 144 Hz panel keeps 144; a 60 Hz output holds a 90 target to 60, with a report or under
  `Fifo`.
- The governor is live under `Vsync::On` on any machine with a report, where it used to be
  inert (it had no period); its budget is the display's period.
- The measurement needs a GPU timestamp reading and a fresh one (at most four frames old); an
  adapter without one relies on the report alone.
- **What covers a wrong report.** The report is a floor, and the only thing that lifts a wrong
  low one is a measurement, which needs `Fifo` and busy work at most 40 % of the true period
  (4.4 ms at 90 Hz) in the frames it sees. That is a light scene: a clamped controller fills its
  budget, and every pause and race start drops the evidence. So a wrong low report is in
  practice not corrected; the cost is frames, bounded by the report being wrong, and the
  sources are the platform's own (RandR, `wl_output`, DRM through the OS). A wrong high report
  keeps today's behaviour. Where no report exists (Android) a half-rate read would be the
  answer, so the gate is tighter there (busy at most 25 %, `NO_REPORT_BUSY_SHARE`).
- The settings menu's `warn_when` reads settings only and cannot see a runtime refresh,
  so the row does not warn about the display pairing; the clamp is logged at `debug`.
