---
status: ready-to-wire
titles: [pulse, pure, hd, 2048, omega]
updated: 2026-11-26
summary: "Android and web report no refresh rate, so the benchmark verdict assumes 60 Hz; no per-mod list; HD's page not driven by hand"
categories: [frontend]
size: S (30)
difficulty: easy (40)
---

# The benchmark assumes 60 Hz on a phone and in a browser

## Open

The options page, a run, the stats screen and COPY TO CLIPBOARD are verified on the Galaxy S21
and in headless Chromium (`docs/tools/in-game-benchmark.md`, "Run on a phone and in a browser").
Left:

- **Refresh rate on Android.** winit 0.30's Android `refresh_rate_millihertz` is `None`, so
  `Host::refresh_millihertz` answers nothing and the report says 60 Hz, assumed. The S21 ran at
  120 Hz. It is not the benchmark's alone: frame pacing and the `refresh_rate` choices read the
  same value, so a reader (the activity's `Display.getRefreshRate` over JNI, in `oag-mobile`)
  changes more than this screen and wants its own check on the phone.
- **Refresh rate on the web** is also `None`; the page could time its animation frames. The
  verdict line names the rate with no "assumed"; only DETAILS says it.
- **Stats buttons are 16 grid units high**, about 24 dp on the S21. Every tap landed, edge taps
  included; taller buttons would be kinder to a thumb.
- **A per-mod list** under the MODS row (the thread's "if cheap"): not built; the row is one
  switch for every enabled mod.
- **HD's page by hand.** The page is in every title's tree (a unit test) and ran live on Pulse,
  Pulse PS2 and Pure; HD's strip-layout main menu was not driven to it, and 2048 and Omega have
  no image driven here.
- **Safari and Firefox COPY** were not tried; Safari wants the write inside the gesture's own
  call stack, and the game asks from its next frame.

## Next Steps

1. Read the display's refresh rate on Android through JNI, feed `Host::refresh_millihertz`, and
   rerun a benchmark on a 120 Hz phone (half a day).
2. Drive HD's main menu to BENCHMARK (15 minutes).
