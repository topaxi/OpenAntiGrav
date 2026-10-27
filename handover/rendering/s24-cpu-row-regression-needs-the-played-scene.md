---
status: blocked-hardware
titles: [hd]
updated: 2026-10-27
summary: "S24 CPU 9 -> 12-13 ms (10-22 vs 10-27 build) not reproduced in a scripted race; Deck is faster; needs the phone in the played scene"
categories: [rendering, tooling]
size: S (40)
difficulty: medium (30)
---

# The S24's CPU row rise: re-measure in the maintainer's played scene

The maintainer read the S24's perf meter `CPU` at about 9 ms on `e5c996527` (2026-10-22) and 12 to
13 ms on `78275344a` (2026-10-27), HD/Fury Vineta K, from the flyby on. The 2026-10-27 lane
measured both builds on the Deck (newer is 1.5 to 2.7 ms faster, `submit`) and a scripted race on
the S24 (no gap beyond run-to-run spread, n too small: the phone went back to the maintainer).
Numbers and commands: `docs/rendering/frame-thread-cpu.md`, "The reported phone regression".

## Open

1. A thermally matched old/main A/B on the S24 in the scene the maintainer read: started from the
   menus, with touch and sound, the moment the 9 and 12-13 were read. Blocked on the phone.
2. If it reproduces there and not in `--race`, the difference is the route (front end, audio,
   touch overlay); bisect `e5c996527..78275344a` on the phone (`git log --first-parent
   e5c996527..78275344a -- crates`, 88 commits, about 7 APK builds of 5 to 10 minutes).
3. Split finish on a big.LITTLE phone: `submit` swings 3.5 to 7.7 ms second to second with the
   split, steady 6.4 without, same mean. If (1) or (2) names it, the maintainer's preferred fix is
   a topology probe (split only when every core's `cpu_capacity` is equal), not a Deck-only switch.

## Next Steps

1. Install main (`just deploy android`), then per variant, interleaved, from 27.0 C:
   `scripts/android-race-phases.sh --serial R3CX707JA9T --out main-1.log --cool 270`
   (plus `-- --no-audio`, `-- OAG_SPLIT_FINISH=0`). Read the median of `cpu_ms - present_ms`
   over the `frame-log` lines after the first four.
2. Ask the maintainer how the race was started and when the meter was read; repeat that scene by
   hand on both builds with the overlay on, three times each.
3. The old build needs the launch file to be scripted: apply `crates/mobile/src/android.rs`'s
   `take_launch_args` and `main_body.rs`'s use of it on `e5c996527` (it applies cleanly).
