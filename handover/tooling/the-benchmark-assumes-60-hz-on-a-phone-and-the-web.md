---
status: ready-to-wire
titles: [pulse, pure, hd, 2048, omega]
updated: 2026-11-26
summary: "The web reports no refresh rate, so its benchmark verdict assumes 60 Hz; 60 fps on 120 Hz reads CHOPPY; no per-mod list; HD's page not driven"
categories: [frontend]
size: S (30)
difficulty: easy (40)
---

# The benchmark assumes 60 Hz in a browser

## Open

The options page, a run, the stats screen and COPY TO CLIPBOARD are verified on the Galaxy S21
and in headless Chromium (`docs/tools/in-game-benchmark.md`, "Run on a phone and in a browser").
Left:

- **Refresh rate on the web** is `None`; the page could time its animation frames. The
  verdict line names the rate with no "assumed"; only DETAILS says it.
- **Stats buttons are 16 grid units high**, about 24 dp on the S21. Every tap landed, edge taps
  included; taller buttons would be kinder to a thumb.
- **A steady 60 fps on a 120 Hz screen reads CHOPPY.** The verdict counts frames longer than
  one refresh, so the S21 (about 62 fps, VSYNC on) says "1276 of 1276 frames took longer than one
  screen refresh" ([android.md](../../docs/tools/android.md), "The display's refresh rate"). The
  rule is the same on a desktop's 120 or 144 Hz monitor; whether the words should judge against
  the rate the player asked for is a maintainer's call.
- **A per-mod list** under the MODS row (the thread's "if cheap"): not built; the row is one
  switch for every enabled mod.
- **HD's page by hand.** The page is in every title's tree (a unit test) and ran live on Pulse,
  Pulse PS2 and Pure; HD's strip-layout main menu was not driven to it, and 2048 and Omega have
  no image driven here.
- **Safari and Firefox COPY** were not tried; Safari wants the write inside the gesture's own
  call stack, and the game asks from its next frame.

## Next Steps

1. Time animation frames on the web and feed the same report (half a day).
2. Drive HD's main menu to BENCHMARK (15 minutes).
