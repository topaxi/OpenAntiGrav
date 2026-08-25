# The race mix saturates; the per-voice SAS volume would settle whether it should

2026-08-24. Numbers, chain and next step: [audio-levels.md](../docs/ghidra/functions/psp-pulse-usa/audio-levels.md). Two things not on it: `audio/sfx.rs:271` applies the engine law *after* the volume curve, against `spatial.rs`'s contract (fixing it makes the engine louder, so it waits); and `race::capture` never called `Audio::race_tick`, so race captures before this date dumped music alone.

## Open

- `audio/sfx.rs:271` applies the engine law after the volume curve, against `spatial.rs`'s contract; fixing it makes the engine louder, so the fix is on hold.
- Race captures taken before 2026-08-24 dumped music alone, since `race::capture` never called `Audio::race_tick`.

## Next Steps

- See [audio-levels.md](../docs/ghidra/functions/psp-pulse-usa/audio-levels.md) for the recorded next step on whether per-voice SAS volume should settle the mix saturation.
- Fix the `audio/sfx.rs:271` ordering (engine law after volume curve) once the resulting louder engine is acceptable.
