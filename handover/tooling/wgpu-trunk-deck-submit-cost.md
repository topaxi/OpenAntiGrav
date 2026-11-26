---
status: measure
titles: [hd]
updated: 2026-11-26
summary: "wgpu trunk costs 0.05-0.07 ms a frame at 1280x800 on the Deck (submit up, record down); not separated from #10319"
categories: [rendering]
size: S (30)
difficulty: medium (30)
---

# wgpu trunk's submit cost on the Steam Deck

Numbers and commands:
[`docs/architecture/dependency-upgrades.md`](../../docs/architecture/dependency-upgrades.md), "wgpu trunk on the Steam
Deck". Against 30.0.2, trunk records about 0.07 to 0.10 ms faster and submits 0.10 to 0.17 ms slower; the 1280x800
frame, which is CPU-bound offscreen, is about 1 % longer. Picture identical.

## Open

- Which commit costs the submit time: #10319 or something else in trunk (hypothesis #10319, confidence 40).

## Next Steps

1. Build 30.0.2 with only gfx-rs/wgpu#10319 backported (the measuring build of
   [`docs/tools/android.md`](../../docs/tools/android.md), "gfx-rs/wgpu#10319 on 30.0.2: what one barrier buys") and run
   it against 30.0.2 on the Deck:
   `OAG_RENDER_BENCH_RACE=<stem> oag-game <hd iso> --race --mode single_race --seed 1 --autopilot --no-audio
   --log-every 0 --size 1280x800 --ticks 40000 --screenshot unused.png`, four interleaved rounds each, compare
   `record_us` and `submit_us`.
2. Worth it only if the cost grows with wgpu 31, or a lane needs the 1280x800 CPU frame: at under 1 % of the 90 Hz
   budget it blocks nothing.
