# Each title's own countdown is mostly unlocated

2026-10-19, lane `race-countdown-audio`. The maintainer heard "a single horn" on titles other than Pulse.
Census and evidence: [psp-audio.md](../../docs/formats/psp-audio.md) ("The countdown census, every title")
and [2048's page](../../docs/ghidra/functions/vita-2048-eu-v104/countdown-voice.md). Landed: HD's `ready`
plays `321_GO`'s authored tail instead of a random flat clip.

## Open

- **HD's real countdown is not located.** `speech.bnk` carries `ready` (a `0x08` into `321_GO`), `321_GO`
  (three stages on delays 125, 205, 205 ticks behind a goto that skips them), `go`, `INIT_GO`, and
  `launch.bnk` carries `LAUNCHSEQ` (four beeps on a chain of children) and `LAUNCHBEEP2/3`. None of the
  names is a literal in the executable, so the trigger, the arm of each goto and the timing are unknown.
  Next: an RPCS3 capture with a cue-start log (the `Scream_DoGrain` family, [sound.md](../../docs/ghidra/functions/ps3-hdfury-eu/sound.md)),
  from the first `InGame` tick to 300 ticks past the release.
- **HD's `0x08` handler is unread**; the walk follows Pulse's measured reading.
- **2048's native events** play `321_NGP` then `Go_NGP` (`Weapons_NGP.bnk`), named modes `ready` and `go`.
  Wiring needs opcode `0x04` (LFO) and `0x2e` (switch) read and a way to tell an event from a named
  mode; today 2048 plays nothing at the start. Timing is assumed to be Pulse's.
- **Pure**: lent from Pulse. One PPSSPP run of `scripts/psp-countdown-cues.py` on Pure measures it.
- **Omega**: its banks do not parse as `SBlk`; the start is unchecked.

## Next Steps

1. Measure Pure with the existing PPSSPP script (about 1 hour).
2. RPCS3 cue-start capture on HD (half a day).
3. Read `0x04` and `0x2e` on 2048's `Scream` handlers, then wire `321_NGP`/`Go_NGP` (half a day).
