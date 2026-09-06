# The race mix saturates; the per-voice SAS volume would settle whether it should

2026-08-24. Numbers, chain and next step: [audio-levels.md](../docs/ghidra/functions/psp-pulse-usa/audio-levels.md). Two things not on it: `audio/sfx.rs:271` applies the engine law *after* the volume curve, against `spatial.rs`'s contract (fixing it makes the engine louder, so it waits); and `race::capture` never called `Audio::race_tick`, so race captures before this date dumped music alone.

**2026-09-06**: both items landed or narrowed, and the per-voice SAS volume
question is now closed - live-measured, see below, in the same day's second
pass over this thread.

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
- **The per-voice SAS volume is recovered, live, and it does not explain the
  saturation.** A running `pulse-psp-usa` under PPSSPP (see
  [`ppsspp-debugger.md`](../docs/reverse-engineering/ppsspp-debugger.md)),
  driven into a full-grid Single Race and broken on `Sas_CommitVoices` for
  590 hits across three captures, shows the four fields
  (`+0x40`/`+0x44`/`+0x48`/`+0x4c`) varying per voice, panned L-vs-R, ramping
  in smoothly on a new sound rather than jumping - filtered to voice/tick
  pairs where the dirty flag actually fired, since a leftover stale slot
  reads as a nonzero value forever and is not a driven engine. **The
  decisive check needs no assumed ceiling**: summed across every
  simultaneously-committing voice, dry L reaches two to six times what the
  loudest single voice among them carries, sustained through a capture
  rather than only at the first tick - enough on its own to overload a mix
  with no reserved headroom. That is consistent with several
  honestly-scaled voices summing past full scale, which `audio-levels.md`
  had already established from the DAC/group-volume side; it is not
  consistent with any one voice being set wrong. See
  [audio-levels.md](../docs/ghidra/functions/psp-pulse-usa/audio-levels.md#the-per-voice-sas-volume-measured-live-2026-09-06)
  for the full capture data and the sum/loudest table, and
  [sound.md](../docs/ghidra/functions/psp-pulse-usa/sound.md#corroboration-2026-09-06-sas_setvolume-confirmed-live-and-the-value-it-carries-measured)
  for the live confirmation that this wrapper (now named `Sas_SetVolume`)
  really does end in `jal zz___sceSasSetVolume`, and that `g_sas_voices`'s
  base address is a directly-read immediate (`li s1,0x08B893D0`) rather
  than an inference from loop shape. Confidence 90: runtime trace, three
  agreeing captures, not corroborated against `psp-pulse-eu`. **No
  attenuation is ported** - the recovered values are not maxed or clamped,
  so there is nothing here to trim, and the mix is expected to saturate the
  same way the console's does.

## Open

- **Three `Sas_CommitVoices` wrapper functions are still unnamed** -
  `FUN_08a2ae6c` (`__sceSasSetPitch`), `FUN_08a2af18` (`__sceSasSetNoise`)
  and `FUN_08a2b03c` (`__sceSasSetSL`), all resolved with confidence 90 -
  though the fourth, `FUN_08a2ae08` (`__sceSasSetVolume`), now is: named
  `Sas_SetVolume` as of 2026-09-06, confidence 92, following the
  `Sas_SetVoice` precedent for this exact "guarded call to a named import"
  shape. Not applied to the Ghidra project itself, since its function
  boundary is broken at that address (merged into an unrelated neighbour) -
  the `names.tsv` row is in regardless, per project convention, and the
  database can catch up whenever someone next has the project open for a
  reason that already touches this region.

## Next Steps

- Decide and apply a naming convention for the remaining three wrapper
  functions, then add their `names.tsv` rows. Optional and separate from the
  volume question: tracing `SoundInstance_UpdateSpatial`'s SCREAM-instance
  write forward (or the SAS voice record's `+0x40` backward) until they meet
  would name the exact function that ramps a fading-in voice, which the
  2026-09-06 live capture saw happen but did not localise to an address -
  worth doing for the name, not because the saturation question is still
  open.
