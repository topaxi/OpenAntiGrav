---
status: measure
titles: [pulse, pure, hd, 2048, omega]
updated: 2026-11-25
summary: "Benchmark options page, plain stats screen and COPY TO CLIPBOARD work on desktop; the phone and browser halves are type-checked only"
categories: [frontend]
size: S (20)
difficulty: easy (30)
---

# The benchmark options screen and COPY are unrun on a phone or a browser

## Open

Landed (`docs/tools/in-game-benchmark.md`, "Benchmark options" and "Reading the screen"): the options page before every run, MODS OFF as the game as shipped, the verdict and DETAILS stats screen with every line through a string id, COPY TO CLIPBOARD on desktop (checked under Xvfb with `xclip`), F11 and the touch overlay left out of a run.

Not yet run where it matters:

- **Android.** `oag_mobile::clipboard` (JNI `ClipboardManager`) type-checks for `aarch64-linux-android` and has not run on a phone. Also the options page by touch on the S21 and the stats screen's four buttons at phone size.
- **Web.** `navigator.clipboard.writeText` through `js_sys` type-checks for `wasm32` and has not run in a browser; the promise is not awaited, so a permission refusal after the call shows COPIED.
- **A per-mod list** under the MODS row (the thread's "if cheap"): not built; the row is one switch for every enabled mod.
- **HD's page by hand.** The page is in every title's tree (a unit test) and ran live on Pulse, Pulse PS2 and Pure; HD's strip-layout main menu was not driven to it, and 2048 and Omega have no image driven here.

## Next Steps

1. Run a benchmark on the S21 from the BENCHMARK OPTIONS page by touch, press COPY TO CLIPBOARD, paste into another app (15 minutes with the phone on `om`).
2. Click COPY in the web build on a secure page and paste (10 minutes).
3. If either refuses, fix `crates/mobile/src/android.rs::clipboard_set` or `crates/game/src/app/clipboard.rs::write` (the web one).
