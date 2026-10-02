# The default log is quiet now; four things in it are still not right

2026-10-02. A launch to a race used to print 260-440 lines at the default
filter (`oag=debug`); it now prints 6-19 of our own and about 19 from the
graphics stack, headless and debug build. The rule, the `loader_log` helper and
the `RUST_LOG` recipes are in
[`docs/architecture/logging.md`](../../docs/architecture/logging.md). These are
what that work found and did not touch.

## Open

1. **`frame: N ms` is a per-frame warn** (`crates/game/src/main/session/frame.rs`,
   the 40 ms stutter line). On software Vulkan (lavapipe on Xvfb, 5-20 fps) it
   fired 1,520 times in a 90 s race; on a real GPU it is silent until the
   machine stutters. It pairs with the audio-health warns on purpose (its own
   comment), so it stayed at `warn`. A rate limit (the first, then one summary
   every few seconds) keeps that pairing without the flood.
2. **A genuine shader note is in the log as a wgpu warn.** `Vertex shader with
   entry point vs_main outputs a @builtin(position) without the @invariant
   attribute and is used in a pipeline with Equal` appears twice per debug race:
   a depth-`Equal` pass whose vertex stage is not invariant can z-fight on some
   drivers. It is ours to fix in the WGSL, not to filter. The Vulkan validation
   `PERFORMANCE` lines beside it are a debug-build artefact.
3. **The track-audio report prints twice per race**
   (`track audio: N node(s) name ... and play nothing`, `crates/game/src/audio/sfx/track.rs`):
   two report passes over the same circuit. Harmless, and it doubles the
   warns that line produces.
4. **A DLC pack that fails to open is not a warning on the race path.**
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
2. Mark `vs_main` position `@invariant` in the Equal-depth shaders and re-check
   the two naga warns are gone (item 2); a render lane's call, an hour with a
   before/after capture.
3. Items 3 and 4 are each a few lines; do them next time `loader_log` is touched.
