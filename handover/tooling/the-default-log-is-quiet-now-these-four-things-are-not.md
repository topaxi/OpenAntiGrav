---
status: measure
titles: [all]
updated: 2026-11-02
summary: "A partial absence (HD track.vex: 70 of 126 nodes drawn) logs at debug; decide whether loader_log should warn on it"
size: XS (45)
difficulty: easy (40)
---

# The default log is quiet now; one question about it is unsettled

2026-10-02. A launch to a race used to print 260-440 lines at the default
filter (`oag=debug`); it now prints 6-19 of our own and about 19 from the
graphics stack, headless and debug build. The rule, the `loader_log` helper and
the `RUST_LOG` recipes are in
[`docs/architecture/logging.md`](../../docs/architecture/logging.md).

2026-10-06: the ghost's `@invariant`/`Equal` warn and the doubled track-audio
report are fixed.

2026-11-02: the `frame: N ms` stutter warn is rate-limited at its one call site
(`crates/game/src/main/session/stutter.rs`: the first slow frame at once, then
one summary per 5 s window while frames stay slow), and a DLC pack that fails to
open is a `warn` on every path (`oag_raceplay::loader_log::dlc_problem`, phrase
`(not mounted)` in `ABSENCE`).

## Open

1. **A partial absence is `debug`.** HD's `track.vex: 70 of 126 mesh node(s)
   drawn ... 56 addressed no chunk` logs at `debug`, because only
   `(0 triangle(s))` is treated as an absence. Whether "N of M drawn" with
   N < M should be `warn` is undecided: it would also escalate every
   legitimately partial model.

The four Vulkan-loader `ERROR` records on a machine with the `lsfg-vk` layer
installed are the loader's, not ours; nothing to do here.

## Next Steps

1. Census the `N of M mesh node(s) drawn` lines on every title's race loads
   (`--screenshot` with `RUST_LOG=oag=debug`), then decide; 30 minutes.
