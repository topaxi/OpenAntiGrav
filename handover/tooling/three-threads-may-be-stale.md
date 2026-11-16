---
status: measure
titles: [pulse]
updated: 2026-11-16
summary: "A Pulse gap scout found three threads whose Open looks closed by later commits; each needs a hand check, then deletion or a rewritten Open"
size: S (30)
difficulty: easy (30)
---

# Three threads may be stale

2026-11-15, a read-only Pulse gap scout. Three threads read as finished against the code; the
check was by commit and grep, not by running the game, so each needs a hand pass before it is
deleted.

## Open

- `handover/gameplay/tournament-scoring-is-read-and-the-mode-is-next.md`: the mode is built
  (`crates/race/src/tournament.rs`, `crates/race/src/mode.rs`; commits `10ff0f9d8`, `ba6171471`,
  `0749485ae`, `f9da8419b`). What is left reads as unread `DAT_` addresses only.
- `handover/gameplay/the-lap-count-is-per-speed-class.md`: most of its markers are closed.
- `handover/gameplay/the-original-resets-a-craft-airborne-four-seconds-and-ours-now-does.md`: the
  reset trigger and shield charge are ported; the remaining line is a 05_Track autopilot wall
  grind, which `an-ace-craft-should-lap-every-circuit-without-touching-a-wall.md` also carries.
- Also plausible from a crude marker census, not hand-checked:
  `which-movie-cut-plays-and-what-plays-the`, `the-pulse-plasma-head-and-blast-differ-from-the-original`,
  `pulses-endrace-screens-are-read-and-not-drawn`, `the-pre-race-flyby-lacks-its-panel-and-its-settle`,
  `a-circuits-billboard-slots-are-a-9-entry`.

## Next Steps

1. For each thread: read its Open against the code and docs; move any still-true item into the
   thread that owns it, then delete the file (CLAUDE.md, threads are deleted once the work lands).
2. Delete this thread when the list is empty.
