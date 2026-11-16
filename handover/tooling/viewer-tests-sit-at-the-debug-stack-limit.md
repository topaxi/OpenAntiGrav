---
status: ready-to-wire
titles: [pulse, pure, hd, 2048, omega]
updated: 2026-11-16
summary: "Twelve viewer tests overflowed the 2 MiB test stack and now run on a 16 MiB thread; the 809 KB frame is untouched"
size: S (40)
difficulty: easy (40)
---

# The viewer tests sit at the debug stack limit

2026-11-16, lane `hd-plasma-3`. Twelve `tests::viewer` tests overflowed nextest's 2 MiB test
thread stack under the gate's feature set: about 2.1 MB of frames in total, of which
`Timeline::open_within` alone is 809 KB. They now open on a 16 MiB thread, as `timeline.rs`
already does for its own tests. No production code changed, so a player build still carries the
same frame sizes.

## Open

- What makes `Timeline::open_within`'s frame 809 KB in a debug build (a large array or struct by
  value, most likely), and whether a release build on a small-stack platform (web worker, mobile
  thread) is near its limit too.
- Any other test that grows past 2 MiB will fail the same way, without a hint.

## Next Steps

1. Measure the frame: `cargo +nightly rustc -- -Z emit-stack-sizes` or a `-Zprint-type-sizes`
   pass over the timeline types; find the by-value aggregate.
2. Box or borrow it so the frame drops well under 1 MB, then drop the 16 MiB thread from the
   viewer tests and keep them green.
