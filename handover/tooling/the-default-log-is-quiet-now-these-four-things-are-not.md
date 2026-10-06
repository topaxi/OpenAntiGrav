# The default log is quiet now; two things in it are still not right

2026-10-02. A launch to a race used to print 260-440 lines at the default
filter (`oag=debug`); it now prints 6-19 of our own and about 19 from the
graphics stack, headless and debug build. The rule, the `loader_log` helper and
the `RUST_LOG` recipes are in
[`docs/architecture/logging.md`](../../docs/architecture/logging.md). These are
what that work found and did not touch.

2026-10-06: the ghost's `@invariant`/`Equal` warn (`crates/render/shaders/ghost.wesl`)
and the doubled track-audio report (`oag_raceplay::load::audio` logged its
reports early *and* through the race's full report) are fixed; two remain.

## Open

1. **`frame: N ms` is a per-frame warn** (`crates/game/src/main/session/frame.rs`,
   the 40 ms stutter line). On software Vulkan (lavapipe on Xvfb, 5-20 fps) it
   fired 1,520 times in a 90 s race; on a real GPU it is silent until the
   machine stutters. It pairs with the audio-health warns on purpose (its own
   comment), so it stayed at `warn`. A rate limit (the first, then one summary
   every few seconds) keeps that pairing without the flood.
2. **A DLC pack that fails to open is not a warning on the race path.**
   `race/load.rs` and `boot.rs` push `dlc: {problem}` into the report with no
   absence phrase, so `loader_log` logs it at `debug`; only the windowed
   placeholder path warns. Either add a phrase to `loader_log::ABSENCE` or warn
   at the producer.

Also unsettled and smaller: a partial absence such as HD's `track.vex: 70 of 126
mesh node(s) drawn ... 56 addressed no chunk` is `debug` because only
`(0 triangle(s))` is treated as an absence; and the four Vulkan-loader `ERROR`
records on a machine with the `lsfg-vk` layer installed are the loader's, not
ours.

## Next Steps

1. Rate-limit the stutter warn (item 1); 20 minutes, one call site.
2. Item 2 is a few lines; do it next time `loader_log` is touched.
