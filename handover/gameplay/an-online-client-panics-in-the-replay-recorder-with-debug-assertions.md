---
status: re-first
titles: [pulse]
updated: 2026-11-17
summary: "A debug oag-game online client trips debug_assert_eq! in replay/recording.rs at about tick 30; a release or assertion-free build is unaffected"
---

# An online client panics in the replay recorder when debug assertions are on

2026-11-17, lane `net-remote` (found while measuring the internet test, `docs/networking/online-lobby.md`).

## Reproduction

Debug-profile `oag-game --online --net-bot` against any lobby (a local `oag-server lobby` on 127.0.0.1 with
`--max-ticks 300` reproduces it), two clients, Pulse PSP EU, `--ticks 150`:

```
panicked at crates/raceplay/src/replay/recording.rs:353:9:
assertion `left == right` failed
  left: 12
 right: 13
```

Both seats, about tick 30 in the remote runs (33 vs 68). Not the network path: it fails the same on localhost.
`CARGO_PROFILE_DEV_DEBUG_ASSERTIONS=false` makes the race run to its limit with 0 bad hashes.

## Open

- The assertion is `debug_assert_eq!(recording.base + recording.poses.len() as u64, tick + 1)` at the end of the lap
  recorder. Hypothesis, not read out of the code: an online client that stalls and replays ticks does not hand the
  recorder contiguous ticks. `git log -- crates/raceplay/src/replay/` shows the recorder changed on 2026-11-16.
- Whether `just test-data` online tests with debug assertions on are hit.

## Next Steps

1. Run the reproduction above with `RUST_BACKTRACE=1` and read which caller passes the tick.
2. Decide whether the recorder should skip a tick it already has or the online client should not record rolled-back ticks.
