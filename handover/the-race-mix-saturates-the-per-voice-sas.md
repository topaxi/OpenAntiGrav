# The race mix saturates; the per-voice SAS volume would settle whether it should

2026-08-24. Numbers, chain and next step: [audio-levels.md](../docs/ghidra/functions/psp-pulse-usa/audio-levels.md). Two things not on it: `audio/sfx.rs:271` applies the engine law *after* the volume curve, against `spatial.rs`'s contract (fixing it makes the engine louder, so it waits); and `race::capture` never called `Audio::race_tick`, so race captures before this date dumped music alone.

**2026-09-06**: both items landed or narrowed. The ordering fix is in
(`crates/game/src/audio/sfx/engine.rs`, not `crates/audio` - the thread's own
path was one crate off); `race::capture` already called `Audio::race_tick`
as of `d55b0ffc` (2026-08-25), one commit after this thread opened, so that
half was stale. The per-voice SAS volume question is genuinely narrower now,
not closed - see below.

## Resolved

- **The ordering fix landed.** `Engine::tick` (`crates/game/src/audio/sfx/engine.rs`)
  now builds the engine law (`intensity * 0.6 + 0.4`, `x 0.85` for
  non-player craft) into the `volume` argument `oag_audio::Emitter::place`
  takes, instead of multiplying it onto the already-curved gain afterwards -
  matching `spatial.rs`'s own documented contract (`place`'s doc comment
  already named this exact law as the example). `curve(atten) * law !=
  curve(atten * law)`: the old order was measurably wrong, not just
  differently-ordered.
- **Measured, not assumed: the fix does not settle the saturation, and makes
  it slightly worse.** One 30-second `--race --autopilot --ticks 1800`
  capture of `pulse-psp-eu.chd`, single race, all volumes pinned to 100:
  before the ordering fix, 76,064 of 2,880,000 samples clipped (2.641 %);
  after, 80,164 (2.783 %). Peak is 0 dBFS either way (the mix already sits at
  full scale before the fix). This confirms the thread's own note that
  "fixing it makes the engine louder" was correct, and settles that the
  ordering bug and the saturation are two separate findings, not the same one
  in disguise.
- **`race::capture` already calls `Audio::race_tick`** (`crates/game/src/race/capture.rs:856`,
  landed in `d55b0ffc`, 2026-08-25). Nothing to fix there; the bullet was
  stale from before that commit.

## Open

- **What value lands in the SAS voice record's `+0x40`/`+0x44`/`+0x48`/`+0x4c`
  is still not recovered**, and per-voice SAS volume is still the only
  candidate lever for the saturation that has not been ruled out.
  [sound.md's 2026-09-06 correction](../docs/ghidra/functions/psp-pulse-usa/sound.md#correction-2026-09-06-the-guess-was-backwards-and-0x40-is-the-per-voice-volume)
  found that `Sas_CommitVoices`'s bit `0x0f` dispatch **does** call
  `__sceSasSetVolume` a second time, per voice, per frame - the thing
  `audio-levels.md` had read as called only once, from `Sas_Init` - but
  nothing read yet traces those four fields back to a writer.
  `SoundInstance_UpdateSpatial` is the only known producer of a comparable
  volume pair and it writes into a SCREAM software instance's own fade
  engine, not into the SAS voice record; the bridge between the two is
  unread. Until it is, the honest port is the one that reproduces the
  recovered law and saturates where the hardware saturates - no attenuation
  has been added, because none has been measured, and CLAUDE.md's rule
  against inventing what the assets already author applies to a volume
  number exactly as it does to a particle effect.
- **Four `Sas_CommitVoices` wrapper functions are still unnamed**, though
  their call targets are now resolved with confidence 90 each
  (`FUN_08a2ae08` -> `__sceSasSetVolume`, `FUN_08a2ae6c` -> `__sceSasSetPitch`,
  `FUN_08a2af18` -> `__sceSasSetNoise`, `FUN_08a2b03c` -> `__sceSasSetSL`).
  Not renamed: this project has no established naming convention yet for "a
  thin guarded wrapper around a named import" beyond `Sas_SetVoice`'s own
  one-off shape, and inventing one wasn't this thread's job.

## Next Steps

- Trace `SoundInstance_UpdateSpatial`'s SCREAM-instance write forward (or the
  SAS voice record's `+0x40` backward) until they meet, the way `sound.md`
  walked `Sas_CommitVoices` back to `Sas_SetVoice`. That is what would turn
  "no attenuation has been measured" into an actual number, one way or the
  other.
- Decide and apply a naming convention for the four resolved wrapper
  functions, then add their `names.tsv` rows.
